use crate::config::Pool;
use crate::decoder;
use crate::oracle::PriceOracle;
use crate::price::{calculate_swap_price, non_zero_amount, to_usd};
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent};
use ethers::prelude::*;
use ethers::types::{Address, Log};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

// Logs for one block arrive together and confirmation depth keeps them close in time,
// so a small window captures nearly all reuse without growing unbounded.
const BLOCK_TIMESTAMP_CACHE_SIZE: usize = 64;

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
        let Some(&topic) = log.topics.first() else {
            tracing::debug!(pool = %self.pool.pair_address, "Skipping log with no topics");
            return Ok(None);
        };

        tracing::trace!(
            pool = %self.pool.pair_address,
            data_len = log.data.len(),
            block = ?log.block_number,
            tx = ?log.transaction_hash,
            "Processing log"
        );

        let event = if topic == decoder::swap_event_topic() {
            self.parse_swap_event(log).await?
        } else if topic == decoder::mint_event_topic() {
            self.parse_mint_event(log).await?
        } else if topic == decoder::burn_event_topic() {
            self.parse_burn_event(log).await?
        } else {
            tracing::trace!(pool = %self.pool.pair_address, topic = ?topic, "Skipping unhandled event type");
            return Ok(None);
        };

        Ok(Some(event))
    }

    async fn parse_swap_event(&self, log: Log) -> Result<Event> {
        let swap_data = decoder::parse_swap_log(&log)?;
        let block_number = log.block_number.context("log missing block number")?;
        let tx_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let (event_id, log_index, event_timestamp) = log_fields(&log)?;

        // Four independent lookups; running them in sequence would multiply per-log latency.
        let (block_timestamp, token0_decimals, token1_decimals, gas_used) = tokio::join!(
            self.fetch_block_timestamp(block_number),
            self.fetch_token_decimals(self.token0_address),
            self.fetch_token_decimals(self.token1_address),
            self.fetch_gas_used(tx_hash),
        );
        let block_timestamp = block_timestamp?;
        let token0_decimals = token0_decimals?;
        let token1_decimals = token1_decimals?;

        let price = calculate_swap_price(
            &swap_data.amount0_in,
            &swap_data.amount1_in,
            &swap_data.amount0_out,
            &swap_data.amount1_out,
            token0_decimals,
            token1_decimals,
        );

        let volume_usd = self
            .calculate_volume_usd(&swap_data, token0_decimals, token1_decimals, price)
            .await;

        Ok(Event::Swap(SwapEvent {
            event_id,
            block_number: block_number.as_u64() as i64,
            block_timestamp,
            transaction_hash: format!("{:?}", tx_hash),
            log_index,
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", self.token0_address),
            token1: format!("{:?}", self.token1_address),
            token0_symbol: None,
            token1_symbol: None,
            sender: format!("{:?}", swap_data.sender),
            recipient: format!("{:?}", swap_data.recipient),
            amount0_in: swap_data.amount0_in.to_string(),
            amount1_in: swap_data.amount1_in.to_string(),
            amount0_out: swap_data.amount0_out.to_string(),
            amount1_out: swap_data.amount1_out.to_string(),
            price,
            volume_usd,
            gas_used,
            gas_price: "0".to_string(),
            event_timestamp,
        }))
    }

    async fn parse_mint_event(&self, log: Log) -> Result<Event> {
        let mint_data = decoder::parse_mint_log(&log)?;
        let block_number = log.block_number.context("log missing block number")?;
        let (event_id, log_index, event_timestamp) = log_fields(&log)?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        Ok(Event::Mint(MintEvent {
            event_id,
            block_number: block_number.as_u64() as i64,
            block_timestamp,
            transaction_hash: format!(
                "{:?}",
                log.transaction_hash
                    .context("log missing transaction hash")?
            ),
            log_index,
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", self.token0_address),
            token1: format!("{:?}", self.token1_address),
            token0_symbol: None,
            token1_symbol: None,
            sender: format!("{:?}", mint_data.sender),
            amount0: mint_data.amount0.to_string(),
            amount1: mint_data.amount1.to_string(),
            event_timestamp,
        }))
    }

    async fn parse_burn_event(&self, log: Log) -> Result<Event> {
        let burn_data = decoder::parse_burn_log(&log)?;
        let block_number = log.block_number.context("log missing block number")?;
        let (event_id, log_index, event_timestamp) = log_fields(&log)?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        Ok(Event::Burn(BurnEvent {
            event_id,
            block_number: block_number.as_u64() as i64,
            block_timestamp,
            transaction_hash: format!(
                "{:?}",
                log.transaction_hash
                    .context("log missing transaction hash")?
            ),
            log_index,
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", self.token0_address),
            token1: format!("{:?}", self.token1_address),
            token0_symbol: None,
            token1_symbol: None,
            sender: format!("{:?}", burn_data.sender),
            recipient: format!("{:?}", burn_data.recipient),
            amount0: burn_data.amount0.to_string(),
            amount1: burn_data.amount1.to_string(),
            event_timestamp,
        }))
    }

    /// The pair never changes, so its tokens are resolved once rather than per log.
    async fn fetch_token_addresses(
        rpc: &RetryableProvider<P>,
        pair_address: Address,
    ) -> Result<(Address, Address)> {
        let token0_selector = &ethers::utils::keccak256(b"token0()")[..4];
        let token1_selector = &ethers::utils::keccak256(b"token1()")[..4];

        let call_token0 = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
            ethers::types::TransactionRequest {
                to: Some(ethers::types::NameOrAddress::Address(pair_address)),
                data: Some(ethers::types::Bytes::from(token0_selector.to_vec())),
                ..Default::default()
            },
        );

        let call_token1 = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
            ethers::types::TransactionRequest {
                to: Some(ethers::types::NameOrAddress::Address(pair_address)),
                data: Some(ethers::types::Bytes::from(token1_selector.to_vec())),
                ..Default::default()
            },
        );

        let (token0_result, token1_result) = tokio::try_join!(
            rpc.inner().call(&call_token0, None),
            rpc.inner().call(&call_token1, None)
        )
        .context("failed to call token0/token1")?;

        if token0_result.len() < 32 || token1_result.len() < 32 {
            anyhow::bail!("invalid response length from token0/token1 calls");
        }

        Ok((
            Address::from_slice(&token0_result[12..32]),
            Address::from_slice(&token1_result[12..32]),
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

        let decimals_selector = &ethers::utils::keccak256(b"decimals()")[..4];
        let call = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
            ethers::types::TransactionRequest {
                to: Some(ethers::types::NameOrAddress::Address(token_address)),
                data: Some(ethers::types::Bytes::from(decimals_selector.to_vec())),
                ..Default::default()
            },
        );

        let result = self
            .rpc
            .inner()
            .call(&call, None)
            .await
            .context("failed to call decimals()")?;

        // decimals() returns uint8, zero-padded to a full word.
        let decimals = match result.len() {
            32 => result[31],
            1 => result[0],
            len => anyhow::bail!("unexpected decimals() response length: {}", len),
        };

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

    /// Prefers a stablecoin quote, then a priced token, then the swap's own ratio.
    async fn calculate_volume_usd(
        &self,
        swap: &decoder::SwapEventData,
        token0_decimals: u8,
        token1_decimals: u8,
        price: f64,
    ) -> Option<f64> {
        let (token0_price, token1_price) = tokio::join!(
            self.oracle.fetch_price(self.token0_address),
            self.oracle.fetch_price(self.token1_address)
        );

        let volume0 = non_zero_amount(&swap.amount0_in, &swap.amount0_out);
        let volume1 = non_zero_amount(&swap.amount1_in, &swap.amount1_out);

        match (token0_price, token1_price) {
            (_, Some(price1)) if price1 == 1.0 => Some(to_usd(&volume1, token1_decimals, 1.0)),
            (Some(price0), _) if price0 == 1.0 => Some(to_usd(&volume0, token0_decimals, 1.0)),
            (Some(price0), _) if price0 > 0.0 => Some(to_usd(&volume0, token0_decimals, price0)),
            (_, Some(price1)) if price1 > 0.0 => Some(to_usd(&volume1, token1_decimals, price1)),
            _ if price > 0.0 => Some(to_usd(&volume0, token0_decimals, price)),
            _ => None,
        }
    }
}

/// The identity every event shape shares, derived from the log alone.
fn log_fields(log: &Log) -> Result<(String, i32, i64)> {
    let tx_hash = log
        .transaction_hash
        .context("log missing transaction hash")?;
    let log_index = log.log_index.context("log missing log index")?;
    let event_id = format!("{:?}:{}", tx_hash, log_index);
    let event_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default();
    Ok((event_id, log_index.as_u32() as i32, event_timestamp))
}
