use anyhow::{bail, Context, Result};
use ethers::abi::{Abi, RawLog};
use ethers::prelude::*;
use std::sync::OnceLock;

// Event topic hashes (computed lazily to avoid hardcoding)
pub fn swap_event_topic() -> H256 {
    H256::from_slice(&ethers::utils::keccak256(
        b"Swap(address,uint256,uint256,uint256,uint256,address)",
    ))
}

pub fn mint_event_topic() -> H256 {
    H256::from_slice(&ethers::utils::keccak256(b"Mint(address,uint256,uint256)"))
}

pub fn burn_event_topic() -> H256 {
    H256::from_slice(&ethers::utils::keccak256(
        b"Burn(address,uint256,uint256,address)",
    ))
}

pub fn transfer_event_topic() -> H256 {
    H256::from_slice(&ethers::utils::keccak256(
        b"Transfer(address,address,uint256)",
    ))
}

static UNISWAP_V2_ABI: OnceLock<Abi> = OnceLock::new();

/// Get the Uniswap V2 Pair ABI (lazily loaded)
fn get_uniswap_v2_abi() -> &'static Abi {
    UNISWAP_V2_ABI.get_or_init(|| {
        serde_json::from_str(include_str!("../schemas/uniswap_v2_pair.abi.json"))
            .expect("failed to parse Uniswap V2 ABI")
    })
}

/// Decoded Swap event data
#[derive(Debug, Clone, PartialEq)]
pub struct SwapEventData {
    pub sender: Address,
    pub recipient: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
}

/// Decoded Mint event data
#[derive(Debug, Clone, PartialEq)]
pub struct MintEventData {
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
}

/// Decoded Burn event data
#[derive(Debug, Clone, PartialEq)]
pub struct BurnEventData {
    pub sender: Address,
    pub recipient: Address,
    pub amount0: U256,
    pub amount1: U256,
}

/// Decoded Transfer event data
#[derive(Debug, Clone, PartialEq)]
pub struct TransferEventData {
    pub from: Address,
    pub to: Address,
    pub value: U256,
}

/// Parse a Swap event log
pub fn parse_swap_log(log: &Log) -> Result<SwapEventData> {
    if log.topics.len() < 3 {
        bail!(
            "swap log missing indexed topics: expected 3, got {}",
            log.topics.len()
        );
    }

    let abi = get_uniswap_v2_abi();
    let event = abi.event("Swap").context("Swap event not found in ABI")?;

    let decoded = event
        .parse_log(RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        })
        .context("failed to parse Swap log")?;

    // Extract indexed fields from topics
    let sender = Address::from(log.topics[1]);
    let recipient = Address::from(log.topics[2]);

    // Extract non-indexed fields from data
    let amount0_in = decoded.params[0]
        .value
        .clone()
        .into_uint()
        .context("amount0In is not uint256")?;
    let amount1_in = decoded.params[1]
        .value
        .clone()
        .into_uint()
        .context("amount1In is not uint256")?;
    let amount0_out = decoded.params[2]
        .value
        .clone()
        .into_uint()
        .context("amount0Out is not uint256")?;
    let amount1_out = decoded.params[3]
        .value
        .clone()
        .into_uint()
        .context("amount1Out is not uint256")?;

    if sender.is_zero() || recipient.is_zero() {
        bail!("swap addresses cannot be zero");
    }

    Ok(SwapEventData {
        sender,
        recipient,
        amount0_in,
        amount1_in,
        amount0_out,
        amount1_out,
    })
}

/// Parse a Mint event log
pub fn parse_mint_log(log: &Log) -> Result<MintEventData> {
    if log.topics.len() < 2 {
        bail!(
            "mint log missing indexed topics: expected 2, got {}",
            log.topics.len()
        );
    }

    let abi = get_uniswap_v2_abi();
    let event = abi.event("Mint").context("Mint event not found in ABI")?;

    let decoded = event
        .parse_log(RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        })
        .context("failed to parse Mint log")?;

    let sender = Address::from(log.topics[1]);

    let amount0 = decoded.params[0]
        .value
        .clone()
        .into_uint()
        .context("amount0 is not uint256")?;
    let amount1 = decoded.params[1]
        .value
        .clone()
        .into_uint()
        .context("amount1 is not uint256")?;

    if sender.is_zero() {
        bail!("mint sender address cannot be zero");
    }

    Ok(MintEventData {
        sender,
        amount0,
        amount1,
    })
}

