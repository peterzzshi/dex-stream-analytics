use crate::config::Pool;
use crate::decoder;
use crate::oracle::PriceOracle;
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
use dex_ingester_common::{BurnEvent, Event, MintEvent, SwapEvent, TransferEvent};
use ethers::prelude::*;
use ethers::types::{Address, Log, U256};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Listener<O: PriceOracle, P: Middleware> {
    chain: String,
    chain_name: String,
    pool: Pool,
    oracle: Arc<O>,
    rpc: Arc<RetryableProvider<P>>,
    token0_address: Address,
    token1_address: Address,
    token_decimals_cache: std::sync::Mutex<std::collections::HashMap<Address, u8>>,
    block_timestamps: std::sync::Mutex<std::collections::HashMap<U64, i64>>,
}

// Logs for one block arrive together and confirmation depth keeps them close in time,
// so a small window captures nearly all reuse without growing unbounded.
const BLOCK_TIMESTAMP_CACHE_SIZE: usize = 64;

impl<O: PriceOracle, P: Middleware + 'static> Listener<O, P> {
    pub async fn new(
        chain: String,
        chain_name: String,
        pool: Pool,
        oracle: Arc<O>,
        rpc: Arc<RetryableProvider<P>>,
    ) -> Result<Self> {
        let pair_address = Address::from_str(&pool.pair_address)?;
        let mut listener = Self {
            chain,
            chain_name,
            pool,
            oracle,
            rpc,
            token0_address: Address::zero(),
            token1_address: Address::zero(),
            token_decimals_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            block_timestamps: std::sync::Mutex::new(std::collections::HashMap::new()),
        };

        // The pair never changes, so its tokens are resolved once rather than per log.
        let (token0_address, token1_address) = listener.fetch_token_addresses(pair_address).await?;
        listener.token0_address = token0_address;
        listener.token1_address = token1_address;

        Ok(listener)
    }

    /// Process a log and return an event if it's a supported type.
    /// Logs and skips events we want to ignore (Sync, Transfer, unknown topics).
    pub async fn process_log(&self, log: Log) -> Result<Event> {
        if log.topics.is_empty() {
            anyhow::bail!("log has no topics");
        }

        let topic = log.topics[0];

        tracing::debug!(
            pool = %self.pool.pair_address,
            "Processing log: topics={:?}, data_len={}, block={:?}, tx={:?}",
            log.topics,
            log.data.len(),
            log.block_number,
            log.transaction_hash
        );

        match topic {
            t if t == decoder::swap_event_topic() => self.parse_swap_event(log).await,
            t if t == decoder::mint_event_topic() => self.parse_mint_event(log).await,
            t if t == decoder::burn_event_topic() => self.parse_burn_event(log).await,
            t if t == decoder::sync_event_topic() => {
                tracing::trace!(pool = %self.pool.pair_address, "Ignoring Sync event");
                anyhow::bail!("sync event (ignored)")
            }
            t if t == decoder::transfer_event_topic() => {
                tracing::trace!(pool = %self.pool.pair_address, "Ignoring Transfer event");
                anyhow::bail!("transfer event (ignored)")
            }
            _ => {
                tracing::debug!(
                    pool = %self.pool.pair_address,
                    topic = ?topic,
                    "Skipping unknown event type"
                );
                anyhow::bail!("unknown event type")
            }
        }
    }

    async fn parse_swap_event(&self, log: Log) -> Result<Event> {
        let swap_data = decoder::parse_swap_log(&log)?;

        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;
        let (event_id, log_index, event_timestamp) = self.extract_log_fields(&log)?;

        let (token0_address, token1_address) = (self.token0_address, self.token1_address);

        let (token0_decimals, token1_decimals) = tokio::try_join!(
            self.fetch_token_decimals(token0_address),
            self.fetch_token_decimals(token1_address)
        )?;

        let price = calculate_swap_price(
            &swap_data.amount0_in,
            &swap_data.amount1_in,
            &swap_data.amount0_out,
            &swap_data.amount1_out,
            token0_decimals,
            token1_decimals,
        );

        let volume_usd = self
            .calculate_volume_usd(
                &swap_data.amount0_in,
                &swap_data.amount1_in,
                &swap_data.amount0_out,
                &swap_data.amount1_out,
                token0_address,
                token1_address,
                token0_decimals,
                token1_decimals,
                price,
            )
            .await;

        let tx_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let gas_used = self.fetch_gas_used(tx_hash).await.unwrap_or(0) as i64;

        Ok(Event::Swap(SwapEvent {
            event_id,
            block_number: block_number.as_u64() as i64,
            block_timestamp,
            transaction_hash: format!("{:?}", tx_hash),
            log_index,
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", token0_address),
            token1: format!("{:?}", token1_address),
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
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;
        let (event_id, log_index, event_timestamp) = self.extract_log_fields(&log)?;

        let (token0_address, token1_address) = (self.token0_address, self.token1_address);

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
            token0: format!("{:?}", token0_address),
            token1: format!("{:?}", token1_address),
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
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;
        let (event_id, log_index, event_timestamp) = self.extract_log_fields(&log)?;

        let (token0_address, token1_address) = (self.token0_address, self.token1_address);

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
            token0: format!("{:?}", token0_address),
            token1: format!("{:?}", token1_address),
            token0_symbol: None,
            token1_symbol: None,
            sender: format!("{:?}", burn_data.sender),
            recipient: format!("{:?}", burn_data.recipient),
            amount0: burn_data.amount0.to_string(),
            amount1: burn_data.amount1.to_string(),
            event_timestamp,
        }))
    }

    fn extract_log_fields(&self, log: &Log) -> Result<(String, i32, i64)> {
        let tx_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let log_index = log.log_index.context("log missing log index")?;
        let event_id = format!("{:?}:{}", tx_hash, log_index);
        let event_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Ok((event_id, log_index.as_u32() as i32, event_timestamp))
    }

    async fn fetch_token_addresses(&self, pair_address: Address) -> Result<(Address, Address)> {
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
            self.rpc.inner().call(&call_token0, None),
            self.rpc.inner().call(&call_token1, None)
        )
        .context("failed to call token0/token1")?;

        if token0_result.len() < 32 || token1_result.len() < 32 {
            anyhow::bail!("invalid response length from token0/token1 calls");
        }

        let token0_address = Address::from_slice(&token0_result[12..32]);
        let token1_address = Address::from_slice(&token1_result[12..32]);

        Ok((token0_address, token1_address))
    }

    async fn fetch_token_decimals(&self, token_address: Address) -> Result<u8> {
        // Check cache first
        {
            let cache = self.token_decimals_cache.lock().unwrap();
            if let Some(&decimals) = cache.get(&token_address) {
                return Ok(decimals);
            }
        }

        // Call decimals() on the token contract
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

        if result.is_empty() {
            anyhow::bail!("decimals() returned empty response");
        }

        // decimals() returns uint8, which is padded to 32 bytes
        let decimals = if result.len() == 32 {
            result[31]
        } else if result.len() == 1 {
            result[0]
        } else {
            anyhow::bail!("unexpected decimals() response length: {}", result.len());
        };

        // Cache the result
        {
            let mut cache = self.token_decimals_cache.lock().unwrap();
            cache.insert(token_address, decimals);
        }

        Ok(decimals)
    }

    async fn fetch_gas_used(&self, tx_hash: H256) -> Result<u64> {
        let receipt = self
            .rpc
            .get_transaction_receipt(tx_hash)
            .await
            .context("failed to fetch transaction receipt")?;

        match receipt {
            Some(r) => Ok(r.gas_used.map(|g| g.as_u64()).unwrap_or(0)),
            None => anyhow::bail!("transaction receipt not found"),
        }
    }

    async fn fetch_block_timestamp(&self, block_number: U64) -> Result<i64> {
        {
            let cache = self.block_timestamps.lock().unwrap();
            if let Some(&timestamp) = cache.get(&block_number) {
                return Ok(timestamp);
            }
        }

        let block = self
            .rpc
            .get_block(block_number)
            .await
            .context("failed to fetch block")?;

        let timestamp = match block {
            Some(b) => b.timestamp.as_u64() as i64,
            None => anyhow::bail!("block not found"),
        };

        {
            let mut cache = self.block_timestamps.lock().unwrap();
            if cache.len() >= BLOCK_TIMESTAMP_CACHE_SIZE {
                cache.clear();
            }
            cache.insert(block_number, timestamp);
        }

        Ok(timestamp)
    }

    async fn calculate_volume_usd(
        &self,
        amount0_in: &U256,
        amount1_in: &U256,
        amount0_out: &U256,
        amount1_out: &U256,
        token0_address: Address,
        token1_address: Address,
        token0_decimals: u8,
        token1_decimals: u8,
        price: f64,
    ) -> Option<f64> {
        let token0_price = self.oracle.fetch_price(token0_address).await;
        let token1_price = self.oracle.fetch_price(token1_address).await;

        let vol0 = non_zero_amount(amount0_in, amount0_out);
        let vol1 = non_zero_amount(amount1_in, amount1_out);

        if let Some(1.0) = token1_price {
            return Some(to_usd(&vol1, token1_decimals, 1.0));
        }
        if let Some(1.0) = token0_price {
            return Some(to_usd(&vol0, token0_decimals, 1.0));
        }
        if let Some(p0) = token0_price {
            if p0 > 0.0 {
                return Some(to_usd(&vol0, token0_decimals, p0));
            }
        }
        if let Some(p1) = token1_price {
            if p1 > 0.0 {
                return Some(to_usd(&vol1, token1_decimals, p1));
            }
        }
        if price > 0.0 {
            return Some(to_usd(&vol0, token0_decimals, price));
        }

        None
    }
}

