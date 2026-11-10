mod avro;
mod config;
mod envelope;
mod event_processor;
mod finality;
mod publisher;
mod retry;

use serde::{Serialize, Serializer};

pub use apache_avro::Decimal;
pub use avro::AvroEncoder;
pub use config::{DaprConfig, RetryConfig, Secret, SharedConfig, TopicsConfig};
pub use envelope::{dlq_envelope, event_envelope, AVRO_CONTENT_TYPE, DLQ_EVENT_TYPE};
pub use event_processor::{EventProcessor, ProcessOutcome};
pub use finality::{AddResult, FinalityBuffer, FinalityMode};
pub use publisher::{Outcome, Publisher, PublisherConfig};
pub use retry::{retry_with_classifier, Retriability};

pub const BRIDGE_TRANSFER_SCHEMA: &str = include_str!("../../schemas/BridgeTransferEvent.avsc");
pub const BRIDGE_SUPPLY_SCHEMA: &str = include_str!("../../schemas/BridgeSupplyEvent.avsc");

/// Which side of a value movement this event records. A lock takes value into the bridge,
/// an unlock releases it. Carried as a field rather than as a distinct record type: the two
/// records were otherwise identical, so a separate type bought no type safety, only a
/// duplicate struct and a downstream union that could not be matched exhaustively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Lock,
    Unlock,
}

impl TransferDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::Unlock => "unlock",
        }
    }

    /// Canonical event name, from which the CloudEvent type is derived.
    pub fn event_type(self) -> &'static str {
        match self {
            Self::Lock => "bridgelock",
            Self::Unlock => "bridgeunlock",
        }
    }

    pub fn cloudevent_type(self) -> &'static str {
        match self {
            Self::Lock => "com.bridge.events.lock",
            Self::Unlock => "com.bridge.events.unlock",
        }
    }
}

/// Which side of a wrapped-token supply change this event records. A mint creates supply on
/// this chain, a burn destroys it before release on the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SupplyOperation {
    Mint,
    Burn,
}

impl SupplyOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mint => "mint",
            Self::Burn => "burn",
        }
    }

    pub fn event_type(self) -> &'static str {
        match self {
            Self::Mint => "bridgemint",
            Self::Burn => "bridgeburn",
        }
    }

    pub fn cloudevent_type(self) -> &'static str {
        match self {
            Self::Mint => "com.bridge.events.mint",
            Self::Burn => "com.bridge.events.burn",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTransferEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub direction: TransferDirection,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub sender: String,
    pub recipient: String,
    pub token: String,
    pub token_symbol: Option<String>,
    pub amount: Decimal,
    pub destination_chain: Option<String>,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
    pub sequence_number: Option<i64>,
}

impl BridgeTransferEvent {
    pub fn event_type(&self) -> &'static str {
        self.direction.event_type()
    }

    pub fn cloudevent_type(&self) -> &'static str {
        self.direction.cloudevent_type()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSupplyEvent {
    pub event_id: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
    pub log_index: i32,
    pub chain: String,
    pub chain_name: String,
    pub bridge_address: String,
    pub protocol: String,
    pub operation: SupplyOperation,
    pub gas_used: i64,
    pub gas_price: String,
    pub event_timestamp: i64,
    pub recipient: String,
    pub wrapped_token: String,
    pub wrapped_token_symbol: Option<String>,
    pub amount: Decimal,
    pub source_chain: Option<String>,
    pub message_hash: Option<String>,
}

impl BridgeSupplyEvent {
    pub fn event_type(&self) -> &'static str {
        self.operation.event_type()
    }

    pub fn cloudevent_type(&self) -> &'static str {
        self.operation.cloudevent_type()
    }
}

#[derive(Debug, Clone)]
pub struct EventMeta {
    pub event_id: String,
    pub block_number: i64,
    pub bridge_address: String,
    pub cloudevent_type: &'static str,
    pub event_timestamp: i64,
}

