use anyhow::{Context, Result};
use dex_ingester_common::{RetryConfig, Secret, SharedConfig};
use ethers::types::Address;
use std::env;
use std::str::FromStr;

use crate::bridge_decoder::BridgeProtocol;
use crate::bridge_file_config::BridgeFileConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub shared: SharedConfig,
    pub bridges: Vec<Bridge>,
}

#[derive(Debug, Clone)]
pub struct Bridge {
    pub name: String,
    pub protocol: BridgeProtocol,
    pub address: Address,
    pub chain_id: String,
    pub chain_name: String,
    pub rpc: Secret,
    pub confirmations: u64,
    pub retry: RetryConfig,
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        let bridge_path =
            env::var("BRIDGE_CONFIG_FILE").unwrap_or_else(|_| "evm-bridges.toml".to_string());
        let bridge_contents = std::fs::read_to_string(&bridge_path)
            .with_context(|| format!("Failed to read bridge config {bridge_path}"))?;

        Self::load(|key| env::var(key), &bridge_contents)
    }

    fn load<F>(get_env: F, bridge_toml: &str) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let shared = SharedConfig::load(&get_env)?;
        let bridge_file = BridgeFileConfig::parse(bridge_toml)?;

        let rpc = Secret::new(get_env("ETH_RPC_URL").context("ETH_RPC_URL not set")?);

        let bridges = bridge_file
            .bridge
            .into_iter()
            .map(|b| {
                Ok(Bridge {
                    name: b.name,
                    protocol: BridgeProtocol::from_str(&b.protocol)?,
                    address: Address::from_str(&b.address)
                        .with_context(|| format!("Invalid bridge address: {}", b.address))?,
                    chain_id: format!("eip155:{}", b.chain_id),
                    chain_name: b.chain_name,
                    rpc: rpc.clone(),
                    confirmations: 12, // EVM default
                    retry: RetryConfig::default(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Config { shared, bridges })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DAPR_HOST", "evm-ingester-dapr"),
            ("DAPR_HTTP_PORT", "3500"),
            ("PUBSUB_NAME", "kafka-pubsub"),
            ("TOPIC_TRADING", "bridge-transfer-events"),
            ("TOPIC_LIQUIDITY", "bridge-supply-events"),
            ("TOPIC_DLQ", "bridge-dlq-events"),
            ("ETH_RPC_URL", "wss://eth-sepolia.example.com"),
        ]
    }

    fn lookup<'a>(
        vars: &'a [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Result<String, env::VarError> + 'a {
        move |key| {
            vars.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
                .ok_or(env::VarError::NotPresent)
        }
    }

    const BRIDGE_CONFIG: &str = r#"
[[bridge]]
name = "Wormhole"
protocol = "wormhole"
address = "0x3ee18B2214AFF97000D974cf647E7C347E8fa585"
chain_id = 1
chain_name = "ethereum"

[[bridge]]
name = "LayerZero"
protocol = "layerzero"
address = "0x1a44076050125825900e736c501f859c50fE728c"
chain_id = 1
chain_name = "ethereum"
"#;

    #[test]
    fn bridge_config_loads_successfully() {
        let config = Config::load(lookup(&env_vars()), BRIDGE_CONFIG).expect("should load");

        assert_eq!(config.bridges.len(), 2);
        assert_eq!(config.bridges[0].name, "Wormhole");
        assert_eq!(config.bridges[0].protocol, BridgeProtocol::Wormhole);
        assert_eq!(config.bridges[0].chain_id, "eip155:1");
        assert_eq!(config.bridges[0].confirmations, 12);

        assert_eq!(config.bridges[1].name, "LayerZero");
        assert_eq!(config.bridges[1].protocol, BridgeProtocol::LayerZero);
    }

    #[test]
    fn missing_rpc_url_is_named_in_the_error() {
        let mut vars = env_vars();
        vars.retain(|(k, _)| *k != "ETH_RPC_URL");

        let err = Config::load(lookup(&vars), BRIDGE_CONFIG).unwrap_err();

        assert!(format!("{err:#}").contains("ETH_RPC_URL"));
    }

    #[test]
    fn config_debug_output_hides_the_rpc_url() {
        let config = Config::load(lookup(&env_vars()), BRIDGE_CONFIG).expect("should load");

        assert!(!format!("{config:?}").contains("eth-sepolia.example.com"));
    }

    #[test]
    fn invalid_protocol_fails_parse() {
        let bad_config = BRIDGE_CONFIG.replace("wormhole", "unknown");

        let err = Config::load(lookup(&env_vars()), &bad_config).unwrap_err();

        let err_msg = format!("{err:#}");
        assert!(
            err_msg.contains("Invalid protocol") || err_msg.contains("Unknown bridge protocol"),
            "Expected protocol error, got: {err_msg}"
        );
    }
}
