use crate::balances;
use crate::config::{Config, PoolConfig};
use crate::instruction_processor::DecodedInstructionContext;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent};
use std::sync::Arc;

pub struct EventBuilder {
    config: Arc<Config>,
    pool: PoolConfig,
}

impl EventBuilder {
    pub fn new(config: Arc<Config>, pool: PoolConfig) -> Self {
        Self { config, pool }
    }

    /// Builds an event from a decoded instruction. Pure transformation - prices are passed in.
    pub fn build(
        &self,
        ctx: &DecodedInstructionContext,
        token0_symbol: Option<String>,
        token1_symbol: Option<String>,
        price: Option<f64>,
    ) -> Result<Event> {
        match ctx.decoded {
            crate::decoder::DecodedInstruction::Swap => {
                self.build_swap(ctx, token0_symbol, token1_symbol, price)
            }
            crate::decoder::DecodedInstruction::Deposit => {
                self.build_mint(ctx, token0_symbol, token1_symbol)
            }
            crate::decoder::DecodedInstruction::Withdraw => {
                self.build_burn(ctx, token0_symbol, token1_symbol)
            }
        }
    }

    fn build_swap(
        &self,
        ctx: &DecodedInstructionContext,
        token0_symbol: Option<String>,
        token1_symbol: Option<String>,
        oracle_price: Option<f64>,
    ) -> Result<Event> {
        let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
            &ctx.pre_balances,
            &ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute balance diffs for swap")?;

        validate_swap_diffs(
            vault0_diff,
            vault1_diff,
            &ctx.pool_address,
            ctx.vault0_index,
            ctx.vault1_index,
        )?;

        let (amount0_in, amount1_in, amount0_out, amount1_out) =
            balances::extract_swap_amounts(vault0_diff, vault1_diff);

        let (decimals0, decimals1) =
            balances::vault_decimals(&ctx.pre_balances, ctx.vault0_index, ctx.vault1_index)
                .context("Failed to read vault token decimals")?;

        let price = balances::swap_price(
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            decimals0,
            decimals1,
        );

        let volume_usd =
            self.calculate_volume_usd(amount0_in, amount1_in, decimals0, decimals1, oracle_price);

        let event_timestamp = chrono::Utc::now().timestamp();

        Ok(Event::Swap(SwapEvent {
            event_id: format!("{}:{}", ctx.signature, ctx.instruction_index),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.clone(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol,
            token1_symbol,
            sender: ctx.user_authority.clone(),
            recipient: ctx.user_authority.clone(),
            amount0_in: amount0_in.to_string(),
            amount1_in: amount1_in.to_string(),
            amount0_out: amount0_out.to_string(),
            amount1_out: amount1_out.to_string(),
            price,
            volume_usd,
            gas_used: match &ctx.meta.compute_units_consumed {
                solana_transaction_status::option_serializer::OptionSerializer::Some(units) => {
                    *units as i64
                }
                _ => 0,
            },
            gas_price: "0".to_string(),
            event_timestamp,
        }))
    }

    fn build_mint(
        &self,
        ctx: &DecodedInstructionContext,
        token0_symbol: Option<String>,
        token1_symbol: Option<String>,
    ) -> Result<Event> {
        let (amount0, amount1) = balances::compute_liquidity_amounts(
            &ctx.pre_balances,
            &ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute liquidity amounts")?;

        let event_timestamp = chrono::Utc::now().timestamp();

        Ok(Event::Mint(MintEvent {
            event_id: format!("{}:{}", ctx.signature, ctx.instruction_index),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.clone(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol,
            token1_symbol,
            sender: ctx.user_authority.clone(),
            amount0: amount0.to_string(),
            amount1: amount1.to_string(),
            event_timestamp,
        }))
    }

    fn build_burn(
        &self,
        ctx: &DecodedInstructionContext,
        token0_symbol: Option<String>,
        token1_symbol: Option<String>,
    ) -> Result<Event> {
        let (amount0, amount1) = balances::compute_liquidity_amounts(
            &ctx.pre_balances,
            &ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )
        .context("Failed to compute liquidity amounts")?;

        let event_timestamp = chrono::Utc::now().timestamp();

        Ok(Event::Burn(BurnEvent {
            event_id: format!("{}:{}", ctx.signature, ctx.instruction_index),
            block_number: ctx.slot as i64,
            block_timestamp: ctx.block_time,
            transaction_hash: ctx.signature.to_string(),
            log_index: ctx.instruction_index as i32,
            chain: self.config.solana.chain.clone(),
            chain_name: self.config.solana.chain_name.clone(),
            pair_address: ctx.pool_address.clone(),
            token0: self.pool.token0.clone(),
            token1: self.pool.token1.clone(),
            token0_symbol,
            token1_symbol,
            sender: ctx.user_authority.clone(),
            recipient: ctx.user_authority.clone(),
            amount0: amount0.to_string(),
            amount1: amount1.to_string(),
            event_timestamp,
        }))
    }

    /// Only the sold side needs a quote: `swap_price` already prices the bought side,
    /// so a second lookup would fetch a number that is never read.
    fn calculate_volume_usd(
        &self,
        amount0_in: u64,
        amount1_in: u64,
        decimals0: u8,
        decimals1: u8,
        oracle_price: Option<f64>,
    ) -> Option<f64> {
        let (amount, decimals) = if amount0_in > 0 {
            (amount0_in, decimals0)
        } else {
            (amount1_in, decimals1)
        };

        let price = oracle_price.unwrap_or(0.0);
        balances::volume_usd(amount, decimals, price)
    }
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
