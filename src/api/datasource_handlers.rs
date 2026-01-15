use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::datasource::DatasourceManager;
use crate::models::datasource::*;
use super::responses::ErrorResponse;

/// Shared state for datasource endpoints
pub type DatasourceState = Arc<DatasourceManager>;

/// Start FIX connection
#[utoipa::path(
    post,
    path = "/api/v1/datasource/start",
    request_body = StartDatasourceRequest,
    responses(
        (status = 200, description = "FIX connection started successfully", body = StartDatasourceResponse),
        (status = 400, description = "Invalid request or already connected", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    tag = "datasource"
)]
pub async fn start_datasource(
    State(manager): State<DatasourceState>,
    Json(request): Json<StartDatasourceRequest>,
) -> Result<Json<StartDatasourceResponse>, DatasourceError> {
    let config = FixConfig {
        host: request.host,
        port: request.port,
        credentials: request.credentials,
    };

    manager.start_live_fix(config).await.map_err(|e| {
        DatasourceError::StartFailed(e)
    })?;

    Ok(Json(StartDatasourceResponse {
        status: "connecting".to_string(),
        message: "FIX connection initiated. Fetching symbol list...".to_string(),
    }))
}

/// Stop FIX connection
#[utoipa::path(
    post,
    path = "/api/v1/datasource/stop",
    responses(
        (status = 200, description = "FIX connection stopped successfully", body = StopDatasourceResponse),
        (status = 400, description = "No active connection to stop", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    tag = "datasource"
)]
pub async fn stop_datasource(
    State(manager): State<DatasourceState>,
) -> Result<Json<StopDatasourceResponse>, DatasourceError> {
    manager.stop().await.map_err(|e| {
        DatasourceError::StopFailed(e)
    })?;

    Ok(Json(StopDatasourceResponse {
        status: "stopped".to_string(),
        message: "FIX connection stopped successfully".to_string(),
    }))
}

/// Get datasource status
#[utoipa::path(
    get,
    path = "/api/v1/datasource/status",
    responses(
        (status = 200, description = "Current datasource status", body = DatasourceStatus),
    ),
    tag = "datasource"
)]
pub async fn get_datasource_status(
    State(manager): State<DatasourceState>,
) -> Json<DatasourceStatus> {
    Json(manager.get_status().await)
}

/// Get system health
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "System health status", body = HealthStatus),
    ),
    tag = "health"
)]
pub async fn get_health(
    State(manager): State<DatasourceState>,
) -> Json<HealthStatus> {
    Json(manager.get_health().await)
}


/// Datasource-specific errors
#[derive(Debug)]
pub enum DatasourceError {
    StartFailed(String),
    StopFailed(String),
}

impl IntoResponse for DatasourceError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match self {
            DatasourceError::StartFailed(msg) => {
                if msg.contains("Already connected") {
                    (StatusCode::BAD_REQUEST, "already_connected", msg)
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "start_failed", msg)
                }
            }
            DatasourceError::StopFailed(msg) => {
                if msg.contains("No active") {
                    (StatusCode::BAD_REQUEST, "not_connected", msg)
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "stop_failed", msg)
                }
            }
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message: error_message,
        };

        (status, Json(error_response)).into_response()
    }
}
