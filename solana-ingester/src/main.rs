mod balances;
mod blockchain;
mod config;
mod decoder;
mod events;
mod finality;
mod oracle;
mod publisher;
mod rpc_retry;
mod symbols;

use anyhow::{Context, Result};
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};

use crate::blockchain::RpcClient;
use crate::config::{Config, PoolConfig};
use crate::decoder::decode_instruction;
use crate::events::Event;
use crate::finality::FinalityBuffer;
use crate::oracle::PythOracle;
use crate::publisher::Publisher;
use crate::rpc_retry::RetryableRpcClient;
use crate::symbols::SymbolCache;

#[derive(Clone)]
struct SharedResources {
    rpc_client: Arc<RetryableRpcClient>,
    oracle: Arc<tokio::sync::Mutex<PythOracle>>,
    symbols: Arc<tokio::sync::Mutex<SymbolCache>>,
    finality_buffer: Arc<tokio::sync::Mutex<FinalityBuffer>>,
    publisher: Arc<Publisher>,
    config: Arc<Config>,
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

    // Wrap with retry logic
    let retryable_rpc_client = Arc::new(RetryableRpcClient::new(
        rpc_client.clone(),
        config.retry.clone(),
    ));

    // Initialize oracle and symbol cache (they use HTTP RPC URL directly)
    let http_rpc_url = config
        .solana
        .rpc
        .replace("wss://", "https://")
        .replace("ws://", "http://");
    let _oracle = Arc::new(tokio::sync::Mutex::new(PythOracle::new(
        http_rpc_url.clone(),
        config.pyth.clone(),
    )));
    let _symbols = Arc::new(tokio::sync::Mutex::new(SymbolCache::new(
        http_rpc_url,
        config.tokens.clone(),
        config.programs.token_metadata.clone(),
    )));

    // Initialize finality buffer
    let finality_buffer = Arc::new(tokio::sync::Mutex::new(FinalityBuffer::new(
        config.solana.commitment.clone(),
        config.solana.finality_buffer_size,
    )));

    // Initialize publisher
    let publisher = Arc::new(Publisher::new(
        &config.dapr.host,
        config.dapr.http_port,
        &config.dapr.pubsub_name,
        &config.topics.trading,
        &config.topics.liquidity,
        &config.topics.dlq,
        config.retry.publish_max_retries,
        config.retry.publish_initial_backoff_ms,
    )?);

    // Start periodic finality polling task
    let finality_buffer_clone = finality_buffer.clone();
    let publisher_clone = publisher.clone();
    let rpc_client_clone = retryable_rpc_client.clone();
    let poll_interval_secs = config.solana.finality_poll_interval_secs;

