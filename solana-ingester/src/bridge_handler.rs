use crate::blockchain::RpcClient;
use crate::bridge_decoder::{self, BridgeProtocol};
use crate::bridge_normalizer::BridgeNormalizer;
use crate::rpc_retry::RetryableRpcClient;
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

pub struct BridgeHandler {
    normalizer: BridgeNormalizer,
    rpc_client: Arc<RetryableRpcClient>,
}

impl BridgeHandler {
    pub fn new(
        chain_id: String,
        chain_name: String,
        program_id: String,
        protocol: BridgeProtocol,
        rpc_client: Arc<RetryableRpcClient>,
    ) -> Self {
        Self {
            normalizer: BridgeNormalizer::new(chain_id, chain_name, program_id, protocol),
            rpc_client,
        }
    }

    pub fn program_id(&self) -> &str {
        self.normalizer.program_id()
    }

    pub async fn handle(&self, signature: Signature) -> Result<Vec<Event>> {
        let tx = match self.rpc_client.get_transaction(&signature).await? {
            Some(tx) => tx,
            None => {
                warn!("Transaction not found: {}", signature);
                return Ok(vec![]);
            }
        };

        let slot = tx.slot.try_into().context("slot does not fit in u64")?;

        let block_timestamp = tx.block_time.unwrap_or(0);

        let event_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let Some(meta) = tx.transaction.meta else {
            return Ok(vec![]);
        };

        if meta.err.is_some() {
            return Ok(vec![]);
        }

        let Some(tx_data) = tx.transaction.transaction.decode() else {
            warn!("Failed to decode transaction: {}", signature);
            return Ok(vec![]);
        };

        let mut events = Vec::new();

        for instruction in &tx_data.message.instructions {
            match bridge_decoder::decode(instruction, self.normalizer.protocol) {
                Ok(Some(bridge_instruction)) => {
                    let event = self.normalizer.normalize(
                        bridge_instruction,
                        &signature,
                        slot,
                        block_timestamp,
                        event_timestamp,
                    )?;
                    events.push(event);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Failed to decode bridge instruction: {:#}", e);
                }
            }
        }

        Ok(events)
    }

    pub fn protocol(&self) -> BridgeProtocol {
        self.normalizer.protocol
    }
}
