use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
    ExchangeKind,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::RwLock;
use std::time::Duration;
use serde::Serialize;

use super::config::RabbitMQConfig;

/// Error types for RabbitMQ operations
#[derive(Debug, thiserror::Error)]
pub enum RabbitMQError {
    #[error("Connection error: {0}")]
    Connection(#[from] lapin::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Publisher not connected")]
    NotConnected,

    #[error("Channel creation failed: {0}")]
    ChannelCreation(String),

    #[error("Publish failed: {0}")]
    PublishFailed(String),
}

pub type Result<T> = std::result::Result<T, RabbitMQError>;

/// Statistics for RabbitMQ publisher
#[derive(Debug, Clone)]
pub struct PublisherStats {
    pub messages_published: u64,
    pub messages_confirmed: u64,
    pub messages_failed: u64,
    pub is_connected: bool,
    pub reconnect_count: u64,
}

/// RabbitMQ publisher with connection pooling and auto-reconnect
pub struct RabbitMQPublisher {
    config: RabbitMQConfig,
    connection: Arc<RwLock<Option<Connection>>>,
    channel: Arc<RwLock<Option<Channel>>>,
    is_connected: Arc<AtomicBool>,

    // Metrics
    messages_published: Arc<AtomicU64>,
    messages_confirmed: Arc<AtomicU64>,
    messages_failed: Arc<AtomicU64>,
    reconnect_count: Arc<AtomicU64>,
}

impl RabbitMQPublisher {
    /// Create a new RabbitMQ publisher
    pub fn new(config: RabbitMQConfig) -> Self {
        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            channel: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(AtomicBool::new(false)),
            messages_published: Arc::new(AtomicU64::new(0)),
            messages_confirmed: Arc::new(AtomicU64::new(0)),
            messages_failed: Arc::new(AtomicU64::new(0)),
            reconnect_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Connect to RabbitMQ server
    pub async fn connect(&self) -> Result<()> {
        tracing::info!("Connecting to RabbitMQ at {}", self.config.uri);

        // Create connection with timeout
        let connection = tokio::time::timeout(
            Duration::from_secs(self.config.connection_timeout_secs),
            Connection::connect(&self.config.uri, ConnectionProperties::default()),
        )
        .await
        .map_err(|_| RabbitMQError::Connection(
            lapin::Error::InvalidConnectionState(lapin::ConnectionState::Closed)
        ))??;

        // Create channel
        let channel = connection.create_channel().await?;

        // Declare exchange
        channel
            .exchange_declare(
                &self.config.exchange,
                self.parse_exchange_type(),
                ExchangeDeclareOptions {
                    durable: self.config.durable,
                    auto_delete: false,
                    internal: false,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        // Enable publisher confirms if configured
        if self.config.publisher_confirms {
            channel
                .confirm_select(ConfirmSelectOptions::default())
                .await?;
            tracing::info!("Publisher confirms enabled");
        }

        // Store connection and channel
        *self.connection.write().await = Some(connection);
        *self.channel.write().await = Some(channel);
        self.is_connected.store(true, Ordering::Release);

        tracing::info!("Successfully connected to RabbitMQ");
        Ok(())
    }

    /// Disconnect from RabbitMQ
    pub async fn disconnect(&self) -> Result<()> {
        tracing::info!("Disconnecting from RabbitMQ");

        // Close channel
        if let Some(channel) = self.channel.write().await.take() {
            let _ = channel.close(200, "Normal shutdown").await;
        }

        // Close connection
        if let Some(connection) = self.connection.write().await.take() {
            let _ = connection.close(200, "Normal shutdown").await;
        }

        self.is_connected.store(false, Ordering::Release);
        tracing::info!("Disconnected from RabbitMQ");
        Ok(())
    }

    /// Publish a message to the exchange
    pub async fn publish<T: Serialize>(
        &self,
        routing_key: &str,
        message: &T,
    ) -> Result<()> {
        self.publish_with_properties(routing_key, message, BasicProperties::default())
            .await
    }

    /// Publish a message with custom properties
    pub async fn publish_with_properties<T: Serialize>(
        &self,
        routing_key: &str,
        message: &T,
        properties: BasicProperties,
    ) -> Result<()> {
        if !self.is_connected.load(Ordering::Acquire) {
            return Err(RabbitMQError::NotConnected);
        }

        // Serialize message to JSON
        let payload = serde_json::to_vec(message)?;

        // Get channel
        let channel_guard = self.channel.read().await;
        let channel = channel_guard
            .as_ref()
            .ok_or(RabbitMQError::NotConnected)?;

        // Publish message
        let confirm = channel
            .basic_publish(
                &self.config.exchange,
                routing_key,
                BasicPublishOptions::default(),
                &payload,
                properties,
            )
            .await
            .map_err(|e| RabbitMQError::PublishFailed(e.to_string()))?;

        self.messages_published.fetch_add(1, Ordering::Relaxed);

        // Wait for publisher confirm if enabled
        if self.config.publisher_confirms {
            match confirm.await {
                Ok(_) => {
                    self.messages_confirmed.fetch_add(1, Ordering::Relaxed);
                    tracing::trace!("Message confirmed: routing_key={}", routing_key);
                }
                Err(e) => {
                    self.messages_failed.fetch_add(1, Ordering::Relaxed);
                    tracing::error!("Message confirmation failed: {}", e);
                    return Err(RabbitMQError::PublishFailed(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Publish message with retry logic
    pub async fn publish_with_retry<T: Serialize>(
        &self,
        routing_key: &str,
        message: &T,
        max_retries: u32,
    ) -> Result<()> {
        let mut attempts = 0;
        let mut delay_ms = self.config.reconnect.initial_delay_ms;

        loop {
            match self.publish(routing_key, message).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        return Err(e);
                    }

                    tracing::warn!(
                        "Publish failed (attempt {}/{}): {}. Retrying in {}ms",
                        attempts,
                        max_retries,
                        e,
                        delay_ms
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                    // Exponential backoff
                    delay_ms = (delay_ms as f64 * self.config.reconnect.backoff_multiplier) as u64;
                    delay_ms = delay_ms.min(self.config.reconnect.max_delay_ms);

                    // Try to reconnect if not connected
                    if !self.is_connected() {
                        let _ = self.reconnect().await;
                    }
                }
            }
        }
    }

    /// Attempt to reconnect
    pub async fn reconnect(&self) -> Result<()> {
        tracing::info!("Attempting to reconnect to RabbitMQ");

        // Disconnect first
        let _ = self.disconnect().await;

        // Reconnect
        match self.connect().await {
            Ok(_) => {
                self.reconnect_count.fetch_add(1, Ordering::Relaxed);
                tracing::info!("Successfully reconnected to RabbitMQ");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Reconnection failed: {}", e);
                Err(e)
            }
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Acquire)
    }

    /// Get publisher statistics
    pub fn stats(&self) -> PublisherStats {
        PublisherStats {
            messages_published: self.messages_published.load(Ordering::Relaxed),
            messages_confirmed: self.messages_confirmed.load(Ordering::Relaxed),
            messages_failed: self.messages_failed.load(Ordering::Relaxed),
            is_connected: self.is_connected(),
            reconnect_count: self.reconnect_count.load(Ordering::Relaxed),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.messages_published.store(0, Ordering::Relaxed);
        self.messages_confirmed.store(0, Ordering::Relaxed);
        self.messages_failed.store(0, Ordering::Relaxed);
    }

    /// Parse exchange type from string
    fn parse_exchange_type(&self) -> ExchangeKind {
        match self.config.exchange_type.to_lowercase().as_str() {
            "direct" => ExchangeKind::Direct,
            "fanout" => ExchangeKind::Fanout,
            "headers" => ExchangeKind::Headers,
            _ => ExchangeKind::Topic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publisher_creation() {
        let config = RabbitMQConfig::default();
        let publisher = RabbitMQPublisher::new(config);

        assert!(!publisher.is_connected());

        let stats = publisher.stats();
        assert_eq!(stats.messages_published, 0);
        assert_eq!(stats.messages_confirmed, 0);
        assert_eq!(stats.messages_failed, 0);
    }

    #[test]
    fn test_exchange_type_parsing() {
        let mut config = RabbitMQConfig::default();

        config.exchange_type = "topic".to_string();
        let publisher = RabbitMQPublisher::new(config.clone());
        assert!(matches!(publisher.parse_exchange_type(), ExchangeKind::Topic));

        config.exchange_type = "direct".to_string();
        let publisher = RabbitMQPublisher::new(config.clone());
        assert!(matches!(publisher.parse_exchange_type(), ExchangeKind::Direct));

        config.exchange_type = "fanout".to_string();
        let publisher = RabbitMQPublisher::new(config);
        assert!(matches!(publisher.parse_exchange_type(), ExchangeKind::Fanout));
    }
}
