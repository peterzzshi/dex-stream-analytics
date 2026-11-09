use crate::{
    BridgeBurnEvent, BridgeLockEvent, BridgeMintEvent, BridgeUnlockEvent, Event,
    BRIDGE_BURN_SCHEMA, BRIDGE_LOCK_SCHEMA, BRIDGE_MINT_SCHEMA, BRIDGE_UNLOCK_SCHEMA,
};
use anyhow::{Context, Result};
use apache_avro::{to_value, Schema, Writer};

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
#[allow(clippy::enum_variant_names)]
enum EventType {
    BridgeLock = 0,
    BridgeUnlock = 1,
    BridgeMint = 2,
    BridgeBurn = 3,
}

/// Binds an event struct to the one schema it may be encoded under, so `encode_event`
/// cannot be handed a struct that disagrees with its `EventType`.
trait AvroEvent: serde::Serialize {
    const EVENT_TYPE: EventType;
}

impl AvroEvent for BridgeLockEvent {
    const EVENT_TYPE: EventType = EventType::BridgeLock;
}

impl AvroEvent for BridgeUnlockEvent {
    const EVENT_TYPE: EventType = EventType::BridgeUnlock;
}

impl AvroEvent for BridgeMintEvent {
    const EVENT_TYPE: EventType = EventType::BridgeMint;
}

impl AvroEvent for BridgeBurnEvent {
    const EVENT_TYPE: EventType = EventType::BridgeBurn;
}

pub struct AvroEncoder {
    schemas: [Schema; 4],
}

impl AvroEncoder {
    pub fn new() -> Result<Self> {
        let schemas = [
            Schema::parse_str(BRIDGE_LOCK_SCHEMA).context("parse BridgeLockEvent schema")?,
            Schema::parse_str(BRIDGE_UNLOCK_SCHEMA).context("parse BridgeUnlockEvent schema")?,
            Schema::parse_str(BRIDGE_MINT_SCHEMA).context("parse BridgeMintEvent schema")?,
            Schema::parse_str(BRIDGE_BURN_SCHEMA).context("parse BridgeBurnEvent schema")?,
        ];
        Ok(Self { schemas })
    }

    pub fn encode(&self, event: &Event) -> Result<Vec<u8>> {
        match event {
            Event::BridgeLock(e) => self.encode_event(e),
            Event::BridgeUnlock(e) => self.encode_event(e),
            Event::BridgeMint(e) => self.encode_event(e),
            Event::BridgeBurn(e) => self.encode_event(e),
        }
    }

    fn schema(&self, event_type: EventType) -> &Schema {
        &self.schemas[event_type as usize]
    }

    fn encode_event<T: AvroEvent>(&self, event: &T) -> Result<Vec<u8>> {
        let schema = self.schema(T::EVENT_TYPE);
        // The record name comes from the parsed schema rather than a handwritten literal
        // that could drift from the .avsc file.
        let name = schema.name().map_or("event", |name| name.name());
        let value =
            to_value(event).with_context(|| format!("failed to convert {name} to Avro value"))?;
        let mut writer = Writer::new(schema, Vec::new())
            .with_context(|| format!("failed to create {name} Avro writer"))?;
        writer
            .append_value_ref(&value)
            .with_context(|| format!("failed to encode {name} to Avro"))?;
        writer
            .into_inner()
            .with_context(|| format!("failed to finalize {name} Avro writer"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::Reader;

    #[test]
    fn each_bridge_event_type_resolves_to_its_own_schema() {
        // Guards the positional array: reordering `EventType` or `schemas` without the other
        // would silently encode every event under the wrong schema.
        let encoder = AvroEncoder::new().expect("encoder");
        let name = |event_type| {
            encoder
                .schema(event_type)
                .name()
                .expect("record schema")
                .name()
                .to_owned()
        };

        assert_eq!(name(EventType::BridgeLock), "BridgeLockEvent");
        assert_eq!(name(EventType::BridgeUnlock), "BridgeUnlockEvent");
        assert_eq!(name(EventType::BridgeMint), "BridgeMintEvent");
        assert_eq!(name(EventType::BridgeBurn), "BridgeBurnEvent");
    }

    #[test]
    fn bridge_lock_event_encodes_to_a_readable_record() {
        let encoder = AvroEncoder::new().expect("encoder");
        let event = Event::BridgeLock(BridgeLockEvent {
            event_id: "tx:0".into(),
            block_number: 100,
            block_timestamp: 1_700_000_000,
            transaction_hash: "0xtx".into(),
            log_index: 3,
            chain: "eip155:1".into(),
            chain_name: "Ethereum".into(),
            bridge_address: "0xbridge".into(),
            protocol: "wormhole".into(),
            gas_used: 21000,
            gas_price: "50000000000".into(),
            event_timestamp: 1_700_000_001,
            sender: "0xsender".into(),
            recipient: "0xrecipient".into(),
            token: "0xtoken".into(),
            token_symbol: Some("USDC".into()),
            amount: "1000000".into(),
            destination_chain: Some("solana:5eykt".into()),
            source_chain: None,
            message_hash: Some("0xhash".into()),
            sequence_number: Some(42),
        });

        let bytes = encoder.encode(&event).expect("encode");
        let mut reader = Reader::new(&bytes[..]).expect("readable container");

        // Round-tripping through a reader is the check that matters: `encode` could produce
        // a container no consumer can decode, and a field-level assertion would not catch it.
        let record = reader
            .next()
            .expect("one record")
            .expect("decodable record");
        let apache_avro::types::Value::Record(fields) = record else {
            panic!("expected a record, got {record:?}");
        };
        let field = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("missing field {name}"))
        };

        assert_eq!(field("eventId"), "tx:0".into());
        assert_eq!(field("logIndex"), 3.into());
        // protocol is an enum in the schema
        match field("protocol") {
            apache_avro::types::Value::Enum(_, s) => assert_eq!(s, "wormhole"),
            other => panic!("expected protocol to be an enum, got {:?}", other),
        }
        assert!(reader.next().is_none());
    }
}
