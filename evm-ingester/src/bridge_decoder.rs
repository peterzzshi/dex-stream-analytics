use anyhow::{Context, Result};
use ethers::abi::RawLog;
use ethers::prelude::*;

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

#[derive(Debug, Clone)]
pub enum BridgeEvent {
    WormholeLock(WormholeTransferSent),
    WormholeUnlock(WormholeTransferRedeemed),
    LayerZeroLock(LayerZeroPacketSent),
    LayerZeroUnlock(LayerZeroPacketReceived),
    HopLock(HopTransferSentToL2),
    HopUnlock(HopTransferFromL1Completed),
}

pub fn decode(log: &Log, protocol: BridgeProtocol) -> Result<Option<BridgeEvent>> {
    let Some(topic) = log.topics.first() else {
        return Ok(None);
    };

    let raw = RawLog {
        topics: log.topics.clone(),
        data: log.data.to_vec(),
    };

    match protocol {
        BridgeProtocol::Wormhole => decode_wormhole(&raw, topic),
        BridgeProtocol::LayerZero => decode_layerzero(&raw, topic),
        BridgeProtocol::Hop => decode_hop(&raw, topic),
    }
}

fn decode_wormhole(raw: &RawLog, topic: &H256) -> Result<Option<BridgeEvent>> {
    if topic == &WormholeTransferSent::signature() {
        let event: WormholeTransferSent = WormholeTransferSent::decode_log(raw)
            .context("failed to decode WormholeTransferSent")?;
        return Ok(Some(BridgeEvent::WormholeLock(event)));
    }
    if topic == &WormholeTransferRedeemed::signature() {
        let event: WormholeTransferRedeemed = WormholeTransferRedeemed::decode_log(raw)
            .context("failed to decode WormholeTransferRedeemed")?;
        return Ok(Some(BridgeEvent::WormholeUnlock(event)));
    }
    Ok(None)
}

fn decode_layerzero(raw: &RawLog, topic: &H256) -> Result<Option<BridgeEvent>> {
    if topic == &LayerZeroPacketSent::signature() {
        let event: LayerZeroPacketSent =
            LayerZeroPacketSent::decode_log(raw).context("failed to decode LayerZeroPacketSent")?;
        return Ok(Some(BridgeEvent::LayerZeroLock(event)));
    }
    if topic == &LayerZeroPacketReceived::signature() {
        let event: LayerZeroPacketReceived = LayerZeroPacketReceived::decode_log(raw)
            .context("failed to decode LayerZeroPacketReceived")?;
        return Ok(Some(BridgeEvent::LayerZeroUnlock(event)));
    }
    Ok(None)
}

fn decode_hop(raw: &RawLog, topic: &H256) -> Result<Option<BridgeEvent>> {
    if topic == &HopTransferSentToL2::signature() {
        let event: HopTransferSentToL2 =
            HopTransferSentToL2::decode_log(raw).context("failed to decode HopTransferSentToL2")?;
        return Ok(Some(BridgeEvent::HopLock(event)));
    }
    if topic == &HopTransferFromL1Completed::signature() {
        let event: HopTransferFromL1Completed = HopTransferFromL1Completed::decode_log(raw)
            .context("failed to decode HopTransferFromL1Completed")?;
        return Ok(Some(BridgeEvent::HopUnlock(event)));
    }
    Ok(None)
}
