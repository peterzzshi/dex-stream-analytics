use anyhow::{Context, Result};
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::RpcClient as SolanaRpcClient,
    rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::RpcLogsResponse,
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::warn;

const LOG_CHANNEL_CAPACITY: usize = 1024;

pub struct RpcClient {
    http_client: Arc<SolanaRpcClient>,
    ws_url: String,
    commitment: CommitmentConfig,
}

/// Handle to a live log subscription, backed by a task that owns the WebSocket client.
pub struct LogSubscription {
    receiver: mpsc::Receiver<RpcLogsResponse>,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl LogSubscription {
    pub async fn recv(&mut self) -> Option<RpcLogsResponse> {
        self.receiver.recv().await
    }

    /// Unsubscribe server-side, close the socket, and wait for the owning task to finish.
    pub async fn close(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Err(e) = self.task.await {
            warn!("Log subscription task failed: {}", e);
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

    /// Subscribe to logs for a specific program.
    ///
    /// The pubsub client's socket task exits for good when the connection drops and the
    /// client cannot be reused, so each subscription owns its own client. A leaked
    /// `'static` client per reconnect would grow without bound; this one is dropped by
    /// the task when the subscription ends.
    pub async fn subscribe_logs(&self, program_id: Pubkey) -> Result<LogSubscription> {
        let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
        let config = RpcTransactionLogsConfig {
            commitment: Some(CommitmentConfig::confirmed()),
        };

        let client = PubsubClient::new(&self.ws_url)
            .await
            .context("Failed to connect to Solana WebSocket")?;

        let (sender, receiver) = mpsc::channel(LOG_CHANNEL_CAPACITY);
        let (ready_sender, ready_receiver) = oneshot::channel();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

        let task = tokio::spawn(async move {
            let mut shutdown_receiver = shutdown_receiver;
            let (mut stream, unsubscribe) = match client.logs_subscribe(filter, config).await {
                Ok(subscription) => subscription,
                Err(e) => {
                    // Dropping the client fires its shutdown sender, which closes the socket.
                    let _ = ready_sender.send(Err(anyhow::anyhow!("logs_subscribe error: {e:?}")));
                    return;
                }
            };

            if ready_sender.send(Ok(())).is_err() {
                drop(stream);
                unsubscribe().await;
                let _ = client.shutdown().await;
                return;
            }

            loop {
                tokio::select! {
                    message = stream.next() => match message {
                        Some(response) => {
                            if sender.send(response.value).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
                    _ = &mut shutdown_receiver => break,
                }
            }

            // Deregister before closing so the node stops streaming this program's logs.
            drop(stream);
            unsubscribe().await;
            let _ = client.shutdown().await;
        });

        ready_receiver
            .await
            .context("Log subscription task ended before reporting readiness")??;

        Ok(LogSubscription {
            receiver,
            shutdown: Some(shutdown_sender),
            task,
        })
    }

    /// Fetch full transaction details
    pub async fn get_transaction(
        &self,
        signature: Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(self.commitment),
            max_supported_transaction_version: Some(1),
            min_context_slot: None,
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
    use super::parse_commitment;

    #[test]
    fn valid_commitments_are_accepted_and_unknown_ones_rejected() {
        assert!(parse_commitment("finalized").is_ok());
        assert!(parse_commitment("confirmed").is_ok());
        assert!(parse_commitment("processed").is_ok());
        assert!(parse_commitment("invalid").is_err());
    }
}
