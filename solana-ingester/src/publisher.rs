use crate::events::{BurnEvent, Event, MintEvent, SwapEvent};
use anyhow::{Context, Result};
use apache_avro::{Schema, Writer, to_value};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub enum PublishError {
    // Fatal - Exit process (Dapr not running, all publishes will fail)
    DaprSidecarUnreachable(String),

    // Retry scenarios - Transient failures
    RequestTimeout(String),
    ServiceUnavailable(String),
    RateLimitExceeded(String),
    InternalServerError(String),

    // DLQ scenarios - Permanent failures
    BadRequest(String),
    ComponentOrTopicNotFound(String),
    Unauthorized(String),
    SerializationFailed(String),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::DaprSidecarUnreachable(msg) => {
                write!(f, "Dapr sidecar unreachable: {}", msg)
            }
            PublishError::RequestTimeout(msg) => write!(f, "Request timeout: {}", msg),
            PublishError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            PublishError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            PublishError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            PublishError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            PublishError::ComponentOrTopicNotFound(msg) => {
                write!(f, "Component or topic not found: {}", msg)
            }
            PublishError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            PublishError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
        }
    }
}

impl std::error::Error for PublishError {}

pub struct Publisher {
    client: Client,
    dapr_url: String,
    pubsub_name: String,
    trading_topic: String,
    liquidity_topic: String,
    dlq_topic: String,
    max_retries: u32,
    initial_backoff_ms: u64,
    schemas: AvroSchemas,
}

struct AvroSchemas {
    swap_schema: Schema,
    mint_schema: Schema,
    burn_schema: Schema,
}

impl Publisher {
    pub fn new(
        dapr_host: &str,
        dapr_port: u16,
        pubsub_name: &str,
        trading_topic: &str,
        liquidity_topic: &str,
        dlq_topic: &str,
        max_retries: u32,
        initial_backoff_ms: u64,
    ) -> Result<Self> {
        let dapr_url = format!("http://{}:{}", dapr_host, dapr_port);

        // Load embedded Avro schemas
        let swap_schema = Schema::parse_str(include_str!("avro/SwapEvent.avsc"))
            .context("Failed to parse SwapEvent schema")?;
        let mint_schema = Schema::parse_str(include_str!("avro/MintEvent.avsc"))
            .context("Failed to parse MintEvent schema")?;
        let burn_schema = Schema::parse_str(include_str!("avro/BurnEvent.avsc"))
            .context("Failed to parse BurnEvent schema")?;

        Ok(Self {
            client: Client::new(),
            dapr_url,
            pubsub_name: pubsub_name.to_string(),
            trading_topic: trading_topic.to_string(),
            liquidity_topic: liquidity_topic.to_string(),
            dlq_topic: dlq_topic.to_string(),
            max_retries,
            initial_backoff_ms,
            schemas: AvroSchemas {
                swap_schema,
                mint_schema,
                burn_schema,
            },
        })
    }

