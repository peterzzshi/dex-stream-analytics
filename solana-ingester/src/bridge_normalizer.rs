use crate::bridge_decoder::BridgeProtocol;
use anyhow::Result;
use dex_ingester_common::Event;
use solana_sdk::pubkey::Pubkey;

pub struct BridgeNormalizer {
    chain: String,
    chain_name: String,
    program_id: Pubkey,
    protocol: BridgeProtocol,
}

impl BridgeNormalizer {
    #[expect(dead_code)]
    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    pub fn protocol(&self) -> BridgeProtocol {
        self.protocol
    }
}

impl BridgeNormalizer {
    pub fn new(
        chain: String,
        chain_name: String,
        program_id: Pubkey,
        protocol: BridgeProtocol,
    ) -> Self {
        Self {
            chain,
            chain_name,
            program_id,
            protocol,
        }
    }

    pub fn normalize(
        &self,
        slot: u64,
        tx_signature: String,
        instruction_data: &[u8],
        block_timestamp: i64,
        event_timestamp: i64,
    ) -> Result<Event> {
        let event_id = format!("{}:0", tx_signature);

        crate::bridge_decoder::decode_instruction(
            &self.program_id,
            self.protocol,
            instruction_data,
            self.chain.clone(),
            self.chain_name.clone(),
            self.program_id.to_string(),
            event_id,
            slot as i64,
            block_timestamp,
            tx_signature,
            0,
            event_timestamp,
        )
    }
}
