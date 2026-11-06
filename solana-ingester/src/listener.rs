use crate::balances;
use crate::config::{Config, PoolConfig};
use crate::decoder::{self, DecodedInstruction};
use crate::oracle::PythOracle;
use crate::rpc_retry::RetryableRpcClient;
use crate::symbols::SymbolCache;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent};
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, UiInstruction, UiTransactionStatusMeta,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct Listener {
    rpc_client: Arc<RetryableRpcClient>,
    oracle: Arc<PythOracle>,
    symbols: Arc<SymbolCache>,
    config: Arc<Config>,
    pool: PoolConfig,
}

impl Listener {
    pub fn new(
        rpc_client: Arc<RetryableRpcClient>,
        oracle: Arc<PythOracle>,
        symbols: Arc<SymbolCache>,
        config: Arc<Config>,
        pool: PoolConfig,
    ) -> Self {
        Self {
            rpc_client,
            oracle,
            symbols,
            config,
            pool,
        }
    }

    pub async fn fetch_and_process_transaction(
        &self,
        signature: Signature,
        pool_address: &str,
    ) -> Result<Vec<Event>> {
        let tx = self
            .rpc_client
            .get_transaction(signature)
            .await
            .context("Failed to fetch transaction")?;

        let slot = tx.slot;
        let block_time = tx.block_time.context("transaction missing block_time")?;

        let meta = match tx.transaction.meta.as_ref() {
            Some(m) => m,
            None => {
                debug!("Transaction missing metadata");
                return Ok(vec![]);
            }
        };

        if meta.err.is_some() {
            debug!("Skipping failed transaction: {:?}", signature);
            return Ok(vec![]);
        }

        let transaction = match &tx.transaction.transaction {
            solana_transaction_status::EncodedTransaction::Json(ui_tx) => ui_tx,
            _ => {
                debug!("Skipping non-JSON encoded transaction");
                return Ok(vec![]);
            }
        };

        let account_keys = self.extract_account_keys(&transaction.message);
        let touches_pool = account_keys.iter().any(|k| k == pool_address);

        if !touches_pool {
            return Ok(vec![]);
        }

        let instructions = self.flatten_instructions(&transaction.message, &tx);
        let mut events = Vec::new();

        for (idx, instruction) in instructions.iter().enumerate() {
            match self
                .process_instruction(
                    instruction,
                    idx,
                    &account_keys,
                    signature,
                    slot,
                    block_time,
                    meta,
                    pool_address,
                )
                .await
            {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(e) => {
                    warn!(
                        "[{}] Failed to process instruction {} in {}: {:#}",
                        self.pool.name, idx, signature, e
                    );
                }
            }
        }

        Ok(events)
    }

    fn extract_account_keys(&self, message: &solana_transaction_status::UiMessage) -> Vec<String> {
        match message {
            solana_transaction_status::UiMessage::Parsed(parsed) => parsed
                .account_keys
                .iter()
                .map(|k| k.pubkey.clone())
                .collect(),
            solana_transaction_status::UiMessage::Raw(raw) => raw.account_keys.clone(),
        }
    }

