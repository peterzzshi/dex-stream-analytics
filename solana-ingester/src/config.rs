use anyhow::{Context, Result, bail};
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
    pub rpc_max_retries: u32,
    pub rpc_initial_backoff_ms: u64,
    pub rpc_max_backoff_ms: u64,
    pub rpc_timeout_ms: u64,
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
                .context("FINALITY_POLL_INTERVAL_SECS not set")?
                .parse()
                .context("FINALITY_POLL_INTERVAL_SECS must be a valid number")?,
        };

        let programs = ProgramsConfig {
            raydium_amm_v4: get_env("RAYDIUM_AMM_V4_PROGRAM")
                .context("RAYDIUM_AMM_V4_PROGRAM not set")?,
            token_metadata: get_env("TOKEN_METADATA_PROGRAM")
                .context("TOKEN_METADATA_PROGRAM not set")?,
        };

        // Load pools - try indexed env vars first (POOL_0_NAME, POOL_1_NAME, etc.)
        // Fall back to single pool (POOL_NAME) for backward compatibility
        let pools = Self::load_pools(&get_env)?;

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
                .context("PUBLISH_MAX_RETRIES not set")?
                .parse()
                .context("PUBLISH_MAX_RETRIES must be a valid number")?,
            publish_initial_backoff_ms: get_env("PUBLISH_INITIAL_BACKOFF_MS")
                .context("PUBLISH_INITIAL_BACKOFF_MS not set")?
                .parse()
                .context("PUBLISH_INITIAL_BACKOFF_MS must be a valid number")?,
            rpc_max_retries: get_env("RPC_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .context("RPC_MAX_RETRIES must be a valid number")?,
            rpc_initial_backoff_ms: get_env("RPC_INITIAL_BACKOFF_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("RPC_INITIAL_BACKOFF_MS must be a valid number")?,
            rpc_max_backoff_ms: get_env("RPC_MAX_BACKOFF_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .context("RPC_MAX_BACKOFF_MS must be a valid number")?,
            rpc_timeout_ms: get_env("RPC_TIMEOUT_MS")
                .unwrap_or_else(|_| "30000".to_string())
                .parse()
                .context("RPC_TIMEOUT_MS must be a valid number")?,
        };

        Ok(Config {
            dapr,
            topics,
            app,
            solana,
            programs,
            pools,
            pyth,
            tokens,
            retry,
        })
    }

    fn load_pools<F>(get_env: &F) -> Result<Vec<PoolConfig>>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let mut pools = Vec::new();

        // Try loading indexed pools (POOL_0_NAME, POOL_1_NAME, etc.)
        let mut index = 0;
        loop {
            let name_key = format!("POOL_{}_NAME", index);
            match get_env(&name_key) {
                Ok(name) => {
                    let address_key = format!("POOL_{}_ADDRESS", index);
                    let token0_key = format!("POOL_{}_TOKEN0", index);
                    let token1_key = format!("POOL_{}_TOKEN1", index);
                    let vault0_key = format!("POOL_{}_VAULT0_INDEX", index);
                    let vault1_key = format!("POOL_{}_VAULT1_INDEX", index);
                    let user_auth_key = format!("POOL_{}_USER_AUTHORITY_INDEX", index);

                    pools.push(PoolConfig {
                        name,
                        address: get_env(&address_key)
                            .with_context(|| format!("{} not set", address_key))?,
                        token0: get_env(&token0_key).ok(),
                        token1: get_env(&token1_key).ok(),
                        vault0_index: get_env(&vault0_key)
                            .with_context(|| format!("{} not set", vault0_key))?
                            .parse()
                            .with_context(|| format!("{} must be a valid number", vault0_key))?,
                        vault1_index: get_env(&vault1_key)
                            .with_context(|| format!("{} not set", vault1_key))?
                            .parse()
                            .with_context(|| format!("{} must be a valid number", vault1_key))?,
                        user_authority_index: get_env(&user_auth_key)
                            .with_context(|| format!("{} not set", user_auth_key))?
                            .parse()
                            .with_context(|| format!("{} must be a valid number", user_auth_key))?,
                    });
                    index += 1;
                }
                Err(_) => break,
            }
        }

        // If no indexed pools found, try single pool format (backward compatibility)
        if pools.is_empty() {
            if let Ok(name) = get_env("POOL_NAME") {
                pools.push(PoolConfig {
                    name,
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
                });
            }
        }

        if pools.is_empty() {
            bail!("No pools configured. Set either POOL_NAME or POOL_0_NAME environment variables");
        }

        Ok(pools)
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_pool_stops_at_first_gap() {
        let get_env = |key: &str| -> Result<String, env::VarError> {
            match key {
                "POOL_0_NAME" => Ok("pool-0".to_string()),
                "POOL_0_ADDRESS" => Ok("addr0".to_string()),
                "POOL_0_VAULT0_INDEX" => Ok("5".to_string()),
                "POOL_0_VAULT1_INDEX" => Ok("6".to_string()),
                "POOL_0_USER_AUTHORITY_INDEX" => Ok("16".to_string()),
                "POOL_1_NAME" => Ok("pool-1".to_string()),
                "POOL_1_ADDRESS" => Ok("addr1".to_string()),
                "POOL_1_VAULT0_INDEX" => Ok("5".to_string()),
                "POOL_1_VAULT1_INDEX" => Ok("6".to_string()),
                "POOL_1_USER_AUTHORITY_INDEX" => Ok("16".to_string()),
                // POOL_2 is missing - gap
                "POOL_3_NAME" => Ok("pool-3".to_string()),
                _ => Err(env::VarError::NotPresent),
            }
        };

        let pools = Config::load_pools(&get_env).expect("should load pools");
        assert_eq!(pools.len(), 2);
    }

    #[test]
    fn test_fallback_to_single_pool_when_no_indexed() {
        let get_env = |key: &str| -> Result<String, env::VarError> {
            match key {
                "POOL_NAME" => Ok("single-pool".to_string()),
                "POOL_ADDRESS" => Ok("addr".to_string()),
                "POOL_VAULT0_INDEX" => Ok("5".to_string()),
                "POOL_VAULT1_INDEX" => Ok("6".to_string()),
                "POOL_USER_AUTHORITY_INDEX" => Ok("16".to_string()),
                _ => Err(env::VarError::NotPresent),
            }
        };

        let pools = Config::load_pools(&get_env).expect("should load single pool");
        assert_eq!(pools.len(), 1);
    }

    #[test]
    fn test_error_when_no_pools_configured() {
        let get_env = |_: &str| -> Result<String, env::VarError> { Err(env::VarError::NotPresent) };

        let result = Config::load_pools(&get_env);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No pools configured")
        );
    }

    #[test]
    fn test_indexed_pools_take_precedence() {
        let get_env = |key: &str| -> Result<String, env::VarError> {
            match key {
                "POOL_0_NAME" => Ok("indexed-pool".to_string()),
                "POOL_0_ADDRESS" => Ok("indexed-addr".to_string()),
                "POOL_0_VAULT0_INDEX" => Ok("5".to_string()),
                "POOL_0_VAULT1_INDEX" => Ok("6".to_string()),
                "POOL_0_USER_AUTHORITY_INDEX" => Ok("16".to_string()),
                "POOL_NAME" => Ok("single-pool".to_string()),
                "POOL_ADDRESS" => Ok("single-addr".to_string()),
                _ => Err(env::VarError::NotPresent),
            }
        };

        let pools = Config::load_pools(&get_env).expect("should load pools");
        assert_eq!(pools.len(), 1);
    }
}
