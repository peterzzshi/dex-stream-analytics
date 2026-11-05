use crate::{Event, BURN_SCHEMA, MINT_SCHEMA, SWAP_SCHEMA, TRANSFER_SCHEMA};
use anyhow::{Context, Result};
use apache_avro::{to_value, Schema, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventType {
    Swap,
    Mint,
    Burn,
    Transfer,
}

impl EventType {
    fn name(self) -> &'static str {
        match self {
            EventType::Swap => "SwapEvent",
            EventType::Mint => "MintEvent",
            EventType::Burn => "BurnEvent",
            EventType::Transfer => "TransferEvent",
        }
    }
}

pub struct AvroEncoder {
    swap_schema: Schema,
    mint_schema: Schema,
    burn_schema: Schema,
    transfer_schema: Schema,
}

impl AvroEncoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            swap_schema: Schema::parse_str(SWAP_SCHEMA).context("parse SwapEvent schema")?,
            mint_schema: Schema::parse_str(MINT_SCHEMA).context("parse MintEvent schema")?,
            burn_schema: Schema::parse_str(BURN_SCHEMA).context("parse BurnEvent schema")?,
            transfer_schema: Schema::parse_str(TRANSFER_SCHEMA)
                .context("parse TransferEvent schema")?,
        })
    }

    pub fn encode(&self, event: &Event) -> Result<Vec<u8>> {
        match event {
            Event::Swap(e) => self.encode_event(e, EventType::Swap),
            Event::Mint(e) => self.encode_event(e, EventType::Mint),
            Event::Burn(e) => self.encode_event(e, EventType::Burn),
            Event::Transfer(e) => self.encode_event(e, EventType::Transfer),
        }
    }

    fn schema(&self, event_type: EventType) -> &Schema {
        match event_type {
            EventType::Swap => &self.swap_schema,
            EventType::Mint => &self.mint_schema,
            EventType::Burn => &self.burn_schema,
            EventType::Transfer => &self.transfer_schema,
        }
    }

    fn encode_event<T: serde::Serialize>(
        &self,
        event: &T,
        event_type: EventType,
    ) -> Result<Vec<u8>> {
        let name = event_type.name();
        let value =
            to_value(event).with_context(|| format!("failed to convert {name} to Avro value"))?;
        let mut writer = Writer::new(self.schema(event_type), Vec::new())
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
    fn swap_event_encodes_to_a_readable_record() {
        let encoder = AvroEncoder::new().expect("encoder");
        let event = Event::Swap(crate::SwapEvent {
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
