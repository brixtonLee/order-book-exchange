use crate::ctrader_fix::market_data::MarketTick;
use crate::database::models::NewTick;
use crate::database::repositories::TickRepository;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Tick persister with batching for high-throughput writes
///
/// Buffers ticks in memory and flushes to database either when:
/// - Buffer reaches max size (e.g., 1000 ticks)
/// - Time interval elapsed (e.g., 100ms)
pub struct TickPersister {
    tick_repository: Arc<dyn TickRepository>,
    batch_size: usize,
    flush_interval_ms: u64,
}

impl TickPersister {
    /// Create new tick persister
    ///
    /// # Arguments
    /// * `tick_repository` - Repository for database operations
    /// * `batch_size` - Maximum ticks to buffer before flushing (default: 1000)
    /// * `flush_interval_ms` - Milliseconds between flushes (default: 100ms)
    pub fn new(
        tick_repository: Arc<dyn TickRepository>,
        batch_size: usize,
        flush_interval_ms: u64,
    ) -> Self {
        Self {
            tick_repository,
            batch_size,
            flush_interval_ms,
        }
    }

    /// Start tick persistence background task
    ///
    /// Returns a channel sender for submitting ticks
    pub fn start(self) -> mpsc::UnboundedSender<MarketTick> {
        let (tx, mut rx) = mpsc::unbounded_channel::<MarketTick>();

        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(self.batch_size);
            let mut flush_timer = interval(Duration::from_millis(self.flush_interval_ms));

            loop {
                tokio::select! {
                    // Receive ticks from channel
                    Some(market_tick) = rx.recv() => {
                        if let Some(new_tick) = Self::convert_market_tick_to_new_tick(&market_tick) {
                            buffer.push(new_tick);

                            // Flush when buffer is full
                            if buffer.len() >= self.batch_size {
                                self.flush_buffer(&mut buffer).await;
                            }
                        }
                    }

                    // Periodic flush timer
                    _ = flush_timer.tick() => {
                        if !buffer.is_empty() {
                            self.flush_buffer(&mut buffer).await;
                        }
                    }
                }
            }
        });

        tx
    }

    /// Flush buffer to database
    async fn flush_buffer(&self, buffer: &mut Vec<NewTick>) {
        if buffer.is_empty() {
            return;
        }

        let tick_count = buffer.len();

        match self.tick_repository.insert_batch(buffer.clone()) {
            Ok(inserted) => {
                tracing::debug!(
                    "Flushed {} ticks to database ({} inserted, {} duplicates)",
                    tick_count,
                    inserted,
                    tick_count - inserted
                );
            }
            Err(e) => {
                tracing::error!("Failed to flush ticks to database: {}", e);
                // Consider implementing retry logic or dead-letter queue here
            }
        }

        buffer.clear();
    }

    /// Convert MarketTick to NewTick for database insertion
    fn convert_market_tick_to_new_tick(market_tick: &MarketTick) -> Option<NewTick> {
        // Parse symbol_id from string to i64
        let symbol_id = market_tick.symbol_id.parse::<i64>().ok()?;

        // Get prices from MarketTick (already Decimal type)
        let bid_price = market_tick.bid_price?;
        let ask_price = market_tick.ask_price?;

        // Use zero for volumes since MarketTick doesn't have volume data
        let bid_volume = Decimal::ZERO;
        let ask_volume = Decimal::ZERO;

        // Use symbol_id as symbol_name for now (will be enriched later via symbol mapping)
        // In production, you'd look this up from DatasourceManager.get_symbol_name()
        let symbol_name = format!("SYM_{}", symbol_id);

        Some(NewTick::new(
            symbol_id,
            symbol_name,
            market_tick.timestamp,
            bid_price,
            ask_price,
            bid_volume,
            ask_volume,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_market_tick() {
        use rust_decimal_macros::dec;

        let market_tick = MarketTick {
            symbol_id: "1".to_string(),
            timestamp: Utc::now(),
            bid_price: Some(dec!(1.1000)),
            ask_price: Some(dec!(1.1005)),
        };

        let new_tick = TickPersister::convert_market_tick_to_new_tick(&market_tick);
        assert!(new_tick.is_some());

        let tick = new_tick.unwrap();
        assert_eq!(tick.symbol_id, 1);
        assert_eq!(tick.bid_price, dec!(1.1000));
        assert_eq!(tick.ask_price, dec!(1.1005));
    }
}
