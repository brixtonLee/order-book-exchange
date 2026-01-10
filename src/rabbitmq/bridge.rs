use std::sync::Arc;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctrader_fix::market_data::MarketTick;
use super::publisher::RabbitMQPublisher;
use super::config::RoutingKeyBuilder;

/// Message format for RabbitMQ market tick
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RabbitMQMarketTick {
    pub symbol_id: String,
    pub symbol_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_price: Option<rust_decimal::Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_price: Option<rust_decimal::Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_size: Option<rust_decimal::Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_size: Option<rust_decimal::Decimal>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<MarketTick> for RabbitMQMarketTick {
    fn from(tick: MarketTick) -> Self {
        Self {
            symbol_id: tick.symbol_id.clone(),
            symbol_name: tick.symbol_id.clone(), // Will be updated by bridge
            bid_price: tick.bid_price,
            ask_price: tick.ask_price,
            bid_size: tick.bid_size,
            ask_size: tick.ask_size,
            timestamp: tick.timestamp,
        }
    }
}

/// Bridge that streams FIX market ticks to RabbitMQ
pub struct FixToRabbitMQBridge {
    publisher: Arc<RabbitMQPublisher>,
    /// Symbol ID mapping (cTrader ID -> human readable symbol)
    symbol_map: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
    /// Statistics
    ticks_processed: Arc<std::sync::atomic::AtomicU64>,
    ticks_failed: Arc<std::sync::atomic::AtomicU64>,
}

impl FixToRabbitMQBridge {
    /// Create a new FIX to RabbitMQ bridge
    pub fn new(publisher: Arc<RabbitMQPublisher>) -> Self {
        Self {
            publisher,
            symbol_map: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            ticks_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            ticks_failed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get a clone of the symbol map Arc for sharing
    pub fn get_symbol_map(&self) -> Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>> {
        Arc::clone(&self.symbol_map)
    }

    /// Add a custom symbol mapping
    pub async fn add_symbol_mapping(&self, symbol_id: String, symbol_name: String) {
        let mut map = self.symbol_map.write().await;
        map.insert(symbol_id, symbol_name);
    }

    /// Bulk update symbol mappings
    pub async fn update_symbol_mappings(&self, mappings: std::collections::HashMap<String, String>) {
        let mut map = self.symbol_map.write().await;
        for (id, name) in mappings {
            map.insert(id, name);
        }
    }

    /// Get human-readable symbol name
    async fn get_symbol_name(&self, symbol_id: &str) -> String {
        let map = self.symbol_map.read().await;
        map.get(symbol_id)
            .cloned()
            .unwrap_or_else(|| format!("SYM_{}", symbol_id))
    }

    /// Process a single tick and publish to RabbitMQ
    pub async fn process_tick(&self, tick: MarketTick) {
        let symbol_name = self.get_symbol_name(&tick.symbol_id).await;

        // Convert to RabbitMQ message format
        let mut rabbitmq_tick = RabbitMQMarketTick::from(tick);
        rabbitmq_tick.symbol_name = symbol_name.clone();

        // Build routing key: tick.{symbol}
        let routing_key = RoutingKeyBuilder::market_tick(&symbol_name);

        // Publish to RabbitMQ with retry
        match self.publisher.publish_with_retry(&routing_key, &rabbitmq_tick, 3).await {
            Ok(_) => {
                self.ticks_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                tracing::trace!(
                    "Published tick to RabbitMQ: symbol={}, routing_key={}",
                    symbol_name,
                    routing_key
                );
            }
            Err(e) => {
                self.ticks_failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                tracing::error!(
                    "Failed to publish tick to RabbitMQ: symbol={}, error={}",
                    symbol_name,
                    e
                );
            }
        }
    }

    /// Run the bridge - consume ticks from channel and publish to RabbitMQ
    pub async fn run(self, mut tick_receiver: mpsc::UnboundedReceiver<MarketTick>) {
        tracing::info!("🌉 FIX to RabbitMQ bridge started!");
        tracing::info!("   Publishing ticks to RabbitMQ exchange...\n");

        while let Some(tick) = tick_receiver.recv().await {
            self.process_tick(tick).await;
        }

        tracing::info!("🔴 FIX to RabbitMQ bridge stopped");
    }

    /// Get statistics
    pub fn stats(&self) -> BridgeStats {
        BridgeStats {
            ticks_processed: self.ticks_processed.load(std::sync::atomic::Ordering::Relaxed),
            ticks_failed: self.ticks_failed.load(std::sync::atomic::Ordering::Relaxed),
            publisher_stats: self.publisher.stats(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.ticks_processed.store(0, std::sync::atomic::Ordering::Relaxed);
        self.ticks_failed.store(0, std::sync::atomic::Ordering::Relaxed);
        self.publisher.reset_stats();
    }
}

/// Statistics for the RabbitMQ bridge
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BridgeStats {
    pub ticks_processed: u64,
    pub ticks_failed: u64,
    pub publisher_stats: super::publisher::PublisherStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_market_tick_conversion() {
        let mut tick = MarketTick::new("41".to_string());
        tick.bid_price = Some(Decimal::from_str("2650.50").unwrap());
        tick.ask_price = Some(Decimal::from_str("2651.00").unwrap());
        tick.bid_size = Some(Decimal::from_str("1000000").unwrap());
        tick.ask_size = Some(Decimal::from_str("1500000").unwrap());

        let rabbitmq_tick = RabbitMQMarketTick::from(tick);

        assert_eq!(rabbitmq_tick.symbol_id, "41");
        assert_eq!(rabbitmq_tick.bid_price, Some(Decimal::from_str("2650.50").unwrap()));
        assert_eq!(rabbitmq_tick.ask_price, Some(Decimal::from_str("2651.00").unwrap()));
    }

    #[tokio::test]
    async fn test_bridge_symbol_mapping() {
        use super::super::config::RabbitMQConfig;

        let config = RabbitMQConfig::default();
        let publisher = Arc::new(RabbitMQPublisher::new(config));
        let bridge = FixToRabbitMQBridge::new(publisher);

        bridge.add_symbol_mapping("1".to_string(), "EURUSD".to_string()).await;
        bridge.add_symbol_mapping("41".to_string(), "XAUUSD".to_string()).await;

        assert_eq!(bridge.get_symbol_name("1").await, "EURUSD");
        assert_eq!(bridge.get_symbol_name("41").await, "XAUUSD");
        assert_eq!(bridge.get_symbol_name("999").await, "SYM_999");
    }
}
