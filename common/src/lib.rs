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

pub const SWAP_SCHEMA: &str = include_str!("../../schemas/SwapEvent.avsc");
pub const MINT_SCHEMA: &str = include_str!("../../schemas/MintEvent.avsc");
pub const BURN_SCHEMA: &str = include_str!("../../schemas/BurnEvent.avsc");
pub const TRANSFER_SCHEMA: &str = include_str!("../../schemas/TransferEvent.avsc");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub pair_address: String,
    pub token0: String,
    pub token1: String,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub amount0_in: String,
    pub amount1_in: String,
    pub amount0_out: String,
    pub amount1_out: String,
    pub price: f64,
    #[serde(rename = "volumeUSD")]
    pub volume_usd: Option<f64>,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub pair_address: String,
    pub token0: String,
    pub token1: String,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub sender: String,
    pub amount0: String,
    pub amount1: String,
    pub event_timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub pair_address: String,
    pub token0: String,
    pub token1: String,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub amount0: String,
    pub amount1: String,
    pub event_timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub pair_address: String,
    pub token0: String,
    pub token1: String,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub from: String,
    pub to: String,
    pub value: String,
    pub event_timestamp: i64,
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
    Swap(SwapEvent),
    Mint(MintEvent),
    Burn(BurnEvent),
    Transfer(TransferEvent),
}

impl Event {
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::Swap(_) => "swap",
            Event::Mint(_) => "mint",
            Event::Burn(_) => "burn",
            Event::Transfer(_) => "transfer",
        }
    }

    pub fn meta(&self) -> EventMeta {
        match self {
            Event::Swap(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.pair_address.clone(),
                cloudevent_type: "com.dex.events.swap",
                event_timestamp: e.event_timestamp,
            },
            Event::Mint(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.pair_address.clone(),
                cloudevent_type: "com.dex.events.mint",
                event_timestamp: e.event_timestamp,
            },
            Event::Burn(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.pair_address.clone(),
                cloudevent_type: "com.dex.events.burn",
                event_timestamp: e.event_timestamp,
            },
            Event::Transfer(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                pair_address: e.pair_address.clone(),
                cloudevent_type: "com.dex.events.transfer",
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

    fn swap_event() -> SwapEvent {
        SwapEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            pair_address: String::new(),
            token0: String::new(),
            token1: String::new(),
            token0_symbol: None,
            token1_symbol: None,
            sender: String::new(),
            recipient: String::new(),
            amount0_in: String::new(),
            amount1_in: String::new(),
            amount0_out: String::new(),
            amount1_out: String::new(),
            price: 0.0,
            volume_usd: None,
            gas_used: 0,
            gas_price: String::new(),
            event_timestamp: 0,
        }
    }

    fn mint_event() -> MintEvent {
        MintEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            pair_address: String::new(),
            token0: String::new(),
            token1: String::new(),
            token0_symbol: None,
            token1_symbol: None,
            sender: String::new(),
            amount0: String::new(),
            amount1: String::new(),
            event_timestamp: 0,
        }
    }

    fn burn_event() -> BurnEvent {
        BurnEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            pair_address: String::new(),
            token0: String::new(),
            token1: String::new(),
            token0_symbol: None,
            token1_symbol: None,
            sender: String::new(),
            recipient: String::new(),
            amount0: String::new(),
            amount1: String::new(),
            event_timestamp: 0,
        }
    }

    fn transfer_event() -> TransferEvent {
        TransferEvent {
            event_id: String::new(),
            block_number: 0,
            block_timestamp: 0,
            transaction_hash: String::new(),
            log_index: 0,
            chain: String::new(),
            chain_name: String::new(),
            pair_address: String::new(),
            token0: String::new(),
            token1: String::new(),
            token0_symbol: None,
            token1_symbol: None,
            from: String::new(),
            to: String::new(),
            value: String::new(),
            event_timestamp: 0,
        }
    }

    fn all_events() -> [Event; 4] {
        [
            Event::Swap(swap_event()),
            Event::Mint(mint_event()),
            Event::Burn(burn_event()),
            Event::Transfer(transfer_event()),
        ]
    }

    /// The `.avsc` files and these structs are two hand-maintained sources for the same
    /// record, so silent drift between them would corrupt every published event.
    #[test]
    fn every_event_struct_matches_its_schema_fields() {
        let cases = [
            ("swap", SWAP_SCHEMA, struct_field_names(&swap_event())),
            ("mint", MINT_SCHEMA, struct_field_names(&mint_event())),
            ("burn", BURN_SCHEMA, struct_field_names(&burn_event())),
            (
                "transfer",
                TRANSFER_SCHEMA,
                struct_field_names(&transfer_event()),
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
    fn event_type_agrees_with_cloudevent_type_and_serde_tag() {
        for event in all_events() {
            let json = serde_json::to_value(&event).expect("event must serialize");
            assert_eq!(json["eventType"], event.event_type());
            assert_eq!(
                event.meta().cloudevent_type,
                format!("com.dex.events.{}", event.event_type())
            );
        }
    }
}
