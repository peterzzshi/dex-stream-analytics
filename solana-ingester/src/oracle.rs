use anchor_lang::AccountDeserialize;
use anyhow::{bail, Context, Result};
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, VerificationLevel};
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

use crate::config::{PythConfig, PythFeed};
use crate::file_config::{feed_id_hex, FEED_ID_BYTES};

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
                    return Ok(Some(cached.price));
                }
            }
        }

        // Fetch from Pyth
        let Some(feed) = self.config.price_feeds.get(token_mint).copied() else {
            return Ok(None);
        };

        let price = self.fetch_pyth_price(feed).await?;

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

    async fn fetch_pyth_price(&self, feed: PythFeed) -> Result<f64> {
        let price_feed = feed.account;

        // Fetch account data (blocking call, run in tokio blocking thread)
        let rpc_url = self.rpc_url.clone();
        let account_data = tokio::task::spawn_blocking(move || {
            let rpc_client = RpcClient::new(rpc_url);
            rpc_client.get_account_data(&price_feed)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

        // Deserialize the price feed using pyth-solana-receiver-sdk.
        // PriceUpdateV2::price_message is a flat PriceFeedMessage (no separate
        // "trading status" enum in this SDK version, unlike legacy pyth-sdk-solana).
        let price_update = PriceUpdateV2::try_deserialize(&mut account_data.as_slice())
            .context("Failed to deserialize Pyth price feed")?;

        // The account address alone proves nothing: any receiver account deserializes as a
        // PriceUpdateV2. Its feed id is what ties the price to the requested token.
        verify_feed_id(price_update.price_message.feed_id, feed.feed_id)?;

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
            feed_id_hex(&feed.feed_id),
            price,
            price_age
        );

        Ok(price)
    }
}

/// Reject a price update whose embedded feed id is not the one configured for the token.
fn verify_feed_id(actual: [u8; FEED_ID_BYTES], expected: [u8; FEED_ID_BYTES]) -> Result<()> {
    if actual != expected {
        bail!(
            "Pyth feed id mismatch: account holds {} but {} was configured",
            feed_id_hex(&actual),
            feed_id_hex(&expected)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_config::parse_feed_id;

    const SOL_FEED_ID: &str = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
    const RAY_FEED_ID: &str = "91568baa8beb53db23eb3fb7f22c6e8bd303d103919e19733f2bb642d3e7987a";

    #[test]
    fn feed_id_must_match_the_configured_one() {
        let sol = parse_feed_id(SOL_FEED_ID).expect("valid feed id");
        let ray = parse_feed_id(RAY_FEED_ID).expect("valid feed id");

        assert!(verify_feed_id(sol, sol).is_ok());

        let message = format!("{:#}", verify_feed_id(ray, sol).unwrap_err());
        assert!(message.contains(RAY_FEED_ID), "message was {message}");
        assert!(message.contains(SOL_FEED_ID), "message was {message}");
    }
}
