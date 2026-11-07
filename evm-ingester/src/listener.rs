use crate::config::Pool;
use crate::decoder;
use crate::oracle::PriceOracle;
use crate::price::{calculate_swap_price, calculate_volume_usd, non_zero_amount};
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent};
use ethers::abi::Function;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Log};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

// Logs for one block arrive together and confirmation depth keeps them close in time,
// so a small window captures nearly all reuse without growing unbounded.
const BLOCK_TIMESTAMP_CACHE_SIZE: usize = 64;

/// The log-derived and pool-derived fields every published event carries, resolved once
/// per log so each parser only supplies its own payload.
struct EventContext {
    event_id: String,
    block_number: i64,
    block_timestamp: i64,
    transaction_hash: H256,
    log_index: i32,
    event_timestamp: i64,
    chain: String,
    chain_name: String,
    pair_address: String,
    token0: String,
    token1: String,
    token0_symbol: Option<String>,
    token1_symbol: Option<String>,
}

pub struct Listener<O: PriceOracle, P: Middleware> {
    chain: String,
    chain_name: String,
    pool: Pool,
    oracle: Arc<O>,
    rpc: Arc<RetryableProvider<P>>,
    token0_address: Address,
    token1_address: Address,
    token_decimals_cache: Mutex<HashMap<Address, u8>>,
    block_timestamps: Mutex<HashMap<U64, i64>>,
}

