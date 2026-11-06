use crate::{dlq_envelope, event_envelope, AvroEncoder, Event, EventMeta, SharedConfig};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use cloudevents::Event as CloudEvent;
use rand::RngExt;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;
use tracing::{debug, error, warn};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const CLOUDEVENTS_JSON: &str = "application/cloudevents+json";
const MAX_BACKOFF: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct PublisherConfig {
    pub dapr_host: String,
    pub dapr_port: u16,
    pub pubsub_name: String,
    pub trading_topic: String,
    pub liquidity_topic: String,
    pub dlq_topic: String,
    pub source: String,
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
}

impl PublisherConfig {
    pub fn new(shared: &SharedConfig, source: String) -> Self {
        Self {
            dapr_host: shared.dapr.host.clone(),
            dapr_port: shared.dapr.http_port,
            pubsub_name: shared.dapr.pubsub_name.clone(),
            trading_topic: shared.topics.trading.clone(),
            liquidity_topic: shared.topics.liquidity.clone(),
            dlq_topic: shared.topics.dlq.clone(),
            source,
            max_retries: 3,
            initial_backoff_ms: 100,
        }
    }

    fn publish_url(&self, topic: &str) -> String {
        format!(
            "http://{}:{}/v1.0/publish/{}/{}",
            self.dapr_host, self.dapr_port, self.pubsub_name, topic
        )
    }

