use crate::{Event, FinalityBuffer, Publisher};
use std::sync::Arc;
use tracing::{info, warn};

/// Owns the buffer-then-publish boundary shared by both ingesters: everything that
/// touches the finality buffer or the publisher goes through here, so the stream loops
/// stay about streaming.
pub struct EventProcessor {
    buffer: Arc<FinalityBuffer>,
    publisher: Arc<Publisher>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProcessOutcome {
    pub published: usize,
    pub dead_lettered: usize,
}

impl EventProcessor {
    pub fn new(buffer: Arc<FinalityBuffer>, publisher: Arc<Publisher>) -> Self {
        Self { buffer, publisher }
    }

    /// Buffers `event`, publishing whatever it confirms and dead-lettering whatever it evicts.
    /// `publish` already dead-letters its own failures, so a broken sidecar costs events but
    /// never stalls the stream.
    pub async fn process(
        &self,
        event: Event,
        finalized_slot: Option<u64>,
        pool_name: &str,
    ) -> ProcessOutcome {
        let result = self.buffer.add(event, finalized_slot);

        for confirmed in &result.confirmed {
            info!(pool = %pool_name, event_type = confirmed.event_type(), "Published confirmed event");
            self.publisher.publish(confirmed).await;
        }

        if !result.evicted.is_empty() {
            warn!(
                pool = %pool_name,
                evicted = result.evicted.len(),
                "Finality buffer overflow, dead-lettering events"
            );
            for evicted in &result.evicted {
                self.publisher
                    .dead_letter(evicted, "finality buffer overflow")
                    .await;
            }
        }

        ProcessOutcome {
            published: result.confirmed.len(),
            dead_lettered: result.evicted.len(),
        }
    }

    /// Releases everything at or below `finalized_slot`. `Err` means the buffer is not in
    /// external-finality mode, which is a configuration error rather than a data error.
    pub async fn advance_finalized(
        &self,
        finalized_slot: u64,
        pool_name: &str,
    ) -> Result<ProcessOutcome, String> {
        let released = self.buffer.advance_finalized(finalized_slot)?;

        if released.is_empty() {
            return Ok(ProcessOutcome::default());
        }

        info!(
            pool = %pool_name,
            released = released.len(),
            slot = finalized_slot,
            "Finality poll released events"
        );

        for event in &released {
            self.publisher.publish(event).await;
        }

        Ok(ProcessOutcome {
            published: released.len(),
            dead_lettered: 0,
        })
    }

    /// Publishes everything still buffered, for shutdown.
    pub async fn flush(&self, pool_name: &str) -> usize {
        let pending = self.buffer.flush();
        info!(pool = %pool_name, pending = pending.len(), "Flushing pending events");
        for event in &pending {
            self.publisher.publish(event).await;
        }
        pending.len()
    }
}
