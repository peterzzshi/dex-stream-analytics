mod balances;
mod blockchain;
mod config;
mod decoder;
mod file_config;
mod listener;
mod oracle;
mod rpc_retry;
mod symbols;

use anyhow::{Context, Result};
use dex_ingester_common::{
    BurnEvent, Event, FinalityBuffer, FinalityMode, MintEvent, Publisher, PublisherConfig,
    RetryConfig, SidecarUnreachable, SwapEvent,
};
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};

use crate::blockchain::RpcClient;
use crate::config::Config;
use crate::listener::Listener;
use crate::oracle::PythOracle;
use crate::rpc_retry::RetryableRpcClient;
use crate::symbols::SymbolCache;

const EVENT_SOURCE: &str = "ingester/solana/raydium-amm-v4";
const RECONNECT_DELAY_SECS: u64 = 5;
const STATS_INTERVAL_SECS: u64 = 60;

fn exit_sidecar_unreachable(error: SidecarUnreachable) -> ! {
    error!("Fatal: {}", error);
    std::process::exit(1)
}

#[derive(Clone)]
struct SharedResources {
    rpc_client: Arc<RetryableRpcClient>,
    oracle: Arc<PythOracle>,
    symbols: Arc<SymbolCache>,
    finality_buffer: Arc<FinalityBuffer>,
    publisher: Arc<Publisher>,
    config: Arc<Config>,
    shutdown: tokio::sync::watch::Receiver<bool>,
}

