use anyhow::{Context, Result};
use std::env::VarError;
use std::fmt;
use std::str::FromStr;

/// A value that must never reach logs; `Debug` prints a placeholder.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[derive(Debug, Clone)]
pub struct DaprConfig {
    pub host: String,
    pub http_port: u16,
    pub pubsub_name: String,
}

#[derive(Debug, Clone)]
pub struct TopicsConfig {
    pub trading: String,
    pub liquidity: String,
    pub dlq: String,
}

/// Settings every ingester needs, regardless of chain.
#[derive(Debug, Clone)]
pub struct SharedConfig {
    pub dapr: DaprConfig,
    pub topics: TopicsConfig,
}

impl SharedConfig {
    /// `get_env` is injected so loading can be tested without touching process env.
    pub fn load<F>(get_env: &F) -> Result<Self>
    where
        F: Fn(&str) -> Result<String, VarError>,
    {
        Ok(Self {
            dapr: DaprConfig {
                host: required(get_env, "DAPR_HOST")?,
                http_port: required_parsed(get_env, "DAPR_HTTP_PORT")?,
                pubsub_name: required(get_env, "PUBSUB_NAME")?,
            },
            topics: TopicsConfig {
                trading: required(get_env, "TOPIC_TRADING")?,
                liquidity: required(get_env, "TOPIC_LIQUIDITY")?,
                dlq: required(get_env, "TOPIC_DLQ")?,
            },
        })
    }
}

/// RPC retry policy shared by both ingesters; values are not yet configurable.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub rpc_max_retries: u32,
    pub rpc_initial_backoff_ms: u64,
    pub rpc_max_backoff_ms: u64,
    pub rpc_timeout_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            rpc_max_retries: 3,
            rpc_initial_backoff_ms: 100,
            rpc_max_backoff_ms: 5000,
            rpc_timeout_ms: 30_000,
        }
    }
}

fn required<F>(get_env: &F, key: &str) -> Result<String>
where
    F: Fn(&str) -> Result<String, VarError>,
{
    get_env(key).with_context(|| format!("{key} not set"))
}

fn required_parsed<T, F>(get_env: &F, key: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    F: Fn(&str) -> Result<String, VarError>,
{
    required(get_env, key)?
        .parse()
        .with_context(|| format!("{key} has an invalid value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [(&str, &str); 6] = [
        ("DAPR_HOST", "sidecar"),
        ("DAPR_HTTP_PORT", "3500"),
        ("PUBSUB_NAME", "kafka-pubsub"),
        ("TOPIC_TRADING", "trading"),
        ("TOPIC_LIQUIDITY", "liquidity"),
        ("TOPIC_DLQ", "dlq"),
    ];

    fn lookup_without(missing: &'static str) -> impl Fn(&str) -> Result<String, VarError> {
        move |key| {
            ALL.iter()
                .find(|(k, _)| *k == key && *k != missing)
                .map(|(_, v)| v.to_string())
                .ok_or(VarError::NotPresent)
        }
    }

    #[test]
    fn every_required_variable_is_enforced_and_named_in_the_error() {
        assert!(SharedConfig::load(&lookup_without("")).is_ok());

        for (key, _) in ALL {
            let err = SharedConfig::load(&lookup_without(key)).unwrap_err();
            assert!(
                err.to_string().contains(key),
                "error for missing {key} was: {err}"
            );
        }
    }

    #[test]
    fn debug_output_never_contains_the_secret() {
        let secret = Secret::new("wss://node.example/api-key-123".to_string());

        assert!(!format!("{secret:?}").contains("api-key-123"));
        assert_eq!(secret.expose(), "wss://node.example/api-key-123");
    }
}
