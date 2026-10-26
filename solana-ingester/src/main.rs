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
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};

use crate::blockchain::RpcClient;
use crate::config::Config;
use crate::decoder::decode_instruction;
use crate::events::Event;
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
    let program_id =
        Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM).context("Invalid Raydium program ID")?;

    info!("All components initialized successfully");

    // Subscribe to logs
    let mut subscription = rpc_client
        .subscribe_logs(program_id)
        .await
        .context("Failed to subscribe to Raydium program logs")?;

    info!("Subscribed to Raydium AMM v4 program logs");
    info!("Ingester ready. Press Ctrl+C to stop.");

    // Main event loop
    loop {
        tokio::select! {
            // Handle incoming log messages
            log_result = subscription.recv() => {
                match log_result {
                    Some(log_response) => {
                        if let Err(e) = process_log(
                            log_response,
                            &rpc_client,
                            &_oracle,
                            &_symbols,
                            &_finality_buffer,
                            &_publisher,
                            &config,
                            &pool_address,
                        ).await {
                            error!("Failed to process log: {}", e);
                        }
                    }
                    None => {
                        warn!("WebSocket subscription closed, attempting to reconnect...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                        match rpc_client.subscribe_logs(program_id).await {
                            Ok(new_sub) => {
                                subscription = new_sub;
                                info!("Reconnected to Raydium program logs");
                            }
                            Err(e) => {
                                error!("Failed to reconnect: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                            }
                        }
                    }
                }
            }

            // Handle shutdown signal
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    // Flush finality buffer on shutdown
    let pending_events = _finality_buffer.lock().await.flush();
    info!("Flushing {} pending events", pending_events.len());

    for event in pending_events {
        if let Err(e) = _publisher.publish_event(event).await {
            error!("Failed to publish pending event during shutdown: {}", e);
        }
    }

    info!("Solana ingester stopped");
    Ok(())
}

async fn process_log(
    log_response: RpcLogsResponse,
    rpc_client: &Arc<RpcClient>,
    _oracle: &Arc<tokio::sync::Mutex<PythOracle>>,
    _symbols: &Arc<tokio::sync::Mutex<SymbolCache>>,
    finality_buffer: &Arc<tokio::sync::Mutex<FinalityBuffer>>,
    publisher: &Arc<Publisher>,
    config: &Config,
    pool_address: &str,
) -> Result<()> {
    // Extract transaction signature from log response
    let signature = Signature::from_str(&log_response.signature)
        .context("Failed to parse transaction signature")?;

    debug!("Processing transaction: {}", signature);

    // Fetch full transaction details
    let tx = rpc_client
        .get_transaction(signature)
        .await
        .context("Failed to fetch transaction")?;

    // Extract slot and timestamp
    let slot = tx.slot;
    let block_time = tx.block_time.unwrap_or(0);

    // Parse transaction to find Raydium instructions
    let transaction = match &tx.transaction.transaction {
        solana_transaction_status::EncodedTransaction::Json(ui_tx) => ui_tx,
        _ => {
            debug!("Skipping non-JSON encoded transaction");
            return Ok(());
        }
    };

    let message = &transaction.message;
    let account_keys: Vec<String> = match message {
        solana_transaction_status::UiMessage::Parsed(parsed) => parsed
            .account_keys
            .iter()
            .map(|k| k.pubkey.clone())
            .collect(),
        solana_transaction_status::UiMessage::Raw(raw) => raw.account_keys.clone(),
    };

    // Process each instruction based on message type
    match message {
        solana_transaction_status::UiMessage::Parsed(parsed) => {
            for (idx, instruction) in parsed.instructions.iter().enumerate() {
                if let Err(e) = process_instruction(
                    instruction,
                    idx,
                    &account_keys,
                    &tx,
                    signature,
                    slot,
                    block_time,
                    finality_buffer,
                    publisher,
                    config,
                    pool_address,
                    rpc_client,
                )
                .await
                {
                    debug!("Failed to process parsed instruction {}: {}", idx, e);
                }
            }
        }
        solana_transaction_status::UiMessage::Raw(raw) => {
            for (idx, instruction) in raw.instructions.iter().enumerate() {
                // Convert UiCompiledInstruction to UiInstruction for processing
                let ui_instruction =
                    solana_transaction_status::UiInstruction::Compiled(instruction.clone());
                if let Err(e) = process_instruction(
                    &ui_instruction,
                    idx,
                    &account_keys,
                    &tx,
                    signature,
                    slot,
                    block_time,
                    finality_buffer,
                    publisher,
                    config,
                    pool_address,
                    rpc_client,
                )
                .await
                {
                    debug!("Failed to process raw instruction {}: {}", idx, e);
                }
            }
        }
    }

    Ok(())
}

async fn process_instruction(
    instruction: &solana_transaction_status::UiInstruction,
    idx: usize,
    account_keys: &[String],
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    signature: Signature,
    slot: u64,
    block_time: i64,
    finality_buffer: &Arc<tokio::sync::Mutex<FinalityBuffer>>,
    publisher: &Arc<Publisher>,
    config: &Config,
    pool_address: &str,
    rpc_client: &Arc<RpcClient>,
) -> Result<()> {
    // Extract instruction data based on type
    let (program_id_index, data_base58) = match instruction {
        solana_transaction_status::UiInstruction::Compiled(compiled) => {
            (compiled.program_id_index, &compiled.data)
        }
        _ => return Ok(()),
    };

    // Verify this is a Raydium instruction
    let program_id = account_keys
        .get(program_id_index as usize)
        .context("Program ID index out of bounds")?;

    if program_id != RAYDIUM_AMM_V4_PROGRAM {
        return Ok(());
    }

    // Decode base58 instruction data to bytes
    let data = match bs58::decode(data_base58).into_vec() {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to decode base58 instruction data: {}", e);
            return Ok(());
        }
    };

    // Decode instruction
    let decoded = match decode_instruction(&data, account_keys.len()) {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to decode instruction: {}", e);
            return Ok(());
        }
    };

    let meta = match tx.transaction.meta.as_ref() {
        Some(m) => m,
        None => {
            debug!("Transaction missing metadata");
            return Ok(());
        }
    };

    // Extract pre/post token balances
    let (pre_balances, post_balances) = match (&meta.pre_token_balances, &meta.post_token_balances)
    {
        (
            solana_transaction_status::option_serializer::OptionSerializer::Some(pre),
            solana_transaction_status::option_serializer::OptionSerializer::Some(post),
        ) => (pre, post),
        _ => {
            debug!("Missing pre/post token balances");
            return Ok(());
        }
    };

    // Get pool config for token identities
    let pool_config = config.pools.first().context("No pools configured")?;

    let token0 = pool_config.token0.as_ref().context("Pool missing token0")?;
    let token1 = pool_config.token1.as_ref().context("Pool missing token1")?;

    // Extract sender/recipient from account keys (user authority is typically account 16 for swaps)
    let user_authority = account_keys
        .get(16)
        .unwrap_or(&pool_address.to_string())
        .clone();

    let event_timestamp = chrono::Utc::now().timestamp_millis();

    // Create event based on instruction type
    let event = match decoded {
        crate::decoder::DecodedInstruction::Swap { .. } => {
            // Compute actual swap amounts from balance diffs (vault indices 5 and 6 for Raydium)
            let (vault0_diff, vault1_diff) =
                balances::compute_balance_diff(pre_balances, post_balances, 5, 6)
                    .context("Failed to compute balance diffs for swap")?;

            let (amount0_in, amount1_in, amount0_out, amount1_out) =
                balances::extract_swap_amounts(vault0_diff, vault1_diff);

            let price = if amount0_in > 0 {
                (amount1_out as f64) / (amount0_in as f64)
            } else if amount1_in > 0 {
                (amount0_out as f64) / (amount1_in as f64)
            } else {
                0.0
            };

            let volume_usd = Some((amount1_out.max(amount1_in) as f64) / 1e6);

            Event::Swap(crate::events::SwapEvent {
                base: crate::events::BaseEvent {
                    event_type: "Swap".to_string(),
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: config.solana.chain.clone(),
                    chain_name: config.solana.chain_name.clone(),
                    pair_address: pool_address.to_string(),
                    token0: token0.clone(),
                    token1: token1.clone(),
                    token0_symbol: None,
                    token1_symbol: None,
                    event_timestamp,
                },
                sender: user_authority.clone(),
                recipient: user_authority,
                amount0_in: amount0_in.to_string(),
                amount1_in: amount1_in.to_string(),
                amount0_out: amount0_out.to_string(),
                amount1_out: amount1_out.to_string(),
                price,
                volume_usd,
                gas_used: meta.compute_units_consumed.clone().unwrap_or(0) as i64,
                gas_price: "0".to_string(),
            })
        }
        crate::decoder::DecodedInstruction::Deposit { .. } => {
            let (amount0, amount1) =
                balances::compute_liquidity_amounts(pre_balances, post_balances, 5, 6)
                    .context("Failed to compute liquidity amounts")?;

            Event::Mint(crate::events::MintEvent {
                base: crate::events::BaseEvent {
                    event_type: "Mint".to_string(),
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: config.solana.chain.clone(),
                    chain_name: config.solana.chain_name.clone(),
                    pair_address: pool_address.to_string(),
                    token0: token0.clone(),
                    token1: token1.clone(),
                    token0_symbol: None,
                    token1_symbol: None,
                    event_timestamp,
                },
                sender: user_authority,
                amount0: amount0.to_string(),
                amount1: amount1.to_string(),
                liquidity: "0".to_string(),
            })
        }
        crate::decoder::DecodedInstruction::Withdraw { .. } => {
            let (amount0, amount1) =
                balances::compute_liquidity_amounts(pre_balances, post_balances, 5, 6)
                    .context("Failed to compute liquidity amounts")?;

            Event::Burn(crate::events::BurnEvent {
                base: crate::events::BaseEvent {
                    event_type: "Burn".to_string(),
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: config.solana.chain.clone(),
                    chain_name: config.solana.chain_name.clone(),
                    pair_address: pool_address.to_string(),
                    token0: token0.clone(),
                    token1: token1.clone(),
                    token0_symbol: None,
                    token1_symbol: None,
                    event_timestamp,
                },
                sender: user_authority,
                amount0: amount0.to_string(),
                amount1: amount1.to_string(),
                liquidity: "0".to_string(),
            })
        }
    };

    // Add to finality buffer and advance if needed
    let finalized_slot = rpc_client.get_finalized_slot().await.unwrap_or(0);
    let mut buffer = finality_buffer.lock().await;

    // Advance buffer to release finalized events
    let released = buffer.advance_finalized_slot(finalized_slot);
    for released_event in released {
        publisher.publish_event(released_event).await?;
    }

    let result = buffer.add(event, finalized_slot);

    // Publish confirmed events
    for confirmed_event in result.confirmed {
        publisher.publish_event(confirmed_event).await?;
    }

    // Log evicted events (buffer overflow)
    if !result.evicted.is_empty() {
        warn!(
            "Evicted {} events from finality buffer",
            result.evicted.len()
        );
    }

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
