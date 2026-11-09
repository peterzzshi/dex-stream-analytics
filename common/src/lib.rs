mod avro;
mod config;
mod envelope;
mod event_processor;
mod finality;
mod publisher;
mod retry;

use serde::Serialize;

pub use avro::AvroEncoder;
pub use config::{DaprConfig, RetryConfig, Secret, SharedConfig, TopicsConfig};
pub use envelope::{dlq_envelope, event_envelope, AVRO_CONTENT_TYPE, DLQ_EVENT_TYPE};
pub use event_processor::{EventProcessor, ProcessOutcome};
pub use finality::{AddResult, FinalityBuffer, FinalityMode};
pub use publisher::{Outcome, Publisher, PublisherConfig};
pub use retry::{retry_with_classifier, Retriability};

pub const BRIDGE_LOCK_SCHEMA: &str = include_str!("../../schemas/BridgeLockEvent.avsc");
pub const BRIDGE_UNLOCK_SCHEMA: &str = include_str!("../../schemas/BridgeUnlockEvent.avsc");
pub const BRIDGE_MINT_SCHEMA: &str = include_str!("../../schemas/BridgeMintEvent.avsc");
pub const BRIDGE_BURN_SCHEMA: &str = include_str!("../../schemas/BridgeBurnEvent.avsc");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeLockEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub sender: String,
    pub recipient: String,
    pub token: String,
    pub token_symbol: Option<String>,
    pub amount: String,
    pub destination_chain: Option<String>,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
    pub sequence_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeUnlockEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub sender: String,
    pub recipient: String,
    pub token: String,
    pub token_symbol: Option<String>,
    pub amount: String,
    pub destination_chain: Option<String>,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
    pub sequence_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMintEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub recipient: String,
    pub wrapped_token: String,
    pub wrapped_token_symbol: Option<String>,
    pub amount: String,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeBurnEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub recipient: String,
    pub wrapped_token: String,
    pub wrapped_token_symbol: Option<String>,
    pub amount: String,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventMeta {
    pub event_id: String,
    pub block_number: i64,
    pub pair_address: String,
    pub cloudevent_type: &'static str,
    pub event_timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "eventType", rename_all = "lowercase")]
pub enum Event {
    BridgeLock(BridgeLockEvent),
    BridgeUnlock(BridgeUnlockEvent),
    BridgeMint(BridgeMintEvent),
    BridgeBurn(BridgeBurnEvent),
}

impl Event {
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::BridgeLock(_) => "bridgelock",
            Event::BridgeUnlock(_) => "bridgeunlock",
            Event::BridgeMint(_) => "bridgemint",
            Event::BridgeBurn(_) => "bridgeburn",
        }
    }

    pub fn meta(&self) -> EventMeta {
        match self {
            Event::BridgeLock(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.bridge_address.clone(),
                cloudevent_type: "com.bridge.events.lock",
                event_timestamp: e.event_timestamp,
            },
            Event::BridgeUnlock(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.bridge_address.clone(),
                cloudevent_type: "com.bridge.events.unlock",
                event_timestamp: e.event_timestamp,
            },
            Event::BridgeMint(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.bridge_address.clone(),
                cloudevent_type: "com.bridge.events.mint",
                event_timestamp: e.event_timestamp,
            },
            Event::BridgeBurn(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.bridge_address.clone(),
                cloudevent_type: "com.bridge.events.burn",
                event_timestamp: e.event_timestamp,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::Schema;

    fn schema_field_names(schema_str: &str) -> Vec<String> {
        let schema = Schema::parse_str(schema_str).expect("schema must parse");
        let mut names: Vec<String> = match schema {
            Schema::Record(record) => record.fields.iter().map(|f| f.name.clone()).collect(),
            _ => panic!("expected record schema"),
        };
        names.sort();
        names
    }

    fn struct_field_names<T: serde::Serialize>(value: &T) -> Vec<String> {
        let json_value = serde_json::to_value(value).expect("serialize must succeed");
        let mut keys: Vec<String> = json_value
            .as_object()
            .expect("must be object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    fn bridge_lock_event() -> BridgeLockEvent {
        BridgeLockEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            bridge_address: String::new(),
            protocol: String::new(),
            gas_used: 0,
            gas_price: String::new(),
            event_timestamp: 0,
            sender: String::new(),
            recipient: String::new(),
            token: String::new(),
            token_symbol: None,
            amount: String::new(),
            destination_chain: None,
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        }
    }

    fn bridge_unlock_event() -> BridgeUnlockEvent {
        BridgeUnlockEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            bridge_address: String::new(),
            protocol: String::new(),
            gas_used: 0,
            gas_price: String::new(),
            event_timestamp: 0,
            sender: String::new(),
            recipient: String::new(),
            token: String::new(),
            token_symbol: None,
            amount: String::new(),
            destination_chain: None,
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        }
    }

    fn bridge_mint_event() -> BridgeMintEvent {
        BridgeMintEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            bridge_address: String::new(),
            protocol: String::new(),
            gas_used: 0,
            gas_price: String::new(),
            event_timestamp: 0,
            recipient: String::new(),
            wrapped_token: String::new(),
            wrapped_token_symbol: None,
            amount: String::new(),
            source_chain: None,
            message_hash: None,
        }
    }

    fn bridge_burn_event() -> BridgeBurnEvent {
        BridgeBurnEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            bridge_address: String::new(),
            protocol: String::new(),
            gas_used: 0,
            gas_price: String::new(),
            event_timestamp: 0,
            recipient: String::new(),
            wrapped_token: String::new(),
            wrapped_token_symbol: None,
            amount: String::new(),
            source_chain: None,
            message_hash: None,
        }
    }

    fn all_bridge_events() -> [Event; 4] {
        [
            Event::BridgeLock(bridge_lock_event()),
            Event::BridgeUnlock(bridge_unlock_event()),
            Event::BridgeMint(bridge_mint_event()),
            Event::BridgeBurn(bridge_burn_event()),
        ]
    }

    /// The `.avsc` files and these structs are two hand-maintained sources for the same
    /// record, so silent drift between them would corrupt every published event.
    #[test]
    fn every_bridge_event_struct_matches_its_schema_fields() {
        let cases = [
            (
                "bridge_lock",
                BRIDGE_LOCK_SCHEMA,
                struct_field_names(&bridge_lock_event()),
            ),
            (
                "bridge_unlock",
                BRIDGE_UNLOCK_SCHEMA,
                struct_field_names(&bridge_unlock_event()),
            ),
            (
                "bridge_mint",
                BRIDGE_MINT_SCHEMA,
                struct_field_names(&bridge_mint_event()),
            ),
            (
                "bridge_burn",
                BRIDGE_BURN_SCHEMA,
                struct_field_names(&bridge_burn_event()),
            ),
        ];

        for (name, schema, fields) in cases {
            assert_eq!(
                schema_field_names(schema),
                fields,
                "{name} struct drifted from its schema"
            );
        }
    }

    /// `event_type` is the canonical name; the CloudEvent type and the serde `eventType`
    /// tag are both derived from it, so neither can drift from it or from each other.
    #[test]
    fn bridge_event_type_agrees_with_cloudevent_type_and_serde_tag() {
        for event in all_bridge_events() {
            let json = serde_json::to_value(&event).expect("event must serialize");
            assert_eq!(json["eventType"], event.event_type());
        }
    }
}
