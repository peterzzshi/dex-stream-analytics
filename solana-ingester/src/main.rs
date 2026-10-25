mod config;
mod blockchain;
mod decoder;
mod balances;
mod events;
mod oracle;
mod symbols;
mod finality;
mod publisher;

use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("Starting Solana ingester");

    // Load configuration
    let config_path = env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "config/ingester.yaml".to_string());
    let config = config::Config::load(&config_path)
        .context("Failed to load configuration")?;

    info!("Loaded configuration: {:?}", config);

    // TODO: Start health check server on config.app.port

    // TODO: Initialize RPC client, oracle, symbol cache, finality buffer, publisher

    // TODO: Subscribe to Raydium program logs

    // TODO: Main event loop

    // Wait for shutdown signal
    info!("Ingester running. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // TODO: Flush finality buffer and graceful shutdown

    Ok(())
}
