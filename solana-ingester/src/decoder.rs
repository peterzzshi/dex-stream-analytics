use anyhow::{bail, Result};

// Raydium AMM v4 instruction discriminators
const SWAP_BASE_IN: u8 = 9;
const SWAP_BASE_OUT: u8 = 11;
const SWAP_BASE_IN_V2: u8 = 16;
const SWAP_BASE_OUT_V2: u8 = 17;
const DEPOSIT: u8 = 3;
const WITHDRAW: u8 = 4;

#[derive(Debug, Clone)]
pub enum SwapDirection {
    BaseIn,   // Swap with exact input amount
    BaseOut,  // Swap with exact output amount
}

#[derive(Debug, Clone)]
pub enum DecodedInstruction {
    Swap {
        direction: SwapDirection,
        version: InstructionVersion,
        amount_in: u64,
        minimum_amount_out: u64,
    },
    Deposit {
        max_coin: u64,
        max_pc: u64,
        base_side: u64,
    },
    Withdraw {
        amount: u64,
    },
}

#[derive(Debug, Clone)]
pub enum InstructionVersion {
    V1,
    V2,
}

/// Decode Raydium AMM v4 instruction from raw bytes
pub fn decode_instruction(data: &[u8], account_count: usize) -> Result<DecodedInstruction> {
    if data.is_empty() {
        bail!("Empty instruction data");
    }

    let discriminator = data[0];

    match discriminator {
        SWAP_BASE_IN => decode_swap(data, account_count, SwapDirection::BaseIn, InstructionVersion::V1),
        SWAP_BASE_OUT => decode_swap(data, account_count, SwapDirection::BaseOut, InstructionVersion::V1),
        SWAP_BASE_IN_V2 => decode_swap(data, account_count, SwapDirection::BaseIn, InstructionVersion::V2),
        SWAP_BASE_OUT_V2 => decode_swap(data, account_count, SwapDirection::BaseOut, InstructionVersion::V2),
        DEPOSIT => decode_deposit(data, account_count),
        WITHDRAW => decode_withdraw(data, account_count),
        _ => bail!("Unknown instruction discriminator: {}", discriminator),
    }
}

fn decode_swap(
    data: &[u8],
    account_count: usize,
    direction: SwapDirection,
    version: InstructionVersion,
) -> Result<DecodedInstruction> {
    // Validate account count
    let expected_accounts = match version {
        InstructionVersion::V1 => 16..=17,  // V1 has 16-17 accounts (legacy market)
        InstructionVersion::V2 => 8..=8,    // V2 has exactly 8 accounts
    };

    if !expected_accounts.contains(&account_count) {
        bail!(
            "Invalid account count for swap {:?} {:?}: expected {:?}, got {}",
            version, direction, expected_accounts, account_count
        );
    }

    // Parse instruction data: [discriminator: 1 byte][amount_in: 8 bytes][minimum_amount_out: 8 bytes]
    if data.len() < 17 {
        bail!("Swap instruction data too short: {} bytes", data.len());
    }

    let amount_in = u64::from_le_bytes(data[1..9].try_into()?);
    let minimum_amount_out = u64::from_le_bytes(data[9..17].try_into()?);

    Ok(DecodedInstruction::Swap {
        direction,
        version,
        amount_in,
        minimum_amount_out,
    })
}

fn decode_deposit(data: &[u8], account_count: usize) -> Result<DecodedInstruction> {
    // Modern deposit: 11 accounts, legacy: 14-15 accounts
    if account_count < 11 {
        bail!("Invalid account count for deposit: expected >=11, got {}", account_count);
    }

    // Parse instruction data: [discriminator: 1 byte][max_coin: 8 bytes][max_pc: 8 bytes][base_side: 8 bytes]
    if data.len() < 25 {
        bail!("Deposit instruction data too short: {} bytes", data.len());
    }

    let max_coin = u64::from_le_bytes(data[1..9].try_into()?);
    let max_pc = u64::from_le_bytes(data[9..17].try_into()?);
    let base_side = u64::from_le_bytes(data[17..25].try_into()?);

    Ok(DecodedInstruction::Deposit {
        max_coin,
        max_pc,
        base_side,
    })
}

fn decode_withdraw(data: &[u8], account_count: usize) -> Result<DecodedInstruction> {
    // Modern withdraw: 11 accounts, legacy: 20-23 accounts
    if account_count < 11 {
        bail!("Invalid account count for withdraw: expected >=11, got {}", account_count);
    }

    // Parse instruction data: [discriminator: 1 byte][amount: 8 bytes]
    if data.len() < 9 {
        bail!("Withdraw instruction data too short: {} bytes", data.len());
    }

    let amount = u64::from_le_bytes(data[1..9].try_into()?);

    Ok(DecodedInstruction::Withdraw { amount })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_swap_v2() {
        // Mock swap V2 instruction: discriminator=16, amount_in=1000000, min_out=990000
        let mut data = vec![SWAP_BASE_IN_V2];
        data.extend_from_slice(&1000000u64.to_le_bytes());
        data.extend_from_slice(&990000u64.to_le_bytes());

        let result = decode_instruction(&data, 8).unwrap();

        match result {
            DecodedInstruction::Swap { direction, version, amount_in, minimum_amount_out } => {
                assert!(matches!(direction, SwapDirection::BaseIn));
                assert!(matches!(version, InstructionVersion::V2));
                assert_eq!(amount_in, 1000000);
                assert_eq!(minimum_amount_out, 990000);
            }
            _ => panic!("Expected Swap instruction"),
        }
    }

    #[test]
    fn test_decode_deposit() {
        // Mock deposit instruction
        let mut data = vec![DEPOSIT];
        data.extend_from_slice(&5000000u64.to_le_bytes());
        data.extend_from_slice(&5000000u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());

        let result = decode_instruction(&data, 11).unwrap();

        match result {
            DecodedInstruction::Deposit { max_coin, max_pc, base_side } => {
                assert_eq!(max_coin, 5000000);
                assert_eq!(max_pc, 5000000);
                assert_eq!(base_side, 0);
            }
            _ => panic!("Expected Deposit instruction"),
        }
    }

    #[test]
    fn test_invalid_account_count() {
        let data = vec![SWAP_BASE_IN_V2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let result = decode_instruction(&data, 5);  // V2 needs 8 accounts
        assert!(result.is_err());
    }
}
