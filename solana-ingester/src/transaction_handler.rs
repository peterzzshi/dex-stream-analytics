use crate::config::PoolConfig;
use crate::decoder::DecodedInstruction;
use crate::event_builder::EventBuilder;
use crate::instruction_processor::{InstructionProcessor, TransactionData};
use crate::oracle::PythOracle;
use crate::symbols::SymbolCache;
use crate::transaction_fetcher::TransactionFetcher;
use anyhow::Result;
use dex_ingester_common::Event;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use tracing::info;

/// Orchestrates the full fetch→decode→price→build pipeline for one pool.
/// All I/O happens here, then pure transformations are delegated.
pub struct TransactionHandler {
    fetcher: TransactionFetcher,
    processor: InstructionProcessor,
    builder: EventBuilder,
    oracle: Arc<PythOracle>,
    symbols: Arc<SymbolCache>,
    pool: PoolConfig,
}

impl TransactionHandler {
    pub fn new(
        fetcher: TransactionFetcher,
        processor: InstructionProcessor,
        builder: EventBuilder,
        oracle: Arc<PythOracle>,
        symbols: Arc<SymbolCache>,
        pool: PoolConfig,
    ) -> Self {
        Self {
            fetcher,
            processor,
            builder,
            oracle,
            symbols,
            pool,
        }
    }

    /// Fetches the transaction, decodes instructions, fetches prices & symbols, builds events.
    pub async fn handle(&self, signature: Signature, pool_address: &str) -> Result<Vec<Event>> {
        let Some(parsed) = self.fetcher.fetch(signature, pool_address).await? else {
            return Ok(Vec::new());
        };

        let tx_data = TransactionData {
            signature: &signature,
            slot: parsed.slot,
            block_time: parsed.block_time,
            account_keys: &parsed.account_keys,
            instructions: &parsed.instructions,
            meta: &parsed.meta,
        };

        let decoded = self.processor.decode_instructions(&tx_data, pool_address)?;

        if decoded.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch symbols once for the pool (they're static per pool)
        let (token0_symbol, token1_symbol) = tokio::join!(
            self.symbols.get_symbol(&self.pool.token0),
            self.symbols.get_symbol(&self.pool.token1)
        );

        let mut events = Vec::new();
        for ctx in decoded {
            // Fetch price only for swaps (the side being sold)
            let price = match ctx.decoded {
                DecodedInstruction::Swap => self.fetch_swap_price(&ctx).await?,
                _ => None,
            };

            let event =
                self.builder
                    .build(&ctx, token0_symbol.clone(), token1_symbol.clone(), price)?;

            let volume_usd = match &event {
                Event::Swap(swap) => swap.volume_usd,
                _ => None,
            };
            info!(
                "[{}] Ingested {} tx={} slot={} volume_usd={:?}",
                self.pool.name,
                event.event_type(),
                signature,
                ctx.slot,
                volume_usd
            );

            events.push(event);
        }

        Ok(events)
    }

    async fn fetch_swap_price(
        &self,
        ctx: &crate::instruction_processor::DecodedInstructionContext,
    ) -> Result<Option<f64>> {
        use crate::balances;

        let (vault0_diff, vault1_diff) = balances::compute_balance_diff(
            &ctx.pre_balances,
            &ctx.post_balances,
            ctx.vault0_index,
            ctx.vault1_index,
        )?;

        let (amount0_in, _amount1_in, _, _) =
            balances::extract_swap_amounts(vault0_diff, vault1_diff);

        let token = if amount0_in > 0 {
            &self.pool.token0
        } else {
            &self.pool.token1
        };

        self.oracle.fetch_price(token).await
    }
}
