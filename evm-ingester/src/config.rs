use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dapr: DaprConfig,
    pub topics: TopicsConfig,
    pub app: AppConfig,
    #[serde(default)]
    pub buffer: BufferConfig,
    #[serde(default)]
    pub channels: ChannelConfig,
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
pub struct BufferConfig {
    #[serde(default = "default_max_events_per_block")]
    pub max_events_per_block: usize,
    #[serde(default = "default_max_blocks_in_memory")]
    pub max_blocks_in_memory: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_events_per_block: default_max_events_per_block(),
            max_blocks_in_memory: default_max_blocks_in_memory(),
        }
    }
}

fn default_max_events_per_block() -> usize {
    1000
}

fn default_max_blocks_in_memory() -> usize {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    #[serde(default = "default_event_buffer_size")]
    pub event_buffer_size: usize,
    #[serde(default = "default_dlq_buffer_size")]
    pub dlq_buffer_size: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            event_buffer_size: default_event_buffer_size(),
            dlq_buffer_size: default_dlq_buffer_size(),
        }
    }
}

fn default_event_buffer_size() -> usize {
    1000
}

fn default_dlq_buffer_size() -> usize {
    100
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
    #[serde(default)]
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_rpc_max_retries")]
    pub rpc_max_retries: u32,
    #[serde(default = "default_rpc_initial_backoff_ms")]
    pub rpc_initial_backoff_ms: u64,
    #[serde(default = "default_rpc_max_backoff_ms")]
    pub rpc_max_backoff_ms: u64,
    #[serde(default = "default_rpc_timeout_ms")]
    pub rpc_timeout_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            rpc_max_retries: default_rpc_max_retries(),
            rpc_initial_backoff_ms: default_rpc_initial_backoff_ms(),
            rpc_max_backoff_ms: default_rpc_max_backoff_ms(),
            rpc_timeout_ms: default_rpc_timeout_ms(),
        }
    }
}

fn default_rpc_max_retries() -> u32 {
    3
}

fn default_rpc_initial_backoff_ms() -> u64 {
    100
}

fn default_rpc_max_backoff_ms() -> u64 {
    5000
}

fn default_rpc_timeout_ms() -> u64 {
    30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub name: String,
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        Self::load(|key| env::var(key))
    }

    fn load<F>(get_env: F) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let dapr = DaprConfig {
            host: get_env("DAPR_HOST").context("DAPR_HOST not set")?,
            http_port: get_env("DAPR_HTTP_PORT")
                .context("DAPR_HTTP_PORT not set")?
                .parse()
                .context("DAPR_HTTP_PORT must be a valid port number")?,
            pubsub_name: get_env("PUBSUB_NAME").context("PUBSUB_NAME not set")?,
        };

        let topics = TopicsConfig {
            trading: get_env("TOPIC_TRADING").context("TOPIC_TRADING not set")?,
            liquidity: get_env("TOPIC_LIQUIDITY").context("TOPIC_LIQUIDITY not set")?,
        };

        let app = AppConfig {
            port: get_env("APP_PORT")
                .context("APP_PORT not set")?
                .parse()
                .context("APP_PORT must be a valid port number")?,
        };

        let buffer = BufferConfig {
            max_events_per_block: get_env("BUFFER_MAX_EVENTS_PER_BLOCK")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_max_events_per_block()),
            max_blocks_in_memory: get_env("BUFFER_MAX_BLOCKS_IN_MEMORY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_max_blocks_in_memory()),
        };

        let channels = ChannelConfig {
            event_buffer_size: get_env("CHANNEL_EVENT_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_event_buffer_size()),
            dlq_buffer_size: get_env("CHANNEL_DLQ_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_dlq_buffer_size()),
        };

        let chain = Chain {
            chain: get_env("CHAIN_ID").context("CHAIN_ID not set")?,
            name: get_env("CHAIN_NAME").context("CHAIN_NAME not set")?,
            rpc: get_env("ETH_RPC_URL").context("ETH_RPC_URL not set")?,
            venue: get_env("CHAIN_VENUE").unwrap_or_else(|_| "uniswap-v2".to_string()),
            confirmations: get_env("CHAIN_CONFIRMATIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            pools: vec![Pool {
                pair_address: get_env("POOL_ADDRESS").context("POOL_ADDRESS not set")?,
                name: get_env("POOL_NAME").context("POOL_NAME not set")?,
            }],
            retry: RetryConfig::default(),
        };

        // Validate pool addresses
        for pool in &chain.pools {
            if !is_valid_eth_address(&pool.pair_address) {
                anyhow::bail!(
                    "Invalid EVM address for pool {}: {}",
                    pool.name,
                    pool.pair_address
                );
            }
        }

        Ok(Config {
            dapr,
            topics,
            app,
            buffer,
            channels,
            chains: vec![chain],
        })
    }
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
    fn test_load_applies_defaults_for_optional_vars() {
        let config = Config::load(lookup(&required_vars())).expect("should load");

        assert_eq!(config.dapr.http_port, 3500);
        assert_eq!(config.chains[0].venue, "uniswap-v2");
        assert_eq!(config.chains[0].confirmations, 3);
        assert_eq!(config.buffer.max_blocks_in_memory, 100);
        assert_eq!(config.channels.event_buffer_size, 1000);
    }

    #[test]
    fn test_load_reads_overrides() {
        let mut vars = required_vars();
        vars.push(("CHAIN_VENUE", "sushiswap-v2"));
        vars.push(("CHAIN_CONFIRMATIONS", "12"));
        vars.push(("BUFFER_MAX_BLOCKS_IN_MEMORY", "7"));

        let config = Config::load(lookup(&vars)).expect("should load");

        assert_eq!(config.chains[0].venue, "sushiswap-v2");
        assert_eq!(config.chains[0].confirmations, 12);
        assert_eq!(config.buffer.max_blocks_in_memory, 7);
    }

    #[test]
    fn test_load_fails_when_required_var_missing() {
        let vars: Vec<_> = required_vars()
            .into_iter()
            .filter(|(k, _)| *k != "TOPIC_LIQUIDITY")
            .collect();

        let err = Config::load(lookup(&vars)).unwrap_err();

        assert!(err.to_string().contains("TOPIC_LIQUIDITY"));
    }

    #[test]
    fn test_load_rejects_invalid_pool_address() {
        let mut vars = required_vars();
        vars.retain(|(k, _)| *k != "POOL_ADDRESS");
        vars.push(("POOL_ADDRESS", "not-an-address"));

        let err = Config::load(lookup(&vars)).unwrap_err();

        assert!(err.to_string().contains("Invalid EVM address"));
    }

    fn required_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DAPR_HOST", "evm-ingester-dapr"),
            ("DAPR_HTTP_PORT", "3500"),
            ("PUBSUB_NAME", "kafka-pubsub"),
            ("TOPIC_TRADING", "dex-trading-events"),
            ("TOPIC_LIQUIDITY", "dex-liquidity-events"),
            ("APP_PORT", "3000"),
            ("CHAIN_ID", "eip155:11155111"),
            ("CHAIN_NAME", "sepolia"),
            ("ETH_RPC_URL", "wss://eth-sepolia.example.com"),
            ("POOL_ADDRESS", "0xBCE3c0dCA52B1e7053142749296A03f5044017dD"),
            ("POOL_NAME", "TEST/WETH"),
        ]
    }

    fn lookup<'a>(
        vars: &'a [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Result<String, env::VarError> + 'a {
        move |key| {
            vars.iter()
                .rev()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
                .ok_or(env::VarError::NotPresent)
        }
    }
}
