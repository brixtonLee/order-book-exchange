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

        // Parse prices and volumes to Decimal
        let bid_price = Self::parse_decimal(&market_tick.bid_price)?;
        let ask_price = Self::parse_decimal(&market_tick.ask_price)?;

        // Use default volume if not available
        let bid_volume = market_tick
            .bid_size
            .as_ref()
            .and_then(|s| Self::parse_decimal(s))
            .unwrap_or(Decimal::ZERO);

        let ask_volume = market_tick
            .ask_size
            .as_ref()
            .and_then(|s| Self::parse_decimal(s))
            .unwrap_or(Decimal::ZERO);

        Some(NewTick::new(
            symbol_id,
            market_tick.symbol_name.clone(),
            Self::parse_timestamp(&market_tick.timestamp)?,
            bid_price,
            ask_price,
            bid_volume,
            ask_volume,
        ))
    }

    /// Parse string to Decimal
    fn parse_decimal(value: &str) -> Option<Decimal> {
        value.parse::<Decimal>().ok()
    }

    /// Parse timestamp string to DateTime<Utc>
    fn parse_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
        // Try parsing RFC3339 format first
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
            return Some(dt.with_timezone(&Utc));
        }

        // Fallback to Unix timestamp (milliseconds)
        if let Ok(millis) = timestamp.parse::<i64>() {
            return DateTime::from_timestamp_millis(millis);
        }

        // If parsing fails, use current time as fallback
        tracing::warn!("Failed to parse timestamp '{}', using current time", timestamp);
        Some(Utc::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decimal() {
        assert_eq!(
            TickPersister::parse_decimal("1.12345"),
            Some(Decimal::new(112345, 5))
        );
        assert_eq!(TickPersister::parse_decimal("invalid"), None);
    }

    #[test]
    fn test_parse_timestamp() {
        // RFC3339 format
        let result = TickPersister::parse_timestamp("2024-01-10T12:00:00Z");
        assert!(result.is_some());

        // Unix timestamp (milliseconds)
        let result = TickPersister::parse_timestamp("1704888000000");
        assert!(result.is_some());
    }
}
