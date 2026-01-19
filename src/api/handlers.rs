use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::engine::{OrderBookEngine, OrderBookError};
use crate::metrics::{calculate_spread_metrics, MicrostructureMetrics};
use crate::models::Order;

use super::responses::*;

/// Shared application state
pub type AppState = Arc<OrderBookEngine>;

/// Query parameters for order book depth
#[derive(Debug, Deserialize)]
pub struct DepthQuery {
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    10
}

/// Query parameters for trade list
#[derive(Debug, Deserialize)]
pub struct TradeQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Convert OrderBookError to HTTP response
impl IntoResponse for OrderBookError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            OrderBookError::OrderNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            OrderBookError::InvalidPrice(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::InvalidQuantity(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::InvalidExpireTime(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::InsufficientLiquidity => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::SelfTrade => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::DuplicateOrder(_) => (StatusCode::CONFLICT, self.to_string()),
            OrderBookError::InvalidSymbol(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::OrderNotActive(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OrderBookError::MatchingError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        let body = Json(ErrorResponse {
            error: status.to_string(),
            message: error_message,
        });

        (status, body).into_response()
    }
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

/// Submit a new order
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    tag = "Orders",
    request_body = SubmitOrderRequest,
    responses(
        (status = 201, description = "Order submitted successfully", body = SubmitOrderResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Duplicate order", body = ErrorResponse)
    )
)]
pub async fn submit_order(
    State(engine): State<AppState>,
    Json(request): Json<SubmitOrderRequest>,
) -> Result<(StatusCode, Json<SubmitOrderResponse>), OrderBookError> {
    // Create iceberg config if both fields are provided
    let iceberg = if let (Some(total), Some(display)) = (request.iceberg_total_quantity, request.iceberg_display_quantity) {
        Some(crate::models::IcebergConfig::new(total, display))
    } else {
        None
    };

    // Create order with all options
    let mut order = Order::new_with_options(
        request.symbol,
        request.side,
        request.order_type,
        request.price,
        request.quantity,
        request.user_id,
        request.time_in_force,
        request.stp_mode,
        request.post_only,
        request.expire_time,
    );

    // Attach iceberg config if provided
    if let Some(iceberg_config) = iceberg {
        order.iceberg = Some(iceberg_config);
        // Override quantity with display quantity for iceberg orders
        order.quantity = request.iceberg_display_quantity.unwrap();
    }

    // Add to engine
    let (filled_order, trades) = engine.add_order(order)?;

    // Convert trades to response format
    let trade_responses: Vec<TradeResponse> = trades.into_iter().map(|t| t.into()).collect();

    let response = SubmitOrderResponse {
        order_id: filled_order.id,
        status: filled_order.status,
        filled_quantity: filled_order.filled_quantity,
        trades: trade_responses,
        timestamp: filled_order.timestamp,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get order status
#[utoipa::path(
    get,
    path = "/api/v1/orders/{symbol}/{order_id}",
    tag = "Orders",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)"),
        ("order_id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order found", body = OrderResponse),
        (status = 404, description = "Order not found", body = ErrorResponse)
    )
)]
pub async fn get_order(
    State(engine): State<AppState>,
    Path((symbol, order_id)): Path<(String, Uuid)>,
) -> Result<Json<OrderResponse>, OrderBookError> {
    let order = engine.get_order(&symbol, order_id)?;
    Ok(Json(order.into()))
}

/// Cancel an order
#[utoipa::path(
    delete,
    path = "/api/v1/orders/{symbol}/{order_id}",
    tag = "Orders",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)"),
        ("order_id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order cancelled", body = CancelOrderResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 400, description = "Order cannot be cancelled", body = ErrorResponse)
    )
)]
pub async fn cancel_order(
    State(engine): State<AppState>,
    Path((symbol, order_id)): Path<(String, Uuid)>,
) -> Result<Json<CancelOrderResponse>, OrderBookError> {
    let cancelled_order = engine.cancel_order(&symbol, order_id)?;

    let response = CancelOrderResponse {
        order_id: cancelled_order.id,
        status: cancelled_order.status,
        filled_quantity: cancelled_order.filled_quantity,
        remaining_quantity: cancelled_order.remaining_quantity(),
    };

    Ok(Json(response))
}

/// Convert a price level to response format
fn price_level_to_response(level: &crate::models::PriceLevel) -> PriceLevelResponse {
    PriceLevelResponse {
        price: level.price,
        quantity: level.total_quantity,
        orders: level.orders.len(),
    }
}

