use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, warn};

use crate::Event;

const DEFAULT_MAX_PENDING_BLOCKS: usize = 1000;
const DEFAULT_MAX_PENDING_EVENTS: usize = 100_000;
const DEDUP_RETENTION_BLOCKS: u64 = 100;

#[derive(Debug, Clone, Copy)]
pub enum FinalityMode {
    Depth(u64),
    External,
}

pub struct FinalityBuffer {
    mode: FinalityMode,
    max_pending_blocks: usize,
    max_pending_events: usize,
    inner: Mutex<BufferInner>,
}

struct BufferInner {
    pending: HashMap<u64, Vec<Event>>,
    seen_events: HashMap<String, u64>,
    highest_block: u64,
    last_finalized_block: u64,
}

#[derive(Default)]
pub struct AddResult {
    pub confirmed: Vec<Event>,
    pub evicted: Vec<Event>,
}

impl FinalityBuffer {
    pub fn new(mode: FinalityMode) -> Self {
        Self {
            mode,
            max_pending_blocks: DEFAULT_MAX_PENDING_BLOCKS,
            max_pending_events: DEFAULT_MAX_PENDING_EVENTS,
            inner: Mutex::new(BufferInner {
                pending: HashMap::new(),
                seen_events: HashMap::new(),
                highest_block: 0,
                last_finalized_block: 0,
            }),
        }
    }

    pub fn with_limits(mut self, max_blocks: usize, max_events: usize) -> Self {
        self.max_pending_blocks = max_blocks;
        self.max_pending_events = max_events;
        self
    }

    pub fn with_event_limit(mut self, max_events: usize) -> Self {
        self.max_pending_events = max_events;
        self
    }

    pub fn add(&self, event: Event, external_finalized: Option<u64>) -> AddResult {
        match self.mode {
            FinalityMode::Depth(confirmations) => {
                // Depth(0) bypasses buffering and deduplication entirely, returning events
                // immediately. Duplicates may pass through since no state is tracked.
                if confirmations == 0 {
                    return AddResult {
                        confirmed: vec![event],
                        evicted: vec![],
                    };
                }
                self.add_with_depth(event, confirmations)
            }
            FinalityMode::External => {
                let finalized =
                    external_finalized.expect("External mode requires finalized position");
                self.add_with_external(event, finalized)
            }
        }
    }

    pub fn advance_finalized(&self, finalized: u64) -> Result<Vec<Event>, String> {
        if let FinalityMode::External = self.mode {
            let mut inner = self.inner.lock().unwrap();
            Ok(self.advance_finalized_position(&mut inner, finalized))
        } else {
            Err(format!(
                "advance_finalized only valid for External mode, but buffer is in {:?} mode",
                self.mode
            ))
        }
    }