    /// Publish event to Kafka via Dapr (alias for publish)
    pub async fn publish_event(&self, event: Event) -> Result<()> {
        match self.publish_with_retry(event.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // All retries failed or permanent error - send to DLQ
                error!("Publishing failed after retries, sending to DLQ: {:?}", e);
                if let Err(dlq_err) = self.publish_to_dlq(event, e.to_string()).await {
                    error!("Failed to publish to DLQ: {}", dlq_err);
                }
                // Return success to avoid halting the ingester
                Ok(())
            }
        }
    }

    /// Publish with retry logic based on error type
    async fn publish_with_retry(&self, event: Event) -> Result<(), PublishError> {
        let mut attempt = 0;
        let mut backoff_ms = self.initial_backoff_ms;

        loop {
            match self.publish(event.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    let pub_error = self.classify_publish_error(e);

                    match pub_error {
                        // Fatal: exit immediately
                        PublishError::DaprSidecarUnreachable(ref msg) => {
                            error!("Fatal: Dapr sidecar unreachable: {}", msg);
                            std::process::exit(1);
                        }

                        // Retry scenarios (if attempts remaining)
                        PublishError::RequestTimeout(_)
                        | PublishError::ServiceUnavailable(_)
                        | PublishError::RateLimitExceeded(_)
                        | PublishError::InternalServerError(_)
                            if attempt < self.max_retries =>
                        {
                            warn!(
                                "Transient error (attempt {}/{}): {:?}",
                                attempt + 1,
                                self.max_retries,
                                pub_error
                            );
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            attempt += 1;
                            backoff_ms *= 2; // Exponential backoff
                        }

                        // DLQ scenarios OR exhausted retries
                        _ => {
                            error!("Publish failed, will send to DLQ: {:?}", pub_error);
                            return Err(pub_error);
                        }
                    }
                }
            }
        }
    }

    /// Classify publish errors into concrete error types
    fn classify_publish_error(&self, error: anyhow::Error) -> PublishError {
        // Check if it's a reqwest connection error
        if let Some(reqwest_err) = error.downcast_ref::<reqwest::Error>() {
            if reqwest_err.is_connect() {
                return PublishError::DaprSidecarUnreachable(error.to_string());
            }
            if reqwest_err.is_timeout() {
                return PublishError::RequestTimeout(error.to_string());
            }
        }

        // Check HTTP status codes from error message
        let error_msg = error.to_string();
        if error_msg.contains("400") {
            PublishError::BadRequest(error_msg)
        } else if error_msg.contains("404") {
            PublishError::ComponentOrTopicNotFound(error_msg)
        } else if error_msg.contains("401") || error_msg.contains("403") {
            PublishError::Unauthorized(error_msg)
        } else if error_msg.contains("429") {
            PublishError::RateLimitExceeded(error_msg)
        } else if error_msg.contains("503") {
            PublishError::ServiceUnavailable(error_msg)
        } else if error_msg.contains("500") {
            PublishError::InternalServerError(error_msg)
        } else if error_msg.contains("encode")
            || error_msg.contains("serialize")
            || error_msg.contains("Avro")
        {
            PublishError::SerializationFailed(error_msg)
        } else {
            // Default to retryable for unknown errors (safer)
            PublishError::InternalServerError(error_msg)
        }
    }

    /// Publish failed event to DLQ with error metadata
    async fn publish_to_dlq(&self, event: Event, error_msg: String) -> Result<()> {
        let (event_id, pool_address) = match &event {
            Event::Swap(e) => (e.base.event_id.clone(), e.base.pair_address.clone()),
            Event::Mint(e) => (e.base.event_id.clone(), e.base.pair_address.clone()),
            Event::Burn(e) => (e.base.event_id.clone(), e.base.pair_address.clone()),
        };

        let dlq_payload = json!({
            "event": event,
            "error": error_msg,
            "failed_at": chrono::Utc::now().to_rfc3339(),
            "ingester": "solana-ingester",
        });

        // Build CloudEvents envelope for DLQ
        let cloud_event = json!({
            "specversion": "1.0",
            "id": format!("{}-dlq", event_id),
            "source": "ingester/solana/raydium-amm-v4/dlq",
            "type": "com.dex.events.dlq",
            "datacontenttype": "application/json",
            "subject": pool_address,
            "time": chrono::Utc::now().to_rfc3339(),
            "data": dlq_payload,
        });

        // Publish to DLQ topic (no retry - avoid infinite loops)
        let url = format!(
            "{}/v1.0/publish/{}/{}",
            self.dapr_url, self.pubsub_name, self.dlq_topic
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/cloudevents+json")
            .json(&cloud_event)
            .send()
            .await
            .context("Failed to publish to DLQ")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("DLQ publish failed: {} - {}", status, body);
        }

        debug!("Published event {} to DLQ", event_id);
        Ok(())
    }

    /// Publish event to Kafka via Dapr
    pub async fn publish(&self, event: Event) -> Result<()> {
        let (topic, avro_binary, event_id, pool_address, event_type) = match &event {
            Event::Swap(e) => {
                let binary = self.encode_swap(e)?;
                (
                    &self.trading_topic,
                    binary,
                    e.base.event_id.clone(),
                    e.base.pair_address.clone(),
                    "com.dex.events.swap",
                )
            }
            Event::Mint(e) => {
                let binary = self.encode_mint(e)?;
                (
                    &self.liquidity_topic,
                    binary,
                    e.base.event_id.clone(),
                    e.base.pair_address.clone(),
                    "com.dex.events.mint",
                )
            }
            Event::Burn(e) => {
                let binary = self.encode_burn(e)?;
                (
                    &self.liquidity_topic,
                    binary,
                    e.base.event_id.clone(),
                    e.base.pair_address.clone(),
                    "com.dex.events.burn",
                )
            }
        };

        // Build CloudEvents envelope
        let cloud_event = json!({
            "specversion": "1.0",
            "id": event_id,
            "source": "ingester/solana/raydium-amm-v4",
            "type": event_type,
            "datacontenttype": "application/avro-binary",
            "subject": pool_address,
            "time": chrono::Utc::now().to_rfc3339(),
            "data_base64": base64::encode(&avro_binary),
        });

        // Publish via Dapr
        let url = format!(
            "{}/v1.0/publish/{}/{}",
            self.dapr_url, self.pubsub_name, topic
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/cloudevents+json")
            .header("partitionKey", pool_address)
            .json(&cloud_event)
            .send()
            .await
            .context("Failed to publish to Dapr")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Dapr publish failed: {} - {}", status, body);
        }

        debug!("Published event {} to topic {}", event_id, topic);

        Ok(())
    }

    fn encode_swap(&self, event: &SwapEvent) -> Result<Vec<u8>> {
        self.encode_event(event, &self.schemas.swap_schema, "SwapEvent")
    }

    fn encode_mint(&self, event: &MintEvent) -> Result<Vec<u8>> {
        self.encode_event(event, &self.schemas.mint_schema, "MintEvent")
    }

    fn encode_burn(&self, event: &BurnEvent) -> Result<Vec<u8>> {
        self.encode_event(event, &self.schemas.burn_schema, "BurnEvent")
    }

    fn encode_event<T: serde::Serialize>(
        &self,
        event: &T,
        schema: &Schema,
        event_type: &str,
    ) -> Result<Vec<u8>> {
        let value = to_value(event)
            .with_context(|| format!("Failed to convert {} to Avro value", event_type))?;

        let mut writer = Writer::new(schema, Vec::new())?;
        writer
            .append_value_ref(&value)
            .with_context(|| format!("Failed to encode {} to Avro", event_type))?;

        Ok(writer.into_inner()?)
    }
}

