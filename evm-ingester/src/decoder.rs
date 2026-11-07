use anyhow::{bail, Context, Result};
use ethers::abi::{Abi, AbiParser, Function, RawLog};
use ethers::prelude::*;
use std::sync::OnceLock;

/// The pair ABI is the single reference for every topic and selector this module exposes.
pub fn pair_abi() -> &'static Abi {
    static ABI: OnceLock<Abi> = OnceLock::new();
    ABI.get_or_init(|| {
        serde_json::from_str(include_str!("../../schemas/uniswap_v2_pair.abi.json"))
            .expect("embedded Uniswap V2 pair ABI is invalid")
    })
}

/// A view function on the pair itself, resolved by name from the embedded ABI.
pub fn pair_view_call(name: &str) -> Result<&'static Function> {
    pair_abi()
        .function(name)
        .with_context(|| format!("{name}() missing from pair ABI"))
}

/// `decimals()` lives on the token contracts, not on the pair.
pub fn decimals_call() -> &'static Function {
    static CALL: OnceLock<Function> = OnceLock::new();
    CALL.get_or_init(|| {
        AbiParser::default()
            .parse_function("decimals() returns (uint8)")
            .expect("decimals() signature is valid")
    })
}

/// Field order here *is* the event declaration order: the derive builds the signature from
/// it, so reordering a field changes the topic hash and fails `signatures_match_pair_abi`.
#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "Swap")]
pub struct SwapLog {
    #[ethevent(indexed)]
    pub sender: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
    #[ethevent(indexed)]
    pub recipient: Address,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "Mint")]
pub struct MintLog {
    #[ethevent(indexed)]
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone, PartialEq, EthEvent)]
#[ethevent(name = "Burn")]
pub struct BurnLog {
    #[ethevent(indexed)]
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    #[ethevent(indexed)]
    pub recipient: Address,
}

/// The pool events we turn into domain events. Sync and Transfer are deliberately absent:
/// they are part of the pool's log stream but carry nothing we publish.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolLog {
    Swap(SwapLog),
    Mint(MintLog),
    Burn(BurnLog),
}

/// `Ok(None)` for a topic we do not model — normal log-stream noise, not a failure.
pub fn decode(log: &Log) -> Result<Option<PoolLog>> {
    let Some(topic) = log.topics.first() else {
        return Ok(None);
    };
    let raw = RawLog {
        topics: log.topics.clone(),
        data: log.data.to_vec(),
    };

    if topic == &SwapLog::signature() {
        let swap: SwapLog = decode_as(&raw)?;
        if swap.sender.is_zero() || swap.recipient.is_zero() {
            bail!("swap addresses cannot be zero");
        }
        return Ok(Some(PoolLog::Swap(swap)));
    }
    if topic == &MintLog::signature() {
        let mint: MintLog = decode_as(&raw)?;
        if mint.sender.is_zero() {
            bail!("mint sender address cannot be zero");
        }
        return Ok(Some(PoolLog::Mint(mint)));
    }
    if topic == &BurnLog::signature() {
        let burn: BurnLog = decode_as(&raw)?;
        if burn.sender.is_zero() || burn.recipient.is_zero() {
            bail!("burn addresses cannot be zero");
        }
        return Ok(Some(PoolLog::Burn(burn)));
    }

    Ok(None)
}

/// `decode_log` also verifies `topics[0]` against `E::signature()`, so a topic/struct
/// mismatch cannot silently decode into the wrong shape.
fn decode_as<E: EthEvent>(raw: &RawLog) -> Result<E> {
    E::decode_log(raw).with_context(|| format!("failed to decode {} log", E::abi_signature()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::abi::EventExt;

    fn encode_words(words: &[U256]) -> Vec<u8> {
        let mut data = vec![0u8; 32 * words.len()];
        for (index, word) in words.iter().enumerate() {
            word.to_big_endian(&mut data[index * 32..(index + 1) * 32]);
        }
        data
    }

    #[test]
    fn signatures_match_pair_abi() {
        let abi = pair_abi();
        for (name, derived_signature, derived_abi) in [
            (
                "Swap",
                SwapLog::signature(),
                SwapLog::abi_signature().into_owned(),
            ),
            (
                "Mint",
                MintLog::signature(),
                MintLog::abi_signature().into_owned(),
            ),
            (
                "Burn",
                BurnLog::signature(),
                BurnLog::abi_signature().into_owned(),
            ),
        ] {
            let event = abi.event(name).expect("event present in pair ABI");
            assert_eq!(
                derived_abi,
                event.abi_signature(),
                "{name} signature drifted"
            );
            assert_eq!(derived_signature, event.signature(), "{name} topic drifted");
        }
    }

    #[test]
    fn decodes_swap_with_trailing_indexed_recipient() {
        // Real Uniswap V2 Swap: the recipient is indexed but sits last in the declaration,
        // so it arrives as topics[2] while the four amounts fill the data payload.
        let sender: Address = "0x51c72848c68a965f66fa7a88855f9f7784502a7f"
            .parse()
            .unwrap();
        let amount0_in = U256::from(93905317013992882176u128);
        let amount1_out = U256::from(292629181741008128u128);

        let log = Log {
            topics: vec![SwapLog::signature(), H256::from(sender), H256::from(sender)],
            data: Bytes::from(encode_words(&[
                amount0_in,
                U256::zero(),
                U256::zero(),
                amount1_out,
            ])),
            ..Default::default()
        };

        let PoolLog::Swap(swap) = decode(&log).unwrap().expect("swap is a modelled topic") else {
            panic!("expected a swap");
        };
        assert_eq!(
            swap,
            SwapLog {
                sender,
                amount0_in,
                amount1_in: U256::zero(),
                amount0_out: U256::zero(),
                amount1_out,
                recipient: sender,
            }
        );
    }

    #[test]
    fn unmodelled_topic_is_not_an_error() {
        let log = Log {
            topics: vec![H256::zero()],
            data: Bytes::new(),
            ..Default::default()
        };

        assert_eq!(decode(&log).unwrap(), None);
    }

    #[test]
    fn zero_swap_addresses_are_rejected() {
        let log = Log {
            topics: vec![
                SwapLog::signature(),
                H256::zero(),
                H256::from(Address::repeat_byte(0x11)),
            ],
            data: Bytes::from(encode_words(&[
                U256::one(),
                U256::zero(),
                U256::zero(),
                U256::one(),
            ])),
            ..Default::default()
        };

        assert!(decode(&log)
            .unwrap_err()
            .to_string()
            .contains("cannot be zero"));
    }
}
