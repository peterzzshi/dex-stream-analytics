use anyhow::{Context, Result};
use dex_ingester_common::{Secret, SharedConfig};
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::str::FromStr;

use crate::bridge_file_config::BridgeFileConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub shared: SharedConfig,
    pub solana: SolanaConfig,
    pub bridges: Vec<BridgeConfig>,
}

#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub app_port: u16,
    pub rpc: Secret,
    pub commitment: String,
    #[expect(dead_code)]
    pub chain: String,
    pub chain_name: String,
    pub finality_buffer_size: usize,
    pub finality_poll_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub name: String,
    pub protocol: BridgeProtocol,
    pub program_id: Pubkey,
    pub chain: String,
    pub chain_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeProtocol {
    Wormhole,
    Allbridge,
}

impl BridgeProtocol {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "wormhole" => Ok(BridgeProtocol::Wormhole),
            "allbridge" => Ok(BridgeProtocol::Allbridge),
            other => anyhow::bail!("Unknown bridge protocol: {}", other),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeProtocol::Wormhole => "wormhole",
            BridgeProtocol::Allbridge => "allbridge",
        }
    }
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        let bridge_path =
            env::var("BRIDGE_CONFIG_FILE").unwrap_or_else(|_| "solana-bridges.toml".to_string());
        let bridge_contents = std::fs::read_to_string(&bridge_path)
            .with_context(|| format!("Failed to read bridge config {bridge_path}"))?;

        Self::load(|key| env::var(key), &bridge_contents)
    }

    // Testable pure function - takes env lookup and file contents as parameters
    fn load<F>(get_env: F, bridge_toml: &str) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let shared = SharedConfig::load(&get_env)?;
        let bridge_file = BridgeFileConfig::parse(bridge_toml)?;

        let solana = SolanaConfig {
            app_port: get_env("APP_PORT")
                .context("APP_PORT not set")?
                .parse()
                .context("APP_PORT must be a valid number")?,
            rpc: Secret::new(get_env("SOLANA_RPC_URL").context("SOLANA_RPC_URL not set")?),
            commitment: get_env("SOLANA_COMMITMENT").context("SOLANA_COMMITMENT not set")?,
            chain: "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp".to_string(),
            chain_name: "Solana".to_string(),
            finality_buffer_size: get_env("FINALITY_BUFFER_SIZE")
                .context("FINALITY_BUFFER_SIZE not set")?
                .parse()
                .context("FINALITY_BUFFER_SIZE must be a valid number")?,
            finality_poll_interval_secs: get_env("FINALITY_POLL_INTERVAL_SECS")
                .context("FINALITY_POLL_INTERVAL_SECS not set")?
                .parse()
                .context("FINALITY_POLL_INTERVAL_SECS must be a valid number")?,
        };

        let bridges = bridge_file
            .bridge
            .into_iter()
            .map(|b| {
                Ok(BridgeConfig {
                    name: b.name.clone(),
                    protocol: BridgeProtocol::from_str(&b.protocol)
                        .with_context(|| format!("Invalid protocol for bridge {}", b.name))?,
                    program_id: Pubkey::from_str(&b.program_id)
                        .with_context(|| format!("Invalid program_id for bridge {}", b.name))?,
                    chain: "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp".to_string(),
                    chain_name: b.chain_name,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Config {
            shared,
            solana,
            bridges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_BRIDGES: &str = r#"
[[bridge]]
name = "Wormhole Portal"
protocol = "wormhole"
program_id = "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb"
chain_name = "solana"

[[bridge]]
name = "Allbridge"
protocol = "allbridge"
program_id = "bb1XfoeKzmYZwmW2xxLUHJx8Q8x6dRhvyCMqWze3h4e"
chain_name = "solana"
"#;

    fn env_without(missing: &'static str) -> impl Fn(&str) -> Result<String, env::VarError> {
        move |key| {
            if key == missing {
                return Err(env::VarError::NotPresent);
            }
            match key {
                "DAPR_HOST" => Ok("sidecar".to_string()),
                "DAPR_HTTP_PORT" => Ok("3501".to_string()),
                "PUBSUB_NAME" => Ok("kafka-pubsub".to_string()),
                "TOPIC_TRADING" => Ok("trading".to_string()),
                "TOPIC_LIQUIDITY" => Ok("liquidity".to_string()),
                "TOPIC_DLQ" => Ok("dlq".to_string()),
                "APP_PORT" => Ok("3001".to_string()),
                "SOLANA_RPC_URL" => Ok("https://rpc.example".to_string()),
                "SOLANA_COMMITMENT" => Ok("finalized".to_string()),
                "FINALITY_BUFFER_SIZE" => Ok("100".to_string()),
                "FINALITY_POLL_INTERVAL_SECS" => Ok("1".to_string()),
                _ => Err(env::VarError::NotPresent),
            }
        }
    }

    #[test]
    fn bridge_config_loads_with_parsed_protocol_and_pubkey() {
        let config = Config::load(env_without(""), VALID_BRIDGES).expect("should load");

        assert_eq!(config.solana.chain_name, "Solana");
        assert_eq!(config.bridges.len(), 2);

        assert_eq!(config.bridges[0].name, "Wormhole Portal");
        assert_eq!(config.bridges[0].protocol, BridgeProtocol::Wormhole);
        assert_eq!(
            config.bridges[0].program_id.to_string(),
            "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb"
        );

        assert_eq!(config.bridges[1].name, "Allbridge");
        assert_eq!(config.bridges[1].protocol, BridgeProtocol::Allbridge);
    }

    #[test]
    fn missing_env_var_is_named_in_the_error() {
        let err = Config::load(env_without("SOLANA_RPC_URL"), VALID_BRIDGES).unwrap_err();

        assert!(format!("{err:#}").contains("SOLANA_RPC_URL"));
    }

    #[test]
    fn invalid_protocol_fails_parse() {
        let invalid = r#"
[[bridge]]
name = "Invalid"
protocol = "unknown"
program_id = "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb"
chain_name = "solana"
"#;

        let err = Config::load(env_without(""), invalid).unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("protocol") || msg.contains("Invalid"));
    }

    #[test]
    fn invalid_program_id_fails_parse() {
        let invalid = r#"
[[bridge]]
name = "Invalid"
protocol = "wormhole"
program_id = "not-a-valid-pubkey"
chain_name = "solana"
"#;

        let err = Config::load(env_without(""), invalid).unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("program_id") || msg.contains("Invalid"));
    }
}
