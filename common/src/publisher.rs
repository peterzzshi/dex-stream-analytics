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
use tracing::{debug, error, info, warn};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const CLOUDEVENTS_JSON: &str = "application/cloudevents+json";
const MAX_BACKOFF: Duration = Duration::from_secs(10);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

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
            Event::BridgeLock(_) | Event::BridgeUnlock(_) => &self.trading_topic,
            Event::BridgeMint(_) | Event::BridgeBurn(_) => &self.liquidity_topic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Published,
    DeadLettered,
    Lost,
}

/// One variant per category of delivery failure. Retriability follows from the variant,
/// so no caller has to parse a message to decide whether to try again.
#[derive(Debug, thiserror::Error)]
enum PublishError {
    /// No usable answer from the sidecar: connection refused, reset, or timed out.
    #[error("network error: {0}")]
    Network(String),
    /// The sidecar failed while serving the request (5xx, or 408).
    #[error("sidecar error: HTTP {status}: {body}")]
    Server { status: StatusCode, body: String },
    /// The sidecar asked us to slow down.
    #[error("rate limited: HTTP {status}: {body}")]
    RateLimited { status: StatusCode, body: String },
    /// The sidecar refused this request; repeating it would be refused again.
    #[error("request rejected: HTTP {status}: {body}")]
    Rejected { status: StatusCode, body: String },
    /// The event could not be turned into a payload.
    #[error("event serialisation failed: {0}")]
    Serialisation(String),
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

    /// Single health check at startup. Fails immediately if sidecar is not ready.
    /// Docker Compose dependencies ensure the sidecar starts first.
    pub async fn wait_until_ready(&self) -> Result<()> {
        let url = format!(
            "http://{}:{}/v1.0/healthz",
            self.config.dapr_host, self.config.dapr_port
        );
        let response = self
            .client
            .get(&url)
            .timeout(PROBE_TIMEOUT)
            .send()
            .await
            .with_context(|| format!("sidecar health check to {url} failed"))?;

        if response.status().is_success() {
            info!(
                sidecar = %self.config.dapr_host,
                port = self.config.dapr_port,
                "Sidecar is healthy"
            );
            Ok(())
        } else {
            anyhow::bail!(
                "sidecar {}:{} not ready: HTTP {}",
                self.config.dapr_host,
                self.config.dapr_port,
                response.status()
            )
        }
    }

    /// Publishes with retry, dead-lettering when the failure is not retriable or the
    /// retries run out. Only a lost event is an error path worth alerting on, and that is
    /// reported through `Outcome::Lost`.
    pub async fn publish(&self, event: &Event) -> Outcome {
        let meta = event.meta();

        let result = retry_with_backoff(
            self.config.max_retries,
            Duration::from_millis(self.config.initial_backoff_ms),
            || rand::rng().random_range(0.75..=1.25),
            || self.try_publish(event, &meta),
        )
        .await;

        match result {
            Ok(()) => {
                debug!(event_id = %meta.event_id, event_type = meta.cloudevent_type, "Event published");
                Outcome::Published
            }
            Err(e) => {
                error!(event_id = %meta.event_id, error = %e, "Publish failed, sending to DLQ");
                let outcome = self.dead_letter(event, &e.to_string()).await;
                if outcome == Outcome::Lost {
                    error!(event_id = %meta.event_id, "Event lost after DLQ failure");
                }
                outcome
            }
        }
    }

    /// Single DLQ attempt, no retry, to avoid loops.
    pub async fn dead_letter(&self, event: &Event, reason: &str) -> Outcome {
        let meta = event.meta();
        match self.try_dead_letter(event, &meta, reason).await {
            Ok(()) => Outcome::DeadLettered,
            Err(e) => {
                error!(event_id = %meta.event_id, error = %e, "DLQ publish failed, event lost");
                Outcome::Lost
            }
        }
    }

    async fn try_publish(&self, event: &Event, meta: &EventMeta) -> Result<(), PublishError> {
        let payload = self
            .encoder
            .encode(event)
            .map_err(|e| PublishError::Serialisation(format!("avro encode: {e:#}")))?;
        let envelope = event_envelope(&self.config.source, meta, payload)
            .map_err(|e| PublishError::Serialisation(format!("build envelope: {e:#}")))?;
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
            .map_err(|e| PublishError::Serialisation(format!("build envelope: {e:#}")))?;
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
            .map_err(|e| PublishError::Serialisation(format!("serialise CloudEvent: {e}")))?;

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
    PublishError::Network(error.to_string())
}

/// A timeout or a 5xx means the sidecar is in trouble; a 429 means it wants less traffic.
/// Everything else in the 4xx range will be refused again if repeated.
fn classify_status(status: StatusCode, body: &str) -> PublishError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        PublishError::RateLimited {
            status,
            body: body.to_string(),
        }
    } else if status == StatusCode::REQUEST_TIMEOUT || status.is_server_error() {
        PublishError::Server {
            status,
            body: body.to_string(),
        }
    } else {
        PublishError::Rejected {
            status,
            body: body.to_string(),
        }
    }
}

