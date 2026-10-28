use crate::events::Event;
use std::collections::HashMap;
use tracing::{debug, warn};

const DEFAULT_BUFFER_SIZE_LIMIT: usize = 500;
const DEDUP_RETENTION_SLOTS: u64 = 100;

pub struct FinalityBuffer {
    commitment_level: String,
    pending: HashMap<u64, Vec<Event>>,
    seen_events: HashMap<String, u64>,
    duplicates_filtered: usize,
    highest_slot: u64,
    size_limit: usize,
    last_finalized_slot: u64,
    events_released: usize,
    events_evicted: usize,
}

pub struct AddResult {
    pub confirmed: Vec<Event>,
    pub evicted: Vec<Event>,
}

impl FinalityBuffer {
    pub fn new(commitment_level: String, size_limit: usize) -> Self {
        Self {
            commitment_level,
            pending: HashMap::new(),
            seen_events: HashMap::new(),
            duplicates_filtered: 0,
            highest_slot: 0,
            size_limit,
            last_finalized_slot: 0,
            events_released: 0,
            events_evicted: 0,
        }
    }

    /// Add event to buffer
    /// Returns immediately confirmed events (if buffer is below finalized slot)
    /// and any evicted events (if buffer size limit exceeded)
    pub fn add(&mut self, event: Event, finalized_slot: u64) -> AddResult {
        let slot = event.slot() as u64;

        let event_id = self.get_event_id(&event);

        // Check for duplicate
        if self.seen_events.contains_key(&event_id) {
            self.duplicates_filtered += 1;
            debug!("Filtered duplicate event {} at slot {}", event_id, slot);
            return AddResult::default();
        }
        self.seen_events.insert(event_id, slot);

        // Update highest slot seen
        if slot > self.highest_slot {
            self.highest_slot = slot;
        }

        // If event is already finalized, return immediately
        if slot <= finalized_slot {
            debug!(
                "Event at slot {} already finalized (current: {}), confirming immediately",
                slot, finalized_slot
            );
            return AddResult {
                confirmed: vec![event],
                evicted: vec![],
            };
        }

        // Add to pending buffer
        self.pending
            .entry(slot)
            .or_insert_with(Vec::new)
            .push(event);

        // Check size limit
        let total_events: usize = self.pending.values().map(|v| v.len()).sum();
        if total_events > self.size_limit {
            warn!(
                "Buffer size limit exceeded: {} events (limit: {}), evicting oldest",
                total_events, self.size_limit
            );
            return self.evict_oldest();
        }

        AddResult {
            confirmed: vec![],
            evicted: vec![],
        }
    }

    /// Advance finalized slot and release all events <= slot
    pub fn advance_finalized_slot(&mut self, finalized_slot: u64) -> Vec<Event> {
        // Detect reorg (finalized slot should never go backward)
        if finalized_slot < self.last_finalized_slot {
            warn!(
                "Potential reorg detected: finalized slot went backward from {} to {}",
                self.last_finalized_slot, finalized_slot
            );
        }

        // Skip if no progress
        if finalized_slot <= self.last_finalized_slot {
            return vec![];
        }

        let mut confirmed = Vec::new();

        // Clean up old event IDs (keep retention window beyond finalized slot)
        if finalized_slot > DEDUP_RETENTION_SLOTS {
            let cleanup_threshold = finalized_slot - DEDUP_RETENTION_SLOTS;
            self.seen_events
                .retain(|_, &mut event_slot| event_slot > cleanup_threshold);
            debug!(
                "Cleaned up dedup cache, {} entries remain",
                self.seen_events.len()
            );
        }

        // Release finalized events
        let slots_to_release: Vec<u64> = self
            .pending
            .keys()
            .filter(|&&slot| slot <= finalized_slot)
            .copied()
            .collect();

        for slot in slots_to_release {
            if let Some(events) = self.pending.remove(&slot) {
                let count = events.len();
                debug!("Releasing {} events from slot {}", count, slot);
                self.events_released += count;
                confirmed.extend(events);
            }
        }

        self.last_finalized_slot = finalized_slot;

        // Log buffer metrics periodically
        if !confirmed.is_empty() {
            let pending_count = self.pending_count();
            let slot_lag = self.highest_slot.saturating_sub(finalized_slot);
            debug!(
                "Buffer metrics: pending={}, slot_lag={}, released_total={}, evicted_total={}, duplicates_filtered={}",
                pending_count,
                slot_lag,
                self.events_released,
                self.events_evicted,
                self.duplicates_filtered
            );
        }

        confirmed
    }

    /// Flush all pending events (graceful shutdown)
    pub fn flush(&mut self) -> Vec<Event> {
        debug!(
            "Flushing buffer: {} duplicates filtered total",
            self.duplicates_filtered
        );

        let mut all_events = Vec::new();

        for (slot, events) in self.pending.drain() {
            debug!("Flushing {} events from slot {}", events.len(), slot);
            all_events.extend(events);
        }

        all_events
    }

