use crate::ctrader_fix::{CTraderFixClient, FixToWebSocketBridge};
use crate::ctrader_fix::market_data::MarketTick;
use crate::models::datasource::*;
use crate::websocket::broadcaster::Broadcaster;
use crate::rabbitmq::{RabbitMQPublisher, FixToRabbitMQBridge, RabbitMQConfig};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

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
use crate::ctrader_fix::symbol_data::symbol_parser::SymbolData;

/// Manages FIX data source lifecycle, heartbeat tracking, and symbol subscriptions
pub struct DatasourceManager {
    mode: Arc<RwLock<DatasourceMode>>,
    fix_client_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    bridge_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    rabbitmq_bridge_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    fix_config: Arc<RwLock<Option<FixConfig>>>,
    heartbeat_counter: Arc<AtomicU64>,
    connection_metrics: Arc<RwLock<Option<ConnectionMetrics>>>,
    subscribed_symbols: Arc<RwLock<Vec<SymbolData>>>,
    symbol_mapping: Arc<RwLock<HashMap<String, String>>>,
    broadcaster: Broadcaster,
    rabbitmq_publisher: Arc<RwLock<Option<Arc<RabbitMQPublisher>>>>,
    rabbitmq_config: Arc<RwLock<Option<RabbitMQConfig>>>,
}

impl DatasourceManager {
    /// Create a new datasource manager
    pub fn new(broadcaster: Broadcaster) -> Self {
        Self {
            mode: Arc::new(RwLock::new(DatasourceMode::Disconnected)),
            fix_client_handle: Arc::new(RwLock::new(None)),
            bridge_handle: Arc::new(RwLock::new(None)),
            rabbitmq_bridge_handle: Arc::new(RwLock::new(None)),
            fix_config: Arc::new(RwLock::new(None)),
            heartbeat_counter: Arc::new(AtomicU64::new(0)),
            connection_metrics: Arc::new(RwLock::new(None)),
            subscribed_symbols: Arc::new(RwLock::new(Vec::new())),
            symbol_mapping: Arc::new(RwLock::new(HashMap::new())),
            broadcaster,
            rabbitmq_publisher: Arc::new(RwLock::new(None)),
            rabbitmq_config: Arc::new(RwLock::new(None)),
        }
    }

    /// Start FIX connection with given configuration
    pub async fn start_live_fix(&self, config: FixConfig) -> Result<(), String> {
        self.validate_connection_state().await?;

        tracing::info!("Starting FIX connection to {}:{}", config.host, config.port);

        self.reset_connection_state(&config).await;

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
        let bridge = FixToWebSocketBridge::new(self.broadcaster.clone());

        println!("Setup Fix Callbacks");
        self.setup_fix_callbacks(&mut client, &bridge);

        println!("Spawn Connection Tasks");
        let (client_handle, bridge_handle, rabbitmq_handle) = self.spawn_connection_tasks(client, bridge, tick_receiver).await;

        println!("Finalizing COnnectrion");
        self.finalize_connection(client_handle, bridge_handle, rabbitmq_handle).await;

        tracing::info!("FIX connection started successfully");
        Ok(())
    }

    /// Validate that we're not already connected
    async fn validate_connection_state(&self) -> Result<(), String> {
        let current_mode = *self.mode.read().await;
        if current_mode == DatasourceMode::Connected {
            return Err("Already connected to FIX server".to_string());
        }
        Ok(())
    }

    /// Reset connection state and store new configuration
    async fn reset_connection_state(&self, config: &FixConfig) {
        *self.fix_config.write().await = Some(config.clone());
        self.heartbeat_counter.store(0, Ordering::Relaxed);
        *self.connection_metrics.write().await = Some(ConnectionMetrics::new());
        *self.subscribed_symbols.write().await = Vec::new();
        *self.symbol_mapping.write().await = HashMap::new();
    }

