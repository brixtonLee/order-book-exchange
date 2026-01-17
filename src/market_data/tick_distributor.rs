use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

use crate::ctrader_fix::market_data::MarketTick;

/// Statistics for the tick distributor
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickDistributorStats {
    /// Number of registered consumers
    pub consumer_count: usize,
    /// Total ticks distributed (lifetime)
    pub total_distributed: u64,
    /// List of registered consumer names
    pub consumers: Vec<String>,
}

/// Centralized tick distributor using pub/sub pattern
///
/// The TickDistributor receives market ticks from the FIX client and broadcasts
/// them to all registered consumers (WebSocket, RabbitMQ, TickQueue, etc.).
///
/// Features:
/// - Dynamic consumer registration
/// - Decoupled architecture (consumers don't know about each other)
/// - Centralized metrics and monitoring
/// - Easy to extend with new consumers
///
/// # Example
/// ```ignore
/// // Create distributor
/// let (distributor, tick_tx) = TickDistributor::new();
/// let distributor = Arc::new(distributor);
///
/// // Register consumers
/// let ws_rx = distributor.register_consumer("websocket".to_string());
/// let rmq_rx = distributor.register_consumer("rabbitmq".to_string());
///
/// // Start broadcast loop
/// distributor.start();
///
/// // FIX client sends ticks to tick_tx
/// tick_tx.send(tick).unwrap();
/// ```
pub struct TickDistributor {
    /// Registry of consumers by name
    consumers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<MarketTick>>>>,

    /// Total ticks distributed (lifetime)
    total_distributed: Arc<AtomicU64>,

    /// Receiver for incoming ticks (taken when start() is called)
    tick_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<MarketTick>>>>,

    /// Handle to the broadcast task
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl TickDistributor {
    /// Create a new tick distributor
    ///
    /// Returns (distributor, sender) where sender is used by FIX client to send ticks
    pub fn new() -> (Self, mpsc::UnboundedSender<MarketTick>) {
        let (tick_tx, tick_rx) = mpsc::unbounded_channel();

        let distributor = Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
            total_distributed: Arc::new(AtomicU64::new(0)),
            tick_rx: Arc::new(Mutex::new(Some(tick_rx))),
            task_handle: Arc::new(Mutex::new(None)),
        };

        tracing::info!("📡 TickDistributor created");

        (distributor, tick_tx)
    }

    /// Register a consumer and get its dedicated receiver
    ///
    /// Each consumer gets its own channel to receive ticks.
    /// Call this BEFORE calling start().
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the consumer (e.g., "websocket", "rabbitmq")
    ///
    /// # Returns
    /// Receiver channel that the consumer should listen on
    pub fn register_consumer(&self, name: String) -> mpsc::UnboundedReceiver<MarketTick> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.consumers.write().insert(name.clone(), tx);

        tracing::info!("📡 Consumer registered: {}", name);

