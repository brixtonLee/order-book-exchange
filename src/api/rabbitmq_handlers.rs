use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::AppState;
use crate::rabbitmq::{RabbitMQConfig, PublisherStats};

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
    pub stats: Option<PublisherStats>,
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
    State(state): State<AppState>,
    Json(request): Json<RabbitMQConnectRequest>,
) -> Result<Json<RabbitMQConnectResponse>, StatusCode> {
    match state.datasource_manager.connect_rabbitmq(request.config).await {
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
    State(state): State<AppState>,
) -> Json<RabbitMQStatusResponse> {
    let connected = state.datasource_manager.is_rabbitmq_connected().await;
    let stats = state.datasource_manager.get_rabbitmq_stats().await;

    // Get exchange name from config if available
    let exchange = if connected {
        Some("market.data".to_string()) // Default from config
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
    State(state): State<AppState>,
) -> Json<RabbitMQConnectResponse> {
    match state.datasource_manager.disconnect_rabbitmq().await {
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
