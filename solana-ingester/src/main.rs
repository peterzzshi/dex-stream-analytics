mod balances;
mod blockchain;
mod config;
mod decoder;
mod event_builder;
mod file_config;
mod instruction_processor;
mod listener;
mod oracle;
mod rpc_retry;
mod symbols;

use anyhow::{Context, Result};
use dex_ingester_common::{
    EventProcessor, FinalityBuffer, FinalityMode, PoolStats, Publisher, PublisherConfig,
    RetryConfig,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch::Receiver;
use tokio::time::{Duration, MissedTickBehavior};
use tracing::{error, info, warn};

use crate::blockchain::RpcClient;
use crate::config::{Config, PoolConfig};
use crate::event_builder::EventBuilder;
use crate::instruction_processor::InstructionProcessor;
use crate::listener::Listener;
use crate::oracle::PythOracle;
use crate::rpc_retry::RetryableRpcClient;
use crate::symbols::SymbolCache;

const EVENT_SOURCE: &str = "ingester/solana/raydium-amm-v4";
const RECONNECT_DELAY_SECS: u64 = 5;
const STATS_INTERVAL_SECS: u64 = 60;

/// Resolves when shutdown is requested, or when the sender is dropped.
async fn shutdown_requested(shutdown: &mut Receiver<bool>) {
    loop {
        if *shutdown.borrow() {
            return;
        }
        if shutdown.changed().await.is_err() {
            return;
        }
    }
}

/// Everything one pool's stream needs, so the loop functions take `&self` instead of
/// six cloned arguments.
struct PoolRunner {
    listener: Listener,
    rpc_client: Arc<RetryableRpcClient>,
    event_processor: Arc<EventProcessor>,
    pool_name: String,
    pool_address: String,
    program_id: Pubkey,
}

impl PoolRunner {
    async fn run(self, mut shutdown: Receiver<bool>) {
        info!("[{}] Starting monitoring task", self.pool_name);

        loop {
            if let Err(e) = self.stream(&mut shutdown).await {
                error!("[{}] Subscription loop failed: {:#}", self.pool_name, e);
            }

            if *shutdown.borrow() {
                info!(
                    "[{}] Shutdown requested, stopping monitoring task",
                    self.pool_name
                );
                return;
            }

            warn!(
                "[{}] WebSocket subscription closed, attempting to reconnect...",
                self.pool_name
            );

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)) => {}
                _ = shutdown_requested(&mut shutdown) => {
                    info!("[{}] Shutdown requested, stopping monitoring task", self.pool_name);
                    return;
                }
            }
        }
    }

    async fn stream(&self, shutdown: &mut Receiver<bool>) -> Result<()> {
        let mut subscription = self
            .rpc_client
            .inner()
            .subscribe_logs(self.program_id)
            .await
            .context("Failed to subscribe to logs")?;

        info!(
            "[{}] Subscribed to Raydium AMM v4 program logs",
            self.pool_name
        );

        let mut stats = PoolStats::default();
        // One long-lived ticker: a sleep recreated inside `select!` restarts on every
        // log line, so the report would never fire on a busy pool.
        let mut ticker = tokio::time::interval(Duration::from_secs(STATS_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;

        loop {
            tokio::select! {
                maybe_log = subscription.recv() => {
                    let Some(log_response) = maybe_log else {
                        warn!("[{}] Subscription stream ended", self.pool_name);
                        break;
                    };

                    stats.received += 1;

                    let signature = match Signature::from_str(&log_response.signature) {
                        Ok(signature) => signature,
                        Err(e) => {
                            warn!("[{}] Failed to parse signature {}: {}", self.pool_name, log_response.signature, e);
                            continue;
                        }
                    };

                    if let Err(e) = self.ingest(signature, &mut stats).await {
                        warn!("[{}] Failed to process transaction {}: {:#}", self.pool_name, signature, e);
                    }
                }
                _ = ticker.tick() => self.report_stats(&mut stats),
                _ = shutdown_requested(shutdown) => {
                    info!("[{}] Shutdown requested in subscription loop", self.pool_name);
                    break;
                }
            }
        }

        subscription.close().await;
        Ok(())
    }

    async fn ingest(&self, signature: Signature, stats: &mut PoolStats) -> Result<()> {
        let events = self
            .listener
            .fetch_and_process_transaction(signature, &self.pool_address)
            .await?;

        if events.is_empty() {
            return Ok(());
        }
        stats.pool_hits += 1;

        // External finality mode panics on a missing position, so a failed read has to
        // drop the transaction rather than buffer it without an upper bound.
        let finalized_slot = self
            .rpc_client
            .get_finalized_slot()
            .await
            .context("Failed to fetch finalized slot")?;

        let released = self
            .event_processor
            .advance_finalized(finalized_slot, &self.pool_name)
            .await
            .map_err(anyhow::Error::msg)?;
        stats.add_published(released.published);
        stats.add_dead_lettered(released.dead_lettered);

        for event in events {
            let outcome = self
                .event_processor
                .process(event, Some(finalized_slot), &self.pool_name)
                .await;

            stats.processed += 1;
            stats.add_published(outcome.published);
            stats.add_dead_lettered(outcome.dead_lettered);
        }

        Ok(())
    }

    fn report_stats(&self, stats: &mut PoolStats) {
        let snapshot = stats.take();
        info!(
            "[{}] Last {}s: {} received, {} touched this pool, {} events processed, {} published, {} dead-lettered",
            self.pool_name,
            STATS_INTERVAL_SECS,
            snapshot.received,
            snapshot.pool_hits,
            snapshot.processed,
            snapshot.published,
            snapshot.dead_lettered
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Solana ingester");

    let config = Arc::new(Config::load_from_env().context("Failed to load configuration")?);

    info!(
        "Loaded configuration for chain: {}",
        config.solana.chain_name
    );
    for pool in &config.pools {
        info!("  - {} at {}", pool.name, pool.address);
    }

    let app_port = config.solana.app_port;
    tokio::spawn(async move {
        if let Err(e) = run_health_server(app_port).await {
            error!("Health server failed: {}", e);
        }
    });

    let rpc_client = Arc::new(RetryableRpcClient::new(
        Arc::new(RpcClient::new(
            config.solana.rpc.expose(),
            config.solana.commitment.clone(),
        )?),
        RetryConfig::default(),
    ));
    info!(
        "Connected to Solana RPC with commitment: {:?}",
        config.solana.commitment
    );

    // The oracle and symbol cache query HTTP endpoints directly, not the WS subscription.
    let http_rpc_url = config
        .solana
        .rpc
        .expose()
        .replace("wss://", "https://")
        .replace("ws://", "http://");

    let oracle = Arc::new(PythOracle::new(http_rpc_url.clone(), config.pyth.clone()));
    let symbols = Arc::new(SymbolCache::new(
        http_rpc_url,
        config.tokens.clone(),
        config.programs.token_metadata.clone(),
    ));

    let event_processor = Arc::new(EventProcessor::new(
        Arc::new(
            FinalityBuffer::new(FinalityMode::External)
                .with_event_limit(config.solana.finality_buffer_size),
        ),
        Arc::new(Publisher::new(PublisherConfig::new(
            &config.shared,
            EVENT_SOURCE.to_string(),
        ))?),
    ));

    // Shutdown is broadcast so every task stops touching the buffer before the final flush.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let poller_task = tokio::spawn(finality_polling_task(
        event_processor.clone(),
        rpc_client.clone(),
        config.solana.finality_poll_interval_secs,
        shutdown_rx.clone(),
    ));
    info!(
        "Started finality polling task (interval: {}s)",
        config.solana.finality_poll_interval_secs
    );

    let program_id =
        Pubkey::from_str(&config.programs.raydium_amm_v4).context("Invalid Raydium program ID")?;

    info!("All components initialized successfully");

    let mut pool_tasks = Vec::new();
    for pool in config.pools.iter() {
        let runner = build_pool_runner(
            pool,
            &config,
            &rpc_client,
            &oracle,
            &symbols,
            &event_processor,
            program_id,
        );
        info!("Started monitoring task for pool: {}", pool.name);
        pool_tasks.push(tokio::spawn(runner.run(shutdown_rx.clone())));
    }

    info!("Ingester ready. Press Ctrl+C to stop.");

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

    let published = event_processor.flush("shutdown").await;
    info!(published, "Flushed pending events");

    info!("Solana ingester stopped");
    Ok(())
}

fn build_pool_runner(
    pool: &PoolConfig,
    config: &Arc<Config>,
    rpc_client: &Arc<RetryableRpcClient>,
    oracle: &Arc<PythOracle>,
    symbols: &Arc<SymbolCache>,
    event_processor: &Arc<EventProcessor>,
    program_id: Pubkey,
) -> PoolRunner {
    let event_builder = EventBuilder::new(oracle.clone(), config.clone(), pool.clone());
    let processor =
        InstructionProcessor::new(event_builder, symbols.clone(), config.clone(), pool.clone());

    PoolRunner {
        listener: Listener::new(rpc_client.clone(), processor, pool.clone()),
        rpc_client: rpc_client.clone(),
        event_processor: event_processor.clone(),
        pool_name: pool.name.clone(),
        pool_address: pool.address.clone(),
        program_id,
    }
}

async fn finality_polling_task(
    event_processor: Arc<EventProcessor>,
    rpc_client: Arc<RetryableRpcClient>,
    interval_secs: u64,
    mut shutdown: Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = shutdown_requested(&mut shutdown) => break,
        }

        let finalized_slot = match rpc_client.get_finalized_slot().await {
            Ok(slot) => slot,
            Err(e) => {
                warn!("Finality poll failed to fetch finalized slot: {:#}", e);
                continue;
            }
        };

        if let Err(message) = event_processor
            .advance_finalized(finalized_slot, "finality-poller")
            .await
        {
            warn!("Finality poll: {}", message);
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