/// Parse a Burn event log
pub fn parse_burn_log(log: &Log) -> Result<BurnEventData> {
    if log.topics.len() < 3 {
        bail!(
            "burn log missing indexed topics: expected 3, got {}",
            log.topics.len()
        );
    }

    let abi = get_uniswap_v2_abi();
    let event = abi.event("Burn").context("Burn event not found in ABI")?;

    let decoded = event
        .parse_log(RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        })
        .context("failed to parse Burn log")?;

    let sender = Address::from(log.topics[1]);
    let recipient = Address::from(log.topics[2]);

    let amount0 = decoded.params[0]
        .value
        .clone()
        .into_uint()
        .context("amount0 is not uint256")?;
    let amount1 = decoded.params[1]
        .value
        .clone()
        .into_uint()
        .context("amount1 is not uint256")?;

    if sender.is_zero() || recipient.is_zero() {
        bail!("burn addresses cannot be zero");
    }

    Ok(BurnEventData {
        sender,
        recipient,
        amount0,
        amount1,
    })
}

/// Parse a Transfer event log
pub fn parse_transfer_log(log: &Log) -> Result<TransferEventData> {
    if log.topics.len() < 3 {
        bail!(
            "transfer log missing indexed topics: expected 3, got {}",
            log.topics.len()
        );
    }

    let from = Address::from(log.topics[1]);
    let to = Address::from(log.topics[2]);

    // Transfer event data is just the uint256 value (32 bytes)
    if log.data.len() != 32 {
        bail!(
            "transfer log data length mismatch: expected 32 bytes, got {}",
            log.data.len()
        );
    }

    let value = U256::from_big_endian(&log.data);

    if from.is_zero() || to.is_zero() {
        bail!("transfer addresses cannot be zero");
    }

    Ok(TransferEventData { from, to, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_topic_constants() {
        // Verify topic hashes match expected values
        let swap_hash =
            ethers::utils::keccak256(b"Swap(address,uint256,uint256,uint256,uint256,address)");
        assert_eq!(H256::from_slice(&swap_hash), swap_event_topic());

        let mint_hash = ethers::utils::keccak256(b"Mint(address,uint256,uint256)");
        assert_eq!(H256::from_slice(&mint_hash), mint_event_topic());

        let burn_hash = ethers::utils::keccak256(b"Burn(address,uint256,uint256,address)");
        assert_eq!(H256::from_slice(&burn_hash), burn_event_topic());

        let transfer_hash = ethers::utils::keccak256(b"Transfer(address,address,uint256)");
        assert_eq!(H256::from_slice(&transfer_hash), transfer_event_topic());
    }

    #[test]
    fn test_uniswap_v2_abi_loads() {
        let abi = get_uniswap_v2_abi();
        assert!(abi.event("Swap").is_ok());
        assert!(abi.event("Mint").is_ok());
        assert!(abi.event("Burn").is_ok());
    }

    #[test]
    fn test_parse_transfer_log_valid() {
        let from: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let to: Address = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();
        let value = U256::from(1000u64);

        let mut data = vec![0u8; 32];
        value.to_big_endian(&mut data);

        let log = Log {
            address: Address::zero(),
            topics: vec![transfer_event_topic(), H256::from(from), H256::from(to)],
            data: Bytes::from(data),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let result = parse_transfer_log(&log).unwrap();
        assert_eq!(result.from, from);
        assert_eq!(result.to, to);
        assert_eq!(result.value, value);
    }

    #[test]
    fn test_parse_transfer_log_missing_topics() {
        let log = Log {
            address: Address::zero(),
            topics: vec![transfer_event_topic()], // Missing from and to
            data: Bytes::from(vec![0u8; 32]),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let result = parse_transfer_log(&log);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing indexed topics"));
    }

    #[test]
    fn test_parse_transfer_log_zero_addresses() {
        let mut data = vec![0u8; 32];
        U256::from(1000u64).to_big_endian(&mut data);

        let log = Log {
            address: Address::zero(),
            topics: vec![
                transfer_event_topic(),
                H256::zero(), // from = zero
                H256::zero(), // to = zero
            ],
            data: Bytes::from(data),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let result = parse_transfer_log(&log);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("addresses cannot be zero"));
    }

    #[test]
    fn test_parse_transfer_log_invalid_data_length() {
        let from: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let to: Address = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();

        let log = Log {
            address: Address::zero(),
            topics: vec![transfer_event_topic(), H256::from(from), H256::from(to)],
            data: Bytes::from(vec![0u8; 16]), // Wrong length
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let result = parse_transfer_log(&log);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("data length mismatch"));
    }
}
