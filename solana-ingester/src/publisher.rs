use crate::events::{BurnEvent, Event, MintEvent, SwapEvent};
use anyhow::{Context, Result};
use apache_avro::{Schema, Writer, to_value};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error};

pub struct Publisher {
    client: Client,
    dapr_url: String,
    pubsub_name: String,
    trading_topic: String,
    liquidity_topic: String,
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
            schemas: AvroSchemas {
                swap_schema,
                mint_schema,
                burn_schema,
            },
        })
    }

    /// Publish event to Kafka via Dapr (alias for publish)
    pub async fn publish_event(&self, event: Event) -> Result<()> {
        self.publish(event).await
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
        let value = to_value(event).context("Failed to convert SwapEvent to Avro value")?;

        let mut writer = Writer::new(&self.schemas.swap_schema, Vec::new())?;
        writer
            .append_value_ref(&value)
            .context("Failed to encode SwapEvent to Avro")?;

        Ok(writer.into_inner()?)
    }

    fn encode_mint(&self, event: &MintEvent) -> Result<Vec<u8>> {
        let value = to_value(event).context("Failed to convert MintEvent to Avro value")?;

        let mut writer = Writer::new(&self.schemas.mint_schema, Vec::new())?;
        writer
            .append_value_ref(&value)
            .context("Failed to encode MintEvent to Avro")?;

        Ok(writer.into_inner()?)
    }

    fn encode_burn(&self, event: &BurnEvent) -> Result<Vec<u8>> {
        let value = to_value(event).context("Failed to convert BurnEvent to Avro value")?;

        let mut writer = Writer::new(&self.schemas.burn_schema, Vec::new())?;
        writer
            .append_value_ref(&value)
            .context("Failed to encode BurnEvent to Avro")?;

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
            "localhost".to_string(),
            3501,
            "kafka-pubsub".to_string(),
            "trading".to_string(),
            "liquidity".to_string(),
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

        let encoded = publisher.encode_swap(&swap);
        assert!(encoded.is_ok());
        assert!(!encoded.unwrap().is_empty());
    }
}
