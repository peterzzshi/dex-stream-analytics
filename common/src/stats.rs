/// Per-pool ingest counters, reset on every reporting interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolStats {
    pub received: u64,
    pub pool_hits: u64,
    pub processed: u64,
    pub published: u64,
    pub dead_lettered: u64,
}

impl PoolStats {
    pub fn add_published(&mut self, count: usize) {
        self.published += count as u64;
    }

    pub fn add_dead_lettered(&mut self, count: usize) {
        self.dead_lettered += count as u64;
    }

    /// Returns the accumulated counters and resets them to zero.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}
