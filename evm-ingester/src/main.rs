use anyhow::{Context, Result};
use dex_ingester_common::{
    Event, EventProcessor, FinalityBuffer, FinalityMode, PoolStats, Publisher, PublisherConfig,
    SharedConfig,
};
use ethers::prelude::*;
use ethers::providers::Ws;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Duration, MissedTickBehavior};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod decoder;
mod file_config;
mod listener;
mod oracle;
mod price;
mod rpc_retry;

use config::{Chain, Config, Pool};
use listener::Listener;
use oracle::ChainlinkOracle;
use rpc_retry::RetryableProvider;

const EVENT_CHANNEL_BUFFER: usize = 1000;
const STATS_INTERVAL_SECS: u64 = 30;
const SHUTDOWN_GRACE_PERIOD_SECS: u64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = match std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase()
        .as_str()
    {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("failed to set global subscriber")?;

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
        provider,
        publisher: Arc::new(
            Publisher::new(PublisherConfig::new(
                shared,
                format!("evm-ingester/{}", chain_config.chain),
            ))
            .context("failed to create publisher")?,
        ),
    };

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
                    Err(e) => {
                        warn!(pool = %pool_address, "Failed to process log: {:#}", e);
                    }
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

async fn process_events(
    mut events: mpsc::Receiver<Event>,
    processor: Arc<EventProcessor>,
    pool_name: String,
    mut shutdown: broadcast::Receiver<()>,
) {
    let mut stats = PoolStats::default();
    // One long-lived ticker: a sleep recreated inside `select!` restarts on every event,
    // so the report would never fire on a busy pool.
    let mut ticker = tokio::time::interval(Duration::from_secs(STATS_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;

    loop {
        tokio::select! {
            maybe_event = events.recv() => {
                let Some(event) = maybe_event else {
                    warn!(pool = %pool_name, "Event channel closed");
                    break;
                };

                stats.received += 1;

                let outcome = processor.process(event, None, &pool_name).await;

                stats.processed += 1;
                stats.add_published(outcome.published);
                stats.add_dead_lettered(outcome.dead_lettered);
            }
            _ = ticker.tick() => report_stats(&pool_name, &mut stats),
            _ = shutdown_requested(&mut shutdown) => {
                info!(pool = %pool_name, "Event processor shutting down");
                break;
            }
        }
    }

    let published = processor.flush(&pool_name).await;
    info!(pool = %pool_name, published, "Flushed pending events");
}

fn report_stats(pool_name: &str, stats: &mut PoolStats) {
    let snapshot = stats.take();
    info!(
        pool = %pool_name,
        received = snapshot.received,
        processed = snapshot.processed,
        published = snapshot.published,
        dead_lettered = snapshot.dead_lettered,
        "Pool statistics (last {}s)",
        STATS_INTERVAL_SECS
    );
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
