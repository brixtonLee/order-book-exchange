use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::{Order, OrderSide, OrderStatus, OrderType, SelfTradePreventionMode, TimeInForce, Trade};

/// Request to submit a new order
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitOrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    #[schema(value_type = Option<String>, example = "150.50")]
    pub price: Option<Decimal>,
    #[schema(value_type = String, example = "100")]
    pub quantity: Decimal,
    pub user_id: String,
    /// Time-in-force (default: GTC)
    #[serde(default)]
    pub time_in_force: TimeInForce,
    /// Self-trade prevention mode (default: None)
    #[serde(default)]
    pub stp_mode: SelfTradePreventionMode,
    /// Post-only order (maker-only, default: false)
    #[serde(default)]
    pub post_only: bool,
    /// Expiration time for GTD orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
    /// Iceberg order total quantity (optional)
    #[schema(value_type = Option<String>, example = "1000")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg_total_quantity: Option<Decimal>,
    /// Iceberg order display quantity (optional)
    #[schema(value_type = Option<String>, example = "100")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg_display_quantity: Option<Decimal>,
}

/// Response after submitting an order
#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitOrderResponse {
    pub order_id: Uuid,
    pub status: OrderStatus,
    #[schema(value_type = String, example = "50")]
    pub filled_quantity: Decimal,
    pub trades: Vec<TradeResponse>,
    pub timestamp: DateTime<Utc>,
}

/// Trade information in response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TradeResponse {
    pub trade_id: Uuid,
    #[schema(value_type = String, example = "150.50")]
    pub price: Decimal,
    #[schema(value_type = String, example = "100")]
    pub quantity: Decimal,
    #[schema(value_type = String, example = "0.15")]
    pub maker_fee: Decimal,
    #[schema(value_type = String, example = "0.30")]
    pub taker_fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

impl From<Trade> for TradeResponse {
    fn from(trade: Trade) -> Self {
        Self {
            trade_id: trade.id,
            price: trade.price,
            quantity: trade.quantity,
            maker_fee: trade.maker_fee,
            taker_fee: trade.taker_fee,
            timestamp: trade.timestamp,
        }
    }
}

/// Order status response
#[derive(Debug, Serialize, ToSchema)]
pub struct OrderResponse {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    #[schema(value_type = Option<String>, example = "150.50")]
    pub price: Option<Decimal>,
    #[schema(value_type = String, example = "100")]
    pub quantity: Decimal,
    #[schema(value_type = String, example = "50")]
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub stp_mode: SelfTradePreventionMode,
    pub post_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
    pub timestamp: DateTime<Utc>,
}

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.id,
            symbol: order.symbol,
            side: order.side,
            order_type: order.order_type,
            price: order.price,
            quantity: order.quantity,
            filled_quantity: order.filled_quantity,
            status: order.status,
            time_in_force: order.time_in_force,
            stp_mode: order.stp_mode,
            post_only: order.post_only,
            expire_time: order.expire_time,
            timestamp: order.timestamp,
        }
    }
}

/// Cancel order response
#[derive(Debug, Serialize, ToSchema)]
pub struct CancelOrderResponse {
    pub order_id: Uuid,
    pub status: OrderStatus,
    #[schema(value_type = String, example = "50")]
    pub filled_quantity: Decimal,
    #[schema(value_type = String, example = "50")]
    pub remaining_quantity: Decimal,
}

/// Price level in order book
#[derive(Debug, Serialize, ToSchema)]
pub struct PriceLevelResponse {
    #[schema(value_type = String, example = "150.50")]
    pub price: Decimal,
    #[schema(value_type = String, example = "1000")]
    pub quantity: Decimal,
    pub orders: usize,
}

/// Order book response
#[derive(Debug, Serialize, ToSchema)]
pub struct OrderBookResponse {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<PriceLevelResponse>,
    pub asks: Vec<PriceLevelResponse>,
    #[schema(value_type = Option<String>, example = "150.45")]
    pub best_bid: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "150.55")]
    pub best_ask: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "0.10")]
    pub spread: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "6.64")]
    pub spread_bps: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "150.50")]
    pub mid_price: Option<Decimal>,
}

/// Spread metrics response (same as SpreadMetrics)
pub use crate::metrics::SpreadMetrics as SpreadMetricsResponse;

/// Trade list response
#[derive(Debug, Serialize, ToSchema)]
pub struct TradeListResponse {
    pub symbol: String,
    pub trades: Vec<TradeResponse>,
    pub count: usize,
}

/// Exchange metrics response
#[derive(Debug, Serialize, ToSchema)]
pub struct ExchangeMetricsResponse {
    pub total_trades: usize,
    #[schema(value_type = String, example = "1500000.00")]
    pub total_volume: Decimal,
    #[schema(value_type = String, example = "2250.50")]
    pub total_fees_collected: Decimal,
    pub active_orders: usize,
    pub symbols: Vec<String>,
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}
