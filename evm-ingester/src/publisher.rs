use crate::avro::AvroEncoder;
use crate::config::Config;
use crate::events::{Event, EventType};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct DaprPublisher {
    client: Client,
    encoder: Arc<AvroEncoder>,
    dapr_host: String,
    dapr_port: u16,
    pubsub_name: String,
    source: String,
    trading_topic: String,
    liquidity_topic: String,
}

impl DaprPublisher {
    pub fn new(config: &Config, venue: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .context("failed to create HTTP client")?;

        let encoder = Arc::new(AvroEncoder::new().context("failed to create Avro encoder")?);

        let source = format!("evm-ingester/{}", venue);

        Ok(Self {
            client,
            encoder,
            dapr_host: config.dapr.host.clone(),
            dapr_port: config.dapr.http_port,
            pubsub_name: config.dapr.pubsub_name.clone(),
            source,
            trading_topic: config.topics.trading.clone(),
            liquidity_topic: config.topics.liquidity.clone(),
        })
    }

    fn get_topic(&self, event_type: &EventType) -> &str {
        match event_type {
            EventType::Swap => &self.trading_topic,
            EventType::Mint | EventType::Burn | EventType::Transfer => &self.liquidity_topic,
        }
    }

    pub async fn publish(&self, event: &dyn Event) {
        let event_id = event.event_id();
        let event_type = event.event_type();
        let pair = event.pair_address();

        match self.create_cloud_event(event).await {
            Ok((cloud_event_json, topic)) => {
                let url = self.build_publish_url(&topic);

                if let Err(e) = self.do_publish(&url, &cloud_event_json, pair).await {
                    warn!(
                        event_id = %event_id,
                        event_type = ?event_type,
                        pair = %pair,
                        error = %e,
                        "Publish failed"
                    );
                } else {
                    info!(
                        event_id = %event_id,
                        event_type = ?event_type,
                        pair = %pair,
                        "Event published"
                    );
                }
            }
            Err(e) => {
                error!(
                    event_id = %event_id,
                    event_type = ?event_type,
                    pair = %pair,
                    error = %e,
                    "Failed to prepare payload"
                );
            }
        }
    }

    async fn create_cloud_event(&self, event: &dyn Event) -> Result<(String, String)> {
        let event_type = event.event_type();
        let topic = self.get_topic(&event_type).to_string();

        let avro_payload = self
            .encoder
            .encode(event)
            .context("failed to encode event to Avro")?;

        let cloud_event = json!({
            "specversion": "1.0",
            "id": event.event_id(),
            "source": self.source,
            "type": event_type.cloud_event_type(),
            "datacontenttype": "application/avro-binary",
            "subject": event.pair_address(),
            "time": chrono::DateTime::from_timestamp(event.event_timestamp(), 0)
                .unwrap_or_default()
                .to_rfc3339(),
            "data_base64": BASE64.encode(&avro_payload),
        });

        let cloud_event_json =
            serde_json::to_string(&cloud_event).context("failed to serialize CloudEvent")?;

        Ok((cloud_event_json, topic))
    }

    async fn do_publish(&self, url: &str, cloud_event_json: &str, pair: &str) -> Result<()> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/cloudevents+json")
            .header("partitionKey", pair)
            .body(cloud_event_json.to_string())
            .send()
            .await
            .context("HTTP request failed")?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!("HTTP {}", status);
        }

        Ok(())
    }

    fn build_publish_url(&self, topic: &str) -> String {
        format!(
            "http://{}:{}/v1.0/publish/{}/{}",
            self.dapr_host, self.dapr_port, self.pubsub_name, topic
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, Chain, DaprConfig, Pool, TopicsConfig};
    use crate::events::EventType;

    fn test_config() -> Config {
        Config {
            dapr: DaprConfig {
                host: "localhost".to_string(),
                http_port: 3500,
                pubsub_name: "pubsub".to_string(),
            },
            topics: TopicsConfig {
                trading: "trading".to_string(),
                liquidity: "liquidity".to_string(),
            },
            app: AppConfig { port: 8080 },
            buffer: Default::default(),
            channels: Default::default(),
            chains: vec![Chain {
                chain: "eip155:1".to_string(),
                name: "ethereum".to_string(),
                rpc: "http://localhost:8545".to_string(),
                venue: "uniswap-v2".to_string(),
                confirmations: 12,
                pools: vec![Pool {
                    pair_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(),
                    name: "test-pool".to_string(),
                }],
            }],
        }
    }

    #[test]
    fn test_event_type_topic_routing() {
        assert_eq!(EventType::Swap.topic(), "trading");
        assert_eq!(EventType::Mint.topic(), "liquidity");
        assert_eq!(EventType::Burn.topic(), "liquidity");
        assert_eq!(EventType::Transfer.topic(), "liquidity");
    }

    #[test]
    fn test_build_publish_url() {
        let config = test_config();
        let publisher = DaprPublisher::new(&config, "uniswap-v2").unwrap();
        let url = publisher.build_publish_url("trading");
        assert_eq!(url, "http://localhost:3500/v1.0/publish/pubsub/trading");
    }

    #[test]
    fn test_source_format() {
        let config = test_config();
        let publisher = DaprPublisher::new(&config, "uniswap-v2").unwrap();
        assert_eq!(publisher.source, "evm-ingester/uniswap-v2");
    }
}
