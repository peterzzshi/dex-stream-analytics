use anyhow::{Context, Result};
use apache_avro::{schema::UnionSchema, types::Value, Schema};
use std::collections::HashMap;

use crate::events::{Event, EventType};

const SWAP_SCHEMA: &str = include_str!("../schemas/SwapEvent.avsc");
const MINT_SCHEMA: &str = include_str!("../schemas/MintEvent.avsc");
const BURN_SCHEMA: &str = include_str!("../schemas/BurnEvent.avsc");
const TRANSFER_SCHEMA: &str = include_str!("../schemas/TransferEvent.avsc");

pub struct AvroEncoder {
    schemas: HashMap<EventType, Schema>,
}

impl AvroEncoder {
    pub fn new() -> Result<Self> {
        let schemas = [
            (
                EventType::Swap,
                Schema::parse_str(SWAP_SCHEMA).context("parse SwapEvent schema")?,
            ),
            (
                EventType::Mint,
                Schema::parse_str(MINT_SCHEMA).context("parse MintEvent schema")?,
            ),
            (
                EventType::Burn,
                Schema::parse_str(BURN_SCHEMA).context("parse BurnEvent schema")?,
            ),
            (
                EventType::Transfer,
                Schema::parse_str(TRANSFER_SCHEMA).context("parse TransferEvent schema")?,
            ),
        ]
        .into_iter()
        .collect();

        Ok(Self { schemas })
    }

    pub fn encode(&self, event: &dyn Event) -> Result<Vec<u8>> {
        let event_type = event.event_type();
        let schema = self
            .schemas
            .get(&event_type)
            .with_context(|| format!("schema not found for event type: {:?}", event_type))?;

        let avro_value = map_to_avro_value(schema, event.to_map())?;
        apache_avro::to_avro_datum(schema, avro_value).context("encode event to Avro")
    }
}

fn map_to_avro_value(schema: &Schema, map: HashMap<String, serde_json::Value>) -> Result<Value> {
    let record_schema = match schema {
        Schema::Record(r) => r,
        _ => anyhow::bail!("expected record schema"),
    };

    record_schema
        .fields
        .iter()
        .map(|field| {
            let json = map
                .get(&field.name)
                .ok_or_else(|| anyhow::anyhow!("missing field: {}", field.name))?;
            let value = coerce(json.clone(), &field.schema)?;
            Ok((field.name.clone(), value))
        })
        .collect::<Result<Vec<_>>>()
        .map(Value::Record)
}

/// Converts a JSON value to an Avro Value using the field's schema for exact type selection.
/// This avoids guessing (e.g. Int vs Long) by reading the schema directly.
fn coerce(json: serde_json::Value, schema: &Schema) -> Result<Value> {
    match schema {
        Schema::Null => Ok(Value::Null),
        Schema::Boolean => match json {
            serde_json::Value::Bool(b) => Ok(Value::Boolean(b)),
            _ => anyhow::bail!("expected boolean, got {:?}", json),
        },
        Schema::Int => match json {
            serde_json::Value::Number(n) => {
                let i = n
                    .as_i64()
                    .and_then(|i| i32::try_from(i).ok())
                    .ok_or_else(|| anyhow::anyhow!("{} does not fit in i32", n))?;
                Ok(Value::Int(i))
            }
            _ => anyhow::bail!("expected int, got {:?}", json),
        },
        Schema::Long => match json {
            serde_json::Value::Number(n) => {
                let i = n
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("{} is not a long", n))?;
                Ok(Value::Long(i))
            }
            _ => anyhow::bail!("expected long, got {:?}", json),
        },
        Schema::Double => match json {
            serde_json::Value::Number(n) => {
                let f = n
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("{} is not a double", n))?;
                Ok(Value::Double(f))
            }
            _ => anyhow::bail!("expected double, got {:?}", json),
        },
        Schema::String => match json {
            serde_json::Value::String(s) => Ok(Value::String(s)),
            _ => anyhow::bail!("expected string, got {:?}", json),
        },
        Schema::Union(union_schema) => coerce_union(json, union_schema),
        _ => anyhow::bail!("unsupported schema type: {:?}", schema),
    }
}

fn coerce_union(json: serde_json::Value, union_schema: &UnionSchema) -> Result<Value> {
    // null -> first null variant
    if json.is_null() {
        let idx = union_schema
            .variants()
            .iter()
            .position(|s| matches!(s, Schema::Null))
            .ok_or_else(|| anyhow::anyhow!("null not a valid union variant"))?;
        return Ok(Value::Union(idx as u32, Box::new(Value::Null)));
    }

    // {"type": value} tagged encoding for non-null variants
    if let serde_json::Value::Object(obj) = json {
        if obj.len() == 1 {
            let (tag, inner_json) = obj.into_iter().next().unwrap();
            let (idx, variant_schema) = union_schema
                .variants()
                .iter()
                .enumerate()
                .find(|(_, s)| schema_tag(s) == tag)
                .ok_or_else(|| anyhow::anyhow!("no union variant matches tag '{}'", tag))?;
            let inner = coerce(inner_json, variant_schema)?;
            return Ok(Value::Union(idx as u32, Box::new(inner)));
        }
    }

    anyhow::bail!("union value must be null or a single-key object {{\"type\": value}}")
}

fn schema_tag(schema: &Schema) -> &'static str {
    match schema {
        Schema::Null => "null",
        Schema::Boolean => "boolean",
        Schema::Int => "int",
        Schema::Long => "long",
        Schema::Float => "float",
        Schema::Double => "double",
        Schema::String => "string",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BaseEvent, EventType, SwapEvent};

    fn mock_swap_event() -> SwapEvent {
        SwapEvent {
            base: BaseEvent {
                event_type: EventType::Swap,
                event_id: "test-event".to_string(),
                block_number: 100,
                block_timestamp: 1000000,
                transaction_hash: "0xabc".to_string(),
                log_index: 0,
                chain: "eip155:1".to_string(),
                chain_name: "Ethereum".to_string(),
                pair_address: "0x123".to_string(),
                token0: "0xtoken0".to_string(),
                token1: "0xtoken1".to_string(),
                token0_symbol: Some("TOKEN0".to_string()),
                token1_symbol: Some("TOKEN1".to_string()),
                event_timestamp: 1000000,
            },
            sender: "0xsender".to_string(),
            recipient: "0xrecipient".to_string(),
            amount0_in: "100".to_string(),
            amount1_in: "0".to_string(),
            amount0_out: "0".to_string(),
            amount1_out: "200".to_string(),
            price: 2.0,
            volume_usd: Some(100.0),
            gas_used: 150000,
            gas_price: "50".to_string(),
        }
    }

    #[test]
    fn test_encode_swap_event() {
        let encoder = AvroEncoder::new().unwrap();
        let bytes = encoder.encode(&mock_swap_event()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_swap_event_null_symbols() {
        let encoder = AvroEncoder::new().unwrap();
        let mut event = mock_swap_event();
        event.base.token0_symbol = None;
        event.base.token1_symbol = None;
        event.volume_usd = None;
        let bytes = encoder.encode(&event).unwrap();
        assert!(!bytes.is_empty());
    }
}