    fn topic_for(&self, event: &Event) -> &str {
        match event {
            Event::Swap(_) => &self.trading_topic,
            Event::Mint(_) | Event::Burn(_) | Event::Transfer(_) => &self.liquidity_topic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Published,
    DeadLettered,
    Lost,
}

/// The Dapr sidecar refused the connection; the ingester cannot do useful work without it.
#[derive(Debug, thiserror::Error)]
#[error("Dapr sidecar unreachable: {0}")]
pub struct SidecarUnreachable(pub String);

#[derive(Debug, thiserror::Error)]
enum PublishError {
    #[error("Dapr sidecar unreachable: {0}")]
    SidecarUnreachable(String),
    #[error("transient failure: {0}")]
    Transient(String),
    #[error("permanent failure: {0}")]
    Permanent(String),
}

pub struct Publisher {
    client: Client,
    encoder: AvroEncoder,
    config: PublisherConfig,
}

impl Publisher {
    pub fn new(config: PublisherConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("failed to create HTTP client")?;
        let encoder = AvroEncoder::new().context("failed to create Avro encoder")?;
        Ok(Self {
            client,
            encoder,
            config,
        })
    }

    /// Publishes with retry, dead-lettering on permanent failure or exhausted retries.
    /// `Err` means the sidecar is gone and the caller should stop.
    pub async fn publish(&self, event: &Event) -> Result<Outcome, SidecarUnreachable> {
        let meta = event.meta();

        let result = retry_with_backoff(
            self.config.max_retries,
            Duration::from_millis(self.config.initial_backoff_ms),
            || self.try_publish(event, &meta),
        )
        .await;

        match result {
            Ok(()) => {
                debug!(event_id = %meta.event_id, event_type = meta.cloudevent_type, "Event published");
                Ok(Outcome::Published)
            }
            Err(PublishError::SidecarUnreachable(msg)) => Err(SidecarUnreachable(msg)),
            Err(e) => {
                error!(event_id = %meta.event_id, error = %e, "Publish failed, sending to DLQ");
                let outcome = self.dead_letter(event, &e.to_string()).await?;
                if outcome == Outcome::Lost {
                    error!(event_id = %meta.event_id, "Event lost after DLQ failure");
                }
                Ok(outcome)
            }
        }
    }

    /// Single DLQ attempt, no retry, to avoid loops.
    pub async fn dead_letter(
        &self,
        event: &Event,
        reason: &str,
    ) -> Result<Outcome, SidecarUnreachable> {
        let meta = event.meta();
        match self.try_dead_letter(event, &meta, reason).await {
            Ok(()) => Ok(Outcome::DeadLettered),
            Err(PublishError::SidecarUnreachable(msg)) => Err(SidecarUnreachable(msg)),
            Err(e) => {
                error!(event_id = %meta.event_id, error = %e, "DLQ publish failed, event lost");
                Ok(Outcome::Lost)
            }
        }
    }

    async fn try_publish(&self, event: &Event, meta: &EventMeta) -> Result<(), PublishError> {
        let payload = self
            .encoder
            .encode(event)
            .map_err(|e| PublishError::Permanent(format!("{e:#}")))?;
        let envelope = event_envelope(&self.config.source, meta, payload)
            .map_err(|e| PublishError::Permanent(e.to_string()))?;
        let url = self.config.publish_url(self.config.topic_for(event));
        self.post(&url, &envelope, Some(&meta.pair_address)).await
    }

    async fn try_dead_letter(
        &self,
        event: &Event,
        meta: &EventMeta,
        reason: &str,
    ) -> Result<(), PublishError> {
        let failed_at = Utc::now();
        let payload = dlq_payload(event, reason, failed_at, &self.config.source);
        let source = format!("{}/dlq", self.config.source);
        let envelope = dlq_envelope(&source, meta, failed_at, payload)
            .map_err(|e| PublishError::Permanent(e.to_string()))?;
        let url = self.config.publish_url(&self.config.dlq_topic);
        self.post(&url, &envelope, None).await
    }

    async fn post(
        &self,
        url: &str,
        envelope: &CloudEvent,
        partition_key: Option<&str>,
    ) -> Result<(), PublishError> {
        let body = serde_json::to_vec(envelope)
            .map_err(|e| PublishError::Permanent(format!("serialize CloudEvent: {e}")))?;

        let mut request = self
            .client
            .post(url)
            .header(CONTENT_TYPE, CLOUDEVENTS_JSON)
            .body(body);
        if let Some(key) = partition_key {
            request = request.header("partitionKey", key);
        }

        let response = request.send().await.map_err(classify_transport_error)?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(classify_status(status, &body))
    }
}

fn dlq_payload(event: &Event, reason: &str, failed_at: DateTime<Utc>, ingester: &str) -> Value {
    json!({
        "event": event,
        "error": reason,
        "failed_at": failed_at.to_rfc3339(),
        "ingester": ingester,
    })
}

fn classify_transport_error(error: reqwest::Error) -> PublishError {
    if error.is_connect() {
        PublishError::SidecarUnreachable(error.to_string())
    } else {
        PublishError::Transient(error.to_string())
    }
}

fn classify_status(status: StatusCode, body: &str) -> PublishError {
    let detail = format!("HTTP {status}: {body}");
    if status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
    {
        PublishError::Transient(detail)
    } else {
        PublishError::Permanent(detail)
    }
}

async fn retry_with_backoff<T, F, Fut>(
    max_retries: u32,
    initial_backoff: Duration,
    mut operation: F,
) -> Result<T, PublishError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, PublishError>>,
{
    let mut attempt = 0;
    let mut backoff = initial_backoff;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(PublishError::Transient(msg)) if attempt < max_retries => {
                let jitter = rand::rng().random_range(0.75..=1.25);
                let delay = backoff.mul_f64(jitter).min(MAX_BACKOFF);
                warn!(
                    attempt = attempt + 1,
                    max_retries,
                    error = %msg,
                    delay_ms = delay.as_millis(),
                    "Transient publish failure, retrying"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MintEvent, SwapEvent};
    use std::cell::Cell;

    fn config() -> PublisherConfig {
        PublisherConfig {
            dapr_host: "localhost".to_string(),
            dapr_port: 3500,
            pubsub_name: "pubsub".to_string(),
            trading_topic: "trading".to_string(),
            liquidity_topic: "liquidity".to_string(),
            dlq_topic: "dlq".to_string(),
            source: "test-ingester".to_string(),
            max_retries: 3,
            initial_backoff_ms: 100,
        }
    }

    fn swap() -> Event {
        Event::Swap(SwapEvent {
            event_id: "swap-1".to_string(),
            block_number: 1,
            block_timestamp: 1_000,
            transaction_hash: "0xabc".to_string(),
            log_index: 0,
            chain: "eip155:1".to_string(),
            chain_name: "Ethereum".to_string(),
            pair_address: "0xpair".to_string(),
            token0: "t0".to_string(),
            token1: "t1".to_string(),
            token0_symbol: None,
            token1_symbol: None,
            sender: "s".to_string(),
            recipient: "r".to_string(),
            amount0_in: "1".to_string(),
            amount1_in: "0".to_string(),
            amount0_out: "0".to_string(),
            amount1_out: "2".to_string(),
            price: 2.0,
            volume_usd: None,
            gas_used: 0,
            gas_price: "0".to_string(),
            event_timestamp: 1_000,
        })
    }

    fn mint() -> Event {
        Event::Mint(MintEvent {
            event_id: "mint-1".to_string(),
            block_number: 1,
            block_timestamp: 1_000,
            transaction_hash: "0xabc".to_string(),
            log_index: 0,
            chain: "eip155:1".to_string(),
            chain_name: "Ethereum".to_string(),
            pair_address: "0xpair".to_string(),
            token0: "t0".to_string(),
            token1: "t1".to_string(),
            token0_symbol: None,
            token1_symbol: None,
            sender: "s".to_string(),
            amount0: "1".to_string(),
            amount1: "2".to_string(),
            event_timestamp: 1_000,
        })
    }

    #[test]
    fn swaps_route_to_trading_and_liquidity_events_to_liquidity() {
        let cfg = config();
        assert_eq!(cfg.topic_for(&swap()), "trading");
        assert_eq!(cfg.topic_for(&mint()), "liquidity");
    }

    #[test]
    fn server_errors_timeouts_and_rate_limits_are_transient() {
        for status in [
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::REQUEST_TIMEOUT,
            StatusCode::TOO_MANY_REQUESTS,
        ] {
            assert!(
                matches!(classify_status(status, ""), PublishError::Transient(_)),
                "{status} should be transient"
            );
        }
    }

    #[test]
    fn other_client_errors_are_permanent() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ] {
            assert!(
                matches!(classify_status(status, ""), PublishError::Permanent(_)),
                "{status} should be permanent"
            );
        }
    }

    #[test]
    fn dlq_payload_wraps_event_with_failure_context() {
        let failed_at = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let payload = dlq_payload(&swap(), "boom", failed_at, "test-ingester");

        assert_eq!(payload["error"], "boom");
        assert_eq!(payload["ingester"], "test-ingester");
        assert_eq!(payload["failed_at"], failed_at.to_rfc3339());
        assert_eq!(payload["event"]["eventId"], "swap-1");
    }

    #[tokio::test(start_paused = true)]
    async fn retries_transient_failures_until_success() {
        let calls = Cell::new(0u32);
        let result = retry_with_backoff(3, Duration::from_millis(100), || {
            calls.set(calls.get() + 1);
            let n = calls.get();
            async move {
                if n < 3 {
                    Err(PublishError::Transient("busy".to_string()))
                } else {
                    Ok(n)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 3);
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_max_retries_with_doubling_backoff() {
        let calls = Cell::new(0u32);
        let start = tokio::time::Instant::now();
        let result: Result<(), _> = retry_with_backoff(3, Duration::from_millis(100), || {
            calls.set(calls.get() + 1);
            async { Err(PublishError::Transient("busy".to_string())) }
        })
        .await;

        assert!(matches!(result, Err(PublishError::Transient(_))));
        assert_eq!(calls.get(), 4);
        assert_eq!(start.elapsed(), Duration::from_millis(100 + 200 + 400));
    }

    #[tokio::test(start_paused = true)]
    async fn permanent_and_sidecar_errors_are_not_retried() {
        let calls = Cell::new(0u32);
        let permanent: Result<(), _> = retry_with_backoff(3, Duration::from_millis(100), || {
            calls.set(calls.get() + 1);
            async { Err(PublishError::Permanent("bad".to_string())) }
        })
        .await;
        assert!(matches!(permanent, Err(PublishError::Permanent(_))));
        assert_eq!(calls.get(), 1);

        calls.set(0);
        let fatal: Result<(), _> = retry_with_backoff(3, Duration::from_millis(100), || {
            calls.set(calls.get() + 1);
            async { Err(PublishError::SidecarUnreachable("down".to_string())) }
        })
        .await;
        assert!(matches!(fatal, Err(PublishError::SidecarUnreachable(_))));
        assert_eq!(calls.get(), 1);
    }
}
