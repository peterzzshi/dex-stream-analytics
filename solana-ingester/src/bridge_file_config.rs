use anyhow::{bail, Context, Result};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeFileConfig {
    pub bridge: Vec<BridgeFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeFile {
    pub name: String,
    pub protocol: String,
    pub program_id: String,
    pub chain_name: String,
}

impl BridgeFileConfig {
    pub fn parse(contents: &str) -> Result<Self> {
        let config: BridgeFileConfig = toml::from_str(contents).context("Invalid bridge config")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.bridge.is_empty() {
            bail!("No bridges configured: add at least one [[bridge]] entry");
        }

        let mut program_ids = HashSet::new();
        for bridge in &self.bridge {
            Pubkey::from_str(&bridge.program_id)
                .with_context(|| format!("Invalid program_id: {}", bridge.program_id))?;

            if !program_ids.insert(bridge.program_id.clone()) {
                bail!("Duplicate bridge program_id: {}", bridge.program_id);
            }

            match bridge.protocol.as_str() {
                "wormhole" | "allbridge" => {}
                other => bail!("Invalid protocol: {other}. Must be one of: wormhole, allbridge"),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
[[bridge]]
name = "Wormhole Portal"
protocol = "wormhole"
program_id = "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb"
chain_name = "solana"

[[bridge]]
name = "Allbridge"
protocol = "allbridge"
program_id = "bb1XfoeKzmYZwmW2xxLUHJx8Q8x6dRhvyCMqWze3h4e"
chain_name = "solana"
"#;

    fn err_of(contents: &str) -> String {
        format!("{:#}", BridgeFileConfig::parse(contents).unwrap_err())
    }

    #[test]
    fn valid_config_parses() {
        let config = BridgeFileConfig::parse(VALID).expect("valid config");

        assert_eq!(config.bridge.len(), 2);
        assert_eq!(config.bridge[0].name, "Wormhole Portal");
        assert_eq!(config.bridge[0].protocol, "wormhole");
        assert_eq!(config.bridge[1].name, "Allbridge");
    }

    #[test]
    fn invalid_program_id_is_rejected() {
        let contents = VALID.replace(
            "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb",
            "not-a-pubkey",
        );

        assert!(err_of(&contents).contains("Invalid program_id"));
    }

    #[test]
    fn duplicate_program_id_is_rejected() {
        let contents = VALID.replace(
            "bb1XfoeKzmYZwmW2xxLUHJx8Q8x6dRhvyCMqWze3h4e",
            "wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb",
        );

        assert!(err_of(&contents).contains("Duplicate bridge program_id"));
    }

    #[test]
    fn invalid_protocol_is_rejected() {
        let contents = VALID.replace("protocol = \"wormhole\"", "protocol = \"unknown\"");

        assert!(err_of(&contents).contains("Invalid protocol"));
    }
}