    fn add_with_depth(&self, event: Event, confirmations: u64) -> AddResult {
        let mut inner = self.inner.lock().unwrap();
        let meta = event.meta();
        let block_num = meta.block_number as u64;

        if inner.seen_events.contains_key(&meta.event_id) {
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

        let evicted = self.enforce_limits(&mut inner);
        let confirmed = self.release_confirmed(&mut inner, confirmations);

        if inner.highest_block >= confirmations {
            let finalized_block = inner.highest_block - confirmations;
            self.cleanup_dedup_cache(&mut inner, finalized_block);
        }

        AddResult { confirmed, evicted }
    }

    fn add_with_external(&self, event: Event, finalized: u64) -> AddResult {
        let mut inner = self.inner.lock().unwrap();
        let meta = event.meta();
        let block_num = meta.block_number as u64;

        if inner.seen_events.contains_key(&meta.event_id) {
            debug!(
                "Filtered duplicate event {} at block {}",
                meta.event_id, block_num
            );
            return AddResult::default();
        }
        inner.seen_events.insert(meta.event_id.clone(), block_num);

        if block_num > inner.highest_block {
            inner.highest_block = block_num;
        }

        if block_num <= finalized {
            debug!(
                "Event at block {} already finalized (current: {}), confirming immediately",
                block_num, finalized
            );
            return AddResult {
                confirmed: vec![event],
                evicted: vec![],
            };
        }

        inner
            .pending
            .entry(block_num)
            .or_insert_with(Vec::new)
            .push(event);

        let evicted = self.enforce_limits(&mut inner);

        AddResult {
            confirmed: vec![],
            evicted,
        }
    }

    fn advance_finalized_position(&self, inner: &mut BufferInner, finalized: u64) -> Vec<Event> {
        if finalized < inner.last_finalized_block {
            warn!(
                "Potential reorg detected: finalized block went backward from {} to {}",
                inner.last_finalized_block, finalized
            );
        }

        if finalized <= inner.last_finalized_block {
            return vec![];
        }

        self.retain_dedup_window(inner, finalized);

        let slots_to_release: Vec<u64> = inner
            .pending
            .keys()
            .filter(|&&slot| slot <= finalized)
            .copied()
            .collect();

        let mut confirmed = Vec::new();
        for slot in slots_to_release {
            if let Some(events) = inner.pending.remove(&slot) {
                debug!("Releasing {} events from block {}", events.len(), slot);
                confirmed.extend(events);
            }
        }

        inner.last_finalized_block = finalized;
        confirmed
    }

    fn cleanup_dedup_cache(&self, inner: &mut BufferInner, finalized_block: u64) {
        if finalized_block <= inner.last_finalized_block {
            return;
        }
        inner.last_finalized_block = finalized_block;

        self.retain_dedup_window(inner, finalized_block);
    }

    /// Drops dedup entries older than the retention window behind `finalized_block`.
    ///
    /// Callers differ in how they treat `last_finalized_block`; only the retention window
    /// itself is shared.
    fn retain_dedup_window(&self, inner: &mut BufferInner, finalized_block: u64) {
        if finalized_block <= DEDUP_RETENTION_BLOCKS {
            return;
        }
        let cleanup_threshold = finalized_block - DEDUP_RETENTION_BLOCKS;
        inner
            .seen_events
            .retain(|_, &mut event_block| event_block > cleanup_threshold);
        debug!(
            "Cleaned up dedup cache, {} entries remain",
            inner.seen_events.len()
        );
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

    pub fn pending(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.pending.values().map(|v| v.len()).sum()
    }

    fn enforce_limits(&self, inner: &mut BufferInner) -> Vec<Event> {
        let total_events: usize = inner.pending.values().map(|v| v.len()).sum();
        let total_blocks = inner.pending.len();

        if total_blocks <= self.max_pending_blocks && total_events <= self.max_pending_events {
            return vec![];
        }

        // Every evicted block moves its events from `pending` to `evicted`, so
        // `evicted.len() + pending total` is just `total_events` on every iteration.
        let over_event_limit = total_events > self.max_pending_events;

        let mut blocks: Vec<u64> = inner.pending.keys().copied().collect();
        blocks.sort_unstable();

        let mut evicted = Vec::new();

        for block_num in blocks {
            if !over_event_limit && inner.pending.len() <= self.max_pending_blocks {
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

    fn release_confirmed(&self, inner: &mut BufferInner, confirmations: u64) -> Vec<Event> {
        if inner.highest_block < confirmations {
            return vec![];
        }

        let finalized_block = inner.highest_block - confirmations;
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
    use crate::{Event, SwapEvent};

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
    fn test_depth_zero_confirmations_passes_through() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(0));
        let result = buffer.add(mock_event(100), None);
        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.evicted.len(), 0);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_depth_buffer_holds_until_confirmed() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(3));

        let result = buffer.add(mock_event(100), None);
        assert_eq!(result.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);

        let result = buffer.add(mock_event(102), None);
        assert_eq!(result.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 2);

        let result = buffer.add(mock_event(103), None);
        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(buffer.pending(), 2);
    }

    #[test]
    fn test_depth_multiple_blocks_released() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(2));

        let r1 = buffer.add(mock_event(100), None);
        assert_eq!(r1.confirmed.len(), 0);

        let r2 = buffer.add(mock_event(101), None);
        assert_eq!(r2.confirmed.len(), 0);

        let r3 = buffer.add(mock_event(102), None);
        assert_eq!(r3.confirmed.len(), 1);
        assert_eq!(r3.confirmed[0].meta().block_number, 100);

        let r4 = buffer.add(mock_event(104), None);
        assert_eq!(r4.confirmed.len(), 2);
    }

