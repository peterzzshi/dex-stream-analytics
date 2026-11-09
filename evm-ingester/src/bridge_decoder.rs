use anyhow::{bail, Context, Result};
use dex_ingester_common::{BridgeLockEvent, BridgeUnlockEvent, Event};
use ethers::contract::{EthEvent, EthLogDecode};
use ethers::prelude::*;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeProtocol {
    Wormhole,
    LayerZero,
    Hop,
}

impl BridgeProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wormhole => "wormhole",
            Self::LayerZero => "layerzero",
            Self::Hop => "hop",
        }
    }
}

impl FromStr for BridgeProtocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "wormhole" => Ok(Self::Wormhole),
            "layerzero" => Ok(Self::LayerZero),
            "hop" => Ok(Self::Hop),
            other => bail!("Unknown bridge protocol: {}", other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "TransferSent")]
pub struct WormholeTransferSent {
    #[ethevent(indexed)]
    pub sender: Address,
    pub target_chain: u16,
    pub recipient: [u8; 32],
    pub amount: U256,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "TransferRedeemed")]
pub struct WormholeTransferRedeemed {
    #[ethevent(indexed)]
    pub recipient: Address,
    pub source_chain: u16,
    pub sender: [u8; 32],
    pub amount: U256,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "PacketSent")]
pub struct LayerZeroPacketSent {
    pub dst_chain_id: u16,
    pub dst_address: Bytes,
    pub nonce: u64,
    pub payload: Bytes,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "PacketReceived")]
pub struct LayerZeroPacketReceived {
    pub src_chain_id: u16,
    pub src_address: Bytes,
    pub nonce: u64,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "TransferSentToL2")]
pub struct HopTransferSentToL2 {
    pub chain_id: U256,
    pub recipient: Address,
    pub amount: U256,
    pub amount_out_min: U256,
    pub deadline: U256,
    pub relayer: Address,
    pub relayer_fee: U256,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "TransferFromL1Completed")]
pub struct HopTransferFromL1Completed {
    pub recipient: Address,
    pub amount: U256,
    pub amount_out_min: U256,
    pub deadline: U256,
    pub relayer: Address,
}

pub fn decode(
    log: &Log,
    protocol: BridgeProtocol,
    chain: String,
    chain_name: String,
    bridge_address: String,
    event_id: String,
    block_number: i64,
    block_timestamp: i64,
    transaction_hash: H256,
    log_index: i32,
    event_timestamp: i64,
) -> Result<Event> {
    match protocol {
        BridgeProtocol::Wormhole => {
            if log.topics[0] == WormholeTransferSent::signature() {
                let event = <WormholeTransferSent as EthLogDecode>::decode_log(&log.clone().into())
                    .context("failed to decode WormholeTransferSent")?;
                Ok(Event::BridgeLock(BridgeLockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else if log.topics[0] == WormholeTransferRedeemed::signature() {
                let event =
                    <WormholeTransferRedeemed as EthLogDecode>::decode_log(&log.clone().into())
                        .context("failed to decode WormholeTransferRedeemed")?;
                Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else {
                bail!("unknown Wormhole event signature: {:?}", log.topics[0])
            }
        }
        BridgeProtocol::LayerZero => {
            if log.topics[0] == LayerZeroPacketSent::signature() {
                let event = <LayerZeroPacketSent as EthLogDecode>::decode_log(&log.clone().into())
                    .context("failed to decode LayerZeroPacketSent")?;
                Ok(Event::BridgeLock(BridgeLockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else if log.topics[0] == LayerZeroPacketReceived::signature() {
                let event =
                    <LayerZeroPacketReceived as EthLogDecode>::decode_log(&log.clone().into())
                        .context("failed to decode LayerZeroPacketReceived")?;
                Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else {
                bail!("unknown LayerZero event signature: {:?}", log.topics[0])
            }
        }
        BridgeProtocol::Hop => {
            if log.topics[0] == HopTransferSentToL2::signature() {
                let event = <HopTransferSentToL2 as EthLogDecode>::decode_log(&log.clone().into())
                    .context("failed to decode HopTransferSentToL2")?;
                Ok(Event::BridgeLock(BridgeLockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else if log.topics[0] == HopTransferFromL1Completed::signature() {
                let event =
                    <HopTransferFromL1Completed as EthLogDecode>::decode_log(&log.clone().into())
                        .context("failed to decode HopTransferFromL1Completed")?;
                Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                    event_id,
                    block_number,
                    block_timestamp,
                    transaction_hash: format!("{:?}", transaction_hash),
                    log_index,
                    chain,
                    chain_name,
                    bridge_address,
                    protocol: protocol.as_str().to_string(),
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
                }))
            } else {
                bail!("unknown Hop event signature: {:?}", log.topics[0])
            }
        }
    }
}
