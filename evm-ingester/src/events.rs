use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    Swap,
    Mint,
    Burn,
    Transfer,
}

impl EventType {
    pub fn cloud_event_type(&self) -> &'static str {
        match self {
            EventType::Swap => "com.dex.events.swap",
            EventType::Mint => "com.dex.events.mint",
            EventType::Burn => "com.dex.events.burn",
            EventType::Transfer => "com.dex.events.transfer",
        }
    }

    #[cfg(test)]
    pub fn topic(&self) -> &'static str {
        match self {
            EventType::Swap => "trading",
            EventType::Mint | EventType::Burn | EventType::Transfer => "liquidity",
        }
    }

    #[cfg(test)]
    pub fn all() -> Vec<EventType> {
        vec![
            EventType::Swap,
            EventType::Mint,
            EventType::Burn,
            EventType::Transfer,
        ]
    }
}

pub trait Event: Send + Sync {
    fn event_type(&self) -> EventType;
    fn event_id(&self) -> &str;
    fn pair_address(&self) -> &str;
    fn event_timestamp(&self) -> i64;
    fn block_number(&self) -> i64;
    fn to_map(&self) -> HashMap<String, serde_json::Value>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseEvent {
    pub event_type: EventType,
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

impl BaseEvent {
    pub fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("eventId".to_string(), serde_json::json!(self.event_id));
        map.insert(
            "blockNumber".to_string(),
            serde_json::json!(self.block_number),
        );
        map.insert(
            "blockTimestamp".to_string(),
            serde_json::json!(self.block_timestamp),
        );
        map.insert(
            "transactionHash".to_string(),
            serde_json::json!(self.transaction_hash),
        );
        map.insert("logIndex".to_string(), serde_json::json!(self.log_index));
        map.insert("chain".to_string(), serde_json::json!(self.chain));
        map.insert("chainName".to_string(), serde_json::json!(self.chain_name));
        map.insert(
            "pairAddress".to_string(),
            serde_json::json!(self.pair_address),
        );
        map.insert("token0".to_string(), serde_json::json!(self.token0));
        map.insert("token1".to_string(), serde_json::json!(self.token1));
        map.insert(
            "token0Symbol".to_string(),
            to_nullable_string(&self.token0_symbol),
        );
        map.insert(
            "token1Symbol".to_string(),
            to_nullable_string(&self.token1_symbol),
        );
        map.insert(
            "eventTimestamp".to_string(),
            serde_json::json!(self.event_timestamp),
        );
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Event for SwapEvent {
    fn event_type(&self) -> EventType {
        self.base.event_type
    }

    fn event_id(&self) -> &str {
        &self.base.event_id
    }

    fn pair_address(&self) -> &str {
        &self.base.pair_address
    }

    fn event_timestamp(&self) -> i64 {
        self.base.event_timestamp
    }

    fn block_number(&self) -> i64 {
        self.base.block_number
    }

    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.base.to_map();
        map.insert("sender".to_string(), serde_json::json!(self.sender));
        map.insert("recipient".to_string(), serde_json::json!(self.recipient));
        map.insert("amount0In".to_string(), serde_json::json!(self.amount0_in));
        map.insert("amount1In".to_string(), serde_json::json!(self.amount1_in));
        map.insert(
            "amount0Out".to_string(),
            serde_json::json!(self.amount0_out),
        );
        map.insert(
            "amount1Out".to_string(),
            serde_json::json!(self.amount1_out),
        );
        map.insert("price".to_string(), serde_json::json!(self.price));
        map.insert("volumeUSD".to_string(), to_nullable_f64(&self.volume_usd));
        map.insert("gasUsed".to_string(), serde_json::json!(self.gas_used));
        map.insert("gasPrice".to_string(), serde_json::json!(self.gas_price));
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub sender: String,
    pub amount0: String,
    pub amount1: String,
}

impl Event for MintEvent {
    fn event_type(&self) -> EventType {
        self.base.event_type
    }

    fn event_id(&self) -> &str {
        &self.base.event_id
    }

    fn pair_address(&self) -> &str {
        &self.base.pair_address
    }

    fn event_timestamp(&self) -> i64 {
        self.base.event_timestamp
    }

    fn block_number(&self) -> i64 {
        self.base.block_number
    }

    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.base.to_map();
        map.insert("sender".to_string(), serde_json::json!(self.sender));
        map.insert("amount0".to_string(), serde_json::json!(self.amount0));
        map.insert("amount1".to_string(), serde_json::json!(self.amount1));
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub sender: String,
    pub recipient: String,
    pub amount0: String,
    pub amount1: String,
}

impl Event for BurnEvent {
    fn event_type(&self) -> EventType {
        self.base.event_type
    }

    fn event_id(&self) -> &str {
        &self.base.event_id
    }

    fn pair_address(&self) -> &str {
        &self.base.pair_address
    }

    fn event_timestamp(&self) -> i64 {
        self.base.event_timestamp
    }

    fn block_number(&self) -> i64 {
        self.base.block_number
    }

    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.base.to_map();
        map.insert("sender".to_string(), serde_json::json!(self.sender));
        map.insert("recipient".to_string(), serde_json::json!(self.recipient));
        map.insert("amount0".to_string(), serde_json::json!(self.amount0));
        map.insert("amount1".to_string(), serde_json::json!(self.amount1));
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub from: String,
    pub to: String,
    pub value: String,
}

impl Event for TransferEvent {
    fn event_type(&self) -> EventType {
        self.base.event_type
    }

    fn event_id(&self) -> &str {
        &self.base.event_id
    }

    fn pair_address(&self) -> &str {
        &self.base.pair_address
    }

    fn event_timestamp(&self) -> i64 {
        self.base.event_timestamp
    }

    fn block_number(&self) -> i64 {
        self.base.block_number
    }

    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = self.base.to_map();
        map.insert("from".to_string(), serde_json::json!(self.from));
        map.insert("to".to_string(), serde_json::json!(self.to));
        map.insert("value".to_string(), serde_json::json!(self.value));
        map
    }
}

fn to_nullable_string(opt: &Option<String>) -> serde_json::Value {
    match opt {
        None => serde_json::Value::Null,
        Some(s) => serde_json::json!({"string": s}),
    }
}

fn to_nullable_f64(opt: &Option<f64>) -> serde_json::Value {
    match opt {
        None => serde_json::Value::Null,
        Some(f) => serde_json::json!({"double": f}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_cloud_event_type() {
        assert_eq!(EventType::Swap.cloud_event_type(), "com.dex.events.swap");
        assert_eq!(EventType::Mint.cloud_event_type(), "com.dex.events.mint");
        assert_eq!(EventType::Burn.cloud_event_type(), "com.dex.events.burn");
        assert_eq!(
            EventType::Transfer.cloud_event_type(),
            "com.dex.events.transfer"
        );
    }

    #[test]
    fn test_event_type_all() {
        let all = EventType::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&EventType::Swap));
        assert!(all.contains(&EventType::Mint));
        assert!(all.contains(&EventType::Burn));
        assert!(all.contains(&EventType::Transfer));
    }

    #[test]
    fn test_nullable_encoding() {
        let none_val = to_nullable_string(&None);
        assert_eq!(none_val, serde_json::Value::Null);

        let some_val = to_nullable_string(&Some("USDC".to_string()));
        assert_eq!(some_val, serde_json::json!({"string": "USDC"}));

        let none_f64 = to_nullable_f64(&None);
        assert_eq!(none_f64, serde_json::Value::Null);

        let some_f64 = to_nullable_f64(&Some(123.45));
        assert_eq!(some_f64, serde_json::json!({"double": 123.45}));
    }
}
