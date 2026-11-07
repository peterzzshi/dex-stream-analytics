use anyhow::{Context, Result};
use dex_ingester_common::{
    Event, EventProcessor, FinalityBuffer, FinalityMode, Publisher, PublisherConfig, SharedConfig,
};
use ethers::prelude::*;
use ethers::providers::Ws;
use evm_ingester::config::{Chain, Config, Pool};
use evm_ingester::listener::Listener;
use evm_ingester::oracle::ChainlinkOracle;
use evm_ingester::rpc_retry::RetryableProvider;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;
use tokio::time::Duration;
use tracing::{error, info, warn};

const EVENT_CHANNEL_BUFFER: usize = 1000;
const SHUTDOWN_GRACE_PERIOD_SECS: u64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("evm_ingester=info,dex_ingester_common=info")
            }),
        )
        .with_target(false)
        .init();

    info!("EVM Ingester starting...");

    if let Err(e) = run().await {
        error!("Fatal error: {:#}", e);
        std::process::exit(1);
    }

    info!("EVM Ingester shutdown complete");
    Ok(())
}

async fn run() -> Result<()> {
    let config = Config::load_from_env()?;
    info!("Configuration loaded successfully");
    info!("Monitoring {} chains", config.chains.len());

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let mut tasks = JoinSet::new();

    for chain_config in &config.chains {
        if let Err(e) = spawn_chain(chain_config, &config.shared, &shutdown_tx, &mut tasks).await {
            // One unreachable chain should not take the others down with it.
            error!(chain = %chain_config.chain, "Skipping chain: {:#}", e);
        }
    }

    info!("All pool listeners started, waiting for shutdown signal");

    signal::ctrl_c()
        .await
        .context("failed to listen for ctrl+c")?;
    info!("Shutdown signal received");

    let _ = shutdown_tx.send(());

    let grace = Duration::from_secs(SHUTDOWN_GRACE_PERIOD_SECS);
    if tokio::time::timeout(grace, async { while tasks.join_next().await.is_some() {} })
        .await
        .is_err()
    {
        warn!("Shutdown timeout exceeded, forcing exit");
    }

    Ok(())
}

/// Per-chain resources every pool on that chain shares.
struct ChainRuntime {
    provider: Arc<Provider<Ws>>,
    rpc: Arc<RetryableProvider<Provider<Ws>>>,
    oracle: Arc<ChainlinkOracle<Provider<Ws>>>,
    publisher: Arc<Publisher>,
}

async fn spawn_chain(
    chain_config: &Chain,
    shared: &SharedConfig,
    shutdown: &broadcast::Sender<()>,
    tasks: &mut JoinSet<()>,
) -> Result<()> {
    info!(
        chain = %chain_config.chain,
        name = %chain_config.name,
        pools = chain_config.pools.len(),
        "Initializing chain"
    );

    let provider = Arc::new(
        Provider::<Ws>::connect(chain_config.rpc.expose())
            .await
            .context("failed to connect to WebSocket RPC")?,
    );

    let runtime = ChainRuntime {
        rpc: Arc::new(RetryableProvider::new(
            Arc::clone(&provider),
            chain_config.retry.clone(),
        )),
        oracle: Arc::new(ChainlinkOracle::new(
            Arc::clone(&provider),
            chain_config.oracle.clone(),
        )),
        publisher: Arc::new(
            Publisher::new(PublisherConfig::new(
                shared,
                format!("evm-ingester/{}", chain_config.chain),
            ))
            .context("failed to create publisher")?,
        ),
        provider,
    };

    // Publishing into a sidecar that is not listening yet dead-letters the event, and the
    // per-publish retry budget is shorter than a cold daprd start.
    runtime
        .publisher
        .wait_until_ready()
        .await
        .context("Dapr sidecar never became healthy")?;

    for pool_config in &chain_config.pools {
        if let Err(e) = spawn_pool(chain_config, &runtime, pool_config, shutdown, tasks).await {
            error!(
                chain = %chain_config.chain,
                pool = %pool_config.name,
                "Skipping pool: {:#}",
                e
            );
        }
    }

    Ok(())
}

