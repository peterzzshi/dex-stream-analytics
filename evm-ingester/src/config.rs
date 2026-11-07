use anyhow::{Context, Result};
use dex_ingester_common::{RetryConfig, Secret, SharedConfig};
use ethers::types::Address;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use crate::file_config::EvmFileConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub shared: SharedConfig,
    pub chains: Vec<Chain>,
}

#[derive(Debug, Clone)]
pub struct Chain {
    pub chain: String,
    pub name: String,
    pub rpc: Secret,
    pub confirmations: u64,
    pub pools: Vec<Pool>,
    pub oracle: OracleConfig,
    pub retry: RetryConfig,
}

#[derive(Debug, Clone)]
pub struct OracleConfig {
    pub price_feeds: HashMap<Address, Address>,
    pub stablecoins: Vec<Address>,
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub pair_address: String,
    pub name: String,
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        let path = env::var("CONFIG_FILE").context("CONFIG_FILE not set")?;
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read CONFIG_FILE {path}"))?;
        Self::load(|key| env::var(key), &contents)
    }

    fn load<F>(get_env: F, config_toml: &str) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let shared = SharedConfig::load(&get_env)?;
        let file = EvmFileConfig::parse(config_toml)?;

        // Addresses were validated by EvmFileConfig::parse, so parsing cannot fail here
        let parse = |s: &str| Address::from_str(s).with_context(|| format!("Invalid address {s}"));

        let mut price_feeds = HashMap::new();
        let mut stablecoins = Vec::new();
        for token in &file.tokens {
            if let Some(feed) = &token.chainlink_feed {
                price_feeds.insert(parse(&token.address)?, parse(feed)?);
            }
            if token.stablecoin {
                stablecoins.push(parse(&token.address)?);
            }
        }

        let chain = Chain {
            chain: file.chain.id,
            name: file.chain.name,
            rpc: Secret::new(get_env("ETH_RPC_URL").context("ETH_RPC_URL not set")?),
            confirmations: file.chain.confirmations,
            pools: file
                .pools
                .into_iter()
                .map(|p| Pool {
                    pair_address: p.address,
                    name: p.name,
                })
                .collect(),
            oracle: OracleConfig {
                price_feeds,
                stablecoins,
            },
            retry: RetryConfig::default(),
        };

        Ok(Config {
            shared,
            chains: vec![chain],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_config::test_support::VALID;

    const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    const WETH_FEED: &str = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
    const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

    fn env_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DAPR_HOST", "evm-ingester-dapr"),
            ("DAPR_HTTP_PORT", "3500"),
            ("PUBSUB_NAME", "kafka-pubsub"),
            ("TOPIC_TRADING", "dex-trading-events"),
            ("TOPIC_LIQUIDITY", "dex-liquidity-events"),
            ("TOPIC_DLQ", "dex-dlq-events"),
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

    #[test]
    fn token_table_feeds_the_oracle_config() {
        let config = Config::load(lookup(&env_vars()), VALID).expect("should load");
        let chain = &config.chains[0];

        assert_eq!(chain.chain, "eip155:1");
        assert_eq!(chain.confirmations, 12);
        assert_eq!(chain.pools[0].name, "USDC/WETH");
        assert_eq!(
            chain.oracle.price_feeds[&Address::from_str(WETH).unwrap()],
            Address::from_str(WETH_FEED).unwrap()
        );
        assert_eq!(chain.oracle.stablecoins, [Address::from_str(USDC).unwrap()]);
    }

    #[test]
    fn missing_rpc_url_is_named_in_the_error() {
        let mut vars = env_vars();
        vars.retain(|(k, _)| *k != "ETH_RPC_URL");

        let err = Config::load(lookup(&vars), VALID).unwrap_err();

        assert!(format!("{err:#}").contains("ETH_RPC_URL"));
    }

    #[test]
    fn malformed_file_fails_load_with_the_offending_key() {
        let contents = VALID.replace("[[pools]]", "[[pools]]\ntypo = 1");

        let err = Config::load(lookup(&env_vars()), &contents).unwrap_err();

        assert!(format!("{err:#}").contains("typo"));
    }

    #[test]
    fn config_debug_output_hides_the_rpc_url() {
        let config = Config::load(lookup(&env_vars()), VALID).expect("should load");

        assert!(!format!("{config:?}").contains("eth-sepolia.example.com"));
    }
}
