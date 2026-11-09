use crate::bridge_decoder::{BridgeInstruction, BridgeProtocol};
use anyhow::Result;
use dex_ingester_common::{
    BridgeBurnEvent, BridgeLockEvent, BridgeMintEvent, BridgeUnlockEvent, Event,
};
use solana_sdk::signature::Signature;

pub struct BridgeNormalizer {
    chain_id: String,
    chain_name: String,
    bridge_address: String,
    protocol: BridgeProtocol,
}

impl BridgeNormalizer {
    pub fn new(
        chain_id: String,
        chain_name: String,
        bridge_address: String,
        protocol: BridgeProtocol,
    ) -> Self {
        Self {
            chain_id,
            chain_name,
            bridge_address,
            protocol,
        }
    }

    pub fn program_id(&self) -> &str {
        &self.bridge_address
    }

    pub fn normalize(
        &self,
        instruction: BridgeInstruction,
        signature: &Signature,
        slot: u64,
        block_timestamp: i64,
        event_timestamp: i64,
    ) -> Result<Event> {
        let event_id = signature.to_string();
        let transaction_hash = signature.to_string();

        match instruction {
            BridgeInstruction::Lock {
                amount,
                destination_chain,
            } => Ok(Event::BridgeLock(BridgeLockEvent {
                event_id,
                block_number: slot as i64,
                block_timestamp,
                transaction_hash,
                log_index: 0,
                chain: self.chain_id.clone(),
                chain_name: self.chain_name.clone(),
                bridge_address: self.bridge_address.clone(),
                protocol: self.protocol.as_str().to_string(),
                gas_used: 0,
                gas_price: "0".to_string(),
                event_timestamp,
                sender: "".to_string(),
                recipient: "".to_string(),
                token: "".to_string(),
                token_symbol: None,
                amount: amount.to_string(),
                destination_chain,
                source_chain: None,
                message_hash: None,
                sequence_number: None,
            })),
            BridgeInstruction::Unlock {
                amount,
                source_chain,
            } => Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                event_id,
                block_number: slot as i64,
                block_timestamp,
                transaction_hash,
                log_index: 0,
                chain: self.chain_id.clone(),
                chain_name: self.chain_name.clone(),
                bridge_address: self.bridge_address.clone(),
                protocol: self.protocol.as_str().to_string(),
                gas_used: 0,
                gas_price: "0".to_string(),
                event_timestamp,
                sender: "".to_string(),
                recipient: "".to_string(),
                token: "".to_string(),
                token_symbol: None,
                amount: amount.to_string(),
                destination_chain: None,
                source_chain,
                message_hash: None,
                sequence_number: None,
            })),
            BridgeInstruction::Mint {
                amount,
                source_chain,
            } => Ok(Event::BridgeMint(BridgeMintEvent {
                event_id,
                block_number: slot as i64,
                block_timestamp,
                transaction_hash,
                log_index: 0,
                chain: self.chain_id.clone(),
                chain_name: self.chain_name.clone(),
                bridge_address: self.bridge_address.clone(),
                protocol: self.protocol.as_str().to_string(),
                gas_used: 0,
                gas_price: "0".to_string(),
                event_timestamp,
                recipient: "".to_string(),
                wrapped_token: "".to_string(),
                wrapped_token_symbol: None,
                amount: amount.to_string(),
                source_chain,
                message_hash: None,
            })),
            BridgeInstruction::Burn {
                amount,
                destination_chain,
            } => Ok(Event::BridgeBurn(BridgeBurnEvent {
                event_id,
                block_number: slot as i64,
                block_timestamp,
                transaction_hash,
                log_index: 0,
                chain: self.chain_id.clone(),
                chain_name: self.chain_name.clone(),
                bridge_address: self.bridge_address.clone(),
                protocol: self.protocol.as_str().to_string(),
                gas_used: 0,
                gas_price: "0".to_string(),
                event_timestamp,
                sender: "".to_string(),
                wrapped_token: "".to_string(),
                wrapped_token_symbol: None,
                amount: amount.to_string(),
                destination_chain,
                message_hash: None,
            })),
        }
    }
}