    fn flatten_instructions(
        &self,
        message: &solana_transaction_status::UiMessage,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Vec<UiInstruction> {
        let top_level: Vec<UiInstruction> = match message {
            solana_transaction_status::UiMessage::Parsed(parsed) => parsed.instructions.clone(),
            solana_transaction_status::UiMessage::Raw(raw) => raw
                .instructions
                .iter()
                .cloned()
                .map(UiInstruction::Compiled)
                .collect(),
        };

        let inner_groups: &[solana_transaction_status::UiInnerInstructions] =
            match tx.transaction.meta.as_ref().map(|m| &m.inner_instructions) {
                Some(solana_transaction_status::option_serializer::OptionSerializer::Some(
                    groups,
                )) => groups.as_slice(),
                _ => &[],
            };

        let mut instructions = Vec::with_capacity(top_level.len());
        for (i, instruction) in top_level.into_iter().enumerate() {
            instructions.push(instruction);
            for group in inner_groups.iter().filter(|g| g.index as usize == i) {
                instructions.extend(group.instructions.iter().cloned());
            }
        }

        instructions
    }

    async fn process_instruction(
        &self,
        instruction: &UiInstruction,
        idx: usize,
        account_keys: &[String],
        signature: Signature,
        slot: u64,
        block_time: i64,
        meta: &UiTransactionStatusMeta,
        pool_address: &str,
    ) -> Result<Option<Event>> {
        let (
            program_id_str,
            data_base58,
            instruction_account_count,
            instruction_touches_pool,
            instruction_accounts,
        ) = match instruction {
            UiInstruction::Compiled(compiled) => {
                let accounts: Vec<_> = compiled.accounts.iter().map(|&i| i as usize).collect();
                (
                    account_keys
                        .get(compiled.program_id_index as usize)
                        .context("Program ID index out of bounds")?,
                    &compiled.data,
                    compiled.accounts.len(),
                    compiled.accounts.iter().any(|&i| {
                        account_keys.get(i as usize).map(String::as_str) == Some(pool_address)
                    }),
                    accounts,
                )
            }
            UiInstruction::Parsed(
                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(partial),
            ) => {
                let accounts: Vec<_> = partial
                    .accounts
                    .iter()
                    .filter_map(|a| account_keys.iter().position(|k| k == a))
                    .collect();
                (
                    &partial.program_id,
                    &partial.data,
                    partial.accounts.len(),
                    partial.accounts.iter().any(|a| a == pool_address),
                    accounts,
                )
            }
            UiInstruction::Parsed(_) => return Ok(None),
        };

        if program_id_str != &self.config.programs.raydium_amm_v4 {
            return Ok(None);
        }

        if !instruction_touches_pool {
            return Ok(None);
        }

        let data = match bs58::decode(data_base58).into_vec() {
            Ok(d) => d,
            Err(e) => {
                debug!(
                    "[{}] Skipping instruction {} in {}: failed to decode base58 data: {}",
                    self.pool.name, idx, signature, e
                );
                return Ok(None);
            }
        };

        let decoded = match decoder::decode_instruction(&data, instruction_account_count) {
            Ok(d) => d,
            Err(e) => {
                debug!(
                    "[{}] Skipping instruction {} in {}: {:#}",
                    self.pool.name, idx, signature, e
                );
                return Ok(None);
            }
        };

        let (pre_balances, post_balances) =
            match (&meta.pre_token_balances, &meta.post_token_balances) {
                (
                    solana_transaction_status::option_serializer::OptionSerializer::Some(pre),
                    solana_transaction_status::option_serializer::OptionSerializer::Some(post),
                ) => (pre, post),
                _ => {
                    debug!(
                        "[{}] Skipping {}: missing pre/post token balances",
                        self.pool.name, signature
                    );
                    return Ok(None);
                }
            };

        let token0 = &self.pool.token0;
        let token1 = &self.pool.token1;

        let authority = instruction_accounts
            .get(2)
            .and_then(|&i| account_keys.get(i))
            .with_context(|| {
                format!("Instruction in {} has no AMM authority account", signature)
            })?;

        let user_authority = account_keys
            .first()
            .cloned()
            .unwrap_or_else(|| pool_address.to_string());

        let event_timestamp = chrono::Utc::now().timestamp();

        let token0_symbol = self.symbols.get_symbol(token0).await;
        let token1_symbol = self.symbols.get_symbol(token1).await;

        let (vault0_index, vault1_index) = balances::resolve_vault_indices(
            pre_balances,
            post_balances,
            &instruction_accounts,
            authority,
            token0,
            token1,
        )
        .with_context(|| format!("Failed to resolve pool vaults for {}", pool_address))?;

        let event = match decoded {
            DecodedInstruction::Swap => {
                let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
                    pre_balances,
                    post_balances,
                    vault0_index,
                    vault1_index,
                )
                .context("Failed to compute balance diffs for swap")?;

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

                let token0_price = self.oracle.fetch_price(token0).await?.unwrap_or(0.0);
                let token1_price = self.oracle.fetch_price(token1).await?.unwrap_or(0.0);

                let volume_usd = if amount0_in > 0 {
                    balances::volume_usd(amount0_in, decimals0, token0_price)
                } else {
                    balances::volume_usd(amount1_in, decimals1, token1_price)
                };

                Event::Swap(SwapEvent {
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: self.config.solana.chain.clone(),
                    chain_name: self.config.solana.chain_name.clone(),
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
            DecodedInstruction::Deposit => {
                let (amount0, amount1) = balances::compute_liquidity_amounts(
                    pre_balances,
                    post_balances,
                    vault0_index,
                    vault1_index,
                )
                .context("Failed to compute liquidity amounts")?;

                Event::Mint(MintEvent {
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: self.config.solana.chain.clone(),
                    chain_name: self.config.solana.chain_name.clone(),
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
            DecodedInstruction::Withdraw => {
                let (amount0, amount1) = balances::compute_liquidity_amounts(
                    pre_balances,
                    post_balances,
                    vault0_index,
                    vault1_index,
                )
                .context("Failed to compute liquidity amounts")?;

                Event::Burn(BurnEvent {
                    event_id: format!("{}:{}", signature, idx),
                    block_number: slot as i64,
                    block_timestamp: block_time,
                    transaction_hash: signature.to_string(),
                    log_index: idx as i32,
                    chain: self.config.solana.chain.clone(),
                    chain_name: self.config.solana.chain_name.clone(),
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

        let kind = match &decoded {
            DecodedInstruction::Swap => "swap",
            DecodedInstruction::Deposit => "mint",
            DecodedInstruction::Withdraw => "burn",
        };

        let volume_usd = match &event {
            Event::Swap(s) => s.volume_usd,
            _ => None,
        };

        info!(
            "[{}] Ingested {} tx={} slot={} volume_usd={:?}",
            self.pool.name, kind, signature, slot, volume_usd
        );

        Ok(Some(event))
    }
}
