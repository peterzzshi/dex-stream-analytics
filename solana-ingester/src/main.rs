mod balances;
mod blockchain;
mod config;
mod decoder;
mod events;
mod file_config;
mod oracle;
mod rpc_retry;
mod symbols;

use anyhow::{Context, Result};
use dex_ingester_common::{
    FinalityBuffer, FinalityMode, Publisher, PublisherConfig, RetryConfig, SidecarUnreachable,
};
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
    oracle: Arc<tokio::sync::Mutex<PythOracle>>,
    symbols: Arc<tokio::sync::Mutex<SymbolCache>>,
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

async fn monitor_pool(pool: PoolConfig, shared: SharedResources, program_id: Pubkey) {
    let pool_address = pool.address.clone();
    let pool_name = pool.name.clone();
    let mut shutdown = shared.shutdown.clone();

    info!("[{}] Starting monitoring task", pool_name);

    loop {
        // Subscribe to logs using the inner RPC client (WebSocket doesn't need retry wrapper)
        match shared.rpc_client.inner().subscribe_logs(program_id).await {
            Ok(mut subscription) => {
                info!("[{}] Subscribed to Raydium AMM v4 program logs", pool_name);

                let mut received = 0u64;
                let mut touched_pool = 0u64;
                let mut failed = 0u64;
                let mut stats_ticker =
                    tokio::time::interval(tokio::time::Duration::from_secs(STATS_INTERVAL_SECS));
                stats_ticker.tick().await;

                // Process logs until the subscription closes or shutdown is requested
                loop {
                    tokio::select! {
                        maybe_log = subscription.recv() => {
                            let Some(log_response) = maybe_log else { break };
                            received += 1;
                            match process_log(
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
                                Ok(true) => touched_pool += 1,
                                Ok(false) => {}
                                Err(e) => {
                                    failed += 1;
                                    error!("[{}] Failed to process log: {:#}", pool_name, e);
                                }
                            }
                        }
                        _ = stats_ticker.tick() => {
                            info!(
                                "[{}] Last {}s: {} program signatures, {} touched this pool, {} failed",
                                pool_name, STATS_INTERVAL_SECS, received, touched_pool, failed
                            );
                            received = 0;
                            touched_pool = 0;
                            failed = 0;
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

async fn process_log(
    log_response: RpcLogsResponse,
    rpc_client: &Arc<RetryableRpcClient>,
    oracle: &Arc<tokio::sync::Mutex<PythOracle>>,
    symbols: &Arc<tokio::sync::Mutex<SymbolCache>>,
    finality_buffer: &Arc<FinalityBuffer>,
    publisher: &Arc<Publisher>,
    config: &Arc<Config>,
    pool_address: &str,
    pool_config: &PoolConfig,
) -> Result<bool> {
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
            return Ok(false);
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

    let touches_pool = account_keys.iter().any(|k| k == pool_address);

    let top_level: Vec<solana_transaction_status::UiInstruction> = match message {
        solana_transaction_status::UiMessage::Parsed(parsed) => parsed.instructions.clone(),
        solana_transaction_status::UiMessage::Raw(raw) => raw
            .instructions
            .iter()
            .cloned()
            .map(solana_transaction_status::UiInstruction::Compiled)
            .collect(),
    };
    let inner_groups: &[solana_transaction_status::UiInnerInstructions] =
        match tx.transaction.meta.as_ref().map(|m| &m.inner_instructions) {
            Some(solana_transaction_status::option_serializer::OptionSerializer::Some(groups)) => {
                groups.as_slice()
            }
            _ => &[],
        };

    // Routers (Jupiter, GMGN) call Raydium via CPI, so the swap is an inner instruction.
    // Flatten in execution order; the position doubles as a unique log index.
    let mut instructions = Vec::with_capacity(top_level.len());
    for (i, instruction) in top_level.into_iter().enumerate() {
        instructions.push(instruction);
        for group in inner_groups.iter().filter(|g| g.index as usize == i) {
            instructions.extend(group.instructions.iter().cloned());
        }
    }

    for (idx, instruction) in instructions.iter().enumerate() {
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
            warn!(
                "[{}] Failed to process instruction {} in {}: {:#}",
                pool_config.name, idx, signature, e
            );
        }
    }

    Ok(touches_pool)
}

async fn process_instruction(
    instruction: &solana_transaction_status::UiInstruction,
    idx: usize,
    account_keys: &[String],
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    signature: Signature,
    slot: u64,
    block_time: i64,
    finality_buffer: &Arc<FinalityBuffer>,
    publisher: &Arc<Publisher>,
    config: &Arc<Config>,
    pool_address: &str,
    pool_config: &PoolConfig,
    rpc_client: &Arc<RetryableRpcClient>,
    symbols: &Arc<tokio::sync::Mutex<SymbolCache>>,
    oracle: &Arc<tokio::sync::Mutex<PythOracle>>,
) -> Result<()> {
    // Extract instruction data based on type
    // JsonParsed returns programs without a built-in parser (Raydium) as PartiallyDecoded
    let (
        program_id_str,
        data_base58,
        instruction_account_count,
        instruction_touches_pool,
        instruction_accounts,
    ) = match instruction {
        solana_transaction_status::UiInstruction::Compiled(compiled) => (
            account_keys
                .get(compiled.program_id_index as usize)
                .context("Program ID index out of bounds")?,
            &compiled.data,
            compiled.accounts.len(),
            compiled
                .accounts
                .iter()
                .any(|&i| account_keys.get(i as usize).map(String::as_str) == Some(pool_address)),
            compiled
                .accounts
                .iter()
                .map(|&i| i as usize)
                .collect::<Vec<_>>(),
        ),
        solana_transaction_status::UiInstruction::Parsed(
            solana_transaction_status::UiParsedInstruction::PartiallyDecoded(partial),
        ) => (
            &partial.program_id,
            &partial.data,
            partial.accounts.len(),
            partial.accounts.iter().any(|a| a == pool_address),
            partial
                .accounts
                .iter()
                .filter_map(|a| account_keys.iter().position(|k| k == a))
                .collect::<Vec<_>>(),
        ),
        solana_transaction_status::UiInstruction::Parsed(_) => return Ok(()),
    };

    if program_id_str != &config.programs.raydium_amm_v4 {
        return Ok(());
    }

    // A transaction can hit several pools; only handle instructions that name this one
    if !instruction_touches_pool {
        return Ok(());
    }

    // Decode base58 instruction data to bytes
    let data = match bs58::decode(data_base58).into_vec() {
        Ok(d) => d,
        Err(e) => {
            info!(
                "[{}] Skipping instruction {} in {}: failed to decode base58 data: {}",
                pool_config.name, idx, signature, e
            );
            return Ok(());
        }
    };

    // Decode instruction
    let decoded = match decode_instruction(&data, instruction_account_count) {
        Ok(d) => d,
        Err(e) => {
            info!(
                "[{}] Skipping instruction {} in {}: {:#}",
                pool_config.name, idx, signature, e
            );
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
        info!(
            "[{}] Skipping failed transaction: {:?}",
            pool_config.name, signature
        );
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
            info!(
                "[{}] Skipping {}: missing pre/post token balances",
                pool_config.name, signature
            );
            return Ok(());
        }
    };

    // Use the pool config passed for this specific pool
    let token0 = &pool_config.token0;
    let token1 = &pool_config.token1;

    // Position 2 is the AMM authority in every swap, deposit and withdraw form.
    // It owns both pool vaults, which is what identifies them in the balance list.
    let authority = instruction_accounts
        .get(2)
        .and_then(|&i| account_keys.get(i))
        .with_context(|| format!("Instruction in {} has no AMM authority account", signature))?;

    // The fee payer is the closest on-chain stand-in for the trader; a routed swap's
    // user authority is the router, not the end user.
    let user_authority = account_keys
        .first()
        .cloned()
        .unwrap_or_else(|| pool_address.to_string());

    let event_timestamp = chrono::Utc::now().timestamp();

    // Fetch token symbols (Gap #1 fix)
    let symbol_cache = symbols.lock().await;
    let token0_symbol = symbol_cache.get_symbol(&token0).await;
    let token1_symbol = symbol_cache.get_symbol(&token1).await;
    drop(symbol_cache); // Release lock early

    let kind = match &decoded {
        crate::decoder::DecodedInstruction::Swap => "swap",
        crate::decoder::DecodedInstruction::Deposit => "mint",
        crate::decoder::DecodedInstruction::Withdraw => "burn",
    };

    // Vault positions vary by instruction form, so resolve them by identity rather
    // than trusting a configured offset.
    let (vault0_index, vault1_index) = balances::resolve_vault_indices(
        pre_balances,
        post_balances,
        &instruction_accounts,
        authority,
        token0,
        token1,
    )
    .with_context(|| format!("Failed to resolve pool vaults for {}", pool_address))?;

    // Create event based on instruction type
    let event = match decoded {
        crate::decoder::DecodedInstruction::Swap => {
            // Compute actual swap amounts from balance diffs
            let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
                pre_balances,
                post_balances,
                vault0_index,
                vault1_index,
            )
            .context("Failed to compute balance diffs for swap")?;

            // Vault diffs must have opposite signs for a valid swap (one in, one out).
            // Same-sign or both-zero diffs indicate the vaults were misidentified.
            if vault0_diff == 0 && vault1_diff == 0 {
                anyhow::bail!(
                    "Both vault diffs are zero for pool {} — resolved vaults ({}, {}) may be wrong",
                    pool_address,
                    vault0_index,
                    vault1_index
                );
            }
            if (vault0_diff > 0) == (vault1_diff > 0) && vault0_diff != 0 {
                anyhow::bail!(
                    "Vault diffs have same sign ({}, {}) for pool {} — resolved vaults ({}, {}) may be wrong",
                    vault0_diff,
                    vault1_diff,
                    pool_address,
                    vault0_index,
                    vault1_index
                );
            }

            let (amount0_in, amount1_in, amount0_out, amount1_out) =
                balances::extract_swap_amounts(vault0_diff, vault1_diff);

            let (decimals0, decimals1) =
                balances::vault_decimals(pre_balances, vault0_index, vault1_index)
                    .context("Failed to read vault token decimals")?;

            let price = balances::swap_price(
                amount0_in,
                amount1_in,
                amount0_out,
                amount1_out,
                decimals0,
                decimals1,
            );

            // Fetch oracle prices for volume USD calculation (Gap #2 fix)
            let oracle_lock = oracle.lock().await;
            let token0_price = oracle_lock.fetch_price(&token0).await?.unwrap_or(0.0);
            let token1_price = oracle_lock.fetch_price(&token1).await?.unwrap_or(0.0);
            drop(oracle_lock); // Release lock early

            // Calculate volume USD using actual token prices
            // Use the "in" amount (what user sold) for volume calculation
            let volume_usd = if amount0_in > 0 {
                balances::volume_usd(amount0_in, decimals0, token0_price)
            } else {
                balances::volume_usd(amount1_in, decimals1, token1_price)
            };

            Event::Swap(crate::events::SwapEvent {
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
                event_timestamp,
            })
        }
        crate::decoder::DecodedInstruction::Deposit => {
            let (amount0, amount1) = balances::compute_liquidity_amounts(
                pre_balances,
                post_balances,
                vault0_index,
                vault1_index,
            )
            .context("Failed to compute liquidity amounts")?;

            Event::Mint(crate::events::MintEvent {
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
                sender: user_authority,
                amount0: amount0.to_string(),
                amount1: amount1.to_string(),
                event_timestamp,
            })
        }
        crate::decoder::DecodedInstruction::Withdraw => {
            let (amount0, amount1) = balances::compute_liquidity_amounts(
                pre_balances,
                post_balances,
                vault0_index,
                vault1_index,
            )
            .context("Failed to compute liquidity amounts")?;

            Event::Burn(crate::events::BurnEvent {
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
                sender: user_authority.clone(),
                recipient: user_authority,
                amount0: amount0.to_string(),
                amount1: amount1.to_string(),
                event_timestamp,
            })
        }
    };

    let volume_usd = match &event {
        Event::Swap(s) => s.volume_usd,
        _ => None,
    };
    info!(
        "[{}] Ingested {} tx={} slot={} volume_usd={:?}",
        pool_config.name, kind, signature, slot, volume_usd
    );

    // Add to finality buffer and advance if needed
    let finalized_slot = rpc_client.get_finalized_slot().await.unwrap_or(0);

    // Advance buffer to release finalized events
    let released = match finality_buffer.advance_finalized(finalized_slot) {
        Ok(events) => events,
        Err(e) => {
            error!(
                "advance_finalized failed (misconfigured buffer mode): {}",
                e
            );
            vec![]
        }
    };
    for released_event in released {
        if let Err(e) = publisher.publish(&released_event).await {
            exit_sidecar_unreachable(e);
        }
    }

    let result = finality_buffer.add(event, Some(finalized_slot));

    // Publish confirmed events
    for confirmed_event in result.confirmed {
        if let Err(e) = publisher.publish(&confirmed_event).await {
            exit_sidecar_unreachable(e);
        }
    }

    // Publish evicted events to DLQ (buffer overflow)
    if !result.evicted.is_empty() {
        warn!(
            "Buffer overflow: evicting {} events at slot {}",
            result.evicted.len(),
            finalized_slot
        );
        let reason = format!("Finality buffer overflow at slot {}", finalized_slot);
        for evicted_event in result.evicted {
            if let Err(e) = publisher.dead_letter(&evicted_event, &reason).await {
                exit_sidecar_unreachable(e);
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