    /// Setup FIX client callbacks (heartbeat and security list)
    fn setup_fix_callbacks(&self, client: &mut CTraderFixClient, bridge: &FixToWebSocketBridge) {
        self.setup_heartbeat_callback(client);
        self.setup_security_list_callback(client, bridge);
    }

    /// Setup heartbeat callback to track connection health
    fn setup_heartbeat_callback(&self, client: &mut CTraderFixClient) {
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
    }

    /// Setup security list callback to populate symbol mappings
    fn setup_security_list_callback(&self, client: &mut CTraderFixClient, bridge: &FixToWebSocketBridge) {
        let subscribed_symbols = Arc::clone(&self.subscribed_symbols);
        let symbol_mapping = Arc::clone(&self.symbol_mapping);
        let bridge_symbol_map = bridge.get_symbol_map();

        client.set_security_list_callback(Arc::new(move |symbols: Vec<SymbolData>| {
            tracing::info!("Received {} symbols from Security List Response", symbols.len());

            let mapping = Self::build_symbol_mapping(&symbols);

            Self::update_subscribed_symbols(&subscribed_symbols, symbols);
            Self::update_symbol_mapping(&symbol_mapping, mapping.clone());
            Self::update_bridge_symbol_mapping(&bridge_symbol_map, mapping);
        }));
    }

    /// Build HashMap of symbol ID -> symbol name
    fn build_symbol_mapping(symbols: &[SymbolData]) -> HashMap<String, String> {
        symbols
            .iter()
            .map(|symbol| (symbol.symbol_id.to_string(), symbol.symbol_name.clone()))
            .collect()
    }

    /// Update subscribed symbols list
    fn update_subscribed_symbols(
        subscribed_symbols: &Arc<RwLock<Vec<SymbolData>>>,
        symbols: Vec<SymbolData>,
    ) {
        if let Ok(mut syms) = subscribed_symbols.try_write() {
            *syms = symbols;
        }
    }

    /// Update symbol mapping (ID -> Name)
    fn update_symbol_mapping(
        symbol_mapping: &Arc<RwLock<HashMap<String, String>>>,
        mapping: HashMap<String, String>,
    ) {
        if let Ok(mut map) = symbol_mapping.try_write() {
            *map = mapping;
        }
    }

    /// Update bridge symbol mapping asynchronously
    fn update_bridge_symbol_mapping(
        bridge_symbol_map: &Arc<RwLock<HashMap<String, String>>>,
        mapping: HashMap<String, String>,
    ) {
        let bridge_map = Arc::clone(bridge_symbol_map);
        tokio::spawn(async move {
            let mut bridge_map = bridge_map.write().await;
            for (id, name) in mapping {
                bridge_map.insert(id, name);
            }
            tracing::info!("Updated bridge symbol mappings");
        });
    }

