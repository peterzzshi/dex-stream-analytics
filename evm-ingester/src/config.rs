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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub name: String,
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        let dapr = DaprConfig {
            host: env::var("DAPR_HOST").context("DAPR_HOST not set")?,
            http_port: env::var("DAPR_HTTP_PORT")
                .context("DAPR_HTTP_PORT not set")?
                .parse()
                .context("DAPR_HTTP_PORT must be a valid port number")?,
            pubsub_name: env::var("PUBSUB_NAME").context("PUBSUB_NAME not set")?,
        };

        let topics = TopicsConfig {
            trading: env::var("TOPIC_TRADING").context("TOPIC_TRADING not set")?,
            liquidity: env::var("TOPIC_LIQUIDITY").context("TOPIC_LIQUIDITY not set")?,
        };

        let app = AppConfig {
            port: env::var("APP_PORT")
                .context("APP_PORT not set")?
                .parse()
                .context("APP_PORT must be a valid port number")?,
        };

        let buffer = BufferConfig {
            max_events_per_block: env::var("BUFFER_MAX_EVENTS_PER_BLOCK")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_max_events_per_block()),
            max_blocks_in_memory: env::var("BUFFER_MAX_BLOCKS_IN_MEMORY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_max_blocks_in_memory()),
        };

        let channels = ChannelConfig {
            event_buffer_size: env::var("CHANNEL_EVENT_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_event_buffer_size()),
            dlq_buffer_size: env::var("CHANNEL_DLQ_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_dlq_buffer_size()),
        };

        let chain = Chain {
            chain: env::var("CHAIN_ID").context("CHAIN_ID not set")?,
            name: env::var("CHAIN_NAME").context("CHAIN_NAME not set")?,
            rpc: env::var("ETH_RPC_URL").context("ETH_RPC_URL not set")?,
            venue: env::var("CHAIN_VENUE").unwrap_or_else(|_| "uniswap-v2".to_string()),
            confirmations: env::var("CHAIN_CONFIRMATIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            pools: vec![Pool {
                pair_address: env::var("POOL_ADDRESS").context("POOL_ADDRESS not set")?,
                name: env::var("POOL_NAME").context("POOL_NAME not set")?,
            }],
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
    fn test_load_from_env() {
        env::set_var("DAPR_HOST", "evm-ingester-dapr");
        env::set_var("DAPR_HTTP_PORT", "3500");
        env::set_var("PUBSUB_NAME", "kafka-pubsub");
        env::set_var("TOPIC_TRADING", "dex-trading-events");
        env::set_var("TOPIC_LIQUIDITY", "dex-liquidity-events");
        env::set_var("APP_PORT", "3000");
        env::set_var("CHAIN_ID", "eip155:11155111");
        env::set_var("CHAIN_NAME", "sepolia");
        env::set_var("ETH_RPC_URL", "wss://eth-sepolia.example.com");
        env::set_var("CHAIN_VENUE", "uniswap-v2");
        env::set_var("CHAIN_CONFIRMATIONS", "3");
        env::set_var("POOL_ADDRESS", "0xBCE3c0dCA52B1e7053142749296A03f5044017dD");
        env::set_var("POOL_NAME", "TEST/WETH");

        let config = Config::load_from_env().expect("should load config from env");

        assert_eq!(config.dapr.host, "evm-ingester-dapr");
        assert_eq!(config.dapr.http_port, 3500);
        assert_eq!(config.chains.len(), 1);
        assert_eq!(config.chains[0].name, "sepolia");
        assert_eq!(config.chains[0].confirmations, 3);
        assert_eq!(config.chains[0].pools.len(), 1);
        assert_eq!(config.chains[0].pools[0].name, "TEST/WETH");

        // Clean up
        env::remove_var("DAPR_HOST");
        env::remove_var("DAPR_HTTP_PORT");
        env::remove_var("PUBSUB_NAME");
        env::remove_var("TOPIC_TRADING");
        env::remove_var("TOPIC_LIQUIDITY");
        env::remove_var("APP_PORT");
        env::remove_var("CHAIN_ID");
        env::remove_var("CHAIN_NAME");
        env::remove_var("ETH_RPC_URL");
        env::remove_var("CHAIN_VENUE");
        env::remove_var("CHAIN_CONFIRMATIONS");
        env::remove_var("POOL_ADDRESS");
        env::remove_var("POOL_NAME");
    }
}
