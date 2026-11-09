use anyhow::{Context, Result};
use solana_sdk::instruction::CompiledInstruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeProtocol {
    Wormhole,
    Allbridge,
}

impl BridgeProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wormhole => "wormhole",
            Self::Allbridge => "allbridge",
        }
    }
}

#[derive(Debug, Clone)]
pub enum BridgeInstruction {
    Lock {
        amount: u64,
        destination_chain: Option<String>,
    },
    Unlock {
        amount: u64,
        source_chain: Option<String>,
    },
    Mint {
        amount: u64,
        source_chain: Option<String>,
    },
    Burn {
        amount: u64,
        destination_chain: Option<String>,
    },
}

pub fn decode(
    instruction: &CompiledInstruction,
    protocol: BridgeProtocol,
) -> Result<Option<BridgeInstruction>> {
    if instruction.data.is_empty() {
        return Ok(None);
    }

    let discriminator = instruction.data[0];

    match protocol {
        BridgeProtocol::Wormhole => decode_wormhole(discriminator, &instruction.data),
        BridgeProtocol::Allbridge => decode_allbridge(discriminator, &instruction.data),
    }
}

fn decode_wormhole(discriminator: u8, data: &[u8]) -> Result<Option<BridgeInstruction>> {
    match discriminator {
        // transfer_wrapped: lock operation
        1 => {
            if data.len() < 9 {
                return Ok(None);
            }
            let amount = u64::from_le_bytes(
                data[1..9]
                    .try_into()
                    .context("failed to parse amount in transfer_wrapped")?,
            );
            Ok(Some(BridgeInstruction::Lock {
                amount,
                destination_chain: None,
            }))
        }
        // complete_transfer_native: unlock operation
        2 => {
            if data.len() < 9 {
                return Ok(None);
            }
            let amount = u64::from_le_bytes(
                data[1..9]
                    .try_into()
                    .context("failed to parse amount in complete_transfer_native")?,
            );
            Ok(Some(BridgeInstruction::Unlock {
                amount,
                source_chain: None,
            }))
        }
        // complete_transfer_wrapped: mint operation
        3 => {
            if data.len() < 9 {
                return Ok(None);
            }
            let amount = u64::from_le_bytes(
                data[1..9]
                    .try_into()
                    .context("failed to parse amount in complete_transfer_wrapped")?,
            );
            Ok(Some(BridgeInstruction::Mint {
                amount,
                source_chain: None,
            }))
        }
        _ => Ok(None),
    }
}

fn decode_allbridge(discriminator: u8, data: &[u8]) -> Result<Option<BridgeInstruction>> {
    match discriminator {
        // lock
        10 => {
            if data.len() < 9 {
                return Ok(None);
            }
            let amount = u64::from_le_bytes(
                data[1..9]
                    .try_into()
                    .context("failed to parse amount in lock")?,
            );
            Ok(Some(BridgeInstruction::Lock {
                amount,
                destination_chain: None,
            }))
        }
        // unlock
        11 => {
            if data.len() < 9 {
                return Ok(None);
            }
            let amount = u64::from_le_bytes(
                data[1..9]
                    .try_into()
                    .context("failed to parse amount in unlock")?,
            );
            Ok(Some(BridgeInstruction::Unlock {
                amount,
                source_chain: None,
            }))
        }
        _ => Ok(None),
    }
}