fn calculate_swap_price(
    amount0_in: &U256,
    amount1_in: &U256,
    amount0_out: &U256,
    amount1_out: &U256,
    token0_decimals: u8,
    token1_decimals: u8,
) -> f64 {
    if !amount0_in.is_zero() && !amount1_out.is_zero() {
        return ratio_to_f64(amount1_out, amount0_in, token1_decimals, token0_decimals);
    }
    if !amount1_in.is_zero() && !amount0_out.is_zero() {
        return ratio_to_f64(amount1_in, amount0_out, token1_decimals, token0_decimals);
    }
    0.0
}

fn ratio_to_f64(numerator: &U256, denominator: &U256, num_decimals: u8, denom_decimals: u8) -> f64 {
    if denominator.is_zero() {
        return 0.0;
    }

    let decimal_diff = denom_decimals as i32 - num_decimals as i32;
    let num_f64 = u256_to_f64(numerator);
    let denom_f64 = u256_to_f64(denominator);

    if denom_f64 == 0.0 {
        return 0.0;
    }

    (num_f64 / denom_f64) * 10_f64.powi(decimal_diff)
}

fn u256_to_f64(value: &U256) -> f64 {
    let mut result = 0.0;
    let mut multiplier = 1.0;

    for i in 0..4 {
        let word = value.0[i];
        result += word as f64 * multiplier;
        multiplier *= 2_f64.powi(64);
    }

    result
}

