use crate::engine::OrderBookEngine;
use crate::models::order::{OrderSide, SelfTradePreventionMode, TimeInForce};
use crate::models::stop_order::{StopOrder, StopOrderStatus, StopOrderType, TriggerCondition};
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

/// Request to submit a stop order
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitStopOrderRequest {
    pub symbol: String,
    pub user_id: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub trigger_price: Decimal,
    pub trigger_condition: TriggerCondition,
    pub stop_type: StopOrderType,

    // Optional fields
    pub limit_price: Option<Decimal>,
    pub trail_amount: Option<Decimal>,
    pub trail_percent: Option<Decimal>,
    pub time_in_force: Option<TimeInForce>,
    pub stp_mode: Option<SelfTradePreventionMode>,
    pub post_only: Option<bool>,
    pub expire_time: Option<DateTime<Utc>>,
}

/// Stop order response
#[derive(Debug, Serialize, ToSchema)]
pub struct StopOrderResponse {
    pub stop_order_id: Uuid,
    pub symbol: String,
    pub user_id: String,
    pub status: StopOrderStatus,
    pub message: String,
}

/// Detailed stop order response
#[derive(Debug, Serialize, ToSchema)]
pub struct StopOrderDetailResponse {
    pub id: Uuid,
    pub symbol: String,
    pub user_id: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub trigger_price: Decimal,
    pub trigger_condition: TriggerCondition,
    pub stop_type: StopOrderType,
    pub limit_price: Option<Decimal>,
    pub trail_amount: Option<Decimal>,
    pub trail_percent: Option<Decimal>,
    pub highest_price: Option<Decimal>,
    pub lowest_price: Option<Decimal>,
    pub status: StopOrderStatus,
    pub created_at: DateTime<Utc>,
    pub expire_time: Option<DateTime<Utc>>,
    pub time_in_force: TimeInForce,
    pub stp_mode: SelfTradePreventionMode,
    pub post_only: bool,
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Submit a stop order
#[utoipa::path(
    post,
    path = "/api/v1/stop-orders",
    request_body = SubmitStopOrderRequest,
    responses(
        (status = 201, description = "Stop order submitted", body = StopOrderResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "stop-orders"
)]
pub async fn submit_stop_order(
    State(engine): State<Arc<OrderBookEngine>>,
    Json(request): Json<SubmitStopOrderRequest>,
) -> impl IntoResponse {
    // Validate stop type requirements
    if request.stop_type == StopOrderType::StopLimit && request.limit_price.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "StopLimit orders require limit_price".to_string(),
            }),
        )
            .into_response();
    }

    if request.stop_type == StopOrderType::TrailingStop
        && request.trail_amount.is_none()
        && request.trail_percent.is_none()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "TrailingStop orders require trail_amount or trail_percent".to_string(),
            }),
        )
            .into_response();
    }

    // Create stop order
    let stop_order = StopOrder {
        id: Uuid::new_v4(),
        symbol: request.symbol.clone(),
        user_id: request.user_id.clone(),
        side: request.side,
        quantity: request.quantity,
        trigger_price: request.trigger_price,
        trigger_condition: request.trigger_condition,
        stop_type: request.stop_type,
        limit_price: request.limit_price,
        trail_amount: request.trail_amount,
        trail_percent: request.trail_percent,
        highest_price: if request.stop_type == StopOrderType::TrailingStop {
            Some(request.trigger_price)
        } else {
            None
        },
        lowest_price: if request.stop_type == StopOrderType::TrailingStop {
            Some(request.trigger_price)
        } else {
            None
        },
        status: StopOrderStatus::Pending,
        created_at: Utc::now(),
        expire_time: request.expire_time,
        time_in_force: request.time_in_force.unwrap_or(TimeInForce::GTC),
        stp_mode: request
            .stp_mode
            .unwrap_or(SelfTradePreventionMode::None),
        post_only: request.post_only.unwrap_or(false),
    };

    let stop_order_id = stop_order.id;

    match engine.add_stop_order(stop_order) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(StopOrderResponse {
                stop_order_id,
                symbol: request.symbol,
                user_id: request.user_id,
                status: StopOrderStatus::Pending,
                message: "Stop order submitted successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to submit stop order: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get stop order details
#[utoipa::path(
    get,
    path = "/api/v1/stop-orders/{stop_order_id}",
    params(
        ("stop_order_id" = Uuid, Path, description = "Stop order ID")
    ),
    responses(
        (status = 200, description = "Stop order details", body = StopOrderDetailResponse),
        (status = 404, description = "Stop order not found", body = ErrorResponse)
    ),
    tag = "stop-orders"
)]
pub async fn get_stop_order(
    State(engine): State<Arc<OrderBookEngine>>,
    Path(stop_order_id): Path<Uuid>,
) -> impl IntoResponse {
    match engine.get_stop_order(stop_order_id) {
        Some(stop_order) => (
            StatusCode::OK,
            Json(StopOrderDetailResponse {
                id: stop_order.id,
                symbol: stop_order.symbol,
                user_id: stop_order.user_id,
                side: stop_order.side,
                quantity: stop_order.quantity,
                trigger_price: stop_order.trigger_price,
                trigger_condition: stop_order.trigger_condition,
                stop_type: stop_order.stop_type,
                limit_price: stop_order.limit_price,
                trail_amount: stop_order.trail_amount,
                trail_percent: stop_order.trail_percent,
                highest_price: stop_order.highest_price,
                lowest_price: stop_order.lowest_price,
                status: stop_order.status,
                created_at: stop_order.created_at,
                expire_time: stop_order.expire_time,
                time_in_force: stop_order.time_in_force,
                stp_mode: stop_order.stp_mode,
                post_only: stop_order.post_only,
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Stop order {} not found", stop_order_id),
            }),
        )
            .into_response(),
    }
}

/// Cancel a stop order
#[utoipa::path(
    delete,
    path = "/api/v1/stop-orders/{stop_order_id}",
    params(
        ("stop_order_id" = Uuid, Path, description = "Stop order ID")
    ),
    responses(
        (status = 200, description = "Stop order cancelled", body = StopOrderDetailResponse),
        (status = 404, description = "Stop order not found", body = ErrorResponse)
    ),
    tag = "stop-orders"
)]
pub async fn cancel_stop_order(
    State(engine): State<Arc<OrderBookEngine>>,
    Path(stop_order_id): Path<Uuid>,
) -> impl IntoResponse {
    match engine.cancel_stop_order(stop_order_id) {
        Ok(stop_order) => (
            StatusCode::OK,
            Json(StopOrderDetailResponse {
                id: stop_order.id,
                symbol: stop_order.symbol,
                user_id: stop_order.user_id,
                side: stop_order.side,
                quantity: stop_order.quantity,
                trigger_price: stop_order.trigger_price,
                trigger_condition: stop_order.trigger_condition,
                stop_type: stop_order.stop_type,
                limit_price: stop_order.limit_price,
                trail_amount: stop_order.trail_amount,
                trail_percent: stop_order.trail_percent,
                highest_price: stop_order.highest_price,
                lowest_price: stop_order.lowest_price,
                status: stop_order.status,
                created_at: stop_order.created_at,
                expire_time: stop_order.expire_time,
                time_in_force: stop_order.time_in_force,
                stp_mode: stop_order.stp_mode,
                post_only: stop_order.post_only,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Failed to cancel stop order: {}", e),
            }),
        )
            .into_response(),
    }
}
