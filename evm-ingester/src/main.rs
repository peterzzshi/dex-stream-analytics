use anyhow::{Context, Result};
use dex_ingester_common::{Event, FinalityBuffer, FinalityMode};
use ethers::prelude::*;
use ethers::providers::Ws;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod avro;
mod config;
mod decoder;
mod dlq;
mod events;
mod listener;
mod oracle;
mod publisher;
mod rpc_retry;

use config::Config;
use dlq::FileSink;
use listener::Listener;
use oracle::ChainlinkOracle;
use publisher::DaprPublisher;
use rpc_retry::RetryableProvider;

const EVENT_CHANNEL_BUFFER: usize = 1000;
const DLQ_CHANNEL_BUFFER: usize = 100;
const SHUTDOWN_GRACE_PERIOD_SECS: u64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
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

    let mut dlq_sink =
        FileSink::new(Some("./dlq".to_string())).context("failed to create DLQ sink")?;
    dlq_sink.start_flusher();

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let (dlq_tx, mut dlq_rx) = mpsc::channel::<Event>(DLQ_CHANNEL_BUFFER);

    let mut tasks = JoinSet::new();

    let dlq_sink_clone = Arc::new(tokio::sync::Mutex::new(dlq_sink));
    let dlq_sink_handle = Arc::clone(&dlq_sink_clone);
    let mut shutdown_rx_dlq = shutdown_tx.subscribe();
    tasks.spawn(async move {
        loop {
            tokio::select! {
                Some(event) = dlq_rx.recv() => {
                    if let Err(e) = dlq_sink_handle.lock().await.write(&event) {
                        error!("Failed to write to DLQ: {}", e);
                    }
                }
                _ = shutdown_rx_dlq.recv() => {
                    info!("DLQ consumer shutting down");
                    break;
                }
            }
        }
    });

    for chain_config in config.chains.iter() {
        info!(
            chain = %chain_config.chain,
            name = %chain_config.name,
            pools = chain_config.pools.len(),
            "Initializing chain"
        );

        let provider = match Provider::<Ws>::connect(&chain_config.rpc).await {
            Ok(provider) => Arc::new(provider),
            Err(e) => {
                error!(
                    "Failed to connect to WebSocket RPC {}: {}",
                    chain_config.rpc, e
                );
                continue;
            }
        };

        let rpc = Arc::new(RetryableProvider::new(
            Arc::clone(&provider),
            chain_config.retry.clone(),
        ));

        let oracle = Arc::new(ChainlinkOracle::new(Arc::clone(&provider)));

        let publisher = Arc::new(
            DaprPublisher::new(&config, &chain_config.chain)
                .context("failed to create Dapr publisher")?,
        );

        for pool_config in chain_config.pools.iter() {
            info!(
                chain = %chain_config.chain,
                pool = %pool_config.name,
                pair = %pool_config.pair_address,
                "Starting pool listener"
            );

            let buffer = Arc::new(FinalityBuffer::new(FinalityMode::Depth(
                chain_config.confirmations,
            )));

            let listener = Listener::new(
                chain_config.chain.clone(),
                chain_config.name.clone(),
                pool_config.clone(),
                Arc::clone(&oracle),
                Arc::clone(&rpc),
            );

            let (event_tx, mut event_rx) = mpsc::channel::<Event>(EVENT_CHANNEL_BUFFER);

            let provider_clone = Arc::clone(&provider);
            let pool_address = pool_config.pair_address.clone();
            let shutdown_rx_ws = shutdown_tx.subscribe();
            tasks.spawn(async move {
                if let Err(e) = subscribe_to_logs(
                    provider_clone,
                    &pool_address,
                    listener,
                    event_tx,
                    shutdown_rx_ws,
                )
                .await
                {
                    error!(pool = %pool_address, "WebSocket subscription failed: {}", e);
                }
            });

            let buffer_clone = Arc::clone(&buffer);
            let publisher_clone = Arc::clone(&publisher);
            let dlq_tx_clone = dlq_tx.clone();
            let pool_name = pool_config.name.clone();
            let mut shutdown_rx_proc = shutdown_tx.subscribe();

            tasks.spawn(async move {
                loop {
                    tokio::select! {
                        Some(event) = event_rx.recv() => {
                            let result = buffer_clone.add(event, None);

                            for evicted in result.evicted {
                                if let Err(e) = dlq_tx_clone.send(evicted).await {
                                    error!(pool = %pool_name, "Failed to route evicted event to DLQ: {}", e);
                                }
                            }

                            for confirmed in result.confirmed {
                                publisher_clone.publish(&confirmed).await;
                            }
                        }
                        _ = shutdown_rx_proc.recv() => {
                            info!(pool = %pool_name, "Event processor shutting down, flushing buffer");
                            let pending = buffer_clone.flush();
                            info!(pool = %pool_name, pending = pending.len(), "Flushing pending events");
                            for event in pending {
                                publisher_clone.publish(&event).await;
                            }
                            break;
                        }
                    }
                }
            });
        }
    }

    info!("All pool listeners started, waiting for shutdown signal");

    signal::ctrl_c()
        .await
        .context("failed to listen for ctrl+c")?;
    info!("Shutdown signal received");

    let _ = shutdown_tx.send(());

    let shutdown_timeout = tokio::time::Duration::from_secs(SHUTDOWN_GRACE_PERIOD_SECS);
    tokio::time::timeout(shutdown_timeout, async {
        while tasks.join_next().await.is_some() {}
    })
    .await
    .unwrap_or_else(|_| {
        warn!("Shutdown timeout exceeded, forcing exit");
    });

    if let Ok(sink) = Arc::try_unwrap(dlq_sink_clone) {
        sink.into_inner().close().await?;
    }

    Ok(())
}

async fn subscribe_to_logs(
    provider: Arc<Provider<Ws>>,
    pool_address: &str,
    listener: Listener<ChainlinkOracle<Provider<Ws>>, Provider<Ws>>,
    event_tx: mpsc::Sender<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    use ethers::types::Filter;
    use futures_util::StreamExt;

    let address: ethers::types::Address = pool_address
        .parse()
        .with_context(|| format!("invalid pool address: {}", pool_address))?;

    let filter = Filter::new().address(address);

    let mut stream = provider
        .subscribe_logs(&filter)
        .await
        .context("failed to subscribe to logs")?;

    info!(pool = %pool_address, "WebSocket subscription established");

    loop {
        tokio::select! {
            log_result = stream.next() => {
                match log_result {
                    Some(log) => {
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
                            Ok(event) => {
                                if let Err(e) = event_tx.send(event).await {
                                    error!(pool = %pool_address, "Failed to send event: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(pool = %pool_address, "Failed to process log: {}", e);
                            }
                        }
                    }
                    None => {
                        warn!(pool = %pool_address, "WebSocket stream ended");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!(pool = %pool_address, "WebSocket subscription shutting down");
                break;
            }
        }
    }

    Ok(())
}
