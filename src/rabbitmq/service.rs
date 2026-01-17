use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::ctrader_fix::market_data::MarketTick;
use super::config::RabbitMQConfig;
use super::publisher::RabbitMQPublisher;
use super::bridge::{FixToRabbitMQBridge, BridgeStats};

/// Independent RabbitMQ service with its own lifecycle
/// Decoupled from FIX datasource - can run independently
pub struct RabbitMQService {
    /// RabbitMQ publisher
    publisher: Arc<RabbitMQPublisher>,

    /// Symbol mapping shared with DatasourceManager
    symbol_map: Arc<RwLock<HashMap<String, String>>>,

    /// Background task handle
    task_handle: Arc<RwLock<Option<JoinHandle<()>>>>,

    /// Service running state
    is_running: Arc<AtomicBool>,
}

impl RabbitMQService {
    /// Create a new RabbitMQ service
    pub fn new(config: RabbitMQConfig) -> Self {
        let publisher = Arc::new(RabbitMQPublisher::new(config));

        Self {
            publisher,
            symbol_map: Arc::new(RwLock::new(HashMap::new())),
            task_handle: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the symbol map for sharing with DatasourceManager
    pub fn get_symbol_map(&self) -> Arc<RwLock<HashMap<String, String>>> {
        Arc::clone(&self.symbol_map)
    }

    /// Connect to RabbitMQ and start consuming ticks from distributor
    ///
    /// Accepts a tick receiver from TickDistributor for consuming market ticks
    pub async fn connect(&self, tick_rx: mpsc::UnboundedReceiver<MarketTick>) -> Result<(), String> {
        // Check if already running
        if self.is_running.load(Ordering::Acquire) {
            return Err("RabbitMQ service is already running".to_string());
        }

        // Connect publisher
        self.publisher.connect().await
            .map_err(|e| format!("Failed to connect to RabbitMQ: {}", e))?;

        // Create bridge with publisher
        let bridge = FixToRabbitMQBridge::new(Arc::clone(&self.publisher));

        // Share symbol mapping from service to bridge
        {
            let service_symbol_map = self.symbol_map.read().await;
            bridge.update_symbol_mappings(service_symbol_map.clone()).await;
        }

        // Spawn background task to run the bridge (consumes ticks from TickDistributor)
        let is_running = Arc::clone(&self.is_running);
        let handle = tokio::spawn(async move {
            is_running.store(true, Ordering::Release);
            bridge.run(tick_rx).await;
            is_running.store(false, Ordering::Release);
        });

        *self.task_handle.write().await = Some(handle);

        tracing::info!("✅ RabbitMQ service started successfully");
        Ok(())
    }

    /// Disconnect from RabbitMQ and stop the service
    pub async fn disconnect(&self) -> Result<(), String> {
        if !self.is_running.load(Ordering::Acquire) {
            return Err("RabbitMQ service is not running".to_string());
        }

        // Abort the background task
        if let Some(handle) = self.task_handle.write().await.take() {
            handle.abort();
        }

        // Disconnect publisher
        self.publisher.disconnect().await
            .map_err(|e| format!("Failed to disconnect from RabbitMQ: {}", e))?;

        self.is_running.store(false, Ordering::Release);

        tracing::info!("🔴 RabbitMQ service stopped");
        Ok(())
    }

    /// Check if service is connected and running
    pub fn is_connected(&self) -> bool {
        self.publisher.is_connected() && self.is_running.load(Ordering::Acquire)
    }

    /// Update symbol mappings
    pub async fn update_symbol_mappings(&self, mappings: HashMap<String, String>) {
        // Update service's symbol map
        let mut map = self.symbol_map.write().await;
        for (id, name) in mappings.iter() {
            map.insert(id.clone(), name.clone());
        }
    }

    /// Get service statistics
    /// Returns None if service is not running
    /// TODO: Implement proper stats tracking for the bridge
    pub async fn stats(&self) -> Option<BridgeStats> {
        if self.is_running.load(Ordering::Acquire) {
            // Return basic stats from publisher
            // In the future, we could track stats in the service itself
            Some(BridgeStats {
                ticks_processed: 0, // TODO: track this
                ticks_failed: 0,    // TODO: track this
                publisher_stats: self.publisher.stats(),
            })
        } else {
            None
        }
    }

    /// Get configuration exchange name
    pub fn get_exchange(&self) -> String {
        self.publisher.stats().is_connected.then(|| {
            "market.data".to_string() // This should ideally come from config
        }).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let config = RabbitMQConfig::default();
        let service = RabbitMQService::new(config);

        assert!(!service.is_connected());
    }

    #[tokio::test]
    async fn test_symbol_mapping() {
        let config = RabbitMQConfig::default();
        let service = RabbitMQService::new(config);

        let mut mappings = HashMap::new();
        mappings.insert("1".to_string(), "EURUSD".to_string());
        mappings.insert("41".to_string(), "XAUUSD".to_string());

        service.update_symbol_mappings(mappings).await;

        let symbol_map = service.symbol_map.read().await;
        assert_eq!(symbol_map.get("1"), Some(&"EURUSD".to_string()));
        assert_eq!(symbol_map.get("41"), Some(&"XAUUSD".to_string()));
    }
}
