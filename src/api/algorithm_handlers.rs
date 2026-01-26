use crate::algorithms::{AlgorithmManager, AlgorithmStatus, TwapAlgorithm, TwapStats, VwapAlgorithm, VwapStats};
use crate::models::OrderSide;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Shared state for algorithm endpoints
#[derive(Clone)]
pub struct AlgorithmState {
    pub manager: Arc<AlgorithmManager>,
}

/// Request to submit a TWAP algorithm
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitTwapRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,
    pub total_quantity: Decimal,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub slice_interval_seconds: i64,
    pub limit_price: Option<Decimal>,
    pub urgency: Option<Decimal>,
}

/// Request to submit a VWAP algorithm
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitVwapRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,
    pub total_quantity: Decimal,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

/// Generic algorithm response
#[derive(Debug, Serialize, ToSchema)]
pub struct AlgorithmResponse {
    pub algorithm_id: Uuid,
    pub algorithm_type: String,
    pub status: AlgorithmStatus,
    pub message: String,
}

/// TWAP status response
#[derive(Debug, Serialize, ToSchema)]
pub struct TwapStatusResponse {
    pub algorithm: TwapAlgorithm,
    pub stats: TwapStats,
}

/// VWAP status response
#[derive(Debug, Serialize, ToSchema)]
pub struct VwapStatusResponse {
    pub algorithm: VwapAlgorithm,
    pub stats: VwapStats,
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Submit a TWAP algorithm
#[utoipa::path(
    post,
    path = "/api/v1/algorithms/twap",
    request_body = SubmitTwapRequest,
    responses(
        (status = 201, description = "TWAP algorithm submitted", body = AlgorithmResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn submit_twap(
    State(state): State<Arc<AlgorithmState>>,
    Json(request): Json<SubmitTwapRequest>,
) -> impl IntoResponse {
    // Create TWAP algorithm
    let mut twap = TwapAlgorithm::new(
        request.symbol.clone(),
        request.side,
        request.user_id.clone(),
        request.total_quantity,
        request.start_time,
        request.end_time,
        request.slice_interval_seconds,
    );

    twap.limit_price = request.limit_price;
    twap.urgency = request.urgency.unwrap_or(Decimal::ONE);

    match state.manager.submit_twap(twap) {
        Ok(algorithm_id) => (
            StatusCode::CREATED,
            Json(AlgorithmResponse {
                algorithm_id,
                algorithm_type: "TWAP".to_string(),
                status: AlgorithmStatus::Running,
                message: "TWAP algorithm submitted successfully".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AlgorithmResponse {
                algorithm_id: uuid::Uuid::nil(),
                algorithm_type: "TWAP".to_string(),
                status: AlgorithmStatus::Cancelled,
                message: format!("Failed to submit TWAP algorithm: {}", e),
            }),
        ),
    }
}

/// Submit a VWAP algorithm
#[utoipa::path(
    post,
    path = "/api/v1/algorithms/vwap",
    request_body = SubmitVwapRequest,
    responses(
        (status = 201, description = "VWAP algorithm submitted", body = AlgorithmResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn submit_vwap(
    State(state): State<Arc<AlgorithmState>>,
    Json(request): Json<SubmitVwapRequest>,
) -> impl IntoResponse {
    // Create VWAP algorithm with US equity volume profile
    let vwap = VwapAlgorithm::new(
        request.symbol.clone(),
        request.side,
        request.user_id.clone(),
        request.total_quantity,
        request.start_time,
        request.end_time,
    );

    match state.manager.submit_vwap(vwap) {
        Ok(algorithm_id) => (
            StatusCode::CREATED,
            Json(AlgorithmResponse {
                algorithm_id,
                algorithm_type: "VWAP".to_string(),
                status: AlgorithmStatus::Running,
                message: "VWAP algorithm submitted successfully".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AlgorithmResponse {
                algorithm_id: uuid::Uuid::nil(),
                algorithm_type: "VWAP".to_string(),
                status: AlgorithmStatus::Cancelled,
                message: format!("Failed to submit VWAP algorithm: {}", e),
            }),
        ),
    }
}

/// Get TWAP algorithm status
#[utoipa::path(
    get,
    path = "/api/v1/algorithms/twap/{algorithm_id}",
    params(
        ("algorithm_id" = Uuid, Path, description = "Algorithm ID")
    ),
    responses(
        (status = 200, description = "TWAP status", body = TwapStatusResponse),
        (status = 404, description = "Algorithm not found", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn get_twap_status(
    State(state): State<Arc<AlgorithmState>>,
    Path(algorithm_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.manager.get_twap(algorithm_id) {
        Ok(Some(algo)) => {
            let stats = algo.execution_stats();
            (
                StatusCode::OK,
                Json(TwapStatusResponse {
                    algorithm: algo,
                    stats,
                }),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("TWAP algorithm {} not found", algorithm_id),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get TWAP algorithm: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get VWAP algorithm status
#[utoipa::path(
    get,
    path = "/api/v1/algorithms/vwap/{algorithm_id}",
    params(
        ("algorithm_id" = Uuid, Path, description = "Algorithm ID")
    ),
    responses(
        (status = 200, description = "VWAP status", body = VwapStatusResponse),
        (status = 404, description = "Algorithm not found", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn get_vwap_status(
    State(state): State<Arc<AlgorithmState>>,
    Path(algorithm_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.manager.get_vwap(algorithm_id) {
        Ok(Some(algo)) => {
            let stats = algo.stats();
            (
                StatusCode::OK,
                Json(VwapStatusResponse {
                    algorithm: algo,
                    stats,
                }),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("VWAP algorithm {} not found", algorithm_id),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get VWAP algorithm: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Pause an algorithm
#[utoipa::path(
    post,
    path = "/api/v1/algorithms/{algorithm_id}/pause",
    params(
        ("algorithm_id" = Uuid, Path, description = "Algorithm ID")
    ),
    responses(
        (status = 200, description = "Algorithm paused", body = AlgorithmResponse),
        (status = 404, description = "Algorithm not found", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn pause_algorithm(
    State(state): State<Arc<AlgorithmState>>,
    Path(algorithm_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.manager.pause(algorithm_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(AlgorithmResponse {
                algorithm_id,
                algorithm_type: "Unknown".to_string(),
                status: AlgorithmStatus::Paused,
                message: "Algorithm paused successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: e }),
        )
            .into_response(),
    }
}

/// Resume a paused algorithm
#[utoipa::path(
    post,
    path = "/api/v1/algorithms/{algorithm_id}/resume",
    params(
        ("algorithm_id" = Uuid, Path, description = "Algorithm ID")
    ),
    responses(
        (status = 200, description = "Algorithm resumed", body = AlgorithmResponse),
        (status = 404, description = "Algorithm not found", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn resume_algorithm(
    State(state): State<Arc<AlgorithmState>>,
    Path(algorithm_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.manager.resume(algorithm_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(AlgorithmResponse {
                algorithm_id,
                algorithm_type: "Unknown".to_string(),
                status: AlgorithmStatus::Running,
                message: "Algorithm resumed successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: e }),
        )
            .into_response(),
    }
}

/// Cancel an algorithm
#[utoipa::path(
    post,
    path = "/api/v1/algorithms/{algorithm_id}/cancel",
    params(
        ("algorithm_id" = Uuid, Path, description = "Algorithm ID")
    ),
    responses(
        (status = 200, description = "Algorithm cancelled", body = AlgorithmResponse),
        (status = 404, description = "Algorithm not found", body = ErrorResponse)
    ),
    tag = "algorithms"
)]
pub async fn cancel_algorithm(
    State(state): State<Arc<AlgorithmState>>,
    Path(algorithm_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.manager.cancel(algorithm_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(AlgorithmResponse {
                algorithm_id,
                algorithm_type: "Unknown".to_string(),
                status: AlgorithmStatus::Cancelled,
                message: "Algorithm cancelled successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: e }),
        )
            .into_response(),
    }
}