// Add base64 encoding support
mod base64 {
    use base64::{Engine as _, engine::general_purpose};

    pub fn encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::BaseEvent;

    fn mock_base_event() -> BaseEvent {
        BaseEvent {
            event_type: "Swap".to_string(),
            event_id: "test:0".to_string(),
            block_number: 100,
            block_timestamp: 1000000,
            transaction_hash: "test_tx".to_string(),
            log_index: 0,
            chain: "solana:test".to_string(),
            chain_name: "Solana".to_string(),
            pair_address: "test_pair".to_string(),
            token0: "token0".to_string(),
            token1: "token1".to_string(),
            token0_symbol: Some("TK0".to_string()),
            token1_symbol: Some("TK1".to_string()),
            event_timestamp: 1000000,
        }
    }

    #[test]
    fn test_swap_encoding() {
        let publisher = Publisher::new(
            "localhost",
            3501,
            "kafka-pubsub",
            "trading",
            "liquidity",
            "dlq",
            3,
            100,
        )
        .unwrap();

        let swap = SwapEvent {
            base: mock_base_event(),
            sender: "sender".to_string(),
            recipient: "recipient".to_string(),
            amount0_in: "1000".to_string(),
            amount1_in: "0".to_string(),
            amount0_out: "0".to_string(),
            amount1_out: "990".to_string(),
            price: 0.99,
            volume_usd: Some(1000.0),
            gas_used: 5000,
            gas_price: "1".to_string(),
        };

        let encoded = publisher
            .encode_swap(&swap)
            .expect("encode_swap should succeed");
        assert!(!encoded.is_empty(), "encoded binary should not be empty");
        assert!(
            encoded.len() > 10,
            "encoded data should have reasonable size"
        );
    }
}
