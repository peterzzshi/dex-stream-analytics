use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvmFileConfig {
    pub chain: ChainFile,
    pub tokens: Vec<TokenFile>,
    pub pools: Vec<PoolFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainFile {
    pub id: String,
    pub name: String,
    pub confirmations: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenFile {
    pub address: String,
    pub chainlink_feed: Option<String>,
    #[serde(default)]
    pub stablecoin: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolFile {
    pub address: String,
    pub name: String,
}

impl EvmFileConfig {
    pub fn parse(contents: &str) -> Result<Self> {
        let config: EvmFileConfig = toml::from_str(contents).context("Invalid config file")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        let mut tokens = HashSet::new();
        for token in &self.tokens {
            check_address("tokens.address", &token.address)?;
            if !tokens.insert(token.address.to_lowercase()) {
                bail!("Duplicate token address: {}", token.address);
            }
            if let Some(feed) = &token.chainlink_feed {
                check_address("tokens.chainlink_feed", feed)?;
                if token.stablecoin {
                    bail!(
                        "Token {} is a stablecoin and also has a chainlink_feed",
                        token.address
                    );
                }
            }
        }

        if self.pools.is_empty() {
            bail!("No pools configured: add at least one [[pools]] entry");
        }

        let mut pools = HashSet::new();
        for pool in &self.pools {
            check_address("pools.address", &pool.address)?;
            if !pools.insert(pool.address.to_lowercase()) {
                bail!("Duplicate pool address: {}", pool.address);
            }
        }

        Ok(())
    }
}

fn check_address(field: &str, value: &str) -> Result<()> {
    if !is_valid_eth_address(value) {
        bail!("{field} is not a valid EVM address: {value}");
    }
    Ok(())
}

pub fn is_valid_eth_address(addr: &str) -> bool {
    if !addr.starts_with("0x") {
        return false;
    }
    if addr.len() != 42 {
        return false;
    }
    addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
pub mod test_support {
    pub const VALID: &str = r#"
[chain]
id            = "eip155:1"
name          = "ethereum"
confirmations = 12

[[tokens]]
address        = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
chainlink_feed = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"

[[tokens]]
address    = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
stablecoin = true

[[pools]]
address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
name    = "USDC/WETH"
"#;
}

#[cfg(test)]
mod tests {
    use super::test_support::VALID;
    use super::*;

    fn err_of(contents: &str) -> String {
        format!("{:#}", EvmFileConfig::parse(contents).unwrap_err())
    }

    #[test]
    fn is_valid_eth_address() {
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
    fn valid_config_parses() {
        let config = EvmFileConfig::parse(VALID).expect("valid config");

        assert_eq!(config.chain.confirmations, 12);
        assert_eq!(config.tokens.len(), 2);
        assert_eq!(config.pools[0].name, "USDC/WETH");
    }

    #[test]
    fn unknown_field_is_rejected_and_named() {
        let contents = VALID.replace(
            "name          = \"ethereum\"",
            "name = \"ethereum\"\nnmae = \"x\"",
        );

        assert!(err_of(&contents).contains("nmae"));
    }

    #[test]
    fn missing_required_field_is_rejected_and_named() {
        let contents = VALID.replace("confirmations = 12\n", "");

        assert!(err_of(&contents).contains("confirmations"));
    }

    #[test]
    fn invalid_pool_address_is_rejected_and_field_is_named() {
        let contents = VALID.replace(
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
            "not-an-address",
        );

        assert!(err_of(&contents).contains("pools.address"));
    }

    #[test]
    fn duplicate_token_address_is_rejected_ignoring_case() {
        let contents = format!(
            "{VALID}\n[[tokens]]\naddress = \"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2\"\n"
        );

        assert!(err_of(&contents).contains("Duplicate token address"));
    }

    #[test]
    fn stablecoin_with_feed_is_rejected() {
        let contents = VALID.replace(
            "stablecoin = true",
            "stablecoin = true\nchainlink_feed = \"0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419\"",
        );

        assert!(err_of(&contents).contains("stablecoin and also has a chainlink_feed"));
    }
}
