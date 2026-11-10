use crate::{
    BridgeSupplyEvent, BridgeTransferEvent, Event, BRIDGE_SUPPLY_SCHEMA, BRIDGE_TRANSFER_SCHEMA,
};
use anyhow::{Context, Result};
use apache_avro::{to_value, types::Value, Schema, Writer};

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum EventType {
    Transfer = 0,
    Supply = 1,
}

/// Binds an event struct to the one schema it may be encoded under, so `encode_event`
/// cannot be handed a struct that disagrees with its `EventType`.
trait AvroEvent: serde::Serialize {
    const EVENT_TYPE: EventType;
}

impl AvroEvent for BridgeTransferEvent {
    const EVENT_TYPE: EventType = EventType::Transfer;
}

impl AvroEvent for BridgeSupplyEvent {
    const EVENT_TYPE: EventType = EventType::Supply;
}

pub struct AvroEncoder {
    schemas: [Schema; 2],
}

impl AvroEncoder {
    pub fn new() -> Result<Self> {
        let schemas = [
            Schema::parse_str(BRIDGE_TRANSFER_SCHEMA)
                .context("parse BridgeTransferEvent schema")?,
            Schema::parse_str(BRIDGE_SUPPLY_SCHEMA).context("parse BridgeSupplyEvent schema")?,
        ];
        Ok(Self { schemas })
    }

    pub fn encode(&self, event: &Event) -> Result<Vec<u8>> {
        match event {
            Event::Transfer(e) => self.encode_event(e),
            Event::Supply(e) => self.encode_event(e),
        }
    }

    fn schema(&self, event_type: EventType) -> &Schema {
        &self.schemas[event_type as usize]
    }

    fn encode_event<T: AvroEvent>(&self, event: &T) -> Result<Vec<u8>> {
        let schema = self.schema(T::EVENT_TYPE);
        let name = schema.name().map_or("event", |name| name.name());
        let mut value =
            to_value(event).with_context(|| format!("failed to convert {name} to Avro value"))?;

        // Serde serializes Decimal as raw bytes, but the schema expects Value::Decimal.
        // Patch the amount field to use the decimal logical type representation.
        if let Value::Record(ref mut fields) = value {
            if let Some((_, amount_value)) = fields.iter_mut().find(|(k, _)| k == "amount") {
                if let Value::Bytes(bytes) = amount_value {
                    *amount_value = Value::Decimal(apache_avro::Decimal::from(bytes.clone()));
                }
            }
        }

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
    use crate::{SupplyOperation, TransferDirection};
    use apache_avro::{Decimal, Reader};

    fn transfer_event(direction: TransferDirection) -> BridgeTransferEvent {
        BridgeTransferEvent {
            event_id: "tx:0".into(),
            block_number: 100,
            block_timestamp: 1_700_000_000,
            transaction_hash: "0xtx".into(),
            log_index: 3,
            chain: "eip155:1".into(),
            chain_name: "Ethereum".into(),
            bridge_address: "0xbridge".into(),
            protocol: "wormhole".into(),
            direction,
            gas_used: 21000,
            gas_price: "50000000000".into(),
            event_timestamp: 1_700_000_001,
            sender: "0xsender".into(),
            recipient: "0xrecipient".into(),
            token: "0xtoken".into(),
            token_symbol: Some("USDC".into()),
            amount: Decimal::from(vec![0, 15, 66, 64]),
            destination_chain: Some("solana:5eykt".into()),
            source_chain: None,
            message_hash: Some("0xhash".into()),
            sequence_number: Some(42),
        }
    }

    #[test]
    fn each_event_variant_resolves_to_its_own_schema() {
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

        assert_eq!(name(EventType::Transfer), "BridgeTransferEvent");
        assert_eq!(name(EventType::Supply), "BridgeSupplyEvent");
    }

    #[test]
    fn both_directions_encode_under_the_transfer_schema() {
        // The direction lives in a field now, so the schema cannot distinguish lock from
        // unlock. This pins that both still produce a readable record carrying the field.
        let encoder = AvroEncoder::new().expect("encoder");

        for direction in [TransferDirection::Lock, TransferDirection::Unlock] {
            let bytes = encoder
                .encode(&Event::Transfer(transfer_event(direction)))
                .expect("encode");
            let mut reader = Reader::new(&bytes[..]).expect("readable container");
            let record = reader
                .next()
                .expect("one record")
                .expect("decodable record");

            let apache_avro::types::Value::Record(fields) = record else {
                panic!("expected a record, got {record:?}");
            };
            let direction_field = fields
                .iter()
                .find(|(key, _)| key == "direction")
                .map(|(_, value)| value.clone())
                .expect("direction field");

            match direction_field {
                apache_avro::types::Value::Enum(_, symbol) => {
                    assert_eq!(symbol, direction.as_str());
                }
                other => panic!("expected direction to be an enum, got {other:?}"),
            }
            assert!(reader.next().is_none());
        }
    }

    #[test]
    fn transfer_event_encodes_to_a_readable_record() {
        let encoder = AvroEncoder::new().expect("encoder");
        let event = Event::Transfer(transfer_event(TransferDirection::Lock));

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

    #[test]
    fn supply_events_encode_under_the_supply_schema() {
        let encoder = AvroEncoder::new().expect("encoder");
        let event = Event::Supply(BridgeSupplyEvent {
            event_id: "tx:1".into(),
            block_number: 101,
            block_timestamp: 1_700_000_000,
            transaction_hash: "0xtx".into(),
            log_index: 0,
            chain: "solana:5eykt".into(),
            chain_name: "Solana".into(),
            bridge_address: "0xbridge".into(),
            protocol: "wormhole".into(),
            operation: SupplyOperation::Mint,
            gas_used: 0,
            gas_price: "0".into(),
            event_timestamp: 1_700_000_001,
            recipient: "0xrecipient".into(),
            wrapped_token: "0xwrapped".into(),
            wrapped_token_symbol: None,
            amount: Decimal::from(vec![0, 15, 66, 64]),
            source_chain: Some("eip155:1".into()),
            message_hash: None,
        });

        let bytes = encoder.encode(&event).expect("encode");
        let mut reader = Reader::new(&bytes[..]).expect("readable container");
        let record = reader
            .next()
            .expect("one record")
            .expect("decodable record");

        let apache_avro::types::Value::Record(fields) = record else {
            panic!("expected a record, got {record:?}");
        };
        match fields
            .iter()
            .find(|(key, _)| key == "operation")
            .map(|(_, value)| value.clone())
            .expect("operation field")
        {
            apache_avro::types::Value::Enum(_, symbol) => assert_eq!(symbol, "mint"),
            other => panic!("expected operation to be an enum, got {other:?}"),
        }
        assert!(reader.next().is_none());
    }
}
