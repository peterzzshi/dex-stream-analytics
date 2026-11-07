use crate::{
    BurnEvent, Event, MintEvent, SwapEvent, TransferEvent, BURN_SCHEMA, MINT_SCHEMA, SWAP_SCHEMA,
    TRANSFER_SCHEMA,
};
use anyhow::{Context, Result};
use apache_avro::{to_value, Schema, Writer};

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum EventType {
    Swap = 0,
    Mint = 1,
    Burn = 2,
    Transfer = 3,
}

/// Binds an event struct to the one schema it may be encoded under, so `encode_event`
/// cannot be handed a struct that disagrees with its `EventType`.
trait AvroEvent: serde::Serialize {
    const EVENT_TYPE: EventType;
}

impl AvroEvent for SwapEvent {
    const EVENT_TYPE: EventType = EventType::Swap;
}

impl AvroEvent for MintEvent {
    const EVENT_TYPE: EventType = EventType::Mint;
}

impl AvroEvent for BurnEvent {
    const EVENT_TYPE: EventType = EventType::Burn;
}

impl AvroEvent for TransferEvent {
    const EVENT_TYPE: EventType = EventType::Transfer;
}

pub struct AvroEncoder {
    // Indexed by `EventType as usize`; the order must match the discriminants above.
    schemas: [Schema; 4],
}

impl AvroEncoder {
    pub fn new() -> Result<Self> {
        let schemas = [
            Schema::parse_str(SWAP_SCHEMA).context("parse SwapEvent schema")?,
            Schema::parse_str(MINT_SCHEMA).context("parse MintEvent schema")?,
            Schema::parse_str(BURN_SCHEMA).context("parse BurnEvent schema")?,
            Schema::parse_str(TRANSFER_SCHEMA).context("parse TransferEvent schema")?,
        ];
        Ok(Self { schemas })
    }

    pub fn encode(&self, event: &Event) -> Result<Vec<u8>> {
        match event {
            Event::Swap(e) => self.encode_event(e),
            Event::Mint(e) => self.encode_event(e),
            Event::Burn(e) => self.encode_event(e),
            Event::Transfer(e) => self.encode_event(e),
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
    fn each_event_type_resolves_to_its_own_schema() {
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

        assert_eq!(name(EventType::Swap), "SwapEvent");
        assert_eq!(name(EventType::Mint), "MintEvent");
        assert_eq!(name(EventType::Burn), "BurnEvent");
        assert_eq!(name(EventType::Transfer), "TransferEvent");
    }

    #[test]
    fn swap_event_encodes_to_a_readable_record() {
        let encoder = AvroEncoder::new().expect("encoder");
        let event = Event::Swap(SwapEvent {
            event_id: "tx:0".into(),
            block_number: 100,
            block_timestamp: 1_700_000_000,
            transaction_hash: "0xtx".into(),
            log_index: 3,
            chain: "eip155:1".into(),
            chain_name: "Ethereum".into(),
            pair_address: "0xpair".into(),
            token0: "0xtoken0".into(),
            token1: "0xtoken1".into(),
            token0_symbol: Some("WETH".into()),
            token1_symbol: None,
            sender: "0xsender".into(),
            recipient: "0xrecipient".into(),
            amount0_in: "1000".into(),
            amount1_in: "-500".into(),
            amount0_out: "0".into(),
            amount1_out: "0".into(),
            price: 2.0,
            volume_usd: None,
            gas_used: 21000,
            gas_price: "50000000000".into(),
            event_timestamp: 1_700_000_001,
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
        assert_eq!(
            field("volumeUSD"),
            apache_avro::types::Value::Union(0, Box::new(apache_avro::types::Value::Null))
        );
        assert!(reader.next().is_none());
    }
}
