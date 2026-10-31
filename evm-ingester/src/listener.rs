use crate::config::Pool;
use crate::decoder;
use crate::events::{BaseEvent, BurnEvent, Event, EventType, MintEvent, SwapEvent, TransferEvent};
use crate::oracle::PriceOracle;
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
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
}

impl<O: PriceOracle, P: Middleware + 'static> Listener<O, P> {
    pub fn new(
        chain: String,
        chain_name: String,
        pool: Pool,
        oracle: Arc<O>,
        rpc: Arc<RetryableProvider<P>>,
    ) -> Self {
        Self {
            chain,
            chain_name,
            pool,
            oracle,
            rpc,
        }
    }

    pub async fn process_log(&self, log: Log) -> Result<Box<dyn Event>> {
        if log.topics.is_empty() {
            anyhow::bail!("log has no topics");
        }

        let topic = log.topics[0];

        if topic == decoder::swap_event_topic() {
            self.parse_swap_event(log).await
        } else if topic == decoder::mint_event_topic() {
            self.parse_mint_event(log).await
        } else if topic == decoder::burn_event_topic() {
            self.parse_burn_event(log).await
        } else if topic == decoder::transfer_event_topic() {
            self.parse_transfer_event(log).await
        } else {
            anyhow::bail!("unknown event topic: {:?}", topic);
        }
    }

    async fn parse_swap_event(&self, log: Log) -> Result<Box<dyn Event>> {
        let swap_data = decoder::parse_swap_log(&log)?;

        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        let base = self.build_base_event(&log, EventType::Swap, block_timestamp)?;

        let _pair_address = Address::from_str(&self.pool.pair_address)?;

        // Simplified: use 18 decimals for both tokens as default
        let token0_decimals = 18u8;
        let token1_decimals = 18u8;

        let price = calculate_swap_price(
            &swap_data.amount0_in,
            &swap_data.amount1_in,
            &swap_data.amount0_out,
            &swap_data.amount1_out,
            token0_decimals,
            token1_decimals,
        );

        let pair_address = Address::from_str(&self.pool.pair_address)?;
        let (token0_address, token1_address) = self.fetch_token_addresses(pair_address).await?;

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

        let swap_event = SwapEvent {
            base,
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
        };

        Ok(Box::new(swap_event))
    }

    async fn parse_mint_event(&self, log: Log) -> Result<Box<dyn Event>> {
        let mint_data = decoder::parse_mint_log(&log)?;

        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        let base = self.build_base_event(&log, EventType::Mint, block_timestamp)?;

        let mint_event = MintEvent {
            base,
            sender: format!("{:?}", mint_data.sender),
            amount0: mint_data.amount0.to_string(),
            amount1: mint_data.amount1.to_string(),
        };

        Ok(Box::new(mint_event))
    }

    async fn parse_burn_event(&self, log: Log) -> Result<Box<dyn Event>> {
        let burn_data = decoder::parse_burn_log(&log)?;

        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        let base = self.build_base_event(&log, EventType::Burn, block_timestamp)?;

        let burn_event = BurnEvent {
            base,
            sender: format!("{:?}", burn_data.sender),
            recipient: format!("{:?}", burn_data.recipient),
            amount0: burn_data.amount0.to_string(),
            amount1: burn_data.amount1.to_string(),
        };

        Ok(Box::new(burn_event))
    }

    async fn parse_transfer_event(&self, log: Log) -> Result<Box<dyn Event>> {
        let transfer_data = decoder::parse_transfer_log(&log)?;

        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        let base = self.build_base_event(&log, EventType::Transfer, block_timestamp)?;

        let transfer_event = TransferEvent {
            base,
            from: format!("{:?}", transfer_data.from),
            to: format!("{:?}", transfer_data.to),
            value: transfer_data.value.to_string(),
        };

        Ok(Box::new(transfer_event))
    }

    fn build_base_event(
        &self,
        log: &Log,
        event_type: EventType,
        block_timestamp: i64,
    ) -> Result<BaseEvent> {
        let block_number = log.block_number.context("log missing block number")?;
        let tx_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let log_index = log.log_index.context("log missing log index")?;

        let token0_address = Address::zero();
        let token1_address = Address::zero();

        let event_id = format!("{:?}:{}", tx_hash, log_index);
        let event_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(BaseEvent {
            event_type,
            event_id,
            block_number: block_number.as_u64() as i64,
            block_timestamp,
            transaction_hash: format!("{:?}", tx_hash),
            log_index: log_index.as_u32() as i32,
            chain: self.chain.clone(),
            chain_name: self.chain_name.clone(),
            pair_address: self.pool.pair_address.clone(),
            token0: format!("{:?}", token0_address),
            token1: format!("{:?}", token1_address),
            token0_symbol: None,
            token1_symbol: None,
            event_timestamp,
        })
    }

    /// Fetch token0 and token1 addresses from the Uniswap V2 pair contract
    async fn fetch_token_addresses(&self, pair_address: Address) -> Result<(Address, Address)> {
        // token0() selector: 0x0dfe1681
        let token0_selector = &ethers::utils::keccak256(b"token0()")[..4];
        // token1() selector: 0xd21220a7
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

        let token0_result = self
            .rpc
            .inner()
            .call(&call_token0, None)
            .await
            .context("failed to call token0()")?;

        let token1_result = self
            .rpc
            .inner()
            .call(&call_token1, None)
            .await
            .context("failed to call token1()")?;

        // Decode address from 32-byte response (last 20 bytes)
        if token0_result.len() < 32 || token1_result.len() < 32 {
            anyhow::bail!("invalid response length from token0/token1 calls");
        }

        let token0_address = Address::from_slice(&token0_result[12..32]);
        let token1_address = Address::from_slice(&token1_result[12..32]);

        Ok((token0_address, token1_address))
    }

    /// Fetch gas_used from transaction receipt
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

    /// Fetch block timestamp from blockchain
    async fn fetch_block_timestamp(&self, block_number: U64) -> Result<i64> {
        let block = self
            .rpc
            .get_block(block_number)
            .await
            .context("failed to fetch block")?;

        match block {
            Some(b) => Ok(b.timestamp.as_u64() as i64),
            None => anyhow::bail!("block not found"),
        }
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

        // Priority: stablecoin token1 > stablecoin token0 > oracle token0 > oracle token1 > swap price
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

    let ratio = num_f64 / denom_f64;
    let adjustment = 10_f64.powi(decimal_diff);

    ratio * adjustment
}