        rx
    }

    /// Unregister a consumer
    ///
    /// Removes the consumer from the distribution list.
    pub fn unregister_consumer(&self, name: &str) -> bool {
        let removed = self.consumers.write().remove(name).is_some();

        if removed {
            tracing::info!("📡 Consumer unregistered: {}", name);
        } else {
            tracing::warn!("📡 Attempted to unregister unknown consumer: {}", name);
        }

        removed
    }

    /// Start the broadcast loop
    ///
    /// Spawns a background task that receives ticks and broadcasts them to all consumers.
    /// Can only be called once. Subsequent calls will panic.
    pub fn start(&self) {
        let mut tick_rx = self.tick_rx.lock()
            .take()
            .expect("TickDistributor already started or receiver already taken");

        let consumers = Arc::clone(&self.consumers);
        let total = Arc::clone(&self.total_distributed);

        let handle = tokio::spawn(async move {
            tracing::info!("📡 TickDistributor broadcast loop started");

            while let Some(tick) = tick_rx.recv().await {
                let consumers_guard = consumers.read();
                let consumer_count = consumers_guard.len();

                // Broadcast to all consumers
                for (name, tx) in consumers_guard.iter() {
                    if let Err(e) = tx.send(tick.clone()) {
                        tracing::error!("📡 Failed to send tick to consumer '{}': {}", name, e);
                    }
                }

                // Update stats
                total.fetch_add(consumer_count as u64, Ordering::Relaxed);
            }

            tracing::warn!("📡 TickDistributor broadcast loop ended (sender dropped)");
        });

        *self.task_handle.lock() = Some(handle);

        tracing::info!("📡 TickDistributor started");
    }

    /// Get distributor statistics
    pub fn get_stats(&self) -> TickDistributorStats {
        let consumers_guard = self.consumers.read();

        TickDistributorStats {
            consumer_count: consumers_guard.len(),
            total_distributed: self.total_distributed.load(Ordering::Relaxed),
            consumers: consumers_guard.keys().cloned().collect(),
        }
    }

    /// Check if the distributor is running
    pub fn is_running(&self) -> bool {
        self.task_handle.lock()
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Clone for TickDistributor {
    fn clone(&self) -> Self {
        Self {
            consumers: Arc::clone(&self.consumers),
            total_distributed: Arc::clone(&self.total_distributed),
            tick_rx: Arc::clone(&self.tick_rx),
            task_handle: Arc::clone(&self.task_handle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn create_test_tick(symbol_id: &str) -> MarketTick {
        MarketTick {
            symbol_id: symbol_id.to_string(),
            bid_price: Some(Decimal::new(100, 0)),
            ask_price: Some(Decimal::new(101, 0)),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_distributor_creation() {
        let (distributor, _tx) = TickDistributor::new();
        let stats = distributor.get_stats();

        assert_eq!(stats.consumer_count, 0);
        assert_eq!(stats.total_distributed, 0);
        assert_eq!(stats.consumers.len(), 0);
    }

    #[tokio::test]
    async fn test_consumer_registration() {
        let (distributor, _tx) = TickDistributor::new();

        let _rx1 = distributor.register_consumer("consumer1".to_string());
        let _rx2 = distributor.register_consumer("consumer2".to_string());

        let stats = distributor.get_stats();
        assert_eq!(stats.consumer_count, 2);
        assert!(stats.consumers.contains(&"consumer1".to_string()));
        assert!(stats.consumers.contains(&"consumer2".to_string()));
    }

    #[tokio::test]
    async fn test_consumer_unregistration() {
        let (distributor, _tx) = TickDistributor::new();

        let _rx = distributor.register_consumer("consumer1".to_string());
        assert_eq!(distributor.get_stats().consumer_count, 1);

        let removed = distributor.unregister_consumer("consumer1");
        assert!(removed);
        assert_eq!(distributor.get_stats().consumer_count, 0);

        let removed_again = distributor.unregister_consumer("consumer1");
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_tick_distribution() {
        let (distributor, tx) = TickDistributor::new();

        // Register two consumers
        let mut rx1 = distributor.register_consumer("consumer1".to_string());
        let mut rx2 = distributor.register_consumer("consumer2".to_string());

        // Start distributor
        distributor.start();

        // Send a tick
        let tick = create_test_tick("EURUSD");
        tx.send(tick.clone()).unwrap();

        // Both consumers should receive the tick
        let received1 = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            rx1.recv()
        ).await.unwrap().unwrap();

        let received2 = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            rx2.recv()
        ).await.unwrap().unwrap();

        assert_eq!(received1.symbol_id, "EURUSD");
        assert_eq!(received2.symbol_id, "EURUSD");

        // Give some time for stats to update
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let stats = distributor.get_stats();
        assert_eq!(stats.total_distributed, 2); // 1 tick * 2 consumers
    }

    #[tokio::test]
    async fn test_multiple_ticks() {
        let (distributor, tx) = TickDistributor::new();

        let mut rx = distributor.register_consumer("consumer1".to_string());
        distributor.start();

        // Send 5 ticks
        for i in 0..5 {
            tx.send(create_test_tick(&format!("SYMBOL{}", i))).unwrap();
        }

        // Receive all 5 ticks
        for _ in 0..5 {
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                rx.recv()
            ).await.unwrap().unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let stats = distributor.get_stats();
        assert_eq!(stats.total_distributed, 5);
    }

    #[tokio::test]
    async fn test_is_running() {
        let (distributor, _tx) = TickDistributor::new();

        assert!(!distributor.is_running());

        distributor.start();

        assert!(distributor.is_running());
    }
}