/// Pure so the schedule can be asserted without waiting for it.
fn delay_for(backoff: Duration, jitter: f64) -> Duration {
    backoff.mul_f64(jitter).min(MAX_BACKOFF)
}

/// `jitter` is a parameter rather than a `rand` call inside, so tests assert the exact
/// schedule and production still spreads retries across replicas.
async fn retry_with_backoff<T, F, Fut, J>(
    max_retries: u32,
    initial_backoff: Duration,
    mut jitter: J,
    mut operation: F,
) -> Result<T, PublishError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, PublishError>>,
    J: FnMut() -> f64,
{
    let mut attempt = 0;
    let mut backoff = initial_backoff;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e)
                if attempt < max_retries
                    && matches!(
                        e,
                        PublishError::Network(_)
                            | PublishError::Server { .. }
                            | PublishError::RateLimited { .. }
                    ) =>
            {
                let delay = delay_for(backoff, jitter());
                warn!(
                    attempt = attempt + 1,
                    max_retries,
                    error = %e,
                    delay_ms = delay.as_millis(),
                    "Publish failure, retrying"
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

    /// `(name, failure, attempts)` — a `fn` rather than a value because the retry loop
    /// consumes the error on every attempt.
    type FailureCase = (&'static str, fn() -> PublishError, u32);

    /// One row per failure the publisher can see, with the attempt count it earns at
    /// `max_retries = 3`: retriable failures get 4 attempts, fatal ones get 1. A status
    /// that quietly changes side shows up here as a changed count.
    #[tokio::test(start_paused = true)]
    async fn failure_category_decides_how_many_attempts_an_event_gets() {
        let cases: [FailureCase; 13] = [
            (
                "500",
                || classify_status(StatusCode::INTERNAL_SERVER_ERROR, ""),
                4,
            ),
            ("502", || classify_status(StatusCode::BAD_GATEWAY, ""), 4),
            (
                "503",
                || classify_status(StatusCode::SERVICE_UNAVAILABLE, ""),
                4,
            ),
            (
                "504",
                || classify_status(StatusCode::GATEWAY_TIMEOUT, ""),
                4,
            ),
            (
                "408",
                || classify_status(StatusCode::REQUEST_TIMEOUT, ""),
                4,
            ),
            (
                "429",
                || classify_status(StatusCode::TOO_MANY_REQUESTS, ""),
                4,
            ),
            ("400", || classify_status(StatusCode::BAD_REQUEST, ""), 1),
            ("401", || classify_status(StatusCode::UNAUTHORIZED, ""), 1),
            ("403", || classify_status(StatusCode::FORBIDDEN, ""), 1),
            ("404", || classify_status(StatusCode::NOT_FOUND, ""), 1),
            ("409", || classify_status(StatusCode::CONFLICT, ""), 1),
            (
                "transport failure",
                || PublishError::Network("connection refused".to_string()),
                4,
            ),
            (
                "avro failure",
                || PublishError::Serialisation("bad".to_string()),
                1,
            ),
        ];

        for (name, failure, expected_attempts) in cases {
            let calls = Cell::new(0u32);
            let result: Result<(), _> = retry_with_backoff(
                3,
                Duration::from_millis(100),
                || 1.0,
                || {
                    calls.set(calls.get() + 1);
                    let error = failure();
                    async move { Err(error) }
                },
            )
            .await;

            assert!(result.is_err(), "{name} must reach the caller as an error");
            assert_eq!(calls.get(), expected_attempts, "{name}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn retries_retriable_failures_until_success() {
        let calls = Cell::new(0u32);
        let result = retry_with_backoff(
            3,
            Duration::from_millis(100),
            || 1.0,
            || {
                calls.set(calls.get() + 1);
                let n = calls.get();
                async move {
                    if n < 3 {
                        Err(PublishError::RateLimited {
                            status: StatusCode::TOO_MANY_REQUESTS,
                            body: "busy".to_string(),
                        })
                    } else {
                        Ok(n)
                    }
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 3);
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_max_retries_with_doubling_backoff() {
        let calls = Cell::new(0u32);
        let start = tokio::time::Instant::now();
        let result: Result<(), _> = retry_with_backoff(
            3,
            Duration::from_millis(100),
            || 1.0,
            || {
                calls.set(calls.get() + 1);
                async {
                    Err(PublishError::Server {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        body: "busy".to_string(),
                    })
                }
            },
        )
        .await;

        assert!(matches!(result, Err(PublishError::Server { .. })));
        assert_eq!(calls.get(), 4);
        assert_eq!(start.elapsed(), Duration::from_millis(100 + 200 + 400));
    }

    #[test]
    fn jitter_spreads_retries_within_the_backoff_cap() {
        let nominal = Duration::from_millis(100);

        assert_eq!(delay_for(nominal, 0.75), Duration::from_millis(75));
        assert_eq!(delay_for(nominal, 1.0), nominal);
        assert_eq!(delay_for(nominal, 1.25), Duration::from_millis(125));
        assert_eq!(delay_for(MAX_BACKOFF, 1.25), MAX_BACKOFF);
    }
}
