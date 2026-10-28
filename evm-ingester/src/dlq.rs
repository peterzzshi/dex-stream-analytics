use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use tracing::{error, info};

use crate::events::Event;

const DEFAULT_DLQ_DIR: &str = "./dlq";
const DEFAULT_FLUSH_SIZE: usize = 100;
const DEFAULT_FLUSH_PERIOD_SECS: u64 = 5;

/// FileSink writes DLQ events to disk as newline-delimited JSON.
/// Events are batched and flushed periodically or when batch size is reached.
pub struct FileSink {
    buffer: Arc<Mutex<Buffer>>,
    flush_size: usize,
    flush_period: Duration,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

struct Buffer {
    file: std::fs::File,
    events: Vec<String>,
}

impl FileSink {
    /// Creates a DLQ file sink that writes to the specified directory.
    /// Events are written to a timestamped file in newline-delimited JSON format.
    pub fn new(dir: Option<String>) -> Result<Self> {
        let dir = dir.unwrap_or_else(|| DEFAULT_DLQ_DIR.to_string());

        fs::create_dir_all(&dir)?;

        let filename = format!("dlq_{}.jsonl", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let filepath = Path::new(&dir).join(filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filepath)?;

        info!("Created DLQ file sink at {}", filepath.display());

        let buffer = Arc::new(Mutex::new(Buffer {
            file,
            events: Vec::with_capacity(DEFAULT_FLUSH_SIZE),
        }));

        Ok(Self {
            buffer,
            flush_size: DEFAULT_FLUSH_SIZE,
            flush_period: Duration::from_secs(DEFAULT_FLUSH_PERIOD_SECS),
            shutdown_tx: None,
        })
    }

    /// Starts the background periodic flusher.
    pub fn start_flusher(&mut self) {
        let (tx, mut rx) = mpsc::channel(1);
        self.shutdown_tx = Some(tx);

        let buffer = Arc::clone(&self.buffer);
        let flush_period = self.flush_period;

        tokio::spawn(async move {
            let mut interval = time::interval(flush_period);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::flush_buffer(&buffer) {
                            error!("Periodic DLQ flush failed: {}", e);
                        }
                    }
                    _ = rx.recv() => {
                        break;
                    }
                }
            }
        });
    }

    /// Writes an event to the buffer. Returns immediately; actual disk write is async.
    pub fn write(&self, event: &dyn Event) -> Result<()> {
        let mut buffer = self.buffer.lock().unwrap();

        let event_map = event.to_map();
        let json = serde_json::to_string(&event_map)?;
        buffer.events.push(json);

        if buffer.events.len() >= self.flush_size {
            Self::flush_buffer_locked(&mut buffer)?;
        }

        Ok(())
    }

    /// Flushes all buffered events to disk.
    pub fn flush(&self) -> Result<()> {
        Self::flush_buffer(&self.buffer)
    }

    fn flush_buffer(buffer: &Arc<Mutex<Buffer>>) -> Result<()> {
        let mut buffer = buffer.lock().unwrap();
        Self::flush_buffer_locked(&mut buffer)
    }

    fn flush_buffer_locked(buffer: &mut Buffer) -> Result<()> {
        if buffer.events.is_empty() {
            return Ok(());
        }

        for event_json in &buffer.events {
            writeln!(buffer.file, "{}", event_json)?;
        }

        buffer.file.sync_all()?;

        info!("Flushed {} DLQ events to disk", buffer.events.len());

        buffer.events.clear();
        Ok(())
    }

    /// Closes the sink, flushing remaining events.
    pub async fn close(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        self.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BaseEvent, EventType, SwapEvent};
    use tempfile::TempDir;

    fn mock_event() -> SwapEvent {
        SwapEvent {
            base: BaseEvent {
                event_type: EventType::Swap,
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
                token0_symbol: None,
                token1_symbol: None,
                event_timestamp: 1000000,
            },
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
        }
    }

    #[test]
    fn test_dlq_write_and_flush() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let sink = FileSink::new(Some(temp_dir.path().to_str().unwrap().to_string()))?;

        let event = mock_event();
        sink.write(&event)?;

        sink.flush()?;

        // Check file was created and contains data
        let files: Vec<_> = fs::read_dir(temp_dir.path())?.collect();
        assert_eq!(files.len(), 1);

        Ok(())
    }
}