    tokio::spawn(async move {
        finality_polling_task(
            finality_buffer_clone,
            publisher_clone,
            rpc_client_clone,
            poll_interval_secs,
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
    };

    // Parse Raydium program ID
    let program_id =
        Pubkey::from_str(&config.programs.raydium_amm_v4).context("Invalid Raydium program ID")?;

    info!("All components initialized successfully");

    // Spawn monitoring task for each pool
    let mut pool_tasks = Vec::new();
    for pool in config.pools.iter() {
        let pool_clone = pool.clone();
        let shared_clone = shared.clone();
        let program_id_clone = program_id;

        let task = tokio::spawn(async move {
            monitor_pool(pool_clone, shared_clone, program_id_clone).await;
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

    // Flush finality buffer on shutdown
    let pending_events = shared.finality_buffer.lock().await.flush();
    info!("Flushing {} pending events", pending_events.len());

    for event in pending_events {
        if let Err(e) = shared.publisher.publish_event(event).await {
            error!("Failed to publish pending event during shutdown: {}", e);
        }
    }

    info!("Solana ingester stopped");
    Ok(())
}

async fn finality_polling_task(
    finality_buffer: Arc<tokio::sync::Mutex<FinalityBuffer>>,
    publisher: Arc<Publisher>,
    rpc_client: Arc<RetryableRpcClient>,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        match rpc_client.get_finalized_slot().await {
            Ok(finalized_slot) => {
                let mut buffer = finality_buffer.lock().await;
                let released = buffer.advance_finalized_slot(finalized_slot);

                if !released.is_empty() {
                    debug!(
                        "Finality poll released {} events at slot {}",
                        released.len(),
                        finalized_slot
                    );

                    for event in released {
                        if let Err(e) = publisher.publish_event(event).await {
                            error!("Failed to publish finalized event: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Finality poll failed to fetch finalized slot: {}", e);
            }
        }
    }
}

async fn monitor_pool(pool: PoolConfig, shared: SharedResources, program_id: Pubkey) {
    let pool_address = pool.address.clone();
    let pool_name = pool.name.clone();

    info!("[{}] Starting monitoring task", pool_name);

    loop {
        // Subscribe to logs using the inner RPC client (WebSocket doesn't need retry wrapper)
        match shared.rpc_client.inner().subscribe_logs(program_id).await {
            Ok(mut subscription) => {
                info!("[{}] Subscribed to Raydium AMM v4 program logs", pool_name);

                // Process logs until subscription closes
                while let Some(log_response) = subscription.recv().await {
                    if let Err(e) = process_log(
                        log_response,
                        &shared.rpc_client,
                        &shared.oracle,
                        &shared.symbols,
                        &shared.finality_buffer,
                        &shared.publisher,
                        &shared.config,
                        &pool_address,
                        &pool,
                    )
                    .await
                    {
                        error!("[{}] Failed to process log: {}", pool_name, e);
                    }
                }

                warn!(
                    "[{}] WebSocket subscription closed, attempting to reconnect...",
                    pool_name
                );
            }
            Err(e) => {
                error!("[{}] Failed to subscribe to logs: {}", pool_name, e);
            }
        }

        // Reconnect with exponential backoff
        tokio::time::sleep(tokio::time::Duration::from_secs(
            shared.config.retry.reconnect_delay_short_secs,
        ))
        .await;
    }
}

async fn process_log(
    log_response: RpcLogsResponse,
    rpc_client: &Arc<RetryableRpcClient>,
    oracle: &Arc<tokio::sync::Mutex<PythOracle>>,
    symbols: &Arc<tokio::sync::Mutex<SymbolCache>>,
    finality_buffer: &Arc<tokio::sync::Mutex<FinalityBuffer>>,
    publisher: &Arc<Publisher>,
    config: &Arc<Config>,
    pool_address: &str,
    pool_config: &PoolConfig,
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
    let block_time = tx.block_time.context("transaction missing block_time")?;

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
                    pool_config,
                    rpc_client,
                    symbols,
                    oracle,
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
                    pool_config,
                    rpc_client,
                    symbols,
                    oracle,
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
    config: &Arc<Config>,
    pool_address: &str,
    pool_config: &PoolConfig,
    rpc_client: &Arc<RetryableRpcClient>,
    symbols: &Arc<tokio::sync::Mutex<SymbolCache>>,
    oracle: &Arc<tokio::sync::Mutex<PythOracle>>,
) -> Result<()> {
    // Extract instruction data based on type
    let (program_id_index, data_base58) = match instruction {
        solana_transaction_status::UiInstruction::Compiled(compiled) => {
            (compiled.program_id_index, &compiled.data)
        }
        _ => return Ok(()),
    };

    // Verify this is a Raydium instruction
    let program_id_str = account_keys
        .get(program_id_index as usize)
        .context("Program ID index out of bounds")?;

    if program_id_str != &config.programs.raydium_amm_v4 {
        return Ok(());
    }

    // Filter by pool address - only process logs that reference this specific pool
    if !account_keys.contains(&pool_address.to_string()) {
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

    // Skip failed transactions (Gap #6 fix)
    if meta.err.is_some() {
        debug!("Skipping failed transaction: {:?}", signature);
        return Ok(());
    }

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

    // Use the pool config passed for this specific pool
    let token0 = pool_config.token0.as_ref().context("Pool missing token0")?;
    let token1 = pool_config.token1.as_ref().context("Pool missing token1")?;

    // Extract sender/recipient from account keys
    let user_authority = account_keys
        .get(pool_config.user_authority_index)
        .unwrap_or(&pool_address.to_string())
        .clone();

    let event_timestamp = chrono::Utc::now().timestamp_millis();

    // Fetch token symbols (Gap #1 fix)
    let symbol_cache = symbols.lock().await;
    let token0_symbol = symbol_cache.get_symbol(&token0).await;
    let token1_symbol = symbol_cache.get_symbol(&token1).await;
    drop(symbol_cache); // Release lock early

    // Create event based on instruction type
    let event = match decoded {
        crate::decoder::DecodedInstruction::Swap { .. } => {
            // Compute actual swap amounts from balance diffs
            let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
                pre_balances,
                post_balances,
                pool_config.vault0_index,
                pool_config.vault1_index,
            )
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

            // Fetch oracle prices for volume USD calculation (Gap #2 fix)
            let oracle_lock = oracle.lock().await;
            let token0_price = oracle_lock.fetch_price(&token0).await?.unwrap_or(0.0);
            let token1_price = oracle_lock.fetch_price(&token1).await?.unwrap_or(0.0);
            drop(oracle_lock); // Release lock early

            // Calculate volume USD using actual token prices
            // Use the "in" amount (what user sold) for volume calculation
            let volume_usd = if amount0_in > 0 && token0_price > 0.0 {
                // Token0 sold: SOL has 9 decimals
                Some(token0_price * (amount0_in as f64) / 1e9)
            } else if amount1_in > 0 && token1_price > 0.0 {
                // Token1 sold: USDC has 6 decimals
                Some(token1_price * (amount1_in as f64) / 1e6)
            } else {
                None
            };

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
                    token0_symbol,
                    token1_symbol,
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
            let (amount0, amount1) = balances::compute_liquidity_amounts(
                pre_balances,
                post_balances,
                pool_config.vault0_index,
                pool_config.vault1_index,
            )
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
                    token0_symbol: token0_symbol.clone(),
                    token1_symbol: token1_symbol.clone(),
                    event_timestamp,
                },
                sender: user_authority,
                amount0: amount0.to_string(),
                amount1: amount1.to_string(),
                liquidity: "0".to_string(),
            })
        }
        crate::decoder::DecodedInstruction::Withdraw { .. } => {
            let (amount0, amount1) = balances::compute_liquidity_amounts(
                pre_balances,
                post_balances,
                pool_config.vault0_index,
                pool_config.vault1_index,
            )
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
                    token0_symbol,
                    token1_symbol,
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

    // Publish evicted events to DLQ (buffer overflow)
    if !result.evicted.is_empty() {
        warn!(
            "Buffer overflow: evicting {} events at slot {}",
            result.evicted.len(),
            finalized_slot
        );
        for evicted_event in result.evicted {
            if let Err(e) = publisher
                .publish_to_dlq(
                    evicted_event,
                    format!("Finality buffer overflow at slot {}", finalized_slot),
                )
                .await
            {
                error!("Failed to publish evicted event to DLQ: {}", e);
            }
        }
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
