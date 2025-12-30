use crate::ctrader_fix::{CTraderFixClient, FixToWebSocketBridge};
use crate::ctrader_fix::messages::SymbolData;
use crate::models::datasource::*;
use crate::websocket::broadcaster::Broadcaster;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

// JoinHandle is a Rust type from std::thread and tokio::task modules that represents a handle to a spawned thread or async task.
// It allows you to:
///  1. Wait for completion - Call .await (async) or .join() (sync) to wait for the task to finish
///  2. Get the return value - The task's result is returned when you await/join
///  3. Manage the task lifecycle - You can abort or check if the task is finished

///  Common use cases in this project:
///  - Background tasks: Spawning WebSocket broadcasters, market data processors
///  - Parallel processing: Running multiple tasks concurrently
///  - Cleanup: Keeping handles to abort tasks on shutdown
use tokio::task::JoinHandle;
use chrono::Utc;

/// Manages FIX data source lifecycle, heartbeat tracking, and symbol subscriptions
pub struct DatasourceManager {
    mode: Arc<RwLock<DatasourceMode>>,
    fix_client_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    bridge_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    fix_config: Arc<RwLock<Option<FixConfig>>>,
    heartbeat_counter: Arc<AtomicU64>,
    connection_metrics: Arc<RwLock<Option<ConnectionMetrics>>>,
    subscribed_symbols: Arc<RwLock<Vec<SymbolData>>>,
    symbol_mapping: Arc<RwLock<HashMap<String, String>>>,
    broadcaster: Broadcaster,
}

impl DatasourceManager {
    /// Create a new datasource manager
    pub fn new(broadcaster: Broadcaster) -> Self {
        Self {
            mode: Arc::new(RwLock::new(DatasourceMode::Disconnected)),
            fix_client_handle: Arc::new(RwLock::new(None)),
            bridge_handle: Arc::new(RwLock::new(None)),
            fix_config: Arc::new(RwLock::new(None)),
            heartbeat_counter: Arc::new(AtomicU64::new(0)),
            connection_metrics: Arc::new(RwLock::new(None)),
            subscribed_symbols: Arc::new(RwLock::new(Vec::new())),
            symbol_mapping: Arc::new(RwLock::new(HashMap::new())),
            broadcaster,
        }
    }

    /// Start FIX connection with given configuration
    pub async fn start_live_fix(&self, config: FixConfig) -> Result<(), String> {
        // Check if already connected
        let current_mode = *self.mode.read().await;
        if current_mode == DatasourceMode::Connected {
            return Err("Already connected to FIX server".to_string());
        }

        tracing::info!("Starting FIX connection to {}:{}", config.host, config.port);

        // Store configuration
        *self.fix_config.write().await = Some(config.clone());

        // Reset metrics
        self.heartbeat_counter.store(0, Ordering::Relaxed);
        *self.connection_metrics.write().await = Some(ConnectionMetrics::new());
        *self.subscribed_symbols.write().await = Vec::new();
        *self.symbol_mapping.write().await = HashMap::new();

        // Create FIX client with tick channel
        let (mut client, tick_receiver) = CTraderFixClient::with_tick_channel(
            config.host.clone(),
            config.port,
            config.credentials.sender_comp_id.clone(),
            config.credentials.target_comp_id.clone(),
            config.credentials.sender_sub_id.clone(),
            config.credentials.target_sub_id.clone(),
            config.credentials.username.clone(),
            config.credentials.password.clone(),
        );

        // Arc::clone() creates a new reference-counted pointer to the same heap-allocated data. It's a cheap operation that only increments the reference count, rather than copying the actual data.
        // Set up heartbeat callback
        let heartbeat_counter = Arc::clone(&self.heartbeat_counter);
        let connection_metrics = Arc::clone(&self.connection_metrics);
        client.set_heartbeat_callback(Arc::new(move || {
            heartbeat_counter.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut metrics) = connection_metrics.try_write() {
                if let Some(ref mut m) = *metrics {
                    m.last_heartbeat = Some(std::time::Instant::now());
                }
            }
            tracing::debug!("Heartbeat received from FIX server");
        }));

        // Set up security list callback
        let subscribed_symbols = Arc::clone(&self.subscribed_symbols);
        let symbol_mapping = Arc::clone(&self.symbol_mapping);
        client.set_security_list_callback(Arc::new(move |symbols: Vec<SymbolData>| {
            tracing::info!("Received {} symbols from Security List Response", symbols.len());

            // Build symbol mapping (ID -> Name)
            let mut mapping = HashMap::new();
            for symbol in &symbols {
                mapping.insert(symbol.symbol_id.to_string(), symbol.symbol_name.clone());
            }

            // Update shared state
            if let Ok(mut syms) = subscribed_symbols.try_write() {
                *syms = symbols;
            }
            if let Ok(mut map) = symbol_mapping.try_write() {
                *map = mapping;
            }
        }));

        // Create bridge to convert FIX ticks to WebSocket messages
        let bridge = FixToWebSocketBridge::new(self.broadcaster.clone());

