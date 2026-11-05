use anyhow::{bail, Context, Result};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub chain: ChainFile,
    pub programs: ProgramsFile,
    pub pyth: PythFile,
    pub tokens: Vec<TokenFile>,
    pub pools: Vec<PoolFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainFile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramsFile {
    pub raydium_amm_v4: String,
    pub token_metadata: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythFile {
    pub max_age_seconds: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenFile {
    pub mint: String,
    pub symbol: String,
    pub pyth_account: Option<String>,
    #[serde(default)]
    pub stablecoin: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolFile {
    pub address: String,
    pub token0: String,
    pub token1: String,
}

impl FileConfig {
    pub fn parse(contents: &str) -> Result<Self> {
        let config: FileConfig = toml::from_str(contents).context("Invalid config file")?;
        config.validate()?;
        Ok(config)
    }

    pub fn token(&self, mint: &str) -> Option<&TokenFile> {
        self.tokens.iter().find(|t| t.mint == mint)
    }

    fn validate(&self) -> Result<()> {
        check_pubkey("programs.raydium_amm_v4", &self.programs.raydium_amm_v4)?;
        check_pubkey("programs.token_metadata", &self.programs.token_metadata)?;

        let mut mints = HashSet::new();
        for token in &self.tokens {
            check_pubkey("tokens.mint", &token.mint)?;
            if !mints.insert(token.mint.as_str()) {
                bail!("Duplicate token mint: {}", token.mint);
            }
            if let Some(account) = &token.pyth_account {
                check_pubkey("tokens.pyth_account", account)?;
                if token.stablecoin {
                    bail!(
                        "Token {} is a stablecoin and also has a pyth_account",
                        token.symbol
                    );
                }
            }
        }

        if self.pools.is_empty() {
            bail!("No pools configured: add at least one [[pools]] entry");
        }

        let mut addresses = HashSet::new();
        for pool in &self.pools {
            check_pubkey("pools.address", &pool.address)?;
            if !addresses.insert(pool.address.as_str()) {
                bail!("Duplicate pool address: {}", pool.address);
            }
            for mint in [&pool.token0, &pool.token1] {
                if self.token(mint).is_none() {
                    bail!(
                        "Pool {} references token {} that is not declared in [[tokens]]",
                        pool.address,
                        mint
                    );
                }
            }
        }

        Ok(())
    }
}

fn check_pubkey(field: &str, value: &str) -> Result<()> {
    Pubkey::from_str(value).with_context(|| format!("{field} is not a valid address: {value}"))?;
    Ok(())
}

#[cfg(test)]
pub mod test_support {
    pub const VALID: &str = r#"
[chain]
id   = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
name = "Solana"

[programs]
raydium_amm_v4 = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
token_metadata = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"

[pyth]
max_age_seconds = 60

[[tokens]]
mint         = "So11111111111111111111111111111111111111112"
symbol       = "SOL"
pyth_account = "7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"

[[tokens]]
mint       = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
symbol     = "USDC"
stablecoin = true

[[pools]]
address              = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
token0               = "So11111111111111111111111111111111111111112"
token1               = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
"#;
}

#[cfg(test)]
mod tests {
    use super::test_support::VALID;
    use super::*;

    fn err_of(contents: &str) -> String {
        format!("{:#}", FileConfig::parse(contents).unwrap_err())
    }

    #[test]
    fn valid_config_parses() {
        let config = FileConfig::parse(VALID).expect("valid config");

        assert_eq!(config.pools.len(), 1);
        assert_eq!(config.tokens.len(), 2);
        assert!(config.tokens[1].stablecoin);
        assert!(!config.tokens[0].stablecoin);
    }

    #[test]
    fn unknown_field_is_rejected_and_named() {
        let contents = VALID.replace("name = \"Solana\"", "name = \"Solana\"\nnmae = \"typo\"");

        assert!(err_of(&contents).contains("nmae"));
    }

    #[test]
    fn missing_required_field_is_rejected_and_named() {
        let contents = VALID.replace("symbol       = \"SOL\"\n", "");

        assert!(err_of(&contents).contains("symbol"));
    }

    #[test]
    fn pool_token_must_be_declared() {
        let contents = VALID.replace(
            "token1               = \"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v\"",
            "token1               = \"Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB\"",
        );

        assert!(err_of(&contents).contains("not declared in [[tokens]]"));
    }

    #[test]
    fn duplicate_mint_is_rejected() {
        let contents = format!(
            "{VALID}\n[[tokens]]\nmint = \"So11111111111111111111111111111111111111112\"\nsymbol = \"WSOL\"\n"
        );

        assert!(err_of(&contents).contains("Duplicate token mint"));
    }

    #[test]
    fn stablecoin_with_feed_is_rejected() {
        let contents = VALID.replace(
            "stablecoin = true",
            "stablecoin = true\npyth_account = \"7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE\"",
        );

        assert!(err_of(&contents).contains("stablecoin and also has a pyth_account"));
    }

    #[test]
    fn invalid_address_is_rejected_and_field_is_named() {
        let contents = VALID.replace(
            "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
            "not-an-address",
        );

        assert!(err_of(&contents).contains("pools.address"));
    }

    #[test]
    fn config_without_pools_is_rejected() {
        let start = VALID.find("[[pools]]").unwrap();

        assert!(err_of(&VALID[..start]).contains("pools"));
    }
}
