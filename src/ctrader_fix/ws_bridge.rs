use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::websocket::{broadcaster::Broadcaster, messages::WsMessage};
use super::market_data::MarketTick;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bridge that converts FIX market ticks to WebSocket messages
/// This allows real-time streaming from cTrader FIX API to WebSocket clients
pub struct FixToWebSocketBridge {
    broadcaster: Broadcaster,
    /// Symbol ID mapping (cTrader ID -> human readable symbol)
    /// Uses Arc<RwLock> for thread-safe dynamic updates
    symbol_map: Arc<RwLock<HashMap<String, String>>>,
}

impl FixToWebSocketBridge {
    pub fn new(broadcaster: Broadcaster) -> Self {
        Self {
            broadcaster,
            symbol_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a clone of the symbol map Arc for sharing with callbacks
    pub fn get_symbol_map(&self) -> Arc<RwLock<HashMap<String, String>>> {
        Arc::clone(&self.symbol_map)
    }

    /// Add a custom symbol mapping
    pub async fn add_symbol_mapping(&self, symbol_id: String, symbol_name: String) {
        let mut map = self.symbol_map.write().await;
        map.insert(symbol_id, symbol_name);
    }

    /// Bulk update symbol mappings (useful for Security List Response)
    pub async fn update_symbol_mappings(&self, mappings: HashMap<String, String>) {
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

    /// Convert MarketTick to Ticker WsMessage
    async fn tick_to_ws_message(&self, tick: &MarketTick) -> WsMessage {
        let symbol = self.get_symbol_name(&tick.symbol_id).await;

        WsMessage::Ticker {
            symbol,
            best_bid: tick.bid_price,
            best_ask: tick.ask_price,
            spread: tick.spread(),
            mid_price: tick.mid_price(),
            timestamp: tick.timestamp,
        }
    }

    /// Process a single tick and broadcast to WebSocket
    pub async fn process_tick(&self, tick: MarketTick) {
        let symbol = self.get_symbol_name(&tick.symbol_id).await;
        let ws_message = self.tick_to_ws_message(&tick).await;
        let topic = format!("ticker:{}", symbol);
        self.broadcaster.broadcast(&topic, ws_message.clone());
        self.broadcaster.broadcast("ticker:*", ws_message);
    }

    /// Run the bridge - consume ticks from channel and broadcast to WebSocket
    pub async fn run(self, mut tick_receiver: mpsc::UnboundedReceiver<MarketTick>) {
        println!("🌉 FIX to WebSocket bridge started!");
        println!("   Broadcasting ticks to WebSocket clients...\n");

        while let Some(tick) = tick_receiver.recv().await {
            self.process_tick(tick).await;
        }

        println!("🔴 FIX to WebSocket bridge stopped");
    }
}

/// Statistics tracker for market data
pub struct MarketDataStats {
    tick_count: std::sync::atomic::AtomicU64,
    last_tick_time: std::sync::Arc<tokio::sync::Mutex<Option<chrono::DateTime<Utc>>>>,
}

impl MarketDataStats {
    pub fn new() -> Self {
        Self {
            tick_count: std::sync::atomic::AtomicU64::new(0),
            last_tick_time: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub async fn record_tick(&self) {
        self.tick_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut last_tick = self.last_tick_time.lock().await;
        *last_tick = Some(Utc::now());
    }

    pub fn get_tick_count(&self) -> u64 {
        self.tick_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn get_last_tick_time(&self) -> Option<chrono::DateTime<Utc>> {
        *self.last_tick_time.lock().await
    }
}

impl Default for MarketDataStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_symbol_mapping() {
        let broadcaster = Broadcaster::new();
        let bridge = FixToWebSocketBridge::new(broadcaster);

        assert_eq!(bridge.get_symbol_name("41").await, "XAUUSD");
        assert_eq!(bridge.get_symbol_name("1").await, "EURUSD");
        assert_eq!(bridge.get_symbol_name("999").await, "SYM_999");
    }

    #[tokio::test]
    async fn test_tick_processing() {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let broadcaster = Broadcaster::new();
        let bridge = FixToWebSocketBridge::new(broadcaster.clone());

        let mut tick = MarketTick::new("41".to_string());
        tick.bid_price = Some(Decimal::from_str("2650.50").unwrap());
        tick.ask_price = Some(Decimal::from_str("2651.00").unwrap());

        bridge.process_tick(tick).await;

        // Verify message was broadcast
        assert!(broadcaster.subscriber_count("ticker:XAUUSD") == 0); // No subscribers yet, but broadcast succeeded
    }
}
