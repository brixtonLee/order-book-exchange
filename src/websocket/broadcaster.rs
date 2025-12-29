 use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::messages::WsMessage;

// WebSocket broadcaster for pub/sub pattern
#[derive(Clone)]
pub struct Broadcaster {
    /// Channel subscriptions per topic (e.g., "orderbook:AAPL", "trades:AAPL", "ticker:AAPL")
    /// 
    /// It's optimized for concurrent workloads and generally performs better than a mutex-wrapped HashMap when you have multiple threads accessing the map.
    channels: Arc<DashMap<String, broadcast::Sender<WsMessage>>>,
    /// Default channel capacity
    capacity: usize,
}

impl Broadcaster {
    /// Create a new broadcaster with default capacity
    pub fn new() -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            capacity: 1000,
        }
    }

    /// Create a new broadcaster with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            capacity,
        }
    }

    /// Get or create a channel for a topic
    fn get_or_create_channel(&self, topic: &str) -> broadcast::Sender<WsMessage> {
        self.channels
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(self.capacity).0)
            .clone()
    }

    /// Subscribe to a topic
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<WsMessage> {
        let sender = self.get_or_create_channel(topic);
        sender.subscribe()
    }

    /// Broadcast a message to a topic
    pub fn broadcast(&self, topic: &str, message: WsMessage) {
        if let Some(sender) = self.channels.get(topic) {
            // Ignore if no subscribers
            let _ = sender.send(message);
        }
    }

    /// Broadcast to multiple topics
    pub fn broadcast_multi(&self, topics: &[String], message: WsMessage) {
        for topic in topics {
            self.broadcast(topic, message.clone());
        }
    }

    /// Get subscriber count for a topic
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.channels
            .get(topic)
            .map(|sender| sender.receiver_count())
            .unwrap_or(0)
    }

    /// Remove a channel if it has no subscribers
    pub fn cleanup_empty_channels(&self) {
        self.channels.retain(|_, sender| sender.receiver_count() > 0);
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for topic naming
pub mod topics {
    pub fn orderbook(symbol: &str) -> String {
        format!("orderbook:{}", symbol)
    }

    pub fn trades(symbol: &str) -> String {
        format!("trades:{}", symbol)
    }

    pub fn ticker(symbol: &str) -> String {
        format!("ticker:{}", symbol)
    }

    pub fn all_trades() -> &'static str {
        "trades:*"
    }
}
