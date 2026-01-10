use crate::database::enums::Timeframe;
use crate::database::models::{OhlcCandle, Symbol, Tick};
use crate::database::repositories::{OhlcRepository, SymbolRepository, TickRepository};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

/// Shared state for database API handlers
#[derive(Clone)]
pub struct DatabaseState {
    pub symbol_repository: Arc<dyn SymbolRepository>,
    pub tick_repository: Arc<dyn TickRepository>,
    pub ohlc_repository: Arc<dyn OhlcRepository>,
}

// ============================================================================
// Symbol Endpoints
// ============================================================================

/// Get all symbols
#[utoipa::path(
    get,
    path = "/api/v1/symbols",
    tag = "symbols",
    responses(
        (status = 200, description = "List of all symbols", body = Vec<Symbol>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_symbols(
    State(state): State<DatabaseState>,
) -> Result<Json<Vec<Symbol>>, (StatusCode, String)> {
    state
        .symbol_repository
        .get_all()
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get symbols: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

/// Get symbol by ID
#[utoipa::path(
    get,
    path = "/api/v1/symbols/{symbol_id}",
    tag = "symbols",
    params(
        ("symbol_id" = i64, Path, description = "Symbol ID")
    ),
    responses(
        (status = 200, description = "Symbol details", body = Symbol),
        (status = 404, description = "Symbol not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_symbol_by_id(
    State(state): State<DatabaseState>,
    Path(symbol_id): Path<i64>,
) -> Result<Json<Symbol>, (StatusCode, String)> {
    state
        .symbol_repository
        .find_by_id(symbol_id)
        .map_err(|e| {
            tracing::error!("Failed to get symbol {}: {}", symbol_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Symbol {} not found", symbol_id)))
}

/// Get symbol by name
#[utoipa::path(
    get,
    path = "/api/v1/symbols/name/{symbol_name}",
    tag = "symbols",
    params(
        ("symbol_name" = String, Path, description = "Symbol name (e.g., EURUSD)")
    ),
    responses(
        (status = 200, description = "Symbol details", body = Symbol),
        (status = 404, description = "Symbol not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_symbol_by_name(
    State(state): State<DatabaseState>,
    Path(symbol_name): Path<String>,
) -> Result<Json<Symbol>, (StatusCode, String)> {
    state
        .symbol_repository
        .find_by_name(&symbol_name)
        .map_err(|e| {
            tracing::error!("Failed to get symbol {}: {}", symbol_name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Symbol {} not found", symbol_name)))
}

// ============================================================================
// Tick Endpoints
// ============================================================================

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TickQueryParams {
    /// Start time (RFC3339 format)
    pub from: Option<String>,
    /// End time (RFC3339 format)
    pub to: Option<String>,
    /// Maximum number of ticks to return
    #[serde(default = "default_tick_limit")]
    pub limit: i64,
}

fn default_tick_limit() -> i64 {
    1000
}

/// Get ticks for a symbol
#[utoipa::path(
    get,
    path = "/api/v1/ticks/{symbol_id}",
    tag = "ticks",
    params(
        ("symbol_id" = i64, Path, description = "Symbol ID"),
        TickQueryParams
    ),
    responses(
        (status = 200, description = "List of ticks", body = Vec<Tick>),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_ticks(
    State(state): State<DatabaseState>,
    Path(symbol_id): Path<i64>,
    Query(params): Query<TickQueryParams>,
) -> Result<Json<Vec<Tick>>, (StatusCode, String)> {
    // Parse timestamps
    let from = params
        .from
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid 'from' timestamp: {}", e),
            )
        })?
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));

    let to = params
        .to
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid 'to' timestamp: {}", e),
            )
        })?
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    state
        .tick_repository
        .get_by_symbol_and_time_range(symbol_id, from, to, Some(params.limit))
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get ticks: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

/// Get latest tick for a symbol
#[utoipa::path(
    get,
    path = "/api/v1/ticks/{symbol_id}/latest",
    tag = "ticks",
    params(
        ("symbol_id" = i64, Path, description = "Symbol ID")
    ),
    responses(
        (status = 200, description = "Latest tick", body = Tick),
        (status = 404, description = "No ticks found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_latest_tick(
    State(state): State<DatabaseState>,
    Path(symbol_id): Path<i64>,
) -> Result<Json<Tick>, (StatusCode, String)> {
    state
        .tick_repository
        .get_latest(symbol_id)
        .map_err(|e| {
            tracing::error!("Failed to get latest tick: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No ticks found".to_string()))
}

// ============================================================================
// OHLC Endpoints
// ============================================================================

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct OhlcQueryParams {
    /// Timeframe: 1m, 5m, 15m, 30m, 1h, 4h, 1d
    pub timeframe: String,
    /// Start time (RFC3339 format)
    pub from: Option<String>,
    /// End time (RFC3339 format)
    pub to: Option<String>,
    /// Maximum number of candles to return
    #[serde(default = "default_ohlc_limit")]
    pub limit: i64,
}

fn default_ohlc_limit() -> i64 {
    500
}

/// Get OHLC candles for a symbol
#[utoipa::path(
    get,
    path = "/api/v1/ohlc/{symbol_id}",
    tag = "ohlc",
    params(
        ("symbol_id" = i64, Path, description = "Symbol ID"),
        OhlcQueryParams
    ),
    responses(
        (status = 200, description = "List of OHLC candles", body = Vec<OhlcCandle>),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_ohlc_candles(
    State(state): State<DatabaseState>,
    Path(symbol_id): Path<i64>,
    Query(params): Query<OhlcQueryParams>,
) -> Result<Json<Vec<OhlcCandle>>, (StatusCode, String)> {
    // Parse timeframe
    let timeframe = Timeframe::from_str(&params.timeframe).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid timeframe '{}'. Valid values: 1m, 5m, 15m, 30m, 1h, 4h, 1d",
                params.timeframe
            ),
        )
    })?;

    // Parse timestamps
    let from = params
        .from
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid 'from' timestamp: {}", e),
            )
        })?
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));

    let to = params
        .to
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid 'to' timestamp: {}", e),
            )
        })?
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    state
        .ohlc_repository
        .get_candles(symbol_id, timeframe, from, to, Some(params.limit))
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get OHLC candles: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

/// Get latest OHLC candle for a symbol
#[utoipa::path(
    get,
    path = "/api/v1/ohlc/{symbol_id}/latest",
    tag = "ohlc",
    params(
        ("symbol_id" = i64, Path, description = "Symbol ID"),
        ("timeframe" = String, Query, description = "Timeframe: 1m, 5m, 15m, 30m, 1h, 4h, 1d")
    ),
    responses(
        (status = 200, description = "Latest OHLC candle", body = OhlcCandle),
        (status = 400, description = "Invalid timeframe"),
        (status = 404, description = "No candles found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_latest_ohlc_candle(
    State(state): State<DatabaseState>,
    Path(symbol_id): Path<i64>,
    Query(params): Query<LatestOhlcQueryParams>,
) -> Result<Json<OhlcCandle>, (StatusCode, String)> {
    // Parse timeframe
    let timeframe = Timeframe::from_str(&params.timeframe).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid timeframe '{}'. Valid values: 1m, 5m, 15m, 30m, 1h, 4h, 1d",
                params.timeframe
            ),
        )
    })?;

    state
        .ohlc_repository
        .get_latest_candle(symbol_id, timeframe)
        .map_err(|e| {
            tracing::error!("Failed to get latest OHLC candle: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No candles found".to_string()))
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct LatestOhlcQueryParams {
    pub timeframe: String,
}
