use anyhow::{Context, Result};
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::RpcClient as SolanaRpcClient,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct RpcClient {
    ws_url: String,
    http_url: String,
    commitment: CommitmentConfig,
}

impl RpcClient {
    pub fn new(ws_url: String, commitment: &str) -> Result<Self> {
        // Derive HTTP URL from WebSocket URL
        let http_url = ws_url.replace("wss://", "https://").replace("ws://", "http://");

        let commitment = parse_commitment(commitment)?;

        Ok(Self {
            ws_url,
            http_url,
            commitment,
        })
    }

    /// Subscribe to logs for a specific program
    pub async fn subscribe_logs(&self, program_id: &str) -> Result<PubsubClient> {
        let pubkey = Pubkey::from_str(program_id)
            .with_context(|| format!("Invalid program ID: {}", program_id))?;

        let client = PubsubClient::new(&self.ws_url).await
            .context("Failed to connect to Solana WebSocket")?;

        debug!("Subscribed to logs for program: {}", program_id);

        Ok(client)
    }

    /// Fetch full transaction details
    pub async fn get_transaction(
        &self,
        signature: &Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        let rpc_client = SolanaRpcClient::new_with_commitment(
            self.http_url.clone(),
            self.commitment,
        );

        let tx = rpc_client
            .get_transaction(signature, UiTransactionEncoding::JsonParsed)
            .context("Failed to fetch transaction")?;

        Ok(tx)
    }

    /// Get current finalized slot
    pub async fn get_finalized_slot(&self) -> Result<u64> {
        let rpc_client = SolanaRpcClient::new_with_commitment(
            self.http_url.clone(),
            CommitmentConfig::finalized(),
        );

        let slot = rpc_client.get_slot()
            .context("Failed to get finalized slot")?;

        Ok(slot)
    }

    /// Get slot for a specific commitment level
    pub async fn get_slot(&self) -> Result<u64> {
        let rpc_client = SolanaRpcClient::new_with_commitment(
            self.http_url.clone(),
            self.commitment,
        );

        let slot = rpc_client.get_slot()
            .context("Failed to get slot")?;

        Ok(slot)
    }
}

fn parse_commitment(commitment: &str) -> Result<CommitmentConfig> {
    match commitment.to_lowercase().as_str() {
        "processed" => Ok(CommitmentConfig::processed()),
        "confirmed" => Ok(CommitmentConfig::confirmed()),
        "finalized" => Ok(CommitmentConfig::finalized()),
        _ => anyhow::bail!("Invalid commitment level: {}", commitment),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commitment() {
        assert!(parse_commitment("finalized").is_ok());
        assert!(parse_commitment("confirmed").is_ok());
        assert!(parse_commitment("processed").is_ok());
        assert!(parse_commitment("invalid").is_err());
    }

    #[test]
    fn test_http_url_derivation() {
        let client = RpcClient::new("wss://example.com/ws".to_string(), "finalized").unwrap();
        assert_eq!(client.http_url, "https://example.com/ws");
    }
}
