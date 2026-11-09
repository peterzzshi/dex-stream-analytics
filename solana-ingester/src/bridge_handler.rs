use crate::bridge_decoder::BridgeProtocol;
use crate::bridge_normalizer::BridgeNormalizer;
use crate::rpc_retry::RetryableRpcClient;
use anyhow::Result;
use dex_ingester_common::Event;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::str::FromStr;
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
        let program_id = Pubkey::from_str(&program_id).expect("valid program_id");
        Self {
            normalizer: BridgeNormalizer::new(chain_id, chain_name, program_id, protocol),
            rpc_client,
        }
    }

    #[expect(dead_code)]
    pub fn program_id(&self) -> &Pubkey {
        self.normalizer.program_id()
    }

    pub async fn handle(&self, signature: Signature) -> Result<Vec<Event>> {
        let tx = self.rpc_client.get_transaction(signature).await?;

        let slot = tx.slot;
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

        for instruction in tx_data.message.instructions() {
            let instruction_data = instruction.data.as_slice();
            match self.normalizer.normalize(
                slot,
                signature.to_string(),
                instruction_data,
                block_timestamp,
                event_timestamp,
            ) {
                Ok(event) => {
                    events.push(event);
                }
                Err(e) => {
                    warn!("Failed to decode bridge instruction: {:#}", e);
                }
            }
        }

        Ok(events)
    }

    pub fn protocol(&self) -> BridgeProtocol {
        self.normalizer.protocol()
    }
}
