use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

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
    pub address: String,
    pub chain_id: u64,
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

        let mut addresses = HashSet::new();
        for bridge in &self.bridge {
            check_address("bridge.address", &bridge.address)?;
            if !addresses.insert(bridge.address.to_lowercase()) {
                bail!("Duplicate bridge address: {}", bridge.address);
            }

            match bridge.protocol.as_str() {
                "wormhole" | "layerzero" | "hop" | "allbridge" => {}
                other => bail!("Invalid protocol: {other}. Must be one of: wormhole, layerzero, hop, allbridge"),
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
mod tests {
    use super::*;

    const VALID: &str = r#"
[[bridge]]
name = "Wormhole"
protocol = "wormhole"
address = "0x3ee18B2214AFF97000D974cf647E7C347E8fa585"
chain_id = 1
chain_name = "ethereum"

[[bridge]]
name = "LayerZero"
protocol = "layerzero"
address = "0x1a44076050125825900e736c501f859c50fE728c"
chain_id = 1
chain_name = "ethereum"
"#;

    fn err_of(contents: &str) -> String {
        format!("{:#}", BridgeFileConfig::parse(contents).unwrap_err())
    }

    #[test]
    fn valid_config_parses() {
        let config = BridgeFileConfig::parse(VALID).expect("valid config");

        assert_eq!(config.bridge.len(), 2);
        assert_eq!(config.bridge[0].name, "Wormhole");
        assert_eq!(config.bridge[0].protocol, "wormhole");
        assert_eq!(config.bridge[1].name, "LayerZero");
    }

    #[test]
    fn invalid_bridge_address_is_rejected() {
        let contents = VALID.replace(
            "0x3ee18B2214AFF97000D974cf647E7C347E8fa585",
            "not-an-address",
        );

        assert!(err_of(&contents).contains("bridge.address"));
    }

    #[test]
    fn duplicate_bridge_address_is_rejected() {
        let contents = VALID.replace(
            "0x1a44076050125825900e736c501f859c50fE728c",
            "0x3ee18B2214AFF97000D974cf647E7C347E8fa585",
        );

        assert!(err_of(&contents).contains("Duplicate bridge address"));
    }

    #[test]
    fn invalid_protocol_is_rejected() {
        let contents = VALID.replace("protocol = \"wormhole\"", "protocol = \"unknown\"");

        assert!(err_of(&contents).contains("Invalid protocol"));
    }

    #[test]
    fn accepts_only_well_formed_addresses() {
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
}
