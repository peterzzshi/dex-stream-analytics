use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub dapr: DaprConfig,
    pub topics: TopicsConfig,
    pub app: AppConfig,
    pub solana: SolanaConfig,
    pub pools: Vec<PoolConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DaprConfig {
    pub host: String,
    #[serde(rename = "httpPort")]
    pub http_port: u16,
    #[serde(rename = "pubsubName")]
    pub pubsub_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TopicsConfig {
    pub trading: String,
    pub liquidity: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolanaConfig {
    pub rpc: String,
    pub commitment: String,
    pub chain: String,
    #[serde(rename = "chainName")]
    pub chain_name: String,
    pub venue: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub token0: Option<String>,
    #[serde(default)]
    pub token1: Option<String>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        // Expand environment variables in format ${VAR_NAME}
        let expanded = expand_env_vars(&content)?;

        let config: Config =
            serde_yaml::from_str(&expanded).context("Failed to parse config YAML")?;

        Ok(config)
    }
}

fn expand_env_vars(input: &str) -> Result<String> {
    let mut result = input.to_string();

    // Simple regex-free expansion: find ${...} patterns
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let value = env::var(var_name)
                .with_context(|| format!("Environment variable {} not set", var_name))?;
            result.replace_range(start..start + end + 1, &value);
        } else {
            break;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        env::set_var("TEST_VAR", "test_value");
        let input = "rpc: ${TEST_VAR}";
        let result = expand_env_vars(input).unwrap();
        assert_eq!(result, "rpc: test_value");
    }
}
