pub use crate::config::BridgeProtocol;
use anyhow::{bail, Context, Result};
use dex_ingester_common::{BridgeLockEvent, BridgeMintEvent, BridgeUnlockEvent, Event};
use solana_sdk::pubkey::Pubkey;
use std::convert::TryInto;

pub fn decode_instruction(
    program_id: &Pubkey,
    protocol: BridgeProtocol,
    data: &[u8],
    chain: String,
    chain_name: String,
    bridge_address: String,
    event_id: String,
    block_number: i64,
    block_timestamp: i64,
    transaction_hash: String,
    log_index: i32,
    event_timestamp: i64,
) -> Result<Event> {
    if data.is_empty() {
        bail!("empty instruction data");
    }

    let discriminator = data[0];
    match protocol {
        BridgeProtocol::Wormhole => decode_wormhole(
            discriminator,
            &data[1..],
            chain,
            chain_name,
            bridge_address,
            protocol,
            event_id,
            block_number,
            block_timestamp,
            transaction_hash,
            log_index,
            event_timestamp,
        ),
        BridgeProtocol::Allbridge => decode_allbridge(
            discriminator,
            &data[1..],
            chain,
            chain_name,
            bridge_address,
            protocol,
            event_id,
            block_number,
            block_timestamp,
            transaction_hash,
            log_index,
            event_timestamp,
        ),
    }
    .with_context(|| {
        format!(
            "failed to decode {} instruction for program {}",
            protocol.as_str(),
            program_id
        )
    })
}

fn decode_wormhole(
    discriminator: u8,
    data: &[u8],
    chain: String,
    chain_name: String,
    bridge_address: String,
    protocol: BridgeProtocol,
    event_id: String,
    block_number: i64,
    block_timestamp: i64,
    transaction_hash: String,
    log_index: i32,
    event_timestamp: i64,
) -> Result<Event> {
    match discriminator {
        1 => {
            let amount = u64::from_le_bytes(data[0..8].try_into()?);
            let destination_chain = u16::from_le_bytes(data[8..10].try_into()?);
            Ok(Event::BridgeLock(BridgeLockEvent {
                event_id,
                block_number,
                block_timestamp,
                transaction_hash,
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
                amount: amount.to_string(),
                destination_chain: Some(format!("wormhole:{}", destination_chain)),
                source_chain: None,
                message_hash: None,
                sequence_number: None,
            }))
        }
        2 => {
            let amount = u64::from_le_bytes(data[0..8].try_into()?);
            let source_chain = u16::from_le_bytes(data[8..10].try_into()?);
            Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                event_id,
                block_number,
                block_timestamp,
                transaction_hash,
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
                amount: amount.to_string(),
                destination_chain: None,
                source_chain: Some(format!("wormhole:{}", source_chain)),
                message_hash: None,
                sequence_number: None,
            }))
        }
        3 => {
            let amount = u64::from_le_bytes(data[0..8].try_into()?);
            let source_chain = u16::from_le_bytes(data[8..10].try_into()?);
            Ok(Event::BridgeMint(BridgeMintEvent {
                event_id,
                block_number,
                block_timestamp,
                transaction_hash,
                log_index,
                chain,
                chain_name,
                bridge_address,
                protocol: protocol.as_str().to_string(),
                gas_used: 0,
                gas_price: "0".to_string(),
                event_timestamp,
                recipient: "".to_string(),
                wrapped_token: "".to_string(),
                wrapped_token_symbol: None,
                amount: amount.to_string(),
                source_chain: Some(format!("wormhole:{}", source_chain)),
                message_hash: None,
            }))
        }
        _ => bail!("unknown Wormhole discriminator: {}", discriminator),
    }
}

fn decode_allbridge(
    discriminator: u8,
    data: &[u8],
    chain: String,
    chain_name: String,
    bridge_address: String,
    protocol: BridgeProtocol,
    event_id: String,
    block_number: i64,
    block_timestamp: i64,
    transaction_hash: String,
    log_index: i32,
    event_timestamp: i64,
) -> Result<Event> {
    match discriminator {
        10 => {
            let amount = u64::from_le_bytes(data[0..8].try_into()?);
            let destination_chain = u16::from_le_bytes(data[8..10].try_into()?);
            Ok(Event::BridgeLock(BridgeLockEvent {
                event_id,
                block_number,
                block_timestamp,
                transaction_hash,
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
                amount: amount.to_string(),
                destination_chain: Some(format!("allbridge:{}", destination_chain)),
                source_chain: None,
                message_hash: None,
                sequence_number: None,
            }))
        }
        11 => {
            let amount = u64::from_le_bytes(data[0..8].try_into()?);
            let source_chain = u16::from_le_bytes(data[8..10].try_into()?);
            Ok(Event::BridgeUnlock(BridgeUnlockEvent {
                event_id,
                block_number,
                block_timestamp,
                transaction_hash,
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
                amount: amount.to_string(),
                destination_chain: None,
                source_chain: Some(format!("allbridge:{}", source_chain)),
                message_hash: None,
                sequence_number: None,
            }))
        }
        _ => bail!("unknown Allbridge discriminator: {}", discriminator),
    }
}