/// Get order book
#[utoipa::path(
    get,
    path = "/api/v1/orderbook/{symbol}",
    tag = "Order Book",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)"),
        ("depth" = Option<usize>, Query, description = "Number of price levels to return (default: 10)")
    ),
    responses(
        (status = 200, description = "Order book data", body = OrderBookResponse)
    )
)]
pub async fn get_order_book(
    State(engine): State<AppState>,
    Path(symbol): Path<String>,
    Query(params): Query<DepthQuery>,
) -> Json<OrderBookResponse> {
    let book = engine.get_order_book(&symbol);

    // Convert bids (take top N, highest to lowest)
    let bids: Vec<PriceLevelResponse> = book
        .bids
        .iter()
        .rev()
        .take(params.depth)
        .map(|(_, level)| price_level_to_response(level))
        .collect();

    // Convert asks (take top N, lowest to highest)
    let asks: Vec<PriceLevelResponse> = book
        .asks
        .iter()
        .take(params.depth)
        .map(|(_, level)| price_level_to_response(level))
        .collect();

    let response = OrderBookResponse {
        symbol: book.symbol.clone(),
        timestamp: Utc::now(),
        bids,
        asks,
        best_bid: book.get_best_bid(),
        best_ask: book.get_best_ask(),
        spread: book.get_spread(),
        spread_bps: book.get_spread_bps(),
        mid_price: book.get_mid_price(),
    };

    Json(response)
}

/// Get spread metrics
#[utoipa::path(
    get,
    path = "/api/v1/orderbook/{symbol}/spread",
    tag = "Order Book",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)")
    ),
    responses(
        (status = 200, description = "Spread metrics", body = SpreadMetricsResponse)
    )
)]
pub async fn get_spread_metrics(
    State(engine): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<SpreadMetricsResponse> {
    let book = engine.get_order_book(&symbol);
    let metrics = calculate_spread_metrics(&book);
    Json(metrics)
}

/// Get recent trades
#[utoipa::path(
    get,
    path = "/api/v1/trades/{symbol}",
    tag = "Trades",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)"),
        ("limit" = Option<usize>, Query, description = "Number of trades to return (default: 50)")
    ),
    responses(
        (status = 200, description = "Recent trades", body = TradeListResponse)
    )
)]
pub async fn get_trades(
    State(engine): State<AppState>,
    Path(symbol): Path<String>,
    Query(params): Query<TradeQuery>,
) -> Json<TradeListResponse> {
    let trades = engine.get_recent_trades(&symbol, params.limit);

    let trade_responses: Vec<TradeResponse> = trades.into_iter().map(|t| t.into()).collect();

    let response = TradeListResponse {
        symbol,
        trades: trade_responses.clone(),
        count: trade_responses.len(),
    };

    Json(response)
}

/// Get exchange metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics/exchange",
    tag = "Metrics",
    responses(
        (status = 200, description = "Exchange-wide metrics", body = ExchangeMetricsResponse)
    )
)]
pub async fn get_exchange_metrics(State(engine): State<AppState>) -> Json<ExchangeMetricsResponse> {
    let response = ExchangeMetricsResponse {
        total_trades: engine.get_total_trades(),
        total_volume: engine.get_total_volume(),
        total_fees_collected: engine.get_total_fees(),
        active_orders: engine.get_total_active_orders(),
        symbols: engine.get_symbols(),
    };

    Json(response)
}

/// Get order book microstructure metrics
#[utoipa::path(
    get,
    path = "/api/v1/orderbook/{symbol}/microstructure",
    tag = "Metrics",
    params(
        ("symbol" = String, Path, description = "Trading symbol (e.g., AAPL)"),
        ("depth" = Option<usize>, Query, description = "Number of price levels to analyze (default: 5)")
    ),
    responses(
        (status = 200, description = "Microstructure metrics", body = MicrostructureMetrics)
    )
)]
pub async fn get_microstructure_metrics(
    State(engine): State<AppState>,
    Path(symbol): Path<String>,
    Query(params): Query<DepthQuery>,
) -> Result<Json<MicrostructureMetrics>, StatusCode> {
    let book = engine.get_order_book(&symbol);

    let depth = if params.depth == default_depth() { 5 } else { params.depth };

    match MicrostructureMetrics::from_order_book(&book, depth) {
        Some(metrics) => Ok(Json(metrics)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