impl<O: PriceOracle, P: Middleware + 'static> Listener<O, P> {
    pub async fn new(
        chain: String,
        chain_name: String,
        pool: Pool,
        oracle: Arc<O>,
        rpc: Arc<RetryableProvider<P>>,
    ) -> Result<Self> {
        let pair_address = Address::from_str(&pool.pair_address)?;
        let (token0_address, token1_address) =
            Self::fetch_token_addresses(&rpc, pair_address).await?;

        Ok(Self {
            chain,
            chain_name,
            pool,
            oracle,
            rpc,
            token0_address,
            token1_address,
            token_decimals_cache: Mutex::new(HashMap::new()),
            block_timestamps: Mutex::new(HashMap::new()),
        })
    }

    /// `Ok(None)` for the topics we deliberately ignore (Sync, Transfer, unknown) — a
    /// normal part of reading a pool's log stream, not a failure.
    pub async fn process_log(&self, log: Log) -> Result<Option<Event>> {
        let decoded = match decoder::decode(&log) {
            Ok(decoded) => decoded,
            Err(e) => {
                tracing::warn!(
                    pool = %self.pool.pair_address,
                    tx = ?log.transaction_hash,
                    block = ?log.block_number,
                    topics = ?log.topics,
                    data_hex = %hex::encode(&log.data),
                    data_len = log.data.len(),
                    "Failed to decode pool log: {:#}",
                    e
                );
                return Err(e);
            }
        };

        let Some(decoded) = decoded else {
            tracing::trace!(
                pool = %self.pool.pair_address,
                topic = ?log.topics.first(),
                "Skipping unhandled event type"
            );
            return Ok(None);
        };

        // Every modelled event needs the same log-derived identity, so resolve it once.
        let context = self.event_context(&log).await?;
        let event = match decoded {
            decoder::PoolLog::Swap(swap) => self.parse_swap_event(swap, context).await?,
            decoder::PoolLog::Mint(mint) => Self::parse_mint_event(mint, context),
            decoder::PoolLog::Burn(burn) => Self::parse_burn_event(burn, context),
        };

        Ok(Some(event))
    }

    /// The fields every event shares, resolved from the log once rather than per event type.
    async fn event_context(&self, log: &Log) -> Result<EventContext> {
        let block_number = log.block_number.context("log missing block number")?;
        let transaction_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let log_index = log.log_index.context("log missing log index")?;

        Ok(EventContext {
            event_id: format!("{:?}:{}", transaction_hash, log_index),
            block_number: block_number.as_u64() as i64,
            block_timestamp: self.fetch_block_timestamp(block_number).await?,
            transaction_hash,
            log_index: log_index.as_u32() as i32,
            event_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs() as i64)
                .unwrap_or_default(),
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", self.token0_address),
            token1: format!("{:?}", self.token1_address),
            token0_symbol: None,
            token1_symbol: None,
        })
    }

    async fn parse_swap_event(
        &self,
        swap: decoder::SwapLog,
        context: EventContext,
    ) -> Result<Event> {
        // Three independent lookups; running them in sequence would multiply per-log latency.
        let (token0_decimals, token1_decimals, gas_used) = tokio::join!(
            self.fetch_token_decimals(self.token0_address),
            self.fetch_token_decimals(self.token1_address),
            self.fetch_gas_used(context.transaction_hash),
        );
        let token0_decimals = token0_decimals?;
        let token1_decimals = token1_decimals?;

        let price = calculate_swap_price(
            &swap.amount0_in,
            &swap.amount1_in,
            &swap.amount0_out,
            &swap.amount1_out,
            token0_decimals,
            token1_decimals,
        );

        let volume_usd = self
            .swap_volume_usd(&swap, token0_decimals, token1_decimals, price)
            .await;

        Ok(Event::Swap(SwapEvent {
            event_id: context.event_id,
            block_number: context.block_number,
            block_timestamp: context.block_timestamp,
            transaction_hash: format!("{:?}", context.transaction_hash),
            log_index: context.log_index,
            chain: context.chain,
            chain_name: context.chain_name,
            pair_address: context.pair_address,
            token0: context.token0,
            token1: context.token1,
            token0_symbol: context.token0_symbol,
            token1_symbol: context.token1_symbol,
            sender: format!("{:?}", swap.sender),
            recipient: format!("{:?}", swap.recipient),
            amount0_in: swap.amount0_in.to_string(),
            amount1_in: swap.amount1_in.to_string(),
            amount0_out: swap.amount0_out.to_string(),
            amount1_out: swap.amount1_out.to_string(),
            price,
            volume_usd,
            gas_used,
            gas_price: "0".to_string(),
            event_timestamp: context.event_timestamp,
        }))
    }

    fn parse_mint_event(mint: decoder::MintLog, context: EventContext) -> Event {
        Event::Mint(MintEvent {
            event_id: context.event_id,
            block_number: context.block_number,
            block_timestamp: context.block_timestamp,
            transaction_hash: format!("{:?}", context.transaction_hash),
            log_index: context.log_index,
            chain: context.chain,
            chain_name: context.chain_name,
            pair_address: context.pair_address,
            token0: context.token0,
            token1: context.token1,
            token0_symbol: context.token0_symbol,
            token1_symbol: context.token1_symbol,
            sender: format!("{:?}", mint.sender),
            amount0: mint.amount0.to_string(),
            amount1: mint.amount1.to_string(),
            event_timestamp: context.event_timestamp,
        })
    }

    fn parse_burn_event(burn: decoder::BurnLog, context: EventContext) -> Event {
        Event::Burn(BurnEvent {
            event_id: context.event_id,
            block_number: context.block_number,
            block_timestamp: context.block_timestamp,
            transaction_hash: format!("{:?}", context.transaction_hash),
            log_index: context.log_index,
            chain: context.chain,
            chain_name: context.chain_name,
            pair_address: context.pair_address,
            token0: context.token0,
            token1: context.token1,
            token0_symbol: context.token0_symbol,
            token1_symbol: context.token1_symbol,
            sender: format!("{:?}", burn.sender),
            recipient: format!("{:?}", burn.recipient),
            amount0: burn.amount0.to_string(),
            amount1: burn.amount1.to_string(),
            event_timestamp: context.event_timestamp,
        })
    }

    /// The pair never changes, so its tokens are resolved once rather than per log.
    async fn fetch_token_addresses(
        rpc: &RetryableProvider<P>,
        pair_address: Address,
    ) -> Result<(Address, Address)> {
        let token0_call = decoder::pair_view_call("token0")?;
        let token1_call = decoder::pair_view_call("token1")?;
        let token0_request = view_call(pair_address, token0_call)?;
        let token1_request = view_call(pair_address, token1_call)?;

        let (token0_result, token1_result) = tokio::try_join!(
            rpc.inner().call(&token0_request, None),
            rpc.inner().call(&token1_request, None)
        )
        .context("failed to call token0/token1")?;

        Ok((
            decode_address(token0_call, &token0_result)?,
            decode_address(token1_call, &token1_result)?,
        ))
    }

    async fn fetch_token_decimals(&self, token_address: Address) -> Result<u8> {
        let cached = self
            .lock(&self.token_decimals_cache)
            .get(&token_address)
            .copied();
        if let Some(decimals) = cached {
            return Ok(decimals);
        }

        let decimals_call = decoder::decimals_call();
        let result = self
            .rpc
            .inner()
            .call(&view_call(token_address, decimals_call)?, None)
            .await
            .context("failed to call decimals()")?;

        let value = decimals_call
            .decode_output(&result)
            .context("failed to decode decimals() output")?
            .into_iter()
            .next()
            .context("decimals() returned no value")?
            .into_uint()
            .context("decimals() did not return a uint")?;
        if value > U256::from(u8::MAX) {
            anyhow::bail!("decimals() out of range: {}", value);
        }
        let decimals = value.as_u32() as u8;

        self.lock(&self.token_decimals_cache)
            .insert(token_address, decimals);

        Ok(decimals)
    }

    /// Gas is metadata: a failed receipt lookup degrades the field, it does not lose the swap.
    async fn fetch_gas_used(&self, tx_hash: H256) -> i64 {
        match self.rpc.get_transaction_receipt(tx_hash).await {
            Ok(Some(receipt)) => receipt.gas_used.map(|gas| gas.as_u64()).unwrap_or(0) as i64,
            Ok(None) => {
                tracing::debug!(tx = ?tx_hash, "transaction receipt not found");
                0
            }
            Err(e) => {
                tracing::debug!(tx = ?tx_hash, "failed to fetch transaction receipt: {:#}", e);
                0
            }
        }
    }

    async fn fetch_block_timestamp(&self, block_number: U64) -> Result<i64> {
        let cached = self
            .lock(&self.block_timestamps)
            .get(&block_number)
            .copied();
        if let Some(timestamp) = cached {
            return Ok(timestamp);
        }

        let block = self
            .rpc
            .get_block(block_number)
            .await
            .context("failed to fetch block")?
            .context("block not found")?;
        let timestamp = block.timestamp.as_u64() as i64;

        let mut cache = self.lock(&self.block_timestamps);
        if cache.len() >= BLOCK_TIMESTAMP_CACHE_SIZE {
            cache.clear();
        }
        cache.insert(block_number, timestamp);

        Ok(timestamp)
    }

    /// A poisoned lock still holds usable data, so recover the guard rather than panic.
    fn lock<'a, T>(&self, mutex: &'a Mutex<T>) -> MutexGuard<'a, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Fetches both quotes, then defers the precedence decision to `price`.
    async fn swap_volume_usd(
        &self,
        swap: &decoder::SwapLog,
        token0_decimals: u8,
        token1_decimals: u8,
        swap_price: f64,
    ) -> Option<f64> {
        let (token0_price, token1_price) = tokio::join!(
            self.oracle.fetch_price(self.token0_address),
            self.oracle.fetch_price(self.token1_address)
        );

        calculate_volume_usd(
            &non_zero_amount(&swap.amount0_in, &swap.amount0_out),
            &non_zero_amount(&swap.amount1_in, &swap.amount1_out),
            token0_decimals,
            token1_decimals,
            token0_price,
            token1_price,
            swap_price,
        )
    }
}

/// Selector and calldata come from the ABI, so a renamed or re-typed function cannot drift
/// silently the way a hand-rolled keccak literal can.
fn view_call(to: Address, function: &Function) -> Result<TypedTransaction> {
    let data = function
        .encode_input(&[])
        .with_context(|| format!("failed to encode {} calldata", function.name))?;
    Ok(TypedTransaction::Legacy(TransactionRequest {
        to: Some(NameOrAddress::Address(to)),
        data: Some(Bytes::from(data)),
        ..Default::default()
    }))
}

fn decode_address(function: &Function, output: &Bytes) -> Result<Address> {
    function
        .decode_output(output)
        .with_context(|| format!("failed to decode {} output", function.name))?
        .into_iter()
        .next()
        .with_context(|| format!("{} returned no value", function.name))?
        .into_address()
        .with_context(|| format!("{} did not return an address", function.name))
}
