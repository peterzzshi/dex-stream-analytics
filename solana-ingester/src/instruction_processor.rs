use crate::balances;
use crate::config::{Config, PoolConfig};
use crate::decoder;
use crate::event_builder::{EventBuilder, EventContext};
use crate::symbols::SymbolCache;
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    option_serializer::OptionSerializer, UiInstruction, UiParsedInstruction,
    UiTransactionStatusMeta, UiTransactionTokenBalance,
};
use std::sync::Arc;
use tracing::{debug, info};

pub struct InstructionProcessor {
    event_builder: EventBuilder,
    symbols: Arc<SymbolCache>,
    config: Arc<Config>,
    pool: PoolConfig,
}

impl InstructionProcessor {
    pub fn new(
        event_builder: EventBuilder,
        symbols: Arc<SymbolCache>,
        config: Arc<Config>,
        pool: PoolConfig,
    ) -> Self {
        Self {
            event_builder,
            symbols,
            config,
            pool,
        }
    }

    /// `Ok(None)` means the instruction is irrelevant to this pool — not a failure.
    pub async fn process(
        &self,
        instruction: &UiInstruction,
        index: usize,
        tx: &TransactionContext<'_>,
    ) -> Result<Option<Event>> {
        let Some(view) = InstructionView::parse(instruction, tx.account_keys)? else {
            return Ok(None);
        };

        let Some(decoded) = self.validate_and_decode(&view, index, tx)? else {
            return Ok(None);
        };

        let Some(event_ctx) = self.resolve_context(&view, index, tx).await? else {
            return Ok(None);
        };

        let event = self.event_builder.build(decoded, &event_ctx).await?;

        let volume_usd = match &event {
            Event::Swap(swap) => swap.volume_usd,
            _ => None,
        };
        info!(
            "[{}] Ingested {} tx={} slot={} volume_usd={:?}",
            self.pool.name,
            event.event_type(),
            tx.signature,
            tx.slot,
            volume_usd
        );

        Ok(Some(event))
    }

    fn validate_and_decode(
        &self,
        view: &InstructionView<'_>,
        index: usize,
        tx: &TransactionContext<'_>,
    ) -> Result<Option<decoder::DecodedInstruction>> {
        if view.program_id != self.config.programs.raydium_amm_v4.as_str() {
            return Ok(None);
        }

        if !view.touches(tx.account_keys, tx.pool_address) {
            return Ok(None);
        }

        let data = match bs58::decode(view.data).into_vec() {
            Ok(data) => data,
            Err(e) => {
                debug!(
                    "[{}] Skipping instruction {} in {}: failed to decode base58 data: {}",
                    self.pool.name, index, tx.signature, e
                );
                return Ok(None);
            }
        };

        match decoder::decode_instruction(&data, view.account_count) {
            Ok(decoded) => Ok(Some(decoded)),
            Err(e) => {
                debug!(
                    "[{}] Skipping instruction {} in {}: {:#}",
                    self.pool.name, index, tx.signature, e
                );
                Ok(None)
            }
        }
    }

    async fn resolve_context<'t>(
        &self,
        view: &InstructionView<'_>,
        index: usize,
        tx: &TransactionContext<'t>,
    ) -> Result<Option<EventContext<'t>>> {
        let Some((pre_balances, post_balances)) = token_balances(tx.meta) else {
            debug!(
                "[{}] Skipping {}: missing pre/post token balances",
                self.pool.name, tx.signature
            );
            return Ok(None);
        };

        let authority = view
            .accounts
            .get(2)
            .and_then(|&i| tx.account_keys.get(i))
            .with_context(|| {
                format!(
                    "Instruction in {} has no AMM authority account",
                    tx.signature
                )
            })?;

        let (vault0_index, vault1_index) = balances::resolve_vault_indices(
            pre_balances,
            post_balances,
            &view.accounts,
            authority,
            &self.pool.token0,
            &self.pool.token1,
        )
        .with_context(|| format!("Failed to resolve pool vaults for {}", tx.pool_address))?;

        let (token0_symbol, token1_symbol) = tokio::join!(
            self.symbols.get_symbol(&self.pool.token0),
            self.symbols.get_symbol(&self.pool.token1)
        );

        Ok(Some(EventContext {
            signature: tx.signature,
            slot: tx.slot,
            block_time: tx.block_time,
            instruction_index: index,
            pool_address: tx.pool_address,
            user_authority: tx
                .account_keys
                .first()
                .cloned()
                .unwrap_or_else(|| tx.pool_address.to_string()),
            token0_symbol,
            token1_symbol,
            vault0_index,
            vault1_index,
            pre_balances,
            post_balances,
            meta: tx.meta,
            event_timestamp: chrono::Utc::now().timestamp(),
        }))
    }
}

/// The fields every instruction in one transaction shares, so each call site passes one
/// borrowed value instead of six copied arguments.
pub struct TransactionContext<'a> {
    pub account_keys: &'a [String],
    pub signature: &'a Signature,
    pub slot: u64,
    pub block_time: i64,
    pub meta: &'a UiTransactionStatusMeta,
    pub pool_address: &'a str,
}

/// An instruction resolved against the account table: `accounts` are transaction-wide
/// indices, equal to the declared account count for compiled instructions.
struct InstructionView<'a> {
    program_id: &'a str,
    data: &'a str,
    accounts: Vec<usize>,
    account_count: usize,
}

impl<'a> InstructionView<'a> {
    /// `Ok(None)` for instruction forms that carry no decodable data.
    fn parse(instruction: &'a UiInstruction, account_keys: &'a [String]) -> Result<Option<Self>> {
        Ok(match instruction {
            UiInstruction::Compiled(compiled) => {
                let program_id = account_keys
                    .get(compiled.program_id_index as usize)
                    .context("Program ID index out of bounds")?;
                Some(Self {
                    program_id: program_id.as_str(),
                    data: &compiled.data,
                    accounts: compiled.accounts.iter().map(|&i| i as usize).collect(),
                    account_count: compiled.accounts.len(),
                })
            }
            UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(partial)) => Some(Self {
                program_id: &partial.program_id,
                data: &partial.data,
                accounts: partial
                    .accounts
                    .iter()
                    .filter_map(|account| account_keys.iter().position(|key| key == account))
                    .collect(),
                account_count: partial.accounts.len(),
            }),
            UiInstruction::Parsed(_) => None,
        })
    }

    fn touches(&self, account_keys: &[String], address: &str) -> bool {
        self.accounts
            .iter()
            .any(|&i| account_keys.get(i).map(String::as_str) == Some(address))
    }
}

fn token_balances(
    meta: &UiTransactionStatusMeta,
) -> Option<(&[UiTransactionTokenBalance], &[UiTransactionTokenBalance])> {
    match (&meta.pre_token_balances, &meta.post_token_balances) {
        (OptionSerializer::Some(pre), OptionSerializer::Some(post)) => {
            Some((pre.as_slice(), post.as_slice()))
        }
        _ => None,
    }
}
