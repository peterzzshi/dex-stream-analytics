use anyhow::{bail, Result};

// Raydium AMM v4 instruction discriminators
const SWAP_BASE_IN: u8 = 9;
const SWAP_BASE_OUT: u8 = 11;
const SWAP_BASE_IN_V2: u8 = 16;
const SWAP_BASE_OUT_V2: u8 = 17;
const DEPOSIT: u8 = 3;
const WITHDRAW: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedInstruction {
    Swap,
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone, Copy)]
enum InstructionVersion {
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
        SWAP_BASE_IN | SWAP_BASE_OUT => decode_swap(data, account_count, InstructionVersion::V1),
        SWAP_BASE_IN_V2 | SWAP_BASE_OUT_V2 => {
            decode_swap(data, account_count, InstructionVersion::V2)
        }
        DEPOSIT => decode_deposit(data, account_count),
        WITHDRAW => decode_withdraw(data, account_count),
        _ => bail!("Unknown instruction discriminator: {}", discriminator),
    }
}

fn decode_swap(
    data: &[u8],
    account_count: usize,
    version: InstructionVersion,
) -> Result<DecodedInstruction> {
    let expected_accounts = match version {
        InstructionVersion::V1 => 16..=17, // V1 has 16-17 accounts (legacy market)
        InstructionVersion::V2 => 8..=8,   // V2 has exactly 8 accounts
    };

    if !expected_accounts.contains(&account_count) {
        bail!(
            "Invalid account count for swap {:?}: expected {:?}, got {}",
            version,
            expected_accounts,
            account_count
        );
    }

    // [discriminator: 1 byte][amount_in: 8 bytes][minimum_amount_out: 8 bytes]
    if data.len() < 17 {
        bail!("Swap instruction data too short: {} bytes", data.len());
    }

    Ok(DecodedInstruction::Swap)
}

fn decode_deposit(data: &[u8], account_count: usize) -> Result<DecodedInstruction> {
    // Modern deposit: 11 accounts, legacy: 14-15 accounts
    if account_count < 11 {
        bail!(
            "Invalid account count for deposit: expected >=11, got {}",
            account_count
        );
    }

    // [discriminator: 1 byte][max_coin: 8 bytes][max_pc: 8 bytes][base_side: 8 bytes]
    if data.len() < 25 {
        bail!("Deposit instruction data too short: {} bytes", data.len());
    }

    Ok(DecodedInstruction::Deposit)
}

fn decode_withdraw(data: &[u8], account_count: usize) -> Result<DecodedInstruction> {
    // Modern withdraw: 11 accounts, legacy: 20-23 accounts
    if account_count < 11 {
        bail!(
            "Invalid account count for withdraw: expected >=11, got {}",
            account_count
        );
    }

    // [discriminator: 1 byte][amount: 8 bytes]
    if data.len() < 9 {
        bail!("Withdraw instruction data too short: {} bytes", data.len());
    }

    Ok(DecodedInstruction::Withdraw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(discriminator: u8, data_len: usize) -> Vec<u8> {
        let mut data = vec![0u8; data_len];
        data[0] = discriminator;
        data
    }

    #[test]
    fn discriminators_map_to_instruction_kinds() {
        assert_eq!(
            decode_instruction(&instruction(SWAP_BASE_IN, 17), 16).unwrap(),
            DecodedInstruction::Swap
        );
        assert_eq!(
            decode_instruction(&instruction(SWAP_BASE_OUT_V2, 17), 8).unwrap(),
            DecodedInstruction::Swap
        );
        assert_eq!(
            decode_instruction(&instruction(DEPOSIT, 25), 11).unwrap(),
            DecodedInstruction::Deposit
        );
        assert_eq!(
            decode_instruction(&instruction(WITHDRAW, 9), 11).unwrap(),
            DecodedInstruction::Withdraw
        );
    }

    #[test]
    fn swap_account_count_is_validated_per_version() {
        assert!(decode_instruction(&instruction(SWAP_BASE_IN_V2, 17), 5).is_err());
        assert!(decode_instruction(&instruction(SWAP_BASE_IN, 17), 8).is_err());
    }

    #[test]
    fn truncated_or_unknown_instructions_are_rejected() {
        assert!(decode_instruction(&instruction(SWAP_BASE_IN, 10), 16).is_err());
        assert!(decode_instruction(&instruction(DEPOSIT, 10), 11).is_err());
        assert!(decode_instruction(&instruction(WITHDRAW, 4), 11).is_err());
        assert!(decode_instruction(&instruction(99, 25), 11).is_err());
        assert!(decode_instruction(&[], 11).is_err());
    }
}
