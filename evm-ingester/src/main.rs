use anyhow::{Context, Result};
use dex_ingester_common::{
    Event, EventProcessor, FinalityBuffer, FinalityMode, Publisher, PublisherConfig,
};
use ethers::prelude::*;
use ethers::providers::Ws;
use evm_ingester::bridge_listener::BridgeListener;
use evm_ingester::config::{Bridge, Config};
use evm_ingester::rpc_retry::RetryableProvider;
use futures_util::StreamExt;
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
    info!("Monitoring {} bridges", config.bridges.len());

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let mut tasks = JoinSet::new();

    // Spawn all bridges (parallel connection initialization)
    for bridge_config in &config.bridges {
        if let Err(e) = spawn_bridge(bridge_config, &config, &shutdown_tx, &mut tasks).await {
            error!(bridge = %bridge_config.name, "Skipping bridge: {:#}", e);
        }
    }

    info!("All bridge listeners started, waiting for shutdown signal");

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

async fn spawn_bridge(
    bridge: &Bridge,
    config: &Config,
    shutdown: &broadcast::Sender<()>,
    tasks: &mut JoinSet<()>,
) -> Result<()> {
    info!(
        bridge = %bridge.name,
        protocol = %bridge.protocol.as_str(),
        address = ?bridge.address,
        chain = %bridge.chain_id,
        "Initializing bridge listener"
    );

    let provider = Arc::new(
        Provider::<Ws>::connect(bridge.rpc.expose())
            .await
            .context("failed to connect to WebSocket RPC")?,
    );

    let rpc = Arc::new(RetryableProvider::new(
        Arc::clone(&provider),
        bridge.retry.clone(),
    ));

    let publisher = Arc::new(
        Publisher::new(PublisherConfig::new(
            &config.shared,
            format!("evm-ingester/{}", bridge.chain_id),
        ))
        .context("failed to create publisher")?,
    );

    publisher
        .wait_until_ready()
        .await
        .context("Dapr sidecar never became healthy")?;

    let listener = BridgeListener::new(
        bridge.chain_id.clone(),
        bridge.chain_name.clone(),
        format!("{:?}", bridge.address),
        bridge.protocol,
        Arc::clone(&rpc),
    );

    let processor = Arc::new(EventProcessor::new(
        Arc::new(FinalityBuffer::new(FinalityMode::Depth(
            bridge.confirmations,
        ))),
        Arc::clone(&publisher),
    ));

    let (event_tx, event_rx) = mpsc::channel::<Event>(EVENT_CHANNEL_BUFFER);

    let bridge_address = format!("{:?}", bridge.address);
    let bridge_name = bridge.name.clone();
    let subscription_shutdown = shutdown.subscribe();
    tasks.spawn(async move {
        if let Err(e) = subscribe_to_logs(
            provider,
            &bridge_address,
            listener,
            event_tx,
            subscription_shutdown,
        )
        .await
        {
            error!(bridge = %bridge_address, "WebSocket subscription failed: {:#}", e);
        }
    });

    let processor_shutdown = shutdown.subscribe();
    tasks.spawn(async move {
        process_events(event_rx, processor, bridge_name, processor_shutdown).await;
    });

    Ok(())
}

async fn subscribe_to_logs<P>(
    provider: Arc<Provider<Ws>>,
    bridge_address: &str,
    listener: BridgeListener<P>,
    event_tx: mpsc::Sender<Event>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()>
where
    P: Middleware + 'static,
{
    let address: Address = bridge_address
        .parse()
        .with_context(|| format!("invalid bridge address: {}", bridge_address))?;

    let mut stream = provider
        .subscribe_logs(&Filter::new().address(address))
        .await
        .context("failed to subscribe to logs")?;

    info!(bridge = %bridge_address, "WebSocket subscription established");

    loop {
        tokio::select! {
            log_result = stream.next() => {
                let Some(log) = log_result else {
                    warn!(bridge = %bridge_address, "WebSocket stream ended");
                    break;
                };

                if log.removed == Some(true) {
                    warn!(
                        bridge = %bridge_address,
                        tx_hash = ?log.transaction_hash,
                        log_index = ?log.log_index,
                        "Log removed by chain reorg, dropping"
                    );
                    continue;
                }

                match listener.process_log(log).await {
                    Ok(Some(event)) => {
                        if event_tx.send(event).await.is_err() {
                            warn!(bridge = %bridge_address, "Event processor gone, stopping subscription");
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            }
            _ = shutdown_requested(&mut shutdown) => {
                info!(bridge = %bridge_address, "WebSocket subscription shutting down");
                break;
            }
        }
    }

    Ok(())
}

async fn process_events(
    mut events: mpsc::Receiver<Event>,
    processor: Arc<EventProcessor>,
    bridge_name: String,
    mut shutdown: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            maybe_event = events.recv() => {
                let Some(event) = maybe_event else {
                    warn!(bridge = %bridge_name, "Event channel closed");
                    break;
                };

                processor.process(event, None, &bridge_name).await;
            }
            _ = shutdown_requested(&mut shutdown) => {
                info!(bridge = %bridge_name, "Event processor shutting down");
                break;
            }
        }
    }

    let published = processor.flush(&bridge_name).await;
    info!(bridge = %bridge_name, published, "Flushed pending events");
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
