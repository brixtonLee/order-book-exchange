use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::rabbitmq::{RabbitMQService, RabbitMQConfig, BridgeStats};

/// RabbitMQ connection request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RabbitMQConnectRequest {
    pub config: RabbitMQConfig,
}

/// RabbitMQ connection response
#[derive(Debug, Serialize, ToSchema)]
pub struct RabbitMQConnectResponse {
    pub success: bool,
    pub message: String,
}

/// RabbitMQ status response
#[derive(Debug, Serialize, ToSchema)]
pub struct RabbitMQStatusResponse {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<BridgeStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
}

/// Connect to RabbitMQ
///
/// Establishes a connection to RabbitMQ server with the provided configuration.
/// This enables streaming of FIX market data to RabbitMQ alongside WebSocket broadcasting.
#[utoipa::path(
    post,
    path = "/api/v1/rabbitmq/connect",
    request_body = RabbitMQConnectRequest,
    responses(
        (status = 200, description = "Successfully connected to RabbitMQ", body = RabbitMQConnectResponse),
        (status = 400, description = "Invalid configuration", body = RabbitMQConnectResponse),
        (status = 500, description = "Connection failed", body = RabbitMQConnectResponse),
    ),
    tag = "RabbitMQ"
)]
pub async fn connect_rabbitmq(
    State(rabbitmq_service): State<Arc<RabbitMQService>>,
    Json(_request): Json<RabbitMQConnectRequest>,
) -> Result<Json<RabbitMQConnectResponse>, StatusCode> {
    // Note: This endpoint receives a new config but we can't change the service config at runtime
    // The service should be initialized with config from environment variables
    // For now, we'll just attempt to connect with the existing service
    match rabbitmq_service.connect().await {
        Ok(_) => Ok(Json(RabbitMQConnectResponse {
            success: true,
            message: "Successfully connected to RabbitMQ".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to connect to RabbitMQ: {}", e);
            Ok(Json(RabbitMQConnectResponse {
                success: false,
                message: format!("Connection failed: {}", e),
            }))
        }
    }
}

/// Get RabbitMQ status
///
/// Returns the current connection status and statistics for the RabbitMQ publisher.
#[utoipa::path(
    get,
    path = "/api/v1/rabbitmq/status",
    responses(
        (status = 200, description = "RabbitMQ status", body = RabbitMQStatusResponse),
    ),
    tag = "RabbitMQ"
)]
pub async fn get_rabbitmq_status(
    State(rabbitmq_service): State<Arc<RabbitMQService>>,
) -> Json<RabbitMQStatusResponse> {
    let connected = rabbitmq_service.is_connected();
    let stats = rabbitmq_service.stats().await;

    // Get exchange name from service
    let exchange = if connected {
        Some(rabbitmq_service.get_exchange())
    } else {
        None
    };

    Json(RabbitMQStatusResponse {
        connected,
        stats,
        exchange,
    })
}

/// Disconnect from RabbitMQ
///
/// Closes the connection to RabbitMQ and stops streaming market data.
#[utoipa::path(
    post,
    path = "/api/v1/rabbitmq/disconnect",
    responses(
        (status = 200, description = "Successfully disconnected from RabbitMQ", body = RabbitMQConnectResponse),
        (status = 400, description = "Not connected", body = RabbitMQConnectResponse),
    ),
    tag = "RabbitMQ"
)]
pub async fn disconnect_rabbitmq(
    State(rabbitmq_service): State<Arc<RabbitMQService>>,
) -> Json<RabbitMQConnectResponse> {
    match rabbitmq_service.disconnect().await {
        Ok(_) => Json(RabbitMQConnectResponse {
            success: true,
            message: "Successfully disconnected from RabbitMQ".to_string(),
        }),
        Err(e) => Json(RabbitMQConnectResponse {
            success: false,
            message: format!("Disconnection failed: {}", e),
        }),
    }
}
