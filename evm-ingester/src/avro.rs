use anyhow::{Context, Result};
use apache_avro::{to_value, Schema, Writer};
use dex_ingester_common::{Event, BURN_SCHEMA, MINT_SCHEMA, SWAP_SCHEMA, TRANSFER_SCHEMA};

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
            Event::Swap(e) => self.encode_event(e, &self.swap_schema, "SwapEvent"),
            Event::Mint(e) => self.encode_event(e, &self.mint_schema, "MintEvent"),
            Event::Burn(e) => self.encode_event(e, &self.burn_schema, "BurnEvent"),
            Event::Transfer(e) => self.encode_event(e, &self.transfer_schema, "TransferEvent"),
        }
    }

    fn encode_event<T: serde::Serialize>(
        &self,
        event: &T,
        schema: &Schema,
        event_type: &str,
    ) -> Result<Vec<u8>> {
        let value = to_value(event)
            .with_context(|| format!("failed to convert {} to Avro value", event_type))?;
        let mut writer = Writer::new(schema, Vec::new());
        writer
            .append_value_ref(&value)
            .with_context(|| format!("failed to encode {} to Avro", event_type))?;
        Ok(writer.into_inner()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dex_ingester_common::{Event, SwapEvent};

    fn mock_swap_event() -> Event {
        Event::Swap(SwapEvent {
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
            event_timestamp: 1000000,
        })
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
        let mut swap = match mock_swap_event() {
            Event::Swap(s) => s,
            _ => unreachable!(),
        };
        swap.token0_symbol = None;
        swap.token1_symbol = None;
        swap.volume_usd = None;
        let bytes = encoder.encode(&Event::Swap(swap)).unwrap();
        assert!(!bytes.is_empty());
    }
}
