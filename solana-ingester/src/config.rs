use anyhow::{Context, Result};
use dex_ingester_common::{Secret, SharedConfig};
use std::collections::HashMap;
use std::env;

use crate::file_config::FileConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub shared: SharedConfig,
    pub solana: SolanaConfig,
    pub programs: ProgramsConfig,
    pub pools: Vec<PoolConfig>,
    pub pyth: PythConfig,
    pub tokens: TokensConfig,
}

#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub app_port: u16,
    pub rpc: Secret,
    pub commitment: String,
    pub chain: String,
    pub chain_name: String,
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
    pub token0: String,
    pub token1: String,
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

impl Config {
    pub fn load_from_env() -> Result<Self> {
        let path = env::var("CONFIG_FILE").context("CONFIG_FILE not set")?;
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read CONFIG_FILE {path}"))?;
        Self::load(|key| env::var(key), &contents)
    }

    // Testable pure function - takes env lookup and file contents as parameters
    fn load<F>(get_env: F, config_toml: &str) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, env::VarError>,
    {
        let shared = SharedConfig::load(&get_env)?;
        let file = FileConfig::parse(config_toml)?;

        let solana = SolanaConfig {
            app_port: get_env("APP_PORT")
                .context("APP_PORT not set")?
                .parse()
                .context("APP_PORT must be a valid number")?,
            rpc: Secret::new(get_env("SOLANA_RPC_URL").context("SOLANA_RPC_URL not set")?),
            commitment: get_env("SOLANA_COMMITMENT").context("SOLANA_COMMITMENT not set")?,
            chain: file.chain.id.clone(),
            chain_name: file.chain.name.clone(),
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
            raydium_amm_v4: file.programs.raydium_amm_v4.clone(),
            token_metadata: file.programs.token_metadata.clone(),
        };

        // validate() guarantees every pool token is declared in [[tokens]]
        let symbol_of = |mint: &str| {
            file.token(mint)
                .map(|t| t.symbol.clone())
                .unwrap_or_else(|| mint.to_string())
        };
        let pools = file
            .pools
            .iter()
            .map(|p| PoolConfig {
                name: format!("{}/{}", symbol_of(&p.token0), symbol_of(&p.token1)),
                address: p.address.clone(),
                token0: p.token0.clone(),
                token1: p.token1.clone(),
            })
            .collect();

        let pyth = PythConfig {
            max_age_seconds: file.pyth.max_age_seconds,
            price_feeds: file
                .tokens
                .iter()
                .filter_map(|t| Some((t.mint.clone(), t.pyth_account.clone()?)))
                .collect(),
            stablecoins: file
                .tokens
                .iter()
                .filter(|t| t.stablecoin)
                .map(|t| t.mint.clone())
                .collect(),
        };

        let tokens = TokensConfig {
            known_symbols: file
                .tokens
                .iter()
                .map(|t| (t.mint.clone(), t.symbol.clone()))
                .collect(),
        };

        Ok(Config {
            shared,
            solana,
            programs,
            pools,
            pyth,
            tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_config::test_support::VALID;

    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

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
    fn token_table_feeds_every_runtime_section() {
        let config = Config::load(env_without(""), VALID).expect("should load");

        assert_eq!(config.solana.chain_name, "Solana");
        assert_eq!(config.pools[0].name, "SOL/USDC");
        assert_eq!(config.pools[0].token0, SOL);
        assert_eq!(config.tokens.known_symbols[SOL], "SOL");
        assert_eq!(config.tokens.known_symbols[USDC], "USDC");
        assert_eq!(config.pyth.stablecoins, [USDC]);
        assert_eq!(
            config.pyth.price_feeds.get(SOL).map(String::as_str),
            Some("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE")
        );
        assert!(!config.pyth.price_feeds.contains_key(USDC));
    }

    #[test]
    fn missing_env_var_is_named_in_the_error() {
        let err = Config::load(env_without("SOLANA_RPC_URL"), VALID).unwrap_err();

        assert!(format!("{err:#}").contains("SOLANA_RPC_URL"));
    }

    #[test]
    fn malformed_file_fails_load_with_the_offending_key() {
        let contents = VALID.replace("[pyth]", "[pyth]\nmax_age = 5");

        let err = Config::load(env_without(""), &contents).unwrap_err();

        assert!(format!("{err:#}").contains("max_age"));
    }
}
