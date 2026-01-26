use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;

/// FIX connection configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FixConfig {
    pub host: String,
    pub port: u16,
    pub credentials: FixCredentials,
}

/// FIX protocol credentials
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FixCredentials {
    pub sender_comp_id: String,
    pub target_comp_id: String,
    pub sender_sub_id: String,
    pub target_sub_id: String,
    pub username: String,
    #[serde(skip_serializing)] // Never expose password in responses
    pub password: String,
}

/// Current datasource operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DatasourceMode {
    Disconnected,
    Connected,
}

/// Detailed datasource status response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DatasourceStatus {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_seconds_ago: Option<u64>,
    pub symbols_subscribed: Vec<SymbolInfo>,
    pub total_symbols: usize
}

/// Symbol information from Security List Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SymbolInfo {
    pub symbol_id: String,
    pub symbol_name: String,
    pub symbol_digits: u8,
}

/// Masked connection information (no sensitive data)

/// Health status of the system
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthStatus {
    pub status: HealthState,
    pub fix_connection: ConnectionState,
    pub heartbeat_status: HeartbeatState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_seconds_ago: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HeartbeatState {
    Active,
    Stale,
    None,
}

/// Request to start FIX connection
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartDatasourceRequest {
    pub host: String,
    pub port: u16,
    pub credentials: FixCredentials,
}

/// Response after starting datasource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartDatasourceResponse {
    pub status: String,
    pub message: String,
}

/// Response after stopping datasource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StopDatasourceResponse {
    pub status: String,
    pub message: String,
}

// Internal helper to track connection timing
#[derive(Debug, Clone)]
pub(crate) struct ConnectionMetrics {
    pub start_time: Instant,
    pub last_heartbeat: Option<Instant>,
}

impl ConnectionMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_heartbeat: None,
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub fn last_heartbeat_seconds_ago(&self) -> Option<u64> {
        self.last_heartbeat.map(|t| t.elapsed().as_secs())
    }
}
