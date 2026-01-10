use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// RabbitMQ connection configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RabbitMQConfig {
    /// AMQP URI (e.g., "amqp://user:pass@localhost:5672/%2F")
    pub uri: String,

    /// Exchange name for market data
    #[serde(default = "default_exchange")]
    pub exchange: String,

    /// Exchange type (topic, direct, fanout, headers)
    #[serde(default = "default_exchange_type")]
    pub exchange_type: String,

    /// Whether exchange should be durable (survives broker restart)
    #[serde(default = "default_true")]
    pub durable: bool,

    /// Connection pool size (number of channels to maintain)
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout_secs: u64,

    /// Publisher confirms enabled (ensures delivery)
    #[serde(default = "default_true")]
    pub publisher_confirms: bool,

    /// Reconnection strategy
    #[serde(default)]
    pub reconnect: ReconnectConfig,
}

/// Reconnection configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Initial retry delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Maximum retry attempts (0 = infinite)
    #[serde(default)]
    pub max_attempts: u32,
}

impl Default for RabbitMQConfig {
    fn default() -> Self {
        Self {
            uri: "amqp://admin:admin@localhost:5672/%2F".to_string(),
            exchange: default_exchange(),
            exchange_type: default_exchange_type(),
            durable: true,
            pool_size: default_pool_size(),
            connection_timeout_secs: default_timeout(),
            publisher_confirms: true,
            reconnect: ReconnectConfig::default(),
        }
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            max_attempts: 0, // infinite
        }
    }
}

// Default value functions for serde
fn default_exchange() -> String {
    "market.data".to_string()
}

fn default_exchange_type() -> String {
    "topic".to_string()
}

fn default_true() -> bool {
    true
}

fn default_pool_size() -> usize {
    3
}

fn default_timeout() -> u64 {
    30
}

fn default_initial_delay() -> u64 {
    1000
}

fn default_max_delay() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

/// Routing key builder for market data
pub struct RoutingKeyBuilder;

impl RoutingKeyBuilder {
    /// Build routing key for market tick
    /// Format: tick.{symbol}
    pub fn market_tick(symbol: &str) -> String {
        format!("tick.{}", symbol)
    }

    /// Build routing key for trade
    /// Format: trade.{symbol}
    pub fn trade(symbol: &str) -> String {
        format!("trade.{}", symbol)
    }

    /// Build routing key for order book update
    /// Format: orderbook.{symbol}
    pub fn orderbook(symbol: &str) -> String {
        format!("orderbook.{}", symbol)
    }

    /// Wildcard for all ticks
    pub fn all_ticks() -> String {
        "tick.#".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RabbitMQConfig::default();
        assert_eq!(config.exchange, "market.data");
        assert_eq!(config.exchange_type, "topic");
        assert!(config.durable);
        assert!(config.publisher_confirms);
        assert_eq!(config.pool_size, 3);
    }

    #[test]
    fn test_reconnect_config() {
        let config = ReconnectConfig::default();
        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_attempts, 0);
    }

    #[test]
    fn test_routing_keys() {
        assert_eq!(RoutingKeyBuilder::market_tick("EURUSD"), "tick.EURUSD");
        assert_eq!(RoutingKeyBuilder::trade("XAUUSD"), "trade.XAUUSD");
        assert_eq!(RoutingKeyBuilder::orderbook("BTCUSD"), "orderbook.BTCUSD");
        assert_eq!(RoutingKeyBuilder::all_ticks(), "tick.#");
    }
}
