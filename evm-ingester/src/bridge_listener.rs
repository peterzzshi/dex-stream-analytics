use crate::bridge_decoder::BridgeProtocol;
use crate::bridge_normalizer::BridgeNormalizer;
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use ethers::prelude::*;
use ethers::types::Log;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const BLOCK_TIMESTAMP_CACHE_SIZE: usize = 128;

pub struct BridgeListener<P: Middleware> {
    normalizer: BridgeNormalizer,
    rpc: Arc<RetryableProvider<P>>,
    block_timestamps: Arc<RwLock<LruCache>>,
}

/// Simple LRU cache with deterministic eviction policy
struct LruCache {
    cache: HashMap<U64, (i64, usize)>, // (timestamp, access_order)
    access_counter: usize,
    capacity: usize,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            access_counter: 0,
            capacity,
        }
    }

    fn get(&mut self, key: &U64) -> Option<i64> {
        if let Some((timestamp, access_order)) = self.cache.get_mut(key) {
            let ts = *timestamp;
            self.access_counter += 1;
            *access_order = self.access_counter;
            Some(ts)
        } else {
            None
        }
    }

    fn insert(&mut self, key: U64, timestamp: i64) {
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            // Evict least recently used entry
            if let Some(lru_key) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, access_order))| access_order)
                .map(|(k, _)| *k)
            {
                self.cache.remove(&lru_key);
            }
        }

        self.access_counter += 1;
        self.cache.insert(key, (timestamp, self.access_counter));
    }
}

impl<P: Middleware + 'static> BridgeListener<P> {
    pub fn new(
        chain_id: String,
        chain_name: String,
        bridge_address: String,
        protocol: BridgeProtocol,
        rpc: Arc<RetryableProvider<P>>,
    ) -> Self {
        Self {
            normalizer: BridgeNormalizer::new(chain_id, chain_name, bridge_address, protocol),
            rpc,
            block_timestamps: Arc::new(RwLock::new(LruCache::new(BLOCK_TIMESTAMP_CACHE_SIZE))),
        }
    }

    pub async fn process_log(&self, log: Log) -> Result<Option<Event>> {
        let block_number = log.block_number.context("log missing block number")?;
        let block_timestamp = self.fetch_block_timestamp(block_number).await?;

        let transaction_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;

        let event_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or_default();

        match self
            .normalizer
            .normalize(log, block_timestamp, event_timestamp)
        {
            Ok(event) => Ok(Some(event)),
            Err(e) => {
                tracing::warn!(
                    bridge = %self.normalizer.bridge_address(),
                    tx = ?transaction_hash,
                    "Failed to normalize bridge log: {:#}",
                    e
                );
                Err(e)
            }
        }
    }

    async fn fetch_block_timestamp(&self, block_number: U64) -> Result<i64> {
        // Fast path: read lock for cache hit
        {
            let mut cache = self.block_timestamps.write().await;
            if let Some(timestamp) = cache.get(&block_number) {
                return Ok(timestamp);
            }
        }

        // Slow path: fetch from RPC
        let block = self
            .rpc
            .get_block(block_number)
            .await
            .context("failed to fetch block")?
            .context("block not found")?;
        let timestamp = block.timestamp.as_u64() as i64;

        // Write lock to insert
        {
            let mut cache = self.block_timestamps.write().await;
            cache.insert(block_number, timestamp);
        }

        Ok(timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_cache_evicts_least_recently_used() {
        let mut cache = LruCache::new(3);

        cache.insert(U64::from(1), 100);
        cache.insert(U64::from(2), 200);
        cache.insert(U64::from(3), 300);

        // Access block 1 to make it recently used
        assert_eq!(cache.get(&U64::from(1)), Some(100));

        // Insert block 4, should evict block 2 (least recently used)
        cache.insert(U64::from(4), 400);

        assert_eq!(cache.get(&U64::from(1)), Some(100)); // Still present
        assert_eq!(cache.get(&U64::from(2)), None); // Evicted
        assert_eq!(cache.get(&U64::from(3)), Some(300)); // Still present
        assert_eq!(cache.get(&U64::from(4)), Some(400)); // Newly inserted
    }

    #[test]
    fn lru_cache_updates_access_order_on_get() {
        let mut cache = LruCache::new(2);

        cache.insert(U64::from(1), 100);
        cache.insert(U64::from(2), 200);

        // Access block 1
        cache.get(&U64::from(1));

        // Insert block 3, should evict block 2 (not block 1, which was just accessed)
        cache.insert(U64::from(3), 300);

        assert_eq!(cache.get(&U64::from(1)), Some(100));
        assert_eq!(cache.get(&U64::from(2)), None);
        assert_eq!(cache.get(&U64::from(3)), Some(300));
    }

    #[test]
    fn lru_cache_does_not_evict_when_reinserting_existing_key() {
        let mut cache = LruCache::new(2);

        cache.insert(U64::from(1), 100);
        cache.insert(U64::from(2), 200);

        // Reinsert block 1 with new timestamp
        cache.insert(U64::from(1), 150);

        // Should not have evicted anything
        assert_eq!(cache.get(&U64::from(1)), Some(150));
        assert_eq!(cache.get(&U64::from(2)), Some(200));
    }
}
