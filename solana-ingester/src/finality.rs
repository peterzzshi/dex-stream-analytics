use crate::events::Event;
use std::collections::HashMap;
use tracing::{debug, warn};

const DEFAULT_BUFFER_SIZE_LIMIT: usize = 500;

pub struct FinalityBuffer {
    commitment_level: String,
    pending: HashMap<u64, Vec<Event>>,
    highest_slot: u64,
    size_limit: usize,
}

pub struct AddResult {
    pub confirmed: Vec<Event>,
    pub evicted: Vec<Event>,
}

impl FinalityBuffer {
    pub fn new(commitment_level: String) -> Self {
        Self {
            commitment_level,
            pending: HashMap::new(),
            highest_slot: 0,
            size_limit: DEFAULT_BUFFER_SIZE_LIMIT,
        }
    }

    /// Add event to buffer
    /// Returns immediately confirmed events (if buffer is below finalized slot)
    /// and any evicted events (if buffer size limit exceeded)
    pub fn add(&mut self, event: Event, finalized_slot: u64) -> AddResult {
        let slot = event.slot() as u64;

        // Update highest slot seen
        if slot > self.highest_slot {
            self.highest_slot = slot;
        }

        // If event is already finalized, return immediately
        if slot <= finalized_slot {
            debug!("Event at slot {} already finalized (current: {}), confirming immediately", slot, finalized_slot);
            return AddResult {
                confirmed: vec![event],
                evicted: vec![],
            };
        }

        // Add to pending buffer
        self.pending.entry(slot).or_insert_with(Vec::new).push(event);

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
        let mut confirmed = Vec::new();

        // Collect all slots <= finalized_slot
        let slots_to_release: Vec<u64> = self.pending
            .keys()
            .filter(|&&slot| slot <= finalized_slot)
            .copied()
            .collect();

        for slot in slots_to_release {
            if let Some(events) = self.pending.remove(&slot) {
                debug!("Releasing {} events from slot {}", events.len(), slot);
                confirmed.extend(events);
            }
        }

        confirmed
    }

    /// Flush all pending events (graceful shutdown)
    pub fn flush(&mut self) -> Vec<Event> {
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
                warn!("Evicted {} events from slot {} (oldest)", evicted.len(), slot);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BaseEvent, Event, SwapEvent};

    fn mock_swap_event(slot: i64) -> Event {
        Event::Swap(SwapEvent {
            base: BaseEvent {
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
        let mut buffer = FinalityBuffer::new("finalized".to_string());

        let event = mock_swap_event(100);
        let result = buffer.add(event, 150);  // Event at 100, finalized at 150

        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.evicted.len(), 0);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn test_buffering_and_release() {
        let mut buffer = FinalityBuffer::new("finalized".to_string());

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
        let mut buffer = FinalityBuffer::new("finalized".to_string());

        buffer.add(mock_swap_event(100), 95);
        buffer.add(mock_swap_event(101), 95);

        let flushed = buffer.flush();
        assert_eq!(flushed.len(), 2);
        assert_eq!(buffer.pending_count(), 0);
    }
}