fn u256_to_f64(value: &U256) -> f64 {
    // Convert U256 to f64 - may lose precision for very large values
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
    let amount_f64 = u256_to_f64(amount);
    let divisor = 10_f64.powi(decimals as i32);
    (amount_f64 / divisor) * usd_price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_swap_price_token0_to_token1() {
        let amount0_in = U256::from(1000000000000000000u64); // 1 token0 (18 decimals)
        let amount1_in = U256::zero();
        let amount0_out = U256::zero();
        let amount1_out = U256::from(2000000u64); // 2 token1 (6 decimals)

        let price =
            calculate_swap_price(&amount0_in, &amount1_in, &amount0_out, &amount1_out, 18, 6);

        // price = amount1_out / amount0_in * 10^(18-6) = 2/1 * 10^12 = 2.0
        assert!((price - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_swap_price_token1_to_token0() {
        let amount0_in = U256::zero();
        let amount1_in = U256::from(2000000u64); // 2 token1 (6 decimals)
        let amount0_out = U256::from(1000000000000000000u64); // 1 token0 (18 decimals)
        let amount1_out = U256::zero();

        let price =
            calculate_swap_price(&amount0_in, &amount1_in, &amount0_out, &amount1_out, 18, 6);

        // price = amount1_in / amount0_out * 10^(18-6) = 2/1 * 10^12 = 2.0
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
        let amount = U256::from(1000000u64); // 1 USDC (6 decimals)
        let usd = to_usd(&amount, 6, 1.0);
        assert!((usd - 1.0).abs() < 0.0001);

        let amount_eth = U256::from(1000000000000000000u64); // 1 ETH (18 decimals)
        let usd_eth = to_usd(&amount_eth, 18, 2000.0);
        assert!((usd_eth - 2000.0).abs() < 0.0001);
    }
}
