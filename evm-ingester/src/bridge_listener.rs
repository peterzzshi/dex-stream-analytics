use crate::bridge_decoder::BridgeProtocol;
use crate::bridge_normalizer::BridgeNormalizer;
use crate::rpc_retry::RetryableProvider;
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use ethers::prelude::*;
use ethers::types::Log;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const BLOCK_TIMESTAMP_CACHE_SIZE: usize = 64;

pub struct BridgeListener<P: Middleware> {
    normalizer: BridgeNormalizer,
    rpc: Arc<RetryableProvider<P>>,
    block_timestamps: Mutex<std::collections::HashMap<U64, i64>>,
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
            block_timestamps: Mutex::new(std::collections::HashMap::new()),
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
            Ok(Some(event)) => Ok(Some(event)),
            Ok(None) => {
                tracing::trace!(
                    bridge = %self.normalizer.bridge_address(),
                    tx = ?transaction_hash,
                    "Skipping unhandled bridge event"
                );
                Ok(None)
            }
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

    fn lock<'a, T>(&self, mutex: &'a Mutex<T>) -> MutexGuard<'a, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
