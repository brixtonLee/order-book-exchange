use crate::ctrader_fix::CTraderFixClient;
use crate::ctrader_fix::market_data::MarketTick;
use crate::models::datasource::*;
use std::collections::HashMap;
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
///
/// RabbitMQ integration has been extracted to RabbitMQService (independent lifecycle)
pub struct DatasourceManager {
    fix_client_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    distributor_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    connection_metrics: Arc<RwLock<Option<ConnectionMetrics>>>,
    subscribed_symbols: Arc<RwLock<Vec<SymbolData>>>,
    symbol_mapping: Arc<RwLock<HashMap<String, String>>>,
}

impl DatasourceManager {
    /// Create a new datasource manager
    pub fn new() -> Self {
        Self {
            fix_client_handle: Arc::new(RwLock::new(None)),
            distributor_handle: Arc::new(RwLock::new(None)),
            connection_metrics: Arc::new(RwLock::new(None)),
            subscribed_symbols: Arc::new(RwLock::new(Vec::new())),
            symbol_mapping: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    /// Start FIX connection with given configuration
    ///
    /// Accepts a tick distributor sender for centralized tick broadcasting
    pub async fn start_live_fix(
        &self,
        config: FixConfig,
        tick_distributor_tx: mpsc::UnboundedSender<MarketTick>,
    ) -> Result<(), String> {
        self.validate_connection_state().await?;

        tracing::info!("Starting FIX connection to {}:{}", config.host, config.port);

        self.reset_connection_state().await;

        let (mut client, tick_receiver) = CTraderFixClient::with_tick_channel(
            config
        );

        self.setup_fix_callbacks(&mut client);

        let (client_handle, forward_handle) = self.spawn_connection_tasks(
            client,
            tick_receiver,
            tick_distributor_tx,
        ).await;

        self.finalize_connection(client_handle, forward_handle).await;

        tracing::info!("FIX connection started successfully");
        Ok(())
    }

    /// Validate that we're not already connected
    /// The * tries to dereference and move the Vec out of the guard
    async fn validate_connection_state(&self) -> Result<(), String> {
        if !self.subscribed_symbols.read().await.is_empty() {
            return Err("Already connected to FIX server".to_string());
        }
        Ok(())
    }

    /// Reset connection state and store new configuration
    async fn reset_connection_state(&self) {
        *self.connection_metrics.write().await = Some(ConnectionMetrics::new());
        *self.subscribed_symbols.write().await = Vec::new();
        *self.symbol_mapping.write().await = HashMap::new();
    }

    /// Setup FIX client callbacks (heartbeat and security list)
    fn setup_fix_callbacks(&self, client: &mut CTraderFixClient) {
        self.setup_heartbeat_callback(client);
        self.setup_security_list_callback(client);
    }

    /// Setup heartbeat callback to track connection health
    fn setup_heartbeat_callback(&self, client: &mut CTraderFixClient) {
        let connection_metrics = Arc::clone(&self.connection_metrics);

        client.set_heartbeat_callback(Arc::new(move || {
            if let Ok(mut metrics) = connection_metrics.try_write() {
                if let Some(ref mut m) = *metrics {
                    m.last_heartbeat = Some(std::time::Instant::now());
                }
            }
            tracing::debug!("Heartbeat received from FIX server");
        }));
    }

    /// Setup security list callback to populate symbol mappings
    fn setup_security_list_callback(&self, client: &mut CTraderFixClient) {
        let subscribed_symbols = Arc::clone(&self.subscribed_symbols);
        let symbol_mapping = Arc::clone(&self.symbol_mapping);

        client.set_security_list_callback(Arc::new(move |symbols: Vec<SymbolData>| {
            tracing::info!("Received {} symbols from Security List Response", symbols.len());

            let mapping = Self::build_symbol_mapping(&symbols);

            Self::update_subscribed_symbols(&subscribed_symbols, symbols);
            Self::update_symbol_mapping(&symbol_mapping, mapping);
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

    /// Spawn FIX client task and forward ticks to distributor
    ///
    /// Simplified: sends ticks to TickDistributor which handles broadcasting
    async fn spawn_connection_tasks(
        &self,
        mut client: CTraderFixClient,
        tick_receiver: mpsc::UnboundedReceiver<MarketTick>,
        tick_distributor_tx: mpsc::UnboundedSender<MarketTick>,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let client_handle = tokio::spawn(async move {
            if let Err(e) = client.connect_and_run().await {
                tracing::error!("FIX client error: {}", e);
            }
        });

        // Forward ticks from FIX client to TickDistributor
        // TickDistributor handles broadcasting to all consumers (WebSocket, RabbitMQ, TickQueue)
        let forward_handle = tokio::spawn(async move {
            let mut receiver = tick_receiver;
            while let Some(tick) = receiver.recv().await {
                if let Err(e) = tick_distributor_tx.send(tick) {
                    tracing::error!("Failed to send tick to distributor: {}", e);
                    break;
                }
            }
            tracing::warn!("Tick forwarding task ended");
        });

        (client_handle, forward_handle)
    }

    /// Store task handles and update connection mode
    async fn finalize_connection(&self, client_handle: JoinHandle<()>, forward_handle: JoinHandle<()>) {
        *self.fix_client_handle.write().await = Some(client_handle);
        *self.distributor_handle.write().await = Some(forward_handle);
    }

    /// Stop FIX connection
    pub async fn stop(&self) -> Result<(), String> {
        tracing::info!("Stopping FIX connection");

        // Abort running tasks
        if let Some(handle) = self.fix_client_handle.write().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.distributor_handle.write().await.take() {
            handle.abort();
        }

        // Clear state
        *self.subscribed_symbols.write().await = Vec::new();
        *self.connection_metrics.write().await = None;

        tracing::info!("FIX connection stopped");
        Ok(())
    }

    /// Get current datasource status
    pub async fn get_status(&self) -> DatasourceStatus {
        let symbols = self.subscribed_symbols.read().await;
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

        DatasourceStatus {
            connected: total_symbols > 0,
            uptime_seconds,
            last_heartbeat_seconds_ago,
            symbols_subscribed: symbols_info,
            total_symbols
        }
    }

    /// Get health status
    pub async fn get_health(&self) -> HealthStatus {
        let metrics = self.connection_metrics.read().await;
        let symbols = self.subscribed_symbols.read().await;

        let connection_state = if !symbols.is_empty() {
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

    /// Get all symbol mappings (for symbol sync job)
    pub async fn get_symbol_map(&self) -> HashMap<i64, String> {
        self.symbol_mapping
            .read()
            .await
            .iter()
            .filter_map(|(id_str, name)| {
                id_str.parse::<i64>().ok().map(|id| (id, name.clone()))
            })
            .collect()
    }
}
