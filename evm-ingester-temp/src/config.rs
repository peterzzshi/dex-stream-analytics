use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

pub const ENV_CONFIG_PATH: &str = "CONFIG_PATH";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dapr: DaprConfig,
    pub topics: TopicsConfig,
    pub app: AppConfig,
    pub chains: Vec<Chain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaprConfig {
    pub host: String,
    #[serde(rename = "httpPort")]
    pub http_port: u16,
    #[serde(rename = "pubsubName")]
    pub pubsub_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicsConfig {
    pub trading: String,
    pub liquidity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub chain: String,
    pub name: String,
    pub rpc: String,
    #[serde(default)]
    pub venue: String,
    pub confirmations: u64,
    pub pools: Vec<Pool>,
}

impl Chain {
    pub fn venue_or_default(&self) -> &str {
        if self.venue.is_empty() {
            "uniswap-v2"
        } else {
            &self.venue
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub name: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if path.is_empty() {
            anyhow::bail!("CONFIG_PATH is required");
        }

        let raw = fs::read_to_string(path).with_context(|| format!("cannot read {}", path))?;

        let expanded = expand_env(&raw)?;

        let config: Config = serde_yaml::from_str(&expanded).context("invalid YAML")?;

        config.validate()?;

        Ok(config)
    }

    pub fn load_from_env() -> Result<Self> {
        let path = env::var(ENV_CONFIG_PATH).context("CONFIG_PATH environment variable not set")?;
        Self::load(&path)
    }

    fn validate(&self) -> Result<()> {
        for chain in &self.chains {
            for pool in &chain.pools {
                // Validate EVM address format (0x + 40 hex chars)
                if !is_valid_eth_address(&pool.pair_address) {
                    anyhow::bail!(
                        "chain {:?} pool {:?} has invalid EVM address: {}",
                        chain.chain,
                        pool.name,
                        pool.pair_address
                    );
                }
            }
        }
        Ok(())
    }
}

fn expand_env(text: &str) -> Result<String> {
    let mut missing = Vec::new();
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut var_name = String::new();

            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                var_name.push(chars.next().unwrap());
            }

            match env::var(&var_name) {
                Ok(value) => {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        missing.push(var_name);
                    } else {
                        result.push_str(trimmed);
                    }
                }
                Err(_) => missing.push(var_name),
            }
        } else {
            result.push(ch);
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "config references unset environment variables: {}",
            missing.join(", ")
        );
    }

    Ok(result)
}

fn is_valid_eth_address(addr: &str) -> bool {
    if !addr.starts_with("0x") {
        return false;
    }
    if addr.len() != 42 {
        return false;
    }
    addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_eth_address() {
        assert!(is_valid_eth_address(
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
        ));
        assert!(is_valid_eth_address(
            "0x0000000000000000000000000000000000000000"
        ));

        assert!(!is_valid_eth_address(
            "5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
        )); // no 0x
        assert!(!is_valid_eth_address(
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6"
        )); // too short
        assert!(!is_valid_eth_address(
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6fX"
        )); // too long
        assert!(!is_valid_eth_address(
            "0xZZZZbEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
        )); // invalid hex
    }

    #[test]
    fn test_expand_env() {
        env::set_var("TEST_VAR", "test_value");
        env::set_var("ANOTHER_VAR", "  trimmed  ");

        let input = "host: ${TEST_VAR}, port: ${ANOTHER_VAR}";
        let result = expand_env(input).unwrap();
        assert_eq!(result, "host: test_value, port: trimmed");

        env::remove_var("TEST_VAR");
        env::remove_var("ANOTHER_VAR");
    }

    #[test]
    fn test_expand_env_missing() {
        let input = "host: ${MISSING_VAR}";
        let result = expand_env(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MISSING_VAR"));
    }

    #[test]
    fn test_chain_venue_or_default() {
        let chain_with_venue = Chain {
            chain: "eip155:1".to_string(),
            name: "Ethereum".to_string(),
            rpc: "wss://eth.example.com".to_string(),
            venue: "sushiswap".to_string(),
            confirmations: 12,
            pools: vec![],
        };
        assert_eq!(chain_with_venue.venue_or_default(), "sushiswap");

        let chain_without_venue = Chain {
            chain: "eip155:1".to_string(),
            name: "Ethereum".to_string(),
            rpc: "wss://eth.example.com".to_string(),
            venue: String::new(),
            confirmations: 12,
            pools: vec![],
        };
        assert_eq!(chain_without_venue.venue_or_default(), "uniswap-v2");
    }
}
