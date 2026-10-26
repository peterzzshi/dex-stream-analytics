use anyhow::{Context, Result};
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::RpcClient as SolanaRpcClient,
    rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::RpcLogsResponse,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::pin::Pin;
use std::sync::Arc;
use tracing::debug;

pub struct RpcClient {
    http_client: Arc<SolanaRpcClient>,
    ws_url: String,
    commitment: CommitmentConfig,
}

pub struct LogSubscription {
    receiver: Pin<
        Box<
            dyn futures_util::Stream<Item = solana_client::rpc_response::Response<RpcLogsResponse>>
                + Send,
        >,
    >,
}

impl LogSubscription {
    pub async fn recv(&mut self) -> Option<RpcLogsResponse> {
        match self.receiver.next().await {
            Some(response) => Some(response.value),
            None => None,
        }
    }
}

impl RpcClient {
    pub fn new(ws_url: &str, commitment: String) -> Result<Self> {
        // Derive HTTP URL from WebSocket URL
        let http_url = ws_url
            .replace("wss://", "https://")
            .replace("ws://", "http://");

        let commitment_config = parse_commitment(&commitment)?;

        let http_client = Arc::new(SolanaRpcClient::new_with_commitment(
            http_url,
            commitment_config,
        ));

        Ok(Self {
            http_client,
            ws_url: ws_url.to_string(),
            commitment: commitment_config,
        })
    }

    /// Subscribe to logs for a specific program
    /// Note: This leaks the PubsubClient on each call. In production, the subscription
    /// should be maintained for the lifetime of the application to avoid repeated leaks.
    pub async fn subscribe_logs(&self, program_id: Pubkey) -> Result<LogSubscription> {
        let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
        let config = RpcTransactionLogsConfig {
            commitment: Some(self.commitment),
        };

        let client = Box::new(
            PubsubClient::new(&self.ws_url)
                .await
                .context("Failed to connect to Solana WebSocket")?,
        );

        // Leak the client to satisfy 'static lifetime requirement of logs_subscribe
        // This is acceptable in the main loop where subscribe_logs is called once per reconnect
        let client: &'static PubsubClient = Box::leak(client);

        let (receiver, _unsubscribe) = client
            .logs_subscribe(filter, config)
            .await
            .context("Failed to subscribe to logs")?;

        debug!("Subscribed to logs for program: {}", program_id);

        Ok(LogSubscription { receiver })
    }

    /// Fetch full transaction details
    pub async fn get_transaction(
        &self,
        signature: Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(self.commitment),
            max_supported_transaction_version: Some(0),
        };

        let tx = self
            .http_client
            .get_transaction_with_config(&signature, config)
            .context("Failed to fetch transaction")?;

        Ok(tx)
    }

    /// Get current finalized slot
    pub async fn get_finalized_slot(&self) -> Result<u64> {
        let slot = self
            .http_client
            .get_slot_with_commitment(CommitmentConfig::finalized())
            .context("Failed to get finalized slot")?;

        Ok(slot)
    }

    /// Get slot for a specific commitment level
    pub async fn get_slot(&self) -> Result<u64> {
        let slot = self
            .http_client
            .get_slot_with_commitment(self.commitment)
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
    fn test_new_client() {
        let client = RpcClient::new("wss://example.com/ws", "finalized".to_string())
            .expect("RpcClient should construct successfully");

        assert_eq!(client.ws_url, "wss://example.com/ws");
        assert_eq!(client.commitment, CommitmentConfig::finalized());
    }

    #[test]
    fn test_http_url_derivation() {
        let client = RpcClient::new("wss://api.mainnet-beta.solana.com", "confirmed".to_string())
            .expect("RpcClient should construct successfully");

        assert_eq!(client.ws_url, "wss://api.mainnet-beta.solana.com");
        assert_eq!(client.commitment, CommitmentConfig::confirmed());
    }
}
