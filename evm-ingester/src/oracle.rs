use anyhow::{Context, Result};
use ethers::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::OracleConfig;

const STALE_PRICE_THRESHOLD_SECS: u64 = 24 * 60 * 60; // 24 hours

// ABI function selectors
const DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67]; // decimals()
const LATEST_ROUND_DATA_SELECTOR: [u8; 4] = [0xfe, 0xaf, 0x96, 0x8c]; // latestRoundData()

/// Token USD price oracle
pub trait PriceOracle: Send + Sync {
    fn fetch_price(
        &self,
        token_address: Address,
    ) -> Pin<Box<dyn Future<Output = Option<f64>> + Send + '_>>;
}

/// Chainlink-based price oracle using on-chain data feeds
pub struct ChainlinkOracle<P> {
    provider: Arc<P>,
    price_feeds: HashMap<Address, Address>,
    stablecoins: Vec<Address>,
}

impl<P: Middleware + 'static> ChainlinkOracle<P> {
    pub fn new(provider: Arc<P>, config: OracleConfig) -> Self {
        Self {
            provider,
            price_feeds: config.price_feeds,
            stablecoins: config.stablecoins,
        }
    }

    fn is_stablecoin(&self, address: Address) -> bool {
        self.stablecoins.contains(&address)
    }
}

impl<P: Middleware + 'static> PriceOracle for ChainlinkOracle<P> {
    fn fetch_price(
        &self,
        token_address: Address,
    ) -> Pin<Box<dyn Future<Output = Option<f64>> + Send + '_>> {
        Box::pin(async move {
            // Stablecoins are always $1.00
            if self.is_stablecoin(token_address) {
                return Some(1.0);
            }

            let feed_address = self.price_feeds.get(&token_address)?;

            // Call decimals() on the price feed
            let decimals = self.call_decimals(*feed_address).await.ok()?;

            // Call latestRoundData() on the price feed
            let round_data = self.call_latest_round_data(*feed_address).await.ok()?;

            // Reject stale prices (>24h old)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now.saturating_sub(round_data.updated_at) > STALE_PRICE_THRESHOLD_SECS {
                return None;
            }

            Some(scale_price(round_data.answer, decimals))
        })
    }
}

impl<P: Middleware + 'static> ChainlinkOracle<P> {
    async fn call_decimals(&self, feed_address: Address) -> Result<u8> {
        // ABI: decimals() returns (uint8)
        let tx = TransactionRequest::new()
            .to(feed_address)
            .data(Bytes::from(DECIMALS_SELECTOR.to_vec()));

        let call = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("call decimals")?;

        if call.len() != 32 {
            anyhow::bail!("unexpected decimals response length");
        }

        Ok(call[31])
    }

    async fn call_latest_round_data(&self, feed_address: Address) -> Result<RoundData> {
        // ABI: latestRoundData() returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
        let tx = TransactionRequest::new()
            .to(feed_address)
            .data(Bytes::from(LATEST_ROUND_DATA_SELECTOR.to_vec()));

        let call = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("call latestRoundData")?;

        if call.len() != 160 {
            anyhow::bail!("unexpected latestRoundData response length");
        }

        // Parse response: skip roundId (offset 0), extract answer (offset 32), skip startedAt (offset 64), extract updatedAt (offset 96)
        let answer = I256::from_raw(U256::from_big_endian(&call[32..64]));
        let updated_at = U256::from_big_endian(&call[96..128]).as_u64();

        Ok(RoundData {
            answer: answer.as_i64(),
            updated_at,
        })
    }
}

#[derive(Debug)]
struct RoundData {
    answer: i64,
    updated_at: u64,
}

/// Convert Chainlink price (with decimals) to float64
fn scale_price(answer: i64, decimals: u8) -> f64 {
    let divisor = 10_f64.powi(decimals as i32);
    answer as f64 / divisor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_price() {
        assert_eq!(scale_price(100000000, 8), 1.0);
        assert_eq!(scale_price(200000000, 8), 2.0);
        assert_eq!(scale_price(150000000, 8), 1.5);
        assert_eq!(scale_price(1000, 3), 1.0);
    }
}
