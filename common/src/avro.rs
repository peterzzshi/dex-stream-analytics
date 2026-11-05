use crate::{Event, BURN_SCHEMA, MINT_SCHEMA, SWAP_SCHEMA, TRANSFER_SCHEMA};
use anyhow::{Context, Result};
use apache_avro::{to_value, Schema, Writer};

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
            Event::Swap(e) => Self::encode_event(e, &self.swap_schema, "SwapEvent"),
            Event::Mint(e) => Self::encode_event(e, &self.mint_schema, "MintEvent"),
            Event::Burn(e) => Self::encode_event(e, &self.burn_schema, "BurnEvent"),
            Event::Transfer(e) => Self::encode_event(e, &self.transfer_schema, "TransferEvent"),
        }
    }

    fn encode_event<T: serde::Serialize>(
        event: &T,
        schema: &Schema,
        event_type: &str,
    ) -> Result<Vec<u8>> {
        let value = to_value(event)
            .with_context(|| format!("failed to convert {event_type} to Avro value"))?;
        let mut writer = Writer::new(schema, Vec::new());
        writer
            .append_value_ref(&value)
            .with_context(|| format!("failed to encode {event_type} to Avro"))?;
        Ok(writer.into_inner()?)
    }
}
