use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

use crate::ctrader_fix::market_data::MarketTick;
use super::models::NewTick;

/// Statistics for the tick queue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickQueueStats {
    pub current_size: usize,
    pub max_size: usize,
    pub total_enqueued: u64,
    pub total_flushed: u64,
    pub emergency_flushes: u64,
    pub is_flushing: bool,
}

/// Bounded tick queue for buffering market data before database persistence
///
/// Features:
/// - Bounded memory: max 500k ticks (~100 MB)
/// - Thread-safe: concurrent enqueue from multiple producers
/// - Emergency flush: automatic flush when queue is full
/// - Statistics tracking: monitor queue health
pub struct TickQueue {
    /// Internal queue storage
    queue: Arc<RwLock<VecDeque<NewTick>>>,

    /// Maximum queue size
    max_size: usize,

    /// Total ticks enqueued (lifetime)
    total_enqueued: Arc<AtomicU64>,

    /// Total ticks flushed (lifetime)
    total_flushed: Arc<AtomicU64>,

    /// Emergency flush counter
    emergency_flushes: Arc<AtomicU64>,

    /// Is currently flushing (prevents concurrent emergency flushes)
    is_flushing: Arc<AtomicBool>,

    /// Emergency flush trigger (oneshot channel)
    emergency_flush_tx: Arc<RwLock<Option<mpsc::UnboundedSender<()>>>>,
}

impl TickQueue {
    /// Create a new tick queue with specified max size
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_size.min(1000)))),
            max_size,
            total_enqueued: Arc::new(AtomicU64::new(0)),
            total_flushed: Arc::new(AtomicU64::new(0)),
            emergency_flushes: Arc::new(AtomicU64::new(0)),
            is_flushing: Arc::new(AtomicBool::new(false)),
            emergency_flush_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new tick queue with default size from environment or 500k
    pub fn with_env_config() -> Self {
        let max_size = std::env::var("TICK_QUEUE_MAX_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500_000);

        tracing::info!("📦 Tick Queue initialized: max_size={}", max_size);
        Self::new(max_size)
    }

    /// Set the emergency flush trigger channel
    pub fn set_emergency_flush_trigger(&self, tx: mpsc::UnboundedSender<()>) {
        *self.emergency_flush_tx.write() = Some(tx);
    }

    /// Enqueue a market tick
    ///
    /// If queue is full, triggers emergency flush
    pub fn enqueue(&self, tick: MarketTick) {
        let new_tick = Self::convert_market_tick_to_new_tick(tick);

        let mut queue = self.queue.write();

        // Check if queue is full
        if queue.len() >= self.max_size {
            drop(queue); // Release lock before triggering flush

            // Trigger emergency flush
            if !self.is_flushing.swap(true, Ordering::Acquire) {
                self.emergency_flushes.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    "⚠️  Tick queue full ({} ticks), triggering emergency flush",
                    self.max_size
                );

                // Send emergency flush signal
                if let Some(ref tx) = *self.emergency_flush_tx.read() {
                    let _ = tx.send(());
                }
            }

            // Wait a bit and retry (queue should have space after flush)
            // In the worst case, we'll drop the tick
            std::thread::sleep(std::time::Duration::from_millis(10));

            let mut queue = self.queue.write();
            if queue.len() < self.max_size {
                queue.push_back(new_tick);
                self.total_enqueued.fetch_add(1, Ordering::Relaxed);
            } else {
                tracing::error!("❌ Failed to enqueue tick: queue still full after emergency flush");
            }
        } else {
            // Normal enqueue
            queue.push_back(new_tick);
            self.total_enqueued.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Drain all ticks from the queue
    ///
    /// Returns a Vec of all ticks, leaving the queue empty
    pub fn drain_all(&self) -> Vec<NewTick> {
        let mut queue = self.queue.write();
        let ticks: Vec<NewTick> = queue.drain(..).collect();
        let count = ticks.len();

        self.total_flushed.fetch_add(count as u64, Ordering::Relaxed);
        self.is_flushing.store(false, Ordering::Release);

        if count > 0 {
            tracing::debug!("📤 Drained {} ticks from queue", count);
        }

        ticks
    }

    /// Get current queue statistics
    pub fn stats(&self) -> TickQueueStats {
        let queue = self.queue.read();
        TickQueueStats {
            current_size: queue.len(),
            max_size: self.max_size,
            total_enqueued: self.total_enqueued.load(Ordering::Relaxed),
            total_flushed: self.total_flushed.load(Ordering::Relaxed),
            emergency_flushes: self.emergency_flushes.load(Ordering::Relaxed),
            is_flushing: self.is_flushing.load(Ordering::Acquire),
        }
    }

    /// Convert MarketTick to NewTick for database insertion
    fn convert_market_tick_to_new_tick(tick: MarketTick) -> NewTick {
        use rust_decimal::Decimal;

        NewTick {
            symbol_id: tick.symbol_id.parse::<i64>().unwrap_or(0),
            symbol_name: tick.symbol_id.clone(), // Will be enriched with real name later
            tick_time: tick.timestamp,
            bid_price: tick.bid_price.unwrap_or(Decimal::ZERO),
            ask_price: tick.ask_price.unwrap_or(Decimal::ZERO),
            bid_volume: Decimal::ZERO, // Not available in MarketTick
            ask_volume: Decimal::ZERO, // Not available in MarketTick
        }
    }
}

impl Clone for TickQueue {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            max_size: self.max_size,
            total_enqueued: Arc::clone(&self.total_enqueued),
            total_flushed: Arc::clone(&self.total_flushed),
            emergency_flushes: Arc::clone(&self.emergency_flushes),
            is_flushing: Arc::clone(&self.is_flushing),
            emergency_flush_tx: Arc::clone(&self.emergency_flush_tx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_tick(symbol_id: &str) -> MarketTick {
        let mut tick = MarketTick::new(symbol_id.to_string());
        tick.bid_price = Some(rust_decimal::Decimal::new(100, 0));
        tick.ask_price = Some(rust_decimal::Decimal::new(101, 0));
        tick.timestamp = Utc::now();
        tick
    }

    #[test]
    fn test_queue_creation() {
        let queue = TickQueue::new(1000);
        let stats = queue.stats();

        assert_eq!(stats.current_size, 0);
        assert_eq!(stats.max_size, 1000);
        assert_eq!(stats.total_enqueued, 0);
        assert_eq!(stats.total_flushed, 0);
    }

    #[test]
    fn test_enqueue_and_drain() {
        let queue = TickQueue::new(10);

        // Enqueue 5 ticks
        for i in 0..5 {
            queue.enqueue(create_test_tick(&i.to_string()));
        }

        let stats = queue.stats();
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.total_enqueued, 5);

        // Drain all
        let ticks = queue.drain_all();
        assert_eq!(ticks.len(), 5);

        let stats = queue.stats();
        assert_eq!(stats.current_size, 0);
        assert_eq!(stats.total_flushed, 5);
    }

    #[test]
    fn test_queue_stats_tracking() {
        let queue = TickQueue::new(100);

        // Enqueue and drain multiple times
        for _ in 0..3 {
            for i in 0..10 {
                queue.enqueue(create_test_tick(&i.to_string()));
            }
            let _ = queue.drain_all();
        }

        let stats = queue.stats();
        assert_eq!(stats.total_enqueued, 30);
        assert_eq!(stats.total_flushed, 30);
        assert_eq!(stats.current_size, 0);
    }
}