/// Resolves when shutdown is requested, or when the sender is dropped.
async fn shutdown_requested(shutdown: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *shutdown.borrow() {
            return;
        }
        if shutdown.changed().await.is_err() {
            return;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Solana ingester");

    // Load configuration from environment variables
    let config = Config::load_from_env().context("Failed to load configuration")?;

    info!(
        "Loaded configuration for chain: {}",
        config.solana.chain_name
    );

    info!("Configured pools:");
    for pool in &config.pools {
        info!("  - {} at {}", pool.name, pool.address);
    }

    // Wrap config in Arc for sharing across tasks
    let config = Arc::new(config);

    // Start health check server
    let app_port = config.solana.app_port;
    tokio::spawn(async move {
        if let Err(e) = run_health_server(app_port).await {
            error!("Health server failed: {}", e);
        }
    });

    // Initialize RPC client
    let rpc_client = Arc::new(RpcClient::new(
        config.solana.rpc.expose(),
        config.solana.commitment.clone(),
    )?);
    info!(
        "Connected to Solana RPC with commitment: {:?}",
        config.solana.commitment
    );

    // Wrap with retry logic
    let retryable_rpc_client = Arc::new(RetryableRpcClient::new(
        rpc_client.clone(),
        RetryConfig::default(),
    ));

    // Initialize oracle and symbol cache (they use HTTP RPC URL directly)
    let http_rpc_url = config
        .solana
        .rpc
        .expose()
        .replace("wss://", "https://")
        .replace("ws://", "http://");
    let _oracle = Arc::new(PythOracle::new(http_rpc_url.clone(), config.pyth.clone()));
    let _symbols = Arc::new(SymbolCache::new(
        http_rpc_url,
        config.tokens.clone(),
        config.programs.token_metadata.clone(),
    ));

    // Initialize finality buffer
    let finality_buffer = Arc::new(
        FinalityBuffer::new(FinalityMode::External)
            .with_event_limit(config.solana.finality_buffer_size),
    );

    // Initialize publisher
    let publisher = Arc::new(Publisher::new(PublisherConfig::new(
        &config.shared,
        EVENT_SOURCE.to_string(),
    ))?);

    // Shutdown is broadcast so every task stops touching the buffer before the final flush.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start periodic finality polling task
    let finality_buffer_clone = finality_buffer.clone();
    let publisher_clone = publisher.clone();
    let rpc_client_clone = retryable_rpc_client.clone();
    let poll_interval_secs = config.solana.finality_poll_interval_secs;
    let poller_shutdown = shutdown_rx.clone();

    let poller_task = tokio::spawn(async move {
        finality_polling_task(
            finality_buffer_clone,
            publisher_clone,
            rpc_client_clone,
            poll_interval_secs,
            poller_shutdown,
        )
        .await;
    });

    info!(
        "Started finality polling task (interval: {}s)",
        poll_interval_secs
    );

    // Create shared resources
    let shared = SharedResources {
        rpc_client: retryable_rpc_client.clone(),
        oracle: _oracle,
        symbols: _symbols,
        finality_buffer,
        publisher,
        config: config.clone(),
        shutdown: shutdown_rx.clone(),
    };

    // Parse Raydium program ID
    let program_id =
        Pubkey::from_str(&config.programs.raydium_amm_v4).context("Invalid Raydium program ID")?;

    info!("All components initialized successfully");

    // Spawn monitoring task for each pool
    let mut pool_tasks = Vec::new();
    for pool in config.pools.iter() {
        let listener = Listener::new(
            shared.rpc_client.clone(),
            shared.oracle.clone(),
            shared.symbols.clone(),
            shared.config.clone(),
            pool.clone(),
        );

        let pool_address = pool.address.clone();
        let pool_name = pool.name.clone();
        let shared_clone = shared.clone();

        let task = tokio::spawn(async move {
            monitor_pool(listener, pool_address, pool_name, shared_clone, program_id).await;
        });

        pool_tasks.push(task);
        info!("Started monitoring task for pool: {}", pool.name);
    }

    info!("Ingester ready. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c()
        .await
        .context("Failed to wait for ctrl+c")?;
    info!("Shutdown signal received");

    // Let every task finish before touching the buffer: anything they release or add after
    // the flush would be published to a runtime that is already going away.
    if shutdown_tx.send(true).is_err() {
        warn!("Shutdown receivers already gone");
    }

    for task in pool_tasks {
        if let Err(e) = task.await {
            warn!("Pool monitoring task failed: {}", e);
        }
    }
    if let Err(e) = poller_task.await {
        warn!("Finality polling task failed: {}", e);
    }

    // Flush finality buffer on shutdown
    let pending_events = shared.finality_buffer.flush();
    info!("Flushing {} pending events", pending_events.len());

    for event in pending_events {
        if let Err(e) = shared.publisher.publish(&event).await {
            exit_sidecar_unreachable(e);
        }
    }

    info!("Solana ingester stopped");
    Ok(())
}

async fn finality_polling_task(
    finality_buffer: Arc<FinalityBuffer>,
    publisher: Arc<Publisher>,
    rpc_client: Arc<RetryableRpcClient>,
    interval_secs: u64,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown_requested(&mut shutdown) => break,
        }

        match rpc_client.get_finalized_slot().await {
            Ok(finalized_slot) => match finality_buffer.advance_finalized(finalized_slot) {
                Ok(released) => {
                    if !released.is_empty() {
                        debug!(
                            "Finality poll released {} events at slot {}",
                            released.len(),
                            finalized_slot
                        );
                        for event in released {
                            if let Err(e) = publisher.publish(&event).await {
                                exit_sidecar_unreachable(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "advance_finalized failed (misconfigured buffer mode): {}",
                        e
                    );
                }
            },
            Err(e) => {
                warn!("Finality poll failed to fetch finalized slot: {}", e);
            }
        }
    }
}

async fn monitor_pool(
    listener: Listener,
    pool_address: String,
    pool_name: String,
    shared: SharedResources,
    program_id: Pubkey,
) {
    let mut shutdown = shared.shutdown.clone();

    info!("[{}] Starting monitoring task", pool_name);

    loop {
        match shared.rpc_client.inner().subscribe_logs(program_id).await {
            Ok(mut subscription) => {
                info!("[{}] Subscribed to Raydium AMM v4 program logs", pool_name);

                let mut received = 0u64;
                let mut touched_pool = 0u64;
                let mut processed = 0u64;
                let mut stats_ticker =
                    tokio::time::interval(tokio::time::Duration::from_secs(STATS_INTERVAL_SECS));
                stats_ticker.tick().await;

                loop {
                    tokio::select! {
                        maybe_log = subscription.recv() => {
                            let Some(log_response) = maybe_log else { break };
                            received += 1;

                            let signature = match Signature::from_str(&log_response.signature) {
                                Ok(sig) => sig,
                                Err(e) => {
                                    warn!("[{}] Failed to parse signature: {}", pool_name, e);
                                    continue;
                                }
                            };

                            match listener.fetch_and_process_transaction(signature, &pool_address).await {
                                Ok(events) => {
                                    if !events.is_empty() {
                                        touched_pool += 1;
                                        for event in events {
                                            processed += 1;
                                            let finalized_slot = shared.rpc_client.get_finalized_slot().await.unwrap_or(0);

                                            let released = shared.finality_buffer
                                                .advance_finalized(finalized_slot)
                                                .unwrap_or_else(|e| {
                                                    error!("advance_finalized failed: {}", e);
                                                    vec![]
                                                });

                                            for released_event in released {
                                                if let Err(e) = shared.publisher.publish(&released_event).await {
                                                    exit_sidecar_unreachable(e);
                                                }
                                            }

                                            let result = shared.finality_buffer.add(event, Some(finalized_slot));

                                            for confirmed_event in result.confirmed {
                                                if let Err(e) = shared.publisher.publish(&confirmed_event).await {
                                                    exit_sidecar_unreachable(e);
                                                }
                                            }

                                            if !result.evicted.is_empty() {
                                                warn!(
                                                    "Buffer overflow: evicting {} events at slot {}",
                                                    result.evicted.len(),
                                                    finalized_slot
                                                );
                                                let reason = format!("Finality buffer overflow at slot {}", finalized_slot);
                                                for evicted_event in result.evicted {
                                                    if let Err(e) = shared.publisher.dead_letter(&evicted_event, &reason).await {
                                                        exit_sidecar_unreachable(e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("[{}] Failed to process transaction {}: {:#}", pool_name, signature, e);
                                }
                            }
                        }
                        _ = stats_ticker.tick() => {
                            info!(
                                "[{}] Last {}s: {} program signatures, {} touched this pool, {} events processed",
                                pool_name, STATS_INTERVAL_SECS, received, touched_pool, processed
                            );
                            received = 0;
                            touched_pool = 0;
                            processed = 0;
                        }
                        _ = shutdown_requested(&mut shutdown) => break,
                    }
                }

                subscription.close().await;

                if *shutdown.borrow() {
                    info!(
                        "[{}] Shutdown requested, stopping monitoring task",
                        pool_name
                    );
                    return;
                }

                warn!(
                    "[{}] WebSocket subscription closed, attempting to reconnect...",
                    pool_name
                );
            }
            Err(e) => {
                error!("[{}] Failed to subscribe to logs: {:#}", pool_name, e);
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(RECONNECT_DELAY_SECS)) => {}
            _ = shutdown_requested(&mut shutdown) => {
                info!("[{}] Shutdown requested, stopping monitoring task", pool_name);
                return;
            }
        }
    }
}

async fn run_health_server(port: u16) -> Result<()> {
    use warp::Filter;

    let health =
        warp::path!("health").map(|| warp::reply::json(&serde_json::json!({"status": "ok"})));

    info!("Health server listening on port {}", port);
    warp::serve(health).run(([0, 0, 0, 0], port)).await;

    Ok(())
}
