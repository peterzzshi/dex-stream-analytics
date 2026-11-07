use crate::balances;
use crate::config::{Config, PoolConfig};
use crate::decoder::DecodedInstruction;
use crate::oracle::PythOracle;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent};
use solana_transaction_status::{UiTransactionStatusMeta, UiTransactionTokenBalance};
use std::sync::Arc;

pub struct EventBuilder {
    oracle: Arc<PythOracle>,
    config: Arc<Config>,
    pool: PoolConfig,
}

impl EventBuilder {
    pub fn new(oracle: Arc<PythOracle>, config: Arc<Config>, pool: PoolConfig) -> Self {
        Self {
            oracle,
            config,
            pool,
        }
    }

    pub async fn build(
        &self,
        decoded: DecodedInstruction,
        ctx: &EventContext<'_>,
    ) -> Result<Event> {
        match decoded {
            DecodedInstruction::Swap => self.build_swap(ctx).await,
            DecodedInstruction::Deposit => self.build_mint(ctx).map(Event::Mint),
            DecodedInstruction::Withdraw => self.build_burn(ctx).map(Event::Burn),
        }
    }

    async fn build_swap(&self, ctx: &EventContext<'_>) -> Result<Event> {
        let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
            ctx.pre_balances,
            ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute balance diffs for swap")?;

        validate_swap_diffs(
            vault0_diff,
            vault1_diff,
            ctx.pool_address,
            ctx.vault0_index,
            ctx.vault1_index,
        )?;

        let (amount0_in, amount1_in, amount0_out, amount1_out) =
            balances::extract_swap_amounts(vault0_diff, vault1_diff);

        let (decimals0, decimals1) =
            balances::vault_decimals(ctx.pre_balances, ctx.vault0_index, ctx.vault1_index)
                .context("Failed to read vault token decimals")?;

        let price = balances::swap_price(
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            decimals0,
            decimals1,
        );

        let volume_usd = self
            .calculate_volume_usd(amount0_in, amount1_in, decimals0, decimals1)
            .await
            .context("Failed to fetch oracle price for volume")?;

        Ok(Event::Swap(SwapEvent {
            event_id: event_id(ctx),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.to_string(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol: ctx.token0_symbol.clone(),
            token1_symbol: ctx.token1_symbol.clone(),
            sender: ctx.user_authority.clone(),
            recipient: ctx.user_authority.clone(),
            amount0_in: amount0_in.to_string(),
            amount1_in: amount1_in.to_string(),
            amount0_out: amount0_out.to_string(),
            amount1_out: amount1_out.to_string(),
            price,
            volume_usd,
            gas_used: ctx.meta.compute_units_consumed.unwrap_or(0) as i64,
            gas_price: "0".to_string(),
            event_timestamp: ctx.event_timestamp,
        }))
    }

    fn build_mint(&self, ctx: &EventContext<'_>) -> Result<MintEvent> {
        let (amount0, amount1) = balances::compute_liquidity_amounts(
            ctx.pre_balances,
            ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute liquidity amounts")?;

        Ok(MintEvent {
            event_id: event_id(ctx),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.to_string(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol: ctx.token0_symbol.clone(),
            token1_symbol: ctx.token1_symbol.clone(),
            sender: ctx.user_authority.clone(),
            amount0: amount0.to_string(),
            amount1: amount1.to_string(),
            event_timestamp: ctx.event_timestamp,
        })
    }

    fn build_burn(&self, ctx: &EventContext<'_>) -> Result<BurnEvent> {
        let (amount0, amount1) = balances::compute_liquidity_amounts(
            ctx.pre_balances,
            ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute liquidity amounts")?;

        Ok(BurnEvent {
            event_id: event_id(ctx),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.to_string(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol: ctx.token0_symbol.clone(),
            token1_symbol: ctx.token1_symbol.clone(),
            sender: ctx.user_authority.clone(),
            recipient: ctx.user_authority.clone(),
            amount0: amount0.to_string(),
            amount1: amount1.to_string(),
            event_timestamp: ctx.event_timestamp,
        })
    }

    /// Only the sold side needs a quote: `swap_price` already prices the bought side,
    /// so a second lookup would fetch a number that is never read.
    async fn calculate_volume_usd(
        &self,
        amount0_in: u64,
        amount1_in: u64,
        decimals0: u8,
        decimals1: u8,
    ) -> Result<Option<f64>> {
        let (amount, decimals, token) = if amount0_in > 0 {
            (amount0_in, decimals0, &self.pool.token0)
        } else {
            (amount1_in, decimals1, &self.pool.token1)
        };

        let price = self.oracle.fetch_price(token).await?.unwrap_or(0.0);

        Ok(balances::volume_usd(amount, decimals, price))
    }
}

pub struct EventContext<'a> {
    pub signature: &'a solana_sdk::signature::Signature,
    pub slot: u64,
    pub block_time: i64,
    pub instruction_index: usize,
    pub pool_address: &'a str,
    pub user_authority: String,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub vault0_index: usize,
    pub vault1_index: usize,
    pub pre_balances: &'a [UiTransactionTokenBalance],
    pub post_balances: &'a [UiTransactionTokenBalance],
    pub meta: &'a UiTransactionStatusMeta,
    pub event_timestamp: i64,
}

fn event_id(ctx: &EventContext<'_>) -> String {
    format!("{}:{}", ctx.signature, ctx.instruction_index)
}

fn validate_swap_diffs(
    vault0_diff: i64,
    vault1_diff: i64,
    pool_address: &str,
    vault0_index: usize,
    vault1_index: usize,
) -> Result<()> {
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
    Ok(())
}
