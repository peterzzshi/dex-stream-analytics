use ethers::types::Address;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngesterError {
    #[error("RPC connection failed: {0}")]
    RpcConnection(#[from] ethers::providers::ProviderError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] ethers::providers::WsClientError),

    #[error("invalid pool address: {0}")]
    InvalidPoolAddress(String),

    #[error("invalid chain configuration: {0}")]
    InvalidChainConfig(String),

    #[error("decoder error: {0}")]
    Decoder(String),

    #[error("log has no topics")]
    EmptyTopics,

    #[error("unknown event topic: {0:?}")]
    UnknownEventTopic(ethers::types::H256),

    #[error("Avro encoding failed: {0}")]
    AvroEncoding(String),

    #[error("oracle price fetch failed for token {0:?}: {1}")]
    OracleFetch(Address, String),

    #[error("DLQ write failed: {0}")]
    DlqWrite(#[from] std::io::Error),

    #[error("HTTP publish failed: {0}")]
    HttpPublish(#[from] reqwest::Error),

    #[error("HTTP {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("CloudEvent serialization failed: {0}")]
    CloudEventSerialization(#[from] serde_json::Error),

    #[error("configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, IngesterError>;
