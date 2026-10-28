use anyhow::{Context, Result};
use ethers::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::config::RetryConfig;

pub struct RetryableProvider<P> {
    inner: Arc<P>,
    config: RetryConfig,
}

impl<P> RetryableProvider<P>
where
    P: Middleware,
{
    pub fn new(inner: Arc<P>, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    #[allow(dead_code)]
    pub async fn get_block<T: Into<BlockId> + Send + Sync>(
        &self,
        block_id: T,
    ) -> Result<Option<Block<TxHash>>> {
        let block_id = block_id.into();
        retry_with_backoff(
            || async {
                self.inner
                    .get_block(block_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
            &self.config,
            "get_block",
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn get_transaction<T: Send + Sync + Into<TxHash>>(
        &self,
        tx_hash: T,
    ) -> Result<Option<Transaction>> {
        let tx_hash = tx_hash.into();
        retry_with_backoff(
            || async {
                self.inner
                    .get_transaction(tx_hash)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
            &self.config,
            "get_transaction",
        )
        .await
    }

    pub async fn get_transaction_receipt<T: Send + Sync + Into<TxHash>>(
        &self,
        tx_hash: T,
    ) -> Result<Option<TransactionReceipt>> {
        let tx_hash = tx_hash.into();
        retry_with_backoff(
            || async {
                self.inner
                    .get_transaction_receipt(tx_hash)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
            &self.config,
            "get_transaction_receipt",
        )
        .await
    }

    pub fn inner(&self) -> &Arc<P> {
        &self.inner
    }
}

async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut backoff_ms = config.rpc_initial_backoff_ms;

    loop {
        match tokio::time::timeout(Duration::from_millis(config.rpc_timeout_ms), operation()).await
        {
            Ok(Ok(result)) => {
                if attempt > 0 {
                    debug!("{} succeeded after {} retries", operation_name, attempt);
                }
                return Ok(result);
            }
            Ok(Err(e)) => {
                if !is_retriable_error(&e) || attempt >= config.rpc_max_retries {
                    return Err(e).context(format!(
                        "{} failed after {} attempts",
                        operation_name, attempt
                    ));
                }

                warn!(
                    "{} failed (attempt {}/{}): {}. Retrying in {}ms",
                    operation_name,
                    attempt + 1,
                    config.rpc_max_retries,
                    e,
                    backoff_ms
                );
            }
            Err(_) => {
                if attempt >= config.rpc_max_retries {
                    return Err(anyhow::anyhow!(
                        "{} timed out after {} attempts ({}ms timeout)",
                        operation_name,
                        attempt,
                        config.rpc_timeout_ms
                    ));
                }

                warn!(
                    "{} timed out (attempt {}/{}). Retrying in {}ms",
                    operation_name,
                    attempt + 1,
                    config.rpc_max_retries,
                    backoff_ms
                );
            }
        }

        attempt += 1;

        // Add jitter (±25%) to prevent thundering herd
        let jitter = (backoff_ms as f64 * 0.25 * (rand::random::<f64>() - 0.5)) as u64;
        let delay = backoff_ms + jitter;
        sleep(Duration::from_millis(delay)).await;

        // Exponential backoff with cap
        backoff_ms = (backoff_ms * 2).min(config.rpc_max_backoff_ms);
    }
}

fn is_retriable_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();

    // Retriable: network errors, timeouts, rate limits, server errors
    if error_str.contains("timeout")
        || error_str.contains("connection")
        || error_str.contains("rate limit")
        || error_str.contains("too many requests")
        || error_str.contains("429")
        || error_str.contains("500")
        || error_str.contains("502")
        || error_str.contains("503")
        || error_str.contains("504")
        || error_str.contains("websocket")
        || error_str.contains("transport")
    {
        return true;
    }

    // Non-retriable: client errors (bad request, not found, etc.)
    if error_str.contains("400")
        || error_str.contains("401")
        || error_str.contains("403")
        || error_str.contains("404")
        || error_str.contains("invalid")
        || error_str.contains("not found")
    {
        return false;
    }

    // Default to retriable for unknown errors
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retriable_error() {
        // Retriable errors
        assert!(is_retriable_error(&anyhow::anyhow!("Connection timeout")));
        assert!(is_retriable_error(&anyhow::anyhow!("Rate limit exceeded")));
        assert!(is_retriable_error(&anyhow::anyhow!(
            "HTTP 429 Too Many Requests"
        )));
        assert!(is_retriable_error(&anyhow::anyhow!(
            "HTTP 500 Internal Server Error"
        )));
        assert!(is_retriable_error(&anyhow::anyhow!(
            "HTTP 503 Service Unavailable"
        )));
        assert!(is_retriable_error(&anyhow::anyhow!(
            "WebSocket connection failed"
        )));

        // Non-retriable errors
        assert!(!is_retriable_error(&anyhow::anyhow!(
            "HTTP 400 Bad Request"
        )));
        assert!(!is_retriable_error(&anyhow::anyhow!(
            "HTTP 401 Unauthorized"
        )));
        assert!(!is_retriable_error(&anyhow::anyhow!("HTTP 404 Not Found")));
        assert!(!is_retriable_error(&anyhow::anyhow!("Invalid signature")));

        // Unknown errors default to retriable
        assert!(is_retriable_error(&anyhow::anyhow!("Some unknown error")));
    }
}
