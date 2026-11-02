use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

use dex_ingester_common::Event;

const DEFAULT_MAX_PENDING_BLOCKS: usize = 1000;
const DEFAULT_MAX_PENDING_EVENTS: usize = 100_000;
const DEDUP_RETENTION_BLOCKS: u64 = 100;

pub struct Buffer {
    required_confirmations: u64,
    max_pending_blocks: usize,
    max_pending_events: usize,
    inner: Arc<Mutex<BufferInner>>,
}

struct BufferInner {
    pending: HashMap<u64, Vec<Event>>,
    seen_events: HashMap<String, u64>,
    duplicates_filtered: usize,
    highest_block: u64,
    last_finalized_block: u64,
}

#[derive(Default)]
pub struct AddResult {
    pub confirmed: Vec<Event>,
    pub evicted: Vec<Event>,
}

impl Buffer {
    pub fn new(confirmations: u64) -> Self {
        Self {
            required_confirmations: confirmations,
            max_pending_blocks: DEFAULT_MAX_PENDING_BLOCKS,
            max_pending_events: DEFAULT_MAX_PENDING_EVENTS,
            inner: Arc::new(Mutex::new(BufferInner {
                pending: HashMap::new(),
                seen_events: HashMap::new(),
                duplicates_filtered: 0,
                highest_block: 0,
                last_finalized_block: 0,
            })),
        }
    }

    #[allow(dead_code)]
    pub fn with_limits(mut self, max_blocks: usize, max_events: usize) -> Self {
        self.max_pending_blocks = max_blocks;
        self.max_pending_events = max_events;
        self
    }

    #[allow(dead_code)]
    pub fn required_confirmations(&self) -> u64 {
        self.required_confirmations
    }

    pub fn add(&self, event: Event) -> AddResult {
        if self.required_confirmations == 0 {
            return AddResult {
                confirmed: vec![event],
                evicted: vec![],
            };
        }

        let mut inner = self.inner.lock().unwrap();
        let meta = event.meta();
        let block_num = meta.block_number as u64;

        if inner.seen_events.contains_key(&meta.event_id) {
            inner.duplicates_filtered += 1;
            debug!(
                "Filtered duplicate event {} at block {}",
                meta.event_id, block_num
            );
            return AddResult::default();
        }
        inner.seen_events.insert(meta.event_id, block_num);

        inner
            .pending
            .entry(block_num)
            .or_insert_with(Vec::new)
            .push(event);

        if block_num > inner.highest_block {
            inner.highest_block = block_num;
        }

        let evicted = self.evict_if_over_limit(&mut inner);
        let confirmed = self.release_confirmed(&mut inner);

        if inner.highest_block >= self.required_confirmations {
            let finalized_block = inner.highest_block - self.required_confirmations;
            self.cleanup_dedup_cache(&mut inner, finalized_block);
        }

        AddResult { confirmed, evicted }
    }

    #[allow(dead_code)]
    pub fn advance_tip(&self, block_number: u64) -> AddResult {
        if self.required_confirmations == 0 {
            return AddResult::default();
        }

        let mut inner = self.inner.lock().unwrap();

        if block_number < inner.highest_block {
            warn!(
                "Potential reorg detected: chain tip went backward from {} to {}",
                inner.highest_block, block_number
            );
        }

        if block_number > inner.highest_block {
            inner.highest_block = block_number;
        }

        let confirmed = self.release_confirmed(&mut inner);

        if inner.highest_block >= self.required_confirmations {
            let finalized_block = inner.highest_block - self.required_confirmations;
            self.cleanup_dedup_cache(&mut inner, finalized_block);
        }

        AddResult {
            confirmed,
            evicted: vec![],
        }
    }

    fn cleanup_dedup_cache(&self, inner: &mut BufferInner, finalized_block: u64) {
        if finalized_block <= inner.last_finalized_block {
            return;
        }
        inner.last_finalized_block = finalized_block;

        if finalized_block > DEDUP_RETENTION_BLOCKS {
            let cleanup_threshold = finalized_block - DEDUP_RETENTION_BLOCKS;
            inner
                .seen_events
                .retain(|_, &mut event_block| event_block > cleanup_threshold);
            debug!(
                "Cleaned up dedup cache, {} entries remain",
                inner.seen_events.len()
            );
        }
    }

    pub fn flush(&self) -> Vec<Event> {
        let mut inner = self.inner.lock().unwrap();
        let mut all_events = Vec::new();

        let mut blocks: Vec<u64> = inner.pending.keys().copied().collect();
        blocks.sort_unstable();

        for block_num in blocks {
            if let Some(mut events) = inner.pending.remove(&block_num) {
                all_events.append(&mut events);
            }
        }

        debug!("Flushed {} buffered events", all_events.len());
        all_events
    }

    #[allow(dead_code)]
    pub fn pending(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.pending.values().map(|v| v.len()).sum()
    }

    #[allow(dead_code)]
    pub fn duplicates_filtered(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.duplicates_filtered
    }