/// One variant per topic. The four logical event types are discriminated by the `direction`
/// or `operation` field inside the variant, not by the variant itself.
#[derive(Debug, Clone)]
pub enum Event {
    Transfer(BridgeTransferEvent),
    Supply(BridgeSupplyEvent),
}

/// Emits the event's fields inline alongside an `eventType` discriminant, preserving the JSON
/// shape the DLQ payload relied on when each logical type had its own enum variant.
#[derive(Serialize)]
struct Tagged<'a, T> {
    #[serde(rename = "eventType")]
    event_type: &'static str,
    #[serde(flatten)]
    event: &'a T,
}

impl Serialize for Event {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Event::Transfer(e) => Tagged {
                event_type: e.event_type(),
                event: e,
            }
            .serialize(serializer),
            Event::Supply(e) => Tagged {
                event_type: e.event_type(),
                event: e,
            }
            .serialize(serializer),
        }
    }
}

impl Event {
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::Transfer(e) => e.event_type(),
            Event::Supply(e) => e.event_type(),
        }
    }

    pub fn meta(&self) -> EventMeta {
        match self {
            Event::Transfer(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                bridge_address: e.bridge_address.clone(),
                cloudevent_type: e.cloudevent_type(),
                event_timestamp: e.event_timestamp,
            },
            Event::Supply(e) => EventMeta {
                event_id: e.event_id.clone(),
                block_number: e.block_number,
                bridge_address: e.bridge_address.clone(),
                cloudevent_type: e.cloudevent_type(),
                event_timestamp: e.event_timestamp,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Struct fields are serialized by name, so a field added to the struct but not the schema
    /// (or vice versa) fails only at encode time in production. This pins both directions.
    fn assert_struct_matches_schema<T>(schema: &str, sample: &T)
    where
        T: serde::Serialize,
    {
        let schema = apache_avro::Schema::parse_str(schema).expect("schema parses");
        let apache_avro::Schema::Record(record) = schema else {
            panic!("expected a record schema");
        };
        let schema_fields: Vec<&str> = record.fields.iter().map(|f| f.name.as_str()).collect();

        let json = serde_json::to_value(sample).expect("serializes");
        let struct_fields: Vec<&str> = json
            .as_object()
            .expect("serializes to an object")
            .keys()
            .map(String::as_str)
            .collect();

        for field in &schema_fields {
            assert!(
                struct_fields.contains(field),
                "{name}: schema field {field} has no struct field",
                name = record.name.name()
            );
        }
        for field in &struct_fields {
            assert!(
                schema_fields.contains(field),
                "{name}: struct field {field} is not in the schema",
                name = record.name.name()
            );
        }
    }

    fn transfer_sample() -> BridgeTransferEvent {
        BridgeTransferEvent {
            event_id: "tx:0".into(),
            block_number: 1,
            block_timestamp: 1,
            transaction_hash: "0xtx".into(),
            log_index: 0,
            chain: "eip155:1".into(),
            chain_name: "Ethereum".into(),
            bridge_address: "0xbridge".into(),
            protocol: "wormhole".into(),
            direction: TransferDirection::Lock,
            gas_used: 1,
            gas_price: "1".into(),
            event_timestamp: 1,
            sender: "0xsender".into(),
            recipient: "0xrecipient".into(),
            token: "0xtoken".into(),
            token_symbol: None,
            amount: Decimal::from(vec![1]),
            destination_chain: None,
            source_chain: None,
            message_hash: None,
            sequence_number: None,
        }
    }

    fn supply_sample() -> BridgeSupplyEvent {
        BridgeSupplyEvent {
            event_id: "tx:1".into(),
            block_number: 1,
            block_timestamp: 1,
            transaction_hash: "0xtx".into(),
            log_index: 0,
            chain: "eip155:1".into(),
            chain_name: "Ethereum".into(),
            bridge_address: "0xbridge".into(),
            protocol: "wormhole".into(),
            operation: SupplyOperation::Mint,
            gas_used: 1,
            gas_price: "1".into(),
            event_timestamp: 1,
            recipient: "0xrecipient".into(),
            wrapped_token: "0xwrapped".into(),
            wrapped_token_symbol: None,
            amount: Decimal::from(vec![1]),
            source_chain: None,
            message_hash: None,
        }
    }

    #[test]
    fn every_event_struct_matches_its_schema_fields() {
        assert_struct_matches_schema(BRIDGE_TRANSFER_SCHEMA, &transfer_sample());
        assert_struct_matches_schema(BRIDGE_SUPPLY_SCHEMA, &supply_sample());
    }

    /// Both enums drive three derived strings: the CloudEvent type, the DLQ `eventType` tag, and
    /// the Avro enum symbol. A rename in one but not the others splits a logical type across two
    /// identifiers that no compiler links together.
    #[test]
    fn each_discriminant_keeps_its_cloudevent_type_and_avro_symbol_in_step() {
        let schema_symbols = |schema: &str, field_name: &str| -> Vec<String> {
            let schema = apache_avro::Schema::parse_str(schema).expect("schema parses");
            let apache_avro::Schema::Record(record) = schema else {
                panic!("expected a record schema");
            };
            let field = record
                .fields
                .iter()
                .find(|f| f.name == field_name)
                .unwrap_or_else(|| panic!("no {field_name} field"));
            let apache_avro::Schema::Enum(e) = &field.schema else {
                panic!("{field_name} is not an enum");
            };
            e.symbols.clone()
        };

        let transfers = [TransferDirection::Lock, TransferDirection::Unlock];
        for direction in transfers {
            let avro_symbols = schema_symbols(BRIDGE_TRANSFER_SCHEMA, "direction");
            assert!(
                avro_symbols.contains(&direction.as_str().to_string()),
                "{direction:?} symbol missing from schema"
            );
            assert!(direction
                .cloudevent_type()
                .ends_with(&format!(".{}", direction.as_str())));
        }
        assert_eq!(
            [
                TransferDirection::Lock.as_str(),
                TransferDirection::Unlock.as_str()
            ]
            .as_slice(),
            schema_symbols(BRIDGE_TRANSFER_SCHEMA, "direction").as_slice()
        );

        let supplies = [SupplyOperation::Mint, SupplyOperation::Burn];
        for operation in supplies {
            let avro_symbols = schema_symbols(BRIDGE_SUPPLY_SCHEMA, "operation");
            assert!(
                avro_symbols.contains(&operation.as_str().to_string()),
                "{operation:?} symbol missing from schema"
            );
            assert!(operation
                .cloudevent_type()
                .ends_with(&format!(".{}", operation.as_str())));
        }
        assert_eq!(
            [
                SupplyOperation::Mint.as_str(),
                SupplyOperation::Burn.as_str()
            ]
            .as_slice(),
            schema_symbols(BRIDGE_SUPPLY_SCHEMA, "operation").as_slice()
        );

        // The DLQ payload tags the discriminant alongside the flattened fields; the tag must
        // name the direction/operation, not the topic variant.
        let tagged = serde_json::to_value(Event::Transfer(transfer_sample())).expect("serializes");
        assert_eq!(
            tagged.get("eventType").and_then(Value::as_str),
            Some(TransferDirection::Lock.event_type())
        );
        assert_eq!(
            tagged.get("direction").and_then(Value::as_str),
            Some(TransferDirection::Lock.as_str())
        );

        let tagged = serde_json::to_value(Event::Supply(supply_sample())).expect("serializes");
        assert_eq!(
            tagged.get("eventType").and_then(Value::as_str),
            Some(SupplyOperation::Mint.event_type())
        );
        assert_eq!(
            tagged.get("operation").and_then(Value::as_str),
            Some(SupplyOperation::Mint.as_str())
        );
    }
}