        // Spawn FIX client task
        let client_handle = tokio::spawn(async move {
            if let Err(e) = client.connect_and_run().await {
                tracing::error!("FIX client error: {}", e);
            }
        });

        // Spawn bridge task
        let bridge_handle = tokio::spawn(async move {
            bridge.run(tick_receiver).await;
        });

        // Store task handles
        *self.fix_client_handle.write().await = Some(client_handle);
        *self.bridge_handle.write().await = Some(bridge_handle);

        // Update mode
        *self.mode.write().await = DatasourceMode::Connected;

        tracing::info!("FIX connection started successfully");
        Ok(())
    }

    /// Stop FIX connection
    pub async fn stop(&self) -> Result<(), String> {
        let current_mode = *self.mode.read().await;
        if current_mode == DatasourceMode::Disconnected {
            return Err("No active FIX connection to stop".to_string());
        }

        tracing::info!("Stopping FIX connection");

        // Abort running tasks
        if let Some(handle) = self.fix_client_handle.write().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.bridge_handle.write().await.take() {
            handle.abort();
        }

        // Clear state
        *self.fix_config.write().await = None;
        *self.connection_metrics.write().await = None;
        self.heartbeat_counter.store(0, Ordering::Relaxed);

        // Update mode
        *self.mode.write().await = DatasourceMode::Disconnected;

        tracing::info!("FIX connection stopped");
        Ok(())
    }

    /// Get current datasource status
    pub async fn get_status(&self) -> DatasourceStatus {
        let mode = *self.mode.read().await;
        let connected = mode == DatasourceMode::Connected;
        let heartbeat_count = self.heartbeat_counter.load(Ordering::Relaxed);
        let symbols = self.subscribed_symbols.read().await;
        let config = self.fix_config.read().await;
        let metrics = self.connection_metrics.read().await;

        let (uptime_seconds, last_heartbeat_seconds_ago) = if let Some(ref m) = *metrics {
            (Some(m.uptime_seconds()), m.last_heartbeat_seconds_ago())
        } else {
            (None, None)
        };

        let symbols_info: Vec<SymbolInfo> = symbols
            .iter()
            .map(|s| SymbolInfo {
                symbol_id: s.symbol_id.to_string(),
                symbol_name: s.symbol_name.clone(),
                symbol_digits: s.symbol_digits,
            })
            .collect();

        let total_symbols = symbols_info.len();

        let (fix_server, connection_info) = if let Some(ref cfg) = *config {
            (
                Some(format!("{}:{}", cfg.host, cfg.port)),
                Some(ConnectionInfo {
                    sender_comp_id: cfg.credentials.sender_comp_id.clone(),
                    target_comp_id: cfg.credentials.target_comp_id.clone(),
                }),
            )
        } else {
            (None, None)
        };

        DatasourceStatus {
            mode,
            connected,
            uptime_seconds,
            heartbeat_count,
            last_heartbeat_seconds_ago,
            symbols_subscribed: symbols_info,
            total_symbols,
            fix_server,
            connection_info,
        }
    }

    /// Get health status
    pub async fn get_health(&self) -> HealthStatus {
        let mode = *self.mode.read().await;
        let metrics = self.connection_metrics.read().await;
        let symbols = self.subscribed_symbols.read().await;

        let connection_state = if mode == DatasourceMode::Connected {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        };

        let (uptime_seconds, last_heartbeat_seconds_ago, heartbeat_status, health_state, warning) =
            if let Some(ref m) = *metrics {
                let uptime = m.uptime_seconds();
                let last_hb = m.last_heartbeat_seconds_ago();

                let (hb_state, health, warn) = match last_hb {
                    None => (
                        HeartbeatState::None,
                        HealthState::Degraded,
                        Some("No heartbeat received yet".to_string()),
                    ),
                    Some(seconds) if seconds <= 30 => (
                        HeartbeatState::Active,
                        HealthState::Healthy,
                        None,
                    ),
                    Some(seconds) if seconds <= 60 => (
                        HeartbeatState::Stale,
                        HealthState::Degraded,
                        Some(format!("No heartbeat received in {} seconds", seconds)),
                    ),
                    Some(seconds) => (
                        HeartbeatState::Stale,
                        HealthState::Unhealthy,
                        Some(format!("No heartbeat received in {} seconds", seconds)),
                    ),
                };

                (Some(uptime), last_hb, hb_state, health, warn)
            } else {
                (
                    None,
                    None,
                    HeartbeatState::None,
                    HealthState::Unhealthy,
                    None,
                )
            };

        let symbols_count = if symbols.is_empty() {
            None
        } else {
            Some(symbols.len())
        };

        HealthStatus {
            status: health_state,
            fix_connection: connection_state,
            heartbeat_status,
            last_heartbeat_seconds_ago,
            uptime_seconds,
            symbols_count,
            warning,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Get subscribed symbols
    pub async fn get_subscribed_symbols(&self) -> Vec<SymbolData> {
        self.subscribed_symbols.read().await.clone()
    }

    /// Get symbol name from ID
    pub async fn get_symbol_name(&self, symbol_id: &str) -> Option<String> {
        self.symbol_mapping.read().await.get(symbol_id).cloned()
    }
}
