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
    pub pyth_feed_id: Option<String>,
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
            // Checked before the pairing rule so a stablecoin with a half-filled Pyth
            // config is still reported as the stablecoin violation it is.
            if token.stablecoin && (token.pyth_account.is_some() || token.pyth_feed_id.is_some()) {
                bail!(
                    "Token {} is a stablecoin and must not configure a Pyth feed",
                    token.symbol
                );
            }

            match (&token.pyth_account, &token.pyth_feed_id) {
                (Some(account), Some(feed_id)) => {
                    check_pubkey("tokens.pyth_account", account)?;
                    check_feed_id("tokens.pyth_feed_id", feed_id)?;
                }
                (Some(_), None) => bail!(
                    "Token {} has a pyth_account but no pyth_feed_id; the feed id is what \
                     proves the account holds this token's price",
                    token.symbol
                ),
                (None, Some(_)) => bail!(
                    "Token {} has a pyth_feed_id but no pyth_account to read it from",
                    token.symbol
                ),
                (None, None) => {}
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

fn check_feed_id(field: &str, value: &str) -> Result<()> {
    parse_feed_id(value).with_context(|| format!("{field} is not a Pyth feed id: {value}"))?;
    Ok(())
}

pub const FEED_ID_BYTES: usize = 32;
const FEED_ID_HEX_CHARS: usize = FEED_ID_BYTES * 2;

/// Parse a Pyth feed id from the hex form Pyth publishes it in.
///
/// Length is checked on bytes, not chars, so a multi-byte character in the value is
/// reported as a length/hex error instead of panicking on a mid-character slice.
pub fn parse_feed_id(value: &str) -> Result<[u8; FEED_ID_BYTES]> {
    let bytes = value.as_bytes();
    if bytes.len() != FEED_ID_HEX_CHARS {
        bail!(
            "expected {FEED_ID_HEX_CHARS} hex characters, got {}",
            bytes.len()
        );
    }

    let mut feed_id = [0u8; FEED_ID_BYTES];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        feed_id[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(feed_id)
}

fn hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        other => bail!("invalid hex character {:?}", char::from(other)),
    }
}

pub fn feed_id_hex(feed_id: &[u8; FEED_ID_BYTES]) -> String {
    let mut hex = String::with_capacity(FEED_ID_HEX_CHARS);
    for byte in feed_id {
        hex.push(nibble_char(byte >> 4));
        hex.push(nibble_char(byte & 0x0f));
    }
    hex
}

fn nibble_char(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        _ => char::from(b'a' + nibble - 10),
    }
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
mint          = "So11111111111111111111111111111111111111112"
symbol        = "SOL"
pyth_account  = "7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"
pyth_feed_id  = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"

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
        assert_eq!(
            config.tokens[0].pyth_feed_id.as_deref(),
            Some("ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d")
        );
    }

    #[test]
    fn pyth_account_and_feed_id_must_be_configured_together() {
        let without_feed_id = VALID.replace(
            "pyth_feed_id  = \"ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d\"\n",
            "",
        );
        let without_account = VALID.replace(
            "pyth_account  = \"7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE\"\n",
            "",
        );

        assert!(err_of(&without_feed_id).contains("no pyth_feed_id"));
        assert!(err_of(&without_account).contains("no pyth_account"));
    }

    #[test]
    fn feed_id_of_the_wrong_length_is_rejected() {
        let contents = VALID.replace(
            "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
            "ef0d8b6f",
        );

        assert!(err_of(&contents).contains("tokens.pyth_feed_id"));
    }

    #[test]
    fn feed_id_hex_round_trips_and_rejects_malformed_input() {
        let hex = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

        let feed_id = parse_feed_id(hex).expect("valid feed id");

        assert_eq!(feed_id[0], 0xef);
        assert_eq!(feed_id[31], 0x6d);
        assert_eq!(feed_id_hex(&feed_id), hex);
        assert!(parse_feed_id(&hex.replace('e', "z")).is_err());
        // 32 copies of a two-byte character is 64 bytes but only 32 hex digits,
        // so the byte-length check must not be the only guard.
        assert!(parse_feed_id(&"é".repeat(32)).is_err());
    }

    #[test]
    fn unknown_field_is_rejected_and_named() {
        let contents = VALID.replace("name = \"Solana\"", "name = \"Solana\"\nnmae = \"typo\"");

        assert!(err_of(&contents).contains("nmae"));
    }

    #[test]
    fn missing_required_field_is_rejected_and_named() {
        let contents: String = VALID
            .lines()
            .filter(|line| !line.trim_start().starts_with("symbol"))
            .collect::<Vec<_>>()
            .join("\n");

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
            "stablecoin = true\npyth_account = \"7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE\"\npyth_feed_id = \"ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d\"",
        );

        assert!(err_of(&contents).contains("stablecoin"));
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