    #[test]
    fn test_depth_eviction_on_block_limit() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(10)).with_limits(2, 100_000);

        buffer.add(mock_event(100), None);
        buffer.add(mock_event(101), None);
        let result = buffer.add(mock_event(102), None);

        assert_eq!(result.evicted.len(), 1);
        assert_eq!(result.evicted[0].meta().block_number, 100);
    }

    #[test]
    fn test_depth_duplicate_filtering() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(10));

        let result1 = buffer.add(mock_event(100), None);
        assert_eq!(result1.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);

        let result2 = buffer.add(mock_event(100), None);
        assert_eq!(result2.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);
    }

    #[test]
    fn test_external_immediate_confirmation() {
        let buffer = FinalityBuffer::new(FinalityMode::External);

        let event = mock_event(100);
        let result = buffer.add(event, Some(150));

        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.evicted.len(), 0);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_external_buffering_and_release() {
        let buffer = FinalityBuffer::new(FinalityMode::External);

        buffer.add(mock_event(100), Some(95));
        buffer.add(mock_event(101), Some(95));
        buffer.add(mock_event(102), Some(95));

        assert_eq!(buffer.pending(), 3);

        let confirmed = buffer.advance_finalized(100).unwrap();
        assert_eq!(confirmed.len(), 1);
        assert_eq!(buffer.pending(), 2);

        let confirmed = buffer.advance_finalized(102).unwrap();
        assert_eq!(confirmed.len(), 2);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_external_duplicate_filtering() {
        let buffer = FinalityBuffer::new(FinalityMode::External);

        let event1 = mock_event(100);
        let event2 = mock_event(100);

        let result1 = buffer.add(event1, Some(95));
        assert_eq!(result1.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);

        let result2 = buffer.add(event2, Some(95));
        assert_eq!(result2.confirmed.len(), 0);
        assert_eq!(buffer.pending(), 1);
    }

    #[test]
    fn test_flush_returns_all_events() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(10));

        buffer.add(mock_event(100), None);
        buffer.add(mock_event(101), None);
        buffer.add(mock_event(102), None);

        let flushed = buffer.flush();
        assert_eq!(flushed.len(), 3);
        assert_eq!(buffer.pending(), 0);
    }

    #[test]
    fn test_external_enforces_event_limit_on_burst() {
        let buffer = FinalityBuffer::new(FinalityMode::External).with_event_limit(5);

        // Add 10 events across 10 blocks, all unfinalized
        for i in 100..110 {
            buffer.add(mock_event(i), Some(95));
        }

        // Buffer should have evicted to stay under the 5-event limit
        let pending = buffer.pending();
        assert!(
            pending <= 5,
            "Expected pending <= 5 after burst, got {}",
            pending
        );
    }

    #[test]
    fn test_external_enforces_block_limit_on_burst() {
        let buffer = FinalityBuffer::new(FinalityMode::External).with_limits(3, 100_000);

        // Add 10 events across 10 blocks, all unfinalized
        for i in 100..110 {
            buffer.add(mock_event(i), Some(95));
        }

        // Buffer should have evicted to stay under the 3-block limit
        let inner = buffer.inner.lock().unwrap();
        let pending_blocks = inner.pending.len();
        assert!(
            pending_blocks <= 3,
            "Expected pending blocks <= 3 after burst, got {}",
            pending_blocks
        );
    }

    #[test]
    fn test_external_enforces_both_limits() {
        let buffer = FinalityBuffer::new(FinalityMode::External).with_limits(2, 3);

        // Add 5 events across 5 blocks
        for i in 100..105 {
            buffer.add(mock_event(i), Some(95));
        }

        let inner = buffer.inner.lock().unwrap();
        let pending_blocks = inner.pending.len();
        let pending_events: usize = inner.pending.values().map(|v| v.len()).sum();

        assert!(
            pending_blocks <= 2,
            "Expected pending blocks <= 2, got {}",
            pending_blocks
        );
        assert!(
            pending_events <= 3,
            "Expected pending events <= 3, got {}",
            pending_events
        );
    }

    #[test]
    fn test_depth_enforces_event_limit_on_burst() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(10)).with_event_limit(5);

        // Add 10 events across 10 blocks
        for i in 100..110 {
            buffer.add(mock_event(i), None);
        }

        let pending = buffer.pending();
        assert!(
            pending <= 5,
            "Expected pending <= 5 after burst in depth mode, got {}",
            pending
        );
    }

    #[test]
    fn test_depth_enforces_both_limits() {
        let buffer = FinalityBuffer::new(FinalityMode::Depth(10)).with_limits(2, 3);

        // Add 5 events across 5 blocks
        for i in 100..105 {
            buffer.add(mock_event(i), None);
        }

        let inner = buffer.inner.lock().unwrap();
        let pending_blocks = inner.pending.len();
        let pending_events: usize = inner.pending.values().map(|v| v.len()).sum();

        assert!(
            pending_blocks <= 2,
            "Expected pending blocks <= 2 in depth mode, got {}",
            pending_blocks
        );
        assert!(
            pending_events <= 3,
            "Expected pending events <= 3 in depth mode, got {}",
            pending_events
        );
    }
}
