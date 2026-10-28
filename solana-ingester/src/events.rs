use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseEvent {
    pub event_type: String,
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
    pub event_timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub base: BaseEvent,
    pub sender: String,
    pub recipient: String,
    pub amount0_in: String,
    pub amount1_in: String,
    pub amount0_out: String,
    pub amount1_out: String,
    pub price: f64,
    pub volume_usd: Option<f64>,
    pub gas_used: i64,
    pub gas_price: String,
}

impl Serialize for SwapEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("SwapEvent", 23)?;

        // Serialize all BaseEvent fields
        state.serialize_field("eventId", &self.base.event_id)?;
        state.serialize_field("blockNumber", &self.base.block_number)?;
        state.serialize_field("blockTimestamp", &self.base.block_timestamp)?;
        state.serialize_field("transactionHash", &self.base.transaction_hash)?;
        state.serialize_field("logIndex", &self.base.log_index)?;
        state.serialize_field("chain", &self.base.chain)?;
        state.serialize_field("chainName", &self.base.chain_name)?;
        state.serialize_field("pairAddress", &self.base.pair_address)?;
        state.serialize_field("token0", &self.base.token0)?;
        state.serialize_field("token1", &self.base.token1)?;
        state.serialize_field("token0Symbol", &self.base.token0_symbol)?;
        state.serialize_field("token1Symbol", &self.base.token1_symbol)?;

        // Serialize SwapEvent-specific fields
        state.serialize_field("sender", &self.sender)?;
        state.serialize_field("recipient", &self.recipient)?;
        state.serialize_field("amount0In", &self.amount0_in)?;
        state.serialize_field("amount1In", &self.amount1_in)?;
        state.serialize_field("amount0Out", &self.amount0_out)?;
        state.serialize_field("amount1Out", &self.amount1_out)?;
        state.serialize_field("price", &self.price)?;
        state.serialize_field("volumeUSD", &self.volume_usd)?;
        state.serialize_field("gasUsed", &self.gas_used)?;
        state.serialize_field("gasPrice", &self.gas_price)?;
        state.serialize_field("eventTimestamp", &self.base.event_timestamp)?;

        state.end()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub sender: String,
    pub amount0: String,
    pub amount1: String,
    pub liquidity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub sender: String,
    pub amount0: String,
    pub amount1: String,
    pub liquidity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Event {
    Swap(SwapEvent),
    Mint(MintEvent),
    Burn(BurnEvent),
}

impl Event {
    pub fn event_type(&self) -> &str {
        match self {
            Event::Swap(_) => "SwapEvent",
            Event::Mint(_) => "MintEvent",
            Event::Burn(_) => "BurnEvent",
        }
    }

    pub fn slot(&self) -> i64 {
        match self {
            Event::Swap(e) => e.base.block_number,
            Event::Mint(e) => e.base.block_number,
            Event::Burn(e) => e.base.block_number,
        }
    }
}
