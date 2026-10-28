use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result, bail};
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, VerificationLevel};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::PythConfig;

pub struct PythOracle {
    rpc_url: String,
    config: PythConfig,
    cache: Arc<RwLock<HashMap<String, CachedPrice>>>,
}

#[derive(Clone)]
struct CachedPrice {
    price: f64,
    timestamp: i64,
}

impl PythOracle {
    pub fn new(rpc_url: String, config: PythConfig) -> Self {
        Self {
            rpc_url,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Fetch USD price for a token mint
    /// Returns None if price feed not available or price is stale
    pub async fn fetch_price(&self, token_mint: &str) -> Result<Option<f64>> {
        // Stablecoin shortcut
        if self.config.stablecoins.contains(&token_mint.to_string()) {
            return Ok(Some(1.0));
        }

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(token_mint) {
                let age = chrono::Utc::now().timestamp() - cached.timestamp;
                if age < self.config.max_age_seconds {
                    debug!("Cache hit for {}: ${:.4}", token_mint, cached.price);
                    return Ok(Some(cached.price));
                }
            }
        }

        // Fetch from Pyth
        let price_feed = self.get_price_feed_account(token_mint)?;
        if price_feed.is_none() {
            debug!("No Pyth price feed configured for {}", token_mint);
            return Ok(None);
        }

        let price = self.fetch_pyth_price(price_feed.unwrap()).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                token_mint.to_string(),
                CachedPrice {
                    price,
                    timestamp: chrono::Utc::now().timestamp(),
                },
            );
        }

        Ok(Some(price))
    }

    fn get_price_feed_account(&self, token_mint: &str) -> Result<Option<Pubkey>> {
        // Lookup from config
        let feed_str = match self.config.price_feeds.get(token_mint) {
            Some(feed) => feed,
            None => return Ok(None),
        };

        let pubkey = Pubkey::from_str(feed_str)
            .with_context(|| format!("Invalid Pyth feed address: {}", feed_str))?;

        Ok(Some(pubkey))
    }

    async fn fetch_pyth_price(&self, price_feed: Pubkey) -> Result<f64> {
        // Create RPC client
        let rpc_client = RpcClient::new(self.rpc_url.clone());

        // Fetch the price feed account data
        let account_data = rpc_client
            .get_account_data(&price_feed)
            .context("Failed to fetch Pyth price feed account")?;

        // Deserialize the price feed using pyth-solana-receiver-sdk.
        // PriceUpdateV2::price_message is a flat PriceFeedMessage (no separate
        // "trading status" enum in this SDK version, unlike legacy pyth-sdk-solana).
        let price_update = PriceUpdateV2::try_deserialize(&mut account_data.as_slice())
            .context("Failed to deserialize Pyth price feed")?;

        // Only trust fully Wormhole-verified updates.
        if price_update.verification_level != VerificationLevel::Full {
            bail!(
                "Price feed verification level too low: {:?}",
                price_update.verification_level
            );
        }

        let message = price_update.price_message;

        // Check timestamp freshness (within last MAX_PRICE_AGE_SECONDS)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let price_age = current_time - message.publish_time;
        if price_age > self.config.max_age_seconds {
            bail!("Price feed too old: {} seconds", price_age);
        }

        // Apply exponent to get actual price: price * 10^exponent
        let price = (message.price as f64) * 10f64.powi(message.exponent);

        // Sanity check: price should be positive and reasonable
        if price <= 0.0 {
            bail!("Invalid price: {}", price);
        }

        info!(
            "Fetched Pyth price for feed {}: ${:.4} (age: {}s)",
            price_feed, price, price_age
        );

        Ok(price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PythConfig {
        let mut price_feeds = HashMap::new();
        price_feeds.insert(
            "So11111111111111111111111111111111111111112".to_string(),
            "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG".to_string(),
        );
        price_feeds.insert(
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD".to_string(),
        );

        PythConfig {
            max_age_seconds: 60,
            price_feeds,
            stablecoins: vec!["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()],
        }
    }

    #[tokio::test]
    async fn test_usdc_price() {
        let oracle = PythOracle::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            test_config(),
        );
        let price = oracle
            .fetch_price("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .await
            .unwrap();
        assert_eq!(price, Some(1.0));
    }

    #[test]
    fn test_sol_feed_account() {
        let oracle = PythOracle::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            test_config(),
        );
        let feed = oracle
            .get_price_feed_account("So11111111111111111111111111111111111111112")
            .unwrap();
        assert!(feed.is_some());
    }
}