    fn evict_if_over_limit(&self, inner: &mut BufferInner) -> Vec<Event> {
        let total_events: usize = inner.pending.values().map(|v| v.len()).sum();
        let total_blocks = inner.pending.len();

        if total_blocks <= self.max_pending_blocks && total_events <= self.max_pending_events {
            return vec![];
        }

        let mut blocks: Vec<u64> = inner.pending.keys().copied().collect();
        blocks.sort_unstable();

        let mut evicted = Vec::new();

        for block_num in blocks {
            if inner.pending.len() <= self.max_pending_blocks
                && evicted.len() + inner.pending.values().map(|v| v.len()).sum::<usize>()
                    <= self.max_pending_events
            {
                break;
            }

            if let Some(mut events) = inner.pending.remove(&block_num) {
                warn!(
                    "Evicting {} events from block {} due to buffer pressure",
                    events.len(),
                    block_num
                );
                evicted.append(&mut events);
            }
        }

        evicted
    }

    fn release_confirmed(&self, inner: &mut BufferInner) -> Vec<Event> {
        if inner.highest_block < self.required_confirmations {
            return vec![];
        }

        let finalized_block = inner.highest_block - self.required_confirmations;
        let mut blocks: Vec<u64> = inner.pending.keys().copied().collect();
        blocks.sort_unstable();

        let mut confirmed = Vec::new();

        for block_num in blocks {
            if block_num > finalized_block {
                break;
            }

            if let Some(mut events) = inner.pending.remove(&block_num) {
                debug!(
                    "Releasing {} events from block {} (finalized at {})",
                    events.len(),
                    block_num,
                    finalized_block
                );
                confirmed.append(&mut events);
            }
        }

        confirmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dex_ingester_common::{Event, SwapEvent};

    fn mock_event(block_number: i64) -> Event {
        Event::Swap(SwapEvent {
            event_id: format!("event-{}", block_number),
            block_number,
            block_timestamp: 1000000,
            transaction_hash: "0xabc".to_string(),
            log_index: 0,
            chain: "eip155:1".to_string(),
            chain_name: "Ethereum".to_string(),
            pair_address: "0x123".to_string(),
            token0: "0xtoken0".to_string(),
            token1: "0xtoken1".to_string(),
            token0_symbol: None,
            token1_symbol: None,
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
    fn test_zero_confirmations_passes_through() {
        let buffer = Buffer::new(0);
        let result = buffer.add(mock_event(100));
        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.evicted.len(), 0);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_buffer_holds_until_confirmed() {
        let buffer = Buffer::new(3);

        let result = buffer.add(mock_event(100));
        assert_eq!(result.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);

        let result = buffer.advance_tip(102);
        assert_eq!(result.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);

        let result = buffer.advance_tip(103);
        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_multiple_blocks_released_in_order() {
        let buffer = Buffer::new(2);

        let r1 = buffer.add(mock_event(100));
        assert_eq!(r1.confirmed.len(), 0);

        let r2 = buffer.add(mock_event(101));
        assert_eq!(r2.confirmed.len(), 0);

        let r3 = buffer.add(mock_event(102));
        assert_eq!(r3.confirmed.len(), 1);
        assert_eq!(r3.confirmed[0].meta().block_number, 100);

        let result = buffer.advance_tip(104);
        assert_eq!(result.confirmed.len(), 2);
        assert_eq!(result.confirmed[0].meta().block_number, 101);
        assert_eq!(result.confirmed[1].meta().block_number, 102);
    }

    #[test]
    fn test_eviction_on_block_limit() {
        let buffer = Buffer::new(10).with_limits(2, 100_000);

        buffer.add(mock_event(100));
        buffer.add(mock_event(101));
        let result = buffer.add(mock_event(102));

        assert_eq!(result.evicted.len(), 1);
        assert_eq!(result.evicted[0].meta().block_number, 100);
    }

    #[test]
    fn test_flush_returns_all_events() {
        let buffer = Buffer::new(10);

        buffer.add(mock_event(100));
        buffer.add(mock_event(101));
        buffer.add(mock_event(102));

        let flushed = buffer.flush();
        assert_eq!(flushed.len(), 3);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_duplicate_filtering() {
        let buffer = Buffer::new(10);

        let result1 = buffer.add(mock_event(100));
        assert_eq!(result1.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);
        assert_eq!(buffer.duplicates_filtered(), 0);

        let result2 = buffer.add(mock_event(100));
        assert_eq!(result2.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);
        assert_eq!(buffer.duplicates_filtered(), 1);
    }

    #[test]
    fn test_dedup_cache_cleanup() {
        let buffer = Buffer::new(2);

        buffer.add(mock_event(100));
        buffer.add(mock_event(101));
        buffer.add(mock_event(102));

        buffer.advance_tip(302);

        let result = buffer.add(mock_event(100));
        assert_eq!(buffer.duplicates_filtered(), 0);
        assert_eq!(result.confirmed.len(), 1);
    }

    #[test]
    fn test_reorg_detection_on_advance_tip() {
        let buffer = Buffer::new(3);

        buffer.add(mock_event(100));
        buffer.advance_tip(103);
        assert_eq!(buffer.pending(), 0);

        let result = buffer.advance_tip(95);
        assert_eq!(result.confirmed.len(), 0);
    }
}
