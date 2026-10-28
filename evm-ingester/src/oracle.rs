use anyhow::{Context, Result};
use ethers::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            price_feeds: Self::default_price_feeds(),
            stablecoins: Self::default_stablecoins(),
        }
    }

    /// Default Chainlink price feeds for Polygon mainnet
    fn default_price_feeds() -> HashMap<Address, Address> {
        let mut feeds = HashMap::new();

        // WMATIC/USD
        feeds.insert(
            "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"
                .parse()
                .unwrap(),
            "0xab594600376ec9fd91f8e885dadf0ce036862de0"
                .parse()
                .unwrap(),
        );

        // WETH/USD
        feeds.insert(
            "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
                .parse()
                .unwrap(),
            "0xf9680d99d6c9589e2a93a78a04a279e509205945"
                .parse()
                .unwrap(),
        );

        // WBTC/USD
        feeds.insert(
            "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6"
                .parse()
                .unwrap(),
            "0xc907e116054ad103354f2d350fd2514433d57f6f"
                .parse()
                .unwrap(),
        );

        // USDC/USD
        feeds.insert(
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
                .parse()
                .unwrap(),
            "0xfe4a8cc5b5b2366c1b58bea3858e81843581b2f7"
                .parse()
                .unwrap(),
        );

        // USDT/USD
        feeds.insert(
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
                .parse()
                .unwrap(),
            "0x0a6513e40db6eb1b165753ad52e80663aea50545"
                .parse()
                .unwrap(),
        );

        // DAI/USD
        feeds.insert(
            "0x8f3cf7ad23cd3cadbD9735aff958023239c6a063"
                .parse()
                .unwrap(),
            "0x4746dec9e833a82ec7c2c1356372ccf2cfcd2f3d"
                .parse()
                .unwrap(),
        );

        feeds
    }

    /// Stablecoins that are assumed to be $1.00
    fn default_stablecoins() -> Vec<Address> {
        vec![
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
                .parse()
                .unwrap(), // USDC
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
                .parse()
                .unwrap(), // USDT
            "0x8f3cf7ad23cd3cadbD9735aff958023239c6a063"
                .parse()
                .unwrap(), // DAI
        ]
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

/// Thread-safe price cache with TTL
#[allow(dead_code)]
pub struct PriceCache {
    entries: Arc<RwLock<HashMap<Address, CacheEntry>>>,
    ttl: Duration,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct CacheEntry {
    price: Option<f64>,
    expires_at: SystemTime,
}

#[allow(dead_code)]
impl PriceCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Get price from cache or fetch via oracle
    pub async fn get_or_fetch<O: PriceOracle>(
        &self,
        token_address: Address,
        oracle: &O,
    ) -> Option<f64> {
        // Try cache first
        if let Some(price) = self.get(token_address) {
            return price;
        }

        // Fetch from oracle
        let price = oracle.fetch_price(token_address).await;

        // Cache the result (including None for negative lookups)
        self.set(token_address, price);

        price
    }

    fn get(&self, token_address: Address) -> Option<Option<f64>> {
        let entries = self.entries.read().unwrap();

        if let Some(entry) = entries.get(&token_address) {
            if SystemTime::now() < entry.expires_at {
                return Some(entry.price);
            }
        }

        None
    }

    fn set(&self, token_address: Address, price: Option<f64>) {
        let mut entries = self.entries.write().unwrap();

        entries.insert(
            token_address,
            CacheEntry {
                price,
                expires_at: SystemTime::now() + self.ttl,
            },
        );
    }

    pub fn size(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    /// Evict expired entries
    pub fn evict_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap();
        let now = SystemTime::now();
        let initial_len = entries.len();

        entries.retain(|_, entry| now < entry.expires_at);

        initial_len - entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_scale_price() {
        assert_eq!(scale_price(100000000, 8), 1.0);
        assert_eq!(scale_price(200000000, 8), 2.0);
        assert_eq!(scale_price(150000000, 8), 1.5);
        assert_eq!(scale_price(1000, 3), 1.0);
    }

    #[test]
    fn test_price_cache_set_get() {
        let cache = PriceCache::new(Duration::from_secs(60));
        let addr: Address = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
            .parse()
            .unwrap();

        // Initially empty
        assert!(cache.get(addr).is_none());

        // Set a price
        cache.set(addr, Some(1.5));
        assert_eq!(cache.get(addr), Some(Some(1.5)));

        // Size should be 1
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_price_cache_expiration() {
        let cache = PriceCache::new(Duration::from_millis(100));
        let addr: Address = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
            .parse()
            .unwrap();

        cache.set(addr, Some(1.5));
        assert_eq!(cache.get(addr), Some(Some(1.5)));

        std::thread::sleep(Duration::from_millis(150));

        // Should be expired
        assert!(cache.get(addr).is_none());
    }

    #[test]
    fn test_price_cache_negative_lookup() {
        let cache = PriceCache::new(Duration::from_secs(60));
        let addr: Address = "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap();

        // Cache a negative lookup (None)
        cache.set(addr, None);
        assert_eq!(cache.get(addr), Some(None));
    }

    #[test]
    fn test_price_cache_evict_expired() {
        let cache = PriceCache::new(Duration::from_millis(100));
        let addr1: Address = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
            .parse()
            .unwrap();
        let addr2: Address = "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
            .parse()
            .unwrap();

        cache.set(addr1, Some(1.0));
        cache.set(addr2, Some(2.0));
        assert_eq!(cache.size(), 2);

        std::thread::sleep(Duration::from_millis(150));

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 2);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_chainlink_oracle_stablecoin_detection() {
        let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
        let oracle = ChainlinkOracle::new(provider);

        let usdc: Address = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
            .parse()
            .unwrap();
        let usdt: Address = "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
            .parse()
            .unwrap();
        let dai: Address = "0x8f3cf7ad23cd3cadbD9735aff958023239c6a063"
            .parse()
            .unwrap();
        let weth: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();

        assert!(oracle.is_stablecoin(usdc));
        assert!(oracle.is_stablecoin(usdt));
        assert!(oracle.is_stablecoin(dai));
        assert!(!oracle.is_stablecoin(weth));
    }

    #[test]
    fn test_chainlink_oracle_price_feed_mapping() {
        let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
        let oracle = ChainlinkOracle::new(provider);

        let wmatic: Address = "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"
            .parse()
            .unwrap();
        let weth: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();
        let unknown: Address = "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap();

        assert!(oracle.price_feeds.contains_key(&wmatic));
        assert!(oracle.price_feeds.contains_key(&weth));
        assert!(!oracle.price_feeds.contains_key(&unknown));
    }
}