    /// Spawn FIX client and bridge tasks
    async fn spawn_connection_tasks(
        &self,
        mut client: CTraderFixClient,
        bridge: FixToWebSocketBridge,
        tick_receiver: mpsc::UnboundedReceiver<MarketTick>,
    ) -> (JoinHandle<()>, JoinHandle<()>, Option<JoinHandle<()>>) {
        let client_handle = tokio::spawn(async move {
            if let Err(e) = client.connect_and_run().await {
                tracing::error!("FIX client error: {}", e);
            }
        });

        // Create broadcast channel for dual output (WebSocket + RabbitMQ)
        let (tick_tx, tick_rx_ws) = mpsc::unbounded_channel();
        let tick_rx_rmq = if self.rabbitmq_publisher.read().await.is_some() {
            let (tx, rx) = mpsc::unbounded_channel();
            Some((tx, rx))
        } else {
            None
        };

        // Fan-out task: broadcast ticks to both WebSocket and RabbitMQ channels
        let rmq_tx_clone = tick_rx_rmq.as_ref().map(|(tx, _)| tx.clone());
        tokio::spawn(async move {
            let mut receiver = tick_receiver;
            while let Some(tick) = receiver.recv().await {
                // Send to WebSocket bridge
                let _ = tick_tx.send(tick.clone());

                // Send to RabbitMQ bridge if enabled
                if let Some(ref rmq_tx) = rmq_tx_clone {
                    let _ = rmq_tx.send(tick);
                }
            }
        });

        // Spawn WebSocket bridge task
        let bridge_handle = tokio::spawn(async move {
            bridge.run(tick_rx_ws).await;
        });

        // Spawn RabbitMQ bridge task if configured
        let rabbitmq_handle = if let Some((_, tick_rx_rmq)) = tick_rx_rmq {
            let publisher_guard = self.rabbitmq_publisher.blocking_read();
            if let Some(publisher) = publisher_guard.as_ref() {
                let rmq_bridge = FixToRabbitMQBridge::new(Arc::clone(publisher));

                // Share symbol map with RabbitMQ bridge
                let symbol_mapping = Arc::clone(&self.symbol_mapping);
                tokio::spawn(async move {
                    // Update symbol map from manager
                    let map = symbol_mapping.read().await;
                    rmq_bridge.update_symbol_mappings(map.clone()).await;
                    drop(map);

                    // Run the bridge
                    rmq_bridge.run(tick_rx_rmq).await;
                });
                Some(tokio::spawn(async {}))
            } else {
                None
            }
        } else {
            None
        };

        (client_handle, bridge_handle, rabbitmq_handle)
    }

    /// Store task handles and update connection mode
    async fn finalize_connection(&self, client_handle: JoinHandle<()>, bridge_handle: JoinHandle<()>, rabbitmq_handle: Option<JoinHandle<()>>) {
        *self.fix_client_handle.write().await = Some(client_handle);
        *self.bridge_handle.write().await = Some(bridge_handle);
        *self.rabbitmq_bridge_handle.write().await = rabbitmq_handle;
        *self.mode.write().await = DatasourceMode::Connected;
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
        if let Some(handle) = self.rabbitmq_bridge_handle.write().await.take() {
            handle.abort();
        }

        // Disconnect RabbitMQ if connected
        if let Some(publisher) = self.rabbitmq_publisher.read().await.as_ref() {
            let _ = publisher.disconnect().await;
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

    /// Connect to RabbitMQ with given configuration
    pub async fn connect_rabbitmq(&self, config: RabbitMQConfig) -> Result<(), String> {
        tracing::info!("Connecting to RabbitMQ: {}", config.uri);

        // Create publisher
        let publisher = Arc::new(RabbitMQPublisher::new(config.clone()));

        // Connect
        publisher
            .connect()
            .await
            .map_err(|e| format!("Failed to connect to RabbitMQ: {}", e))?;

        // Store publisher and config
        *self.rabbitmq_publisher.write().await = Some(publisher);
        *self.rabbitmq_config.write().await = Some(config);

        tracing::info!("Successfully connected to RabbitMQ");
        Ok(())
    }

    /// Disconnect from RabbitMQ
    pub async fn disconnect_rabbitmq(&self) -> Result<(), String> {
        if let Some(publisher) = self.rabbitmq_publisher.write().await.take() {
            publisher
                .disconnect()
                .await
                .map_err(|e| format!("Failed to disconnect from RabbitMQ: {}", e))?;
            *self.rabbitmq_config.write().await = None;
            tracing::info!("Disconnected from RabbitMQ");
            Ok(())
        } else {
            Err("RabbitMQ is not connected".to_string())
        }
    }

    /// Check if RabbitMQ is connected
    pub async fn is_rabbitmq_connected(&self) -> bool {
        if let Some(publisher) = self.rabbitmq_publisher.read().await.as_ref() {
            publisher.is_connected()
        } else {
            false
        }
    }

    /// Get RabbitMQ publisher statistics
    pub async fn get_rabbitmq_stats(&self) -> Option<crate::rabbitmq::PublisherStats> {
        self.rabbitmq_publisher
            .read()
            .await
            .as_ref()
            .map(|p| p.stats())
    }
}
