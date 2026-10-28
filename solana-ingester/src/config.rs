use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub dapr: DaprConfig,
    pub topics: TopicsConfig,
    pub app: AppConfig,
    pub solana: SolanaConfig,
    pub programs: ProgramsConfig,
    pub pools: Vec<PoolConfig>,
    pub pyth: PythConfig,
    pub tokens: TokensConfig,
    pub retry: RetryConfig,
}

#[derive(Debug, Clone)]
pub struct DaprConfig {
    pub host: String,
    pub http_port: u16,
    pub pubsub_name: String,
}

#[derive(Debug, Clone)]
pub struct TopicsConfig {
    pub trading: String,
    pub liquidity: String,
    pub dlq: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub rpc: String,
    pub commitment: String,
    pub chain: String,
    pub chain_name: String,
    pub venue: String,
    pub finality_buffer_size: usize,
    pub finality_poll_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct ProgramsConfig {
    pub raydium_amm_v4: String,
    pub token_metadata: String,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub name: String,
    pub address: String,
    pub token0: Option<String>,
    pub token1: Option<String>,
    pub vault0_index: usize,
    pub vault1_index: usize,
    pub user_authority_index: usize,
}

#[derive(Debug, Clone)]
pub struct PythConfig {
    pub max_age_seconds: i64,
    pub price_feeds: HashMap<String, String>,
    pub stablecoins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TokensConfig {
    pub known_symbols: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub reconnect_delay_short_secs: u64,
    pub reconnect_delay_long_secs: u64,
    pub publish_max_retries: u32,
    pub publish_initial_backoff_ms: u64,
}

impl Config {
    pub fn load_from_env() -> Result<Self> {
        Self::load(|key| env::var(key))
    }

    // Testable pure function - takes env lookup as parameter
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
            dlq: get_env("TOPIC_DLQ").context("TOPIC_DLQ not set")?,
        };

        let app = AppConfig {
            port: get_env("APP_PORT")
                .context("APP_PORT not set")?
                .parse()
                .context("APP_PORT must be a valid port number")?,
        };