    /// Evict oldest slot's events when buffer size limit exceeded
    fn evict_oldest(&mut self) -> AddResult {
        let oldest_slot = self.pending.keys().min().copied();

        match oldest_slot {
            Some(slot) => {
                let evicted = self.pending.remove(&slot).unwrap_or_default();
                let count = evicted.len();
                self.events_evicted += count;
                warn!(
                    "Evicted {} events from slot {} (oldest), total_evicted={}",
                    count, slot, self.events_evicted
                );

                AddResult {
                    confirmed: vec![],
                    evicted,
                }
            }
            None => AddResult {
                confirmed: vec![],
                evicted: vec![],
            },
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending.values().map(|v| v.len()).sum()
    }

    pub fn duplicates_filtered(&self) -> usize {
        self.duplicates_filtered
    }

    pub fn slot_lag(&self) -> u64 {
        self.highest_slot.saturating_sub(self.last_finalized_slot)
    }

    pub fn last_finalized_slot(&self) -> u64 {
        self.last_finalized_slot
    }

    fn get_event_id(&self, event: &Event) -> String {
        match event {
            Event::Swap(e) => e.base.event_id.clone(),
            Event::Mint(e) => e.base.event_id.clone(),
            Event::Burn(e) => e.base.event_id.clone(),
        }
    }
}

impl Default for AddResult {
    fn default() -> Self {
        Self {
            confirmed: vec![],
            evicted: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BaseEvent, Event, SwapEvent};

    fn mock_swap_event(slot: i64) -> Event {
        Event::Swap(SwapEvent {
            base: BaseEvent {
                event_type: "Swap".to_string(),
                event_id: format!("test:{}", slot),
                block_number: slot,
                block_timestamp: 1000000,
                transaction_hash: "test_tx".to_string(),
                log_index: 0,
                chain: "solana:test".to_string(),
                chain_name: "Solana".to_string(),
                pair_address: "test_pair".to_string(),
                token0: "token0".to_string(),
                token1: "token1".to_string(),
                token0_symbol: Some("TK0".to_string()),
                token1_symbol: Some("TK1".to_string()),
                event_timestamp: 1000000,
            },
            sender: "sender".to_string(),
            recipient: "recipient".to_string(),
            amount0_in: "1000".to_string(),
            amount1_in: "0".to_string(),
            amount0_out: "0".to_string(),
            amount1_out: "990".to_string(),
            price: 0.99,
            volume_usd: Some(1000.0),
            gas_used: 5000,
            gas_price: "1".to_string(),
        })
    }

    #[test]
    fn test_immediate_confirmation() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        let event = mock_swap_event(100);
        let result = buffer.add(event, 150); // Event at 100, finalized at 150

        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.evicted.len(), 0);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn test_buffering_and_release() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        // Add events at slots 100, 101, 102
        buffer.add(mock_swap_event(100), 95);
        buffer.add(mock_swap_event(101), 95);
        buffer.add(mock_swap_event(102), 95);

        assert_eq!(buffer.pending_count(), 3);

        // Advance to slot 100
        let confirmed = buffer.advance_finalized_slot(100);
        assert_eq!(confirmed.len(), 1);
        assert_eq!(buffer.pending_count(), 2);

        // Advance to slot 102
        let confirmed = buffer.advance_finalized_slot(102);
        assert_eq!(confirmed.len(), 2);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn test_flush() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        buffer.add(mock_swap_event(100), 95);
        buffer.add(mock_swap_event(101), 95);

        let flushed = buffer.flush();
        assert_eq!(flushed.len(), 2);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn test_duplicate_filtering() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        let event1 = mock_swap_event(100);
        let event2 = mock_swap_event(100); // Same event_id

        // Add first event
        let result1 = buffer.add(event1, 95);
        assert_eq!(result1.confirmed.len(), 0);
        assert_eq!(buffer.pending_count(), 1);
        assert_eq!(buffer.duplicates_filtered(), 0);

        // Add duplicate - should be filtered
        let result2 = buffer.add(event2, 95);
        assert_eq!(result2.confirmed.len(), 0);
        assert_eq!(buffer.pending_count(), 1); // Still only 1 event
        assert_eq!(buffer.duplicates_filtered(), 1);
    }

    #[test]
    fn test_dedup_cache_cleanup() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        // Add events at slots 100, 101, 102
        buffer.add(mock_swap_event(100), 95);
        buffer.add(mock_swap_event(101), 95);
        buffer.add(mock_swap_event(102), 95);

        // Advance to slot 250 (triggers cleanup of events before slot 150)
        // cleanup_threshold = 250 - 100 = 150, so slot 100 events are cleaned up
        buffer.advance_finalized_slot(250);

        // Try to add duplicate of slot 100 event - should NOT be filtered
        // because it was cleaned up (slot 100 < threshold 150)
        let duplicate = mock_swap_event(100);
        let result = buffer.add(duplicate, 95);
        assert_eq!(result.confirmed.len(), 0);
        assert_eq!(buffer.duplicates_filtered(), 0); // Not filtered, was cleaned up
    }

    #[test]
    fn test_reorg_detection() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        buffer.add(mock_swap_event(100), 95);
        buffer.advance_finalized_slot(100);

        // Try to advance backward (simulating reorg) - should warn but not release events
        let released = buffer.advance_finalized_slot(90);
        assert_eq!(released.len(), 0);
        assert_eq!(buffer.last_finalized_slot(), 100); // Should stay at 100
    }

    #[test]
    fn test_slot_lag_metric() {
        let mut buffer = FinalityBuffer::new("finalized".to_string(), 1000);

        buffer.add(mock_swap_event(100), 95);
        buffer.add(mock_swap_event(105), 95);

        assert_eq!(buffer.slot_lag(), 105); // highest_slot=105, last_finalized=0

        buffer.advance_finalized_slot(100);
        assert_eq!(buffer.slot_lag(), 5); // highest_slot=105, last_finalized=100
    }
}
