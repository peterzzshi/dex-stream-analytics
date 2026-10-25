mod balances;
mod blockchain;
mod config;
mod decoder;
mod events;
mod finality;
mod oracle;
mod publisher;
mod symbols;

use anyhow::{Context, Result};
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

use crate::blockchain::RpcClient;
use crate::config::Config;
use crate::finality::FinalityBuffer;
use crate::oracle::PythOracle;
use crate::publisher::Publisher;
use crate::symbols::SymbolCache;

const RAYDIUM_AMM_V4_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

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

    // Load configuration
    let config_path =
        env::var("CONFIG_PATH").unwrap_or_else(|_| "config/ingester.yaml".to_string());
    let config = Config::load(&config_path).context("Failed to load configuration")?;

    info!(
        "Loaded configuration for chain: {}",
        config.solana.chain_name
    );

    let pool_address = config
        .pools
        .first()
        .context("No pools configured")?
        .address
        .clone();
    let pool_name = config.pools.first().unwrap().name.clone();

    info!("Monitoring pool: {} at {}", pool_name, pool_address);

    // Start health check server
    let app_port = config.app.port;
    tokio::spawn(async move {
        if let Err(e) = run_health_server(app_port).await {
            error!("Health server failed: {}", e);
        }
    });

    // Initialize RPC client
    let rpc_client = Arc::new(RpcClient::new(
        &config.solana.rpc,
        config.solana.commitment.clone(),
    )?);
    info!(
        "Connected to Solana RPC with commitment: {:?}",
        config.solana.commitment
    );

    // Initialize oracle and symbol cache (they use HTTP RPC URL directly)
    let http_rpc_url = config
        .solana
        .rpc
        .replace("wss://", "https://")
        .replace("ws://", "http://");
    let _oracle = Arc::new(tokio::sync::Mutex::new(PythOracle::new(
        http_rpc_url.clone(),
    )));
    let _symbols = Arc::new(tokio::sync::Mutex::new(SymbolCache::new(http_rpc_url)));

    // Initialize finality buffer
    let _finality_buffer = Arc::new(tokio::sync::Mutex::new(FinalityBuffer::new(
        config.solana.commitment.clone(),
        500,
    )));

    // Initialize publisher
    let _publisher = Arc::new(Publisher::new(
        &config.dapr.host,
        config.dapr.http_port,
        &config.dapr.pubsub_name,
        &config.topics.trading,
        &config.topics.liquidity,
    )?);

    // Parse Raydium program ID
    let _program_id =
        Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM).context("Invalid Raydium program ID")?;

    // TODO: The main event loop implementation requires additional work to handle
    // the Solana SDK 1.18 WebSocket subscription API properly. The logsSubscribe
    // method returns a PubsubClient, not a subscription receiver. Need to:
    // 1. Call logs_subscribe() on PubsubClient to get (RpcLogsReceiver, unsubscribe_fn)
    // 2. Poll the RpcLogsReceiver in the main loop
    // 3. Handle WebSocket reconnection on errors
    // 4. Parse RpcLogsResponse properly (it has .value field, not direct signature access)
    //
    // For now, log that the ingester is ready and wait for shutdown signal.
    warn!("Main event loop not yet implemented - requires WebSocket subscription handling");
    info!("All components initialized successfully");

    // Wait for shutdown signal
    info!("Ingester ready. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // TODO: Flush finality buffer on shutdown
    info!("Solana ingester stopped");
    Ok(())
}

async fn run_health_server(port: u16) -> Result<()> {
    use warp::Filter;

    let health =
        warp::path!("health").map(|| warp::reply::json(&serde_json::json!({"status": "ok"})));

    info!("Health server listening on port {}", port);
    warp::serve(health).run(([0, 0, 0, 0], port)).await;

    Ok(())
}
