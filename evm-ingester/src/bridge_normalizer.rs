use crate::bridge_decoder::{BridgeEvent, BridgeProtocol};
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use ethers::prelude::*;
use ethers::types::Log;
use std::time::{SystemTime, UNIX_EPOCH};

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

    pub async fn normalize(&self, log: Log) -> Result<Option<Event>> {
        let decoded = crate::bridge_decoder::decode(&log, self.protocol)?;
        let Some(bridge_event) = decoded else {
            return Ok(None);
        };

        let block_number = log.block_number.context("log missing block number")?;
        let transaction_hash = log
            .transaction_hash
            .context("log missing transaction hash")?;
        let log_index = log.log_index.context("log missing log index")?;

        let event_id = format!("{:?}:{}", transaction_hash, log_index);
        let block_timestamp = 0i64;
        let event_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let event = match bridge_event {
            BridgeEvent::WormholeLock(e) => self.normalize_wormhole_lock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
            BridgeEvent::WormholeUnlock(e) => self.normalize_wormhole_unlock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
            BridgeEvent::LayerZeroLock(e) => self.normalize_layerzero_lock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
            BridgeEvent::LayerZeroUnlock(e) => self.normalize_layerzero_unlock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
            BridgeEvent::HopLock(e) => self.normalize_hop_lock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
            BridgeEvent::HopUnlock(e) => self.normalize_hop_unlock(
                e,
                event_id,
                block_number.as_u64() as i64,
                block_timestamp,
                transaction_hash,
                log_index.as_u32() as i32,
                event_timestamp,
            ),
        };

        Ok(Some(event))
    }

    fn normalize_wormhole_lock(
        &self,
        event: crate::bridge_decoder::WormholeTransferSent,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeLock(dex_ingester_common::BridgeLockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
            chain: self.chain_id.clone(),
            chain_name: self.chain_name.clone(),
            bridge_address: self.bridge_address.clone(),
            protocol: self.protocol.as_str().to_string(),
            gas_used: 0,
            gas_price: "0".to_string(),
            event_timestamp,
            sender: format!("{:?}", event.sender),
            recipient: format!("0x{}", hex::encode(event.recipient)),
            token: "".to_string(),
            token_symbol: None,
            amount: event.amount.to_string(),
            destination_chain: Some(format!("eip155:{}", event.target_chain)),
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        })
    }

    fn normalize_wormhole_unlock(
        &self,
        event: crate::bridge_decoder::WormholeTransferRedeemed,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeUnlock(dex_ingester_common::BridgeUnlockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
            chain: self.chain_id.clone(),
            chain_name: self.chain_name.clone(),
            bridge_address: self.bridge_address.clone(),
            protocol: self.protocol.as_str().to_string(),
            gas_used: 0,
            gas_price: "0".to_string(),
            event_timestamp,
            sender: format!("0x{}", hex::encode(event.sender)),
            recipient: format!("{:?}", event.recipient),
            token: "".to_string(),
            token_symbol: None,
            amount: event.amount.to_string(),
            destination_chain: None,
            source_chain: Some(format!("eip155:{}", event.source_chain)),
            message_hash: None,
            sequence_number: None,
        })
    }

    fn normalize_layerzero_lock(
        &self,
        event: crate::bridge_decoder::LayerZeroPacketSent,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeLock(dex_ingester_common::BridgeLockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
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
            amount: "0".to_string(),
            destination_chain: Some(format!("eip155:{}", event.dst_chain_id)),
            source_chain: None,
            message_hash: None,
            sequence_number: Some(event.nonce as i64),
        })
    }

    fn normalize_layerzero_unlock(
        &self,
        event: crate::bridge_decoder::LayerZeroPacketReceived,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeUnlock(dex_ingester_common::BridgeUnlockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
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
            amount: "0".to_string(),
            destination_chain: None,
            source_chain: Some(format!("eip155:{}", event.src_chain_id)),
            message_hash: None,
            sequence_number: Some(event.nonce as i64),
        })
    }

    fn normalize_hop_lock(
        &self,
        event: crate::bridge_decoder::HopTransferSentToL2,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeLock(dex_ingester_common::BridgeLockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
            chain: self.chain_id.clone(),
            chain_name: self.chain_name.clone(),
            bridge_address: self.bridge_address.clone(),
            protocol: self.protocol.as_str().to_string(),
            gas_used: 0,
            gas_price: "0".to_string(),
            event_timestamp,
            sender: "".to_string(),
            recipient: format!("{:?}", event.recipient),
            token: "".to_string(),
            token_symbol: None,
            amount: event.amount.to_string(),
            destination_chain: Some(event.chain_id.to_string()),
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        })
    }

    fn normalize_hop_unlock(
        &self,
        event: crate::bridge_decoder::HopTransferFromL1Completed,
        event_id: String,
        block_number: i64,
        block_timestamp: i64,
        transaction_hash: H256,
        log_index: i32,
        event_timestamp: i64,
    ) -> Event {
        Event::BridgeUnlock(dex_ingester_common::BridgeUnlockEvent {
            event_id,
            block_number,
            block_timestamp,
            transaction_hash: format!("{:?}", transaction_hash),
            log_index,
            chain: self.chain_id.clone(),
            chain_name: self.chain_name.clone(),
            bridge_address: self.bridge_address.clone(),
            protocol: self.protocol.as_str().to_string(),
            gas_used: 0,
            gas_price: "0".to_string(),
            event_timestamp,
            sender: "".to_string(),
            recipient: format!("{:?}", event.recipient),
            token: "".to_string(),
            token_symbol: None,
            amount: event.amount.to_string(),
            destination_chain: None,
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        })
    }
}