        let solana = SolanaConfig {
            rpc: get_env("SOLANA_RPC_URL").context("SOLANA_RPC_URL not set")?,
            commitment: get_env("SOLANA_COMMITMENT").context("SOLANA_COMMITMENT not set")?,
            chain: get_env("CHAIN_ID").context("CHAIN_ID not set")?,
            chain_name: get_env("CHAIN_NAME").context("CHAIN_NAME not set")?,
            venue: get_env("CHAIN_VENUE").context("CHAIN_VENUE not set")?,
            finality_buffer_size: get_env("FINALITY_BUFFER_SIZE")
                .context("FINALITY_BUFFER_SIZE not set")?
                .parse()
                .context("FINALITY_BUFFER_SIZE must be a valid number")?,
            finality_poll_interval_secs: get_env("FINALITY_POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("FINALITY_POLL_INTERVAL_SECS must be a valid number")?,
        };

        let programs = ProgramsConfig {
            raydium_amm_v4: get_env("RAYDIUM_AMM_V4_PROGRAM")
                .context("RAYDIUM_AMM_V4_PROGRAM not set")?,
            token_metadata: get_env("TOKEN_METADATA_PROGRAM")
                .context("TOKEN_METADATA_PROGRAM not set")?,
        };

        let pool = PoolConfig {
            name: get_env("POOL_NAME").context("POOL_NAME not set")?,
            address: get_env("POOL_ADDRESS").context("POOL_ADDRESS not set")?,
            token0: get_env("POOL_TOKEN0").ok(),
            token1: get_env("POOL_TOKEN1").ok(),
            vault0_index: get_env("POOL_VAULT0_INDEX")
                .context("POOL_VAULT0_INDEX not set")?
                .parse()
                .context("POOL_VAULT0_INDEX must be a valid number")?,
            vault1_index: get_env("POOL_VAULT1_INDEX")
                .context("POOL_VAULT1_INDEX not set")?
                .parse()
                .context("POOL_VAULT1_INDEX must be a valid number")?,
            user_authority_index: get_env("POOL_USER_AUTHORITY_INDEX")
                .context("POOL_USER_AUTHORITY_INDEX not set")?
                .parse()
                .context("POOL_USER_AUTHORITY_INDEX must be a valid number")?,
        };

        let price_feeds: HashMap<String, String> =
            serde_json::from_str(&get_env("PYTH_PRICE_FEEDS").context("PYTH_PRICE_FEEDS not set")?)
                .context("PYTH_PRICE_FEEDS must be valid JSON")?;

        let stablecoins: Vec<String> =
            serde_json::from_str(&get_env("PYTH_STABLECOINS").context("PYTH_STABLECOINS not set")?)
                .context("PYTH_STABLECOINS must be valid JSON array")?;

        let pyth = PythConfig {
            max_age_seconds: get_env("PYTH_MAX_AGE_SECONDS")
                .context("PYTH_MAX_AGE_SECONDS not set")?
                .parse()
                .context("PYTH_MAX_AGE_SECONDS must be a valid number")?,
            price_feeds,
            stablecoins,
        };

        let known_symbols: HashMap<String, String> = serde_json::from_str(
            &get_env("KNOWN_TOKEN_SYMBOLS").context("KNOWN_TOKEN_SYMBOLS not set")?,
        )
        .context("KNOWN_TOKEN_SYMBOLS must be valid JSON")?;

        let tokens = TokensConfig { known_symbols };

        let retry = RetryConfig {
            reconnect_delay_short_secs: get_env("RECONNECT_DELAY_SHORT_SECS")
                .context("RECONNECT_DELAY_SHORT_SECS not set")?
                .parse()
                .context("RECONNECT_DELAY_SHORT_SECS must be a valid number")?,
            reconnect_delay_long_secs: get_env("RECONNECT_DELAY_LONG_SECS")
                .context("RECONNECT_DELAY_LONG_SECS not set")?
                .parse()
                .context("RECONNECT_DELAY_LONG_SECS must be a valid number")?,
            publish_max_retries: get_env("PUBLISH_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .context("PUBLISH_MAX_RETRIES must be a valid number")?,
            publish_initial_backoff_ms: get_env("PUBLISH_INITIAL_BACKOFF_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("PUBLISH_INITIAL_BACKOFF_MS must be a valid number")?,
        };

        Ok(Config {
            dapr,
            topics,
            app,
            solana,
            programs,
            pools: vec![pool],
            pyth,
            tokens,
            retry,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_parsing_logic() {
        // Test the parsing logic without touching env::var
        let mut mock_env = HashMap::new();
        mock_env.insert("DAPR_HOST", "test-host");
        mock_env.insert("DAPR_HTTP_PORT", "3501");
        mock_env.insert("PUBSUB_NAME", "kafka-pubsub");
        mock_env.insert("TOPIC_TRADING", "dex-trading-events");
        mock_env.insert("TOPIC_LIQUIDITY", "dex-liquidity-events");
        mock_env.insert("TOPIC_DLQ", "dex-dlq");
        mock_env.insert("APP_PORT", "3001");
        mock_env.insert("SOLANA_RPC_URL", "wss://api.mainnet-beta.solana.com");
        mock_env.insert("SOLANA_COMMITMENT", "finalized");
        mock_env.insert("CHAIN_ID", "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp");
        mock_env.insert("CHAIN_NAME", "Solana");
        mock_env.insert("CHAIN_VENUE", "raydium-amm-v4");
        mock_env.insert("FINALITY_BUFFER_SIZE", "500");
        mock_env.insert("FINALITY_POLL_INTERVAL_SECS", "10");
        mock_env.insert(
            "RAYDIUM_AMM_V4_PROGRAM",
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        );
        mock_env.insert(
            "TOKEN_METADATA_PROGRAM",
            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
        );
        mock_env.insert("POOL_NAME", "SOL/USDC");
        mock_env.insert(
            "POOL_ADDRESS",
            "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
        );
        mock_env.insert("POOL_TOKEN0", "So11111111111111111111111111111111111111112");
        mock_env.insert(
            "POOL_TOKEN1",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        );
        mock_env.insert("POOL_VAULT0_INDEX", "5");
        mock_env.insert("POOL_VAULT1_INDEX", "6");
        mock_env.insert("POOL_USER_AUTHORITY_INDEX", "16");
        mock_env.insert("PYTH_MAX_AGE_SECONDS", "60");
        mock_env.insert(
            "PYTH_PRICE_FEEDS",
            r#"{"So11111111111111111111111111111111111111112":"H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"}"#,
        );
        mock_env.insert(
            "PYTH_STABLECOINS",
            r#"["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]"#,
        );
        mock_env.insert(
            "KNOWN_TOKEN_SYMBOLS",
            r#"{"So11111111111111111111111111111111111111112":"SOL"}"#,
        );
        mock_env.insert("RECONNECT_DELAY_SHORT_SECS", "5");
        mock_env.insert("RECONNECT_DELAY_LONG_SECS", "10");

        let config = Config::load(|key| {
            mock_env
                .get(key)
                .map(|s| s.to_string())
                .ok_or(env::VarError::NotPresent)
        })
        .expect("should parse config");

        // Test parsing correctness, not data values
        assert_eq!(config.dapr.http_port, 3501);
        assert_eq!(config.solana.finality_buffer_size, 500);
        assert_eq!(config.pools[0].vault0_index, 5);
        assert_eq!(config.pyth.max_age_seconds, 60);
        assert_eq!(config.retry.reconnect_delay_short_secs, 5);
    }

    #[test]
    fn test_missing_required_field_fails() {
        let config = Config::load(|_| Err(env::VarError::NotPresent));
        assert!(config.is_err());
    }

    #[test]
    fn test_invalid_port_number_fails() {
        let config = Config::load(|key| {
            if key == "DAPR_HTTP_PORT" {
                Ok("not-a-number".to_string())
            } else {
                Err(env::VarError::NotPresent)
            }
        });
        assert!(config.is_err());
    }
}