fn non_zero_amount(amount_in: &U256, amount_out: &U256) -> U256 {
    if !amount_in.is_zero() {
        *amount_in
    } else {
        *amount_out
    }
}

fn to_usd(amount: &U256, decimals: u8, usd_price: f64) -> f64 {
    (u256_to_f64(amount) / 10_f64.powi(decimals as i32)) * usd_price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_swap_price_token0_to_token1() {
        let amount0_in = U256::from(1000000000000000000u64);
        let amount1_in = U256::zero();
        let amount0_out = U256::zero();
        let amount1_out = U256::from(2000000u64);

        let price =
            calculate_swap_price(&amount0_in, &amount1_in, &amount0_out, &amount1_out, 18, 6);
        assert!((price - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_swap_price_token1_to_token0() {
        let amount0_in = U256::zero();
        let amount1_in = U256::from(2000000u64);
        let amount0_out = U256::from(1000000000000000000u64);
        let amount1_out = U256::zero();

        let price =
            calculate_swap_price(&amount0_in, &amount1_in, &amount0_out, &amount1_out, 18, 6);
        assert!((price - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_non_zero_amount() {
        let zero = U256::zero();
        let non_zero = U256::from(100);

        assert_eq!(non_zero_amount(&non_zero, &zero), non_zero);
        assert_eq!(non_zero_amount(&zero, &non_zero), non_zero);
        assert_eq!(non_zero_amount(&zero, &zero), zero);
    }

    #[test]
    fn test_to_usd() {
        let amount = U256::from(1000000u64);
        let usd = to_usd(&amount, 6, 1.0);
        assert!((usd - 1.0).abs() < 0.0001);

        let amount_eth = U256::from(1000000000000000000u64);
        let usd_eth = to_usd(&amount_eth, 18, 2000.0);
        assert!((usd_eth - 2000.0).abs() < 0.0001);
    }
}
