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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapEvent {
    #[serde(flatten)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub sender: String,
    pub amount0: String,
    pub amount1: String,
    pub liquidity: String,
    pub event_timestamp: i64,
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
    pub event_timestamp: i64,
}

#[derive(Debug, Clone)]
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