async fn spawn_pool(
    chain_config: &Chain,
    runtime: &ChainRuntime,
    pool_config: &Pool,
    shutdown: &broadcast::Sender<()>,
    tasks: &mut JoinSet<()>,
) -> Result<()> {
    info!(
        chain = %chain_config.chain,
        pool = %pool_config.name,
        pair = %pool_config.pair_address,
        "Starting pool listener"
    );

    let listener = Listener::new(
        chain_config.chain.clone(),
        chain_config.name.clone(),
        pool_config.clone(),
        Arc::clone(&runtime.oracle),
        Arc::clone(&runtime.rpc),
    )
    .await
    .with_context(|| {
        format!(
            "failed to initialise listener for pool {}",
            pool_config.name
        )
    })?;

    // Depth mode: the buffer holds each event until its own block is `confirmations` deep,
    // so there is no external finality source to poll.
    let processor = Arc::new(EventProcessor::new(
        Arc::new(FinalityBuffer::new(FinalityMode::Depth(
            chain_config.confirmations,
        ))),
        Arc::clone(&runtime.publisher),
    ));

    let (event_tx, event_rx) = mpsc::channel::<Event>(EVENT_CHANNEL_BUFFER);

    let provider = Arc::clone(&runtime.provider);
    let pool_address = pool_config.pair_address.clone();
    let subscription_shutdown = shutdown.subscribe();
    tasks.spawn(async move {
        if let Err(e) = subscribe_to_logs(
            provider,
            &pool_address,
            listener,
            event_tx,
            subscription_shutdown,
        )
        .await
        {
            error!(pool = %pool_address, "WebSocket subscription failed: {:#}", e);
        }
    });

    let pool_name = pool_config.name.clone();
    let processor_shutdown = shutdown.subscribe();
    tasks.spawn(async move {
        process_events(event_rx, processor, pool_name, processor_shutdown).await;
    });

    Ok(())
}

async fn subscribe_to_logs(
    provider: Arc<Provider<Ws>>,
    pool_address: &str,
    listener: Listener<ChainlinkOracle<Provider<Ws>>, Provider<Ws>>,
    event_tx: mpsc::Sender<Event>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    use ethers::types::Filter;
    use futures_util::StreamExt;

    let address: Address = pool_address
        .parse()
        .with_context(|| format!("invalid pool address: {}", pool_address))?;

    let mut stream = provider
        .subscribe_logs(&Filter::new().address(address))
        .await
        .context("failed to subscribe to logs")?;

    info!(pool = %pool_address, "WebSocket subscription established");

    loop {
        tokio::select! {
            log_result = stream.next() => {
                let Some(log) = log_result else {
                    warn!(pool = %pool_address, "WebSocket stream ended");
                    break;
                };

                if log.removed == Some(true) {
                    warn!(
                        pool = %pool_address,
                        tx_hash = ?log.transaction_hash,
                        log_index = ?log.log_index,
                        "Log removed by chain reorg, dropping"
                    );
                    continue;
                }

                match listener.process_log(log).await {
                    Ok(Some(event)) => {
                        if event_tx.send(event).await.is_err() {
                            warn!(pool = %pool_address, "Event processor gone, stopping subscription");
                            break;
                        }
                    }
                    // Sync, Transfer, and unknown topics: normal log stream noise.
                    Ok(None) => {}
                    // Already logged with full context (topics, data, tx) inside process_log.
                    Err(_) => {}
                }
            }
            _ = shutdown_requested(&mut shutdown) => {
                info!(pool = %pool_address, "WebSocket subscription shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Drain and publish. Flow accounting belongs to the analytics service, so the only
/// per-event signal here is the one the processor emits as each event is published.
async fn process_events(
    mut events: mpsc::Receiver<Event>,
    processor: Arc<EventProcessor>,
    pool_name: String,
    mut shutdown: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            maybe_event = events.recv() => {
                let Some(event) = maybe_event else {
                    warn!(pool = %pool_name, "Event channel closed");
                    break;
                };

                processor.process(event, None, &pool_name).await;
            }
            _ = shutdown_requested(&mut shutdown) => {
                info!(pool = %pool_name, "Event processor shutting down");
                break;
            }
        }
    }

    let published = processor.flush(&pool_name).await;
    info!(pool = %pool_name, published, "Flushed pending events");
}

/// Resolves on shutdown, or when every sender has been dropped. A lagged receiver only
/// means the signal was already consumed, not that it should be ignored.
async fn shutdown_requested(shutdown: &mut broadcast::Receiver<()>) {
    loop {
        match shutdown.recv().await {
            Ok(()) | Err(broadcast::error::RecvError::Closed) => return,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(skipped, "Shutdown channel lagged");
            }
        }
    }
}
