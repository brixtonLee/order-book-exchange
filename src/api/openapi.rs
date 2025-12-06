use utoipa::OpenApi;

use crate::api::handlers;
use crate::api::responses::*;
use crate::metrics::SpreadMetrics;
use crate::models::{Order, OrderSide, OrderStatus, OrderType};

/// OpenAPI v1 specification
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Order Book API",
        version = "1.0.0",
        description = "A high-performance order matching engine and REST API built in Rust",
        contact(
            name = "Order Book API",
            url = "https://github.com/yourusername/order-book-api"
        ),
        license(
            name = "MIT"
        )
    ),
    paths(
        handlers::health_check,
        handlers::submit_order,
        handlers::get_order,
        handlers::cancel_order,
        handlers::get_order_book,
        handlers::get_spread_metrics,
        handlers::get_trades,
        handlers::get_exchange_metrics,
    ),
    components(
        schemas(
            Order,
            OrderSide,
            OrderType,
            OrderStatus,
            SubmitOrderRequest,
            SubmitOrderResponse,
            OrderResponse,
            CancelOrderResponse,
            TradeResponse,
            PriceLevelResponse,
            OrderBookResponse,
            SpreadMetrics,
            TradeListResponse,
            ExchangeMetricsResponse,
            ErrorResponse,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Orders", description = "Order management endpoints"),
        (name = "Order Book", description = "Order book and market data endpoints"),
        (name = "Trades", description = "Trade history endpoints"),
        (name = "Metrics", description = "Exchange metrics endpoints"),
    )
)]
pub struct ApiDocV1;

/// OpenAPI v2 specification (future version with additional features)
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Order Book API",
        version = "2.0.0",
        description = "A high-performance order matching engine and REST API built in Rust - Version 2.0 with enhanced features",
        contact(
            name = "Order Book API",
            url = "https://github.com/yourusername/order-book-api"
        ),
        license(
            name = "MIT"
        )
    ),
    paths(
        handlers::health_check,
        handlers::submit_order,
        handlers::get_order,
        handlers::cancel_order,
        handlers::get_order_book,
        handlers::get_spread_metrics,
        handlers::get_trades,
        handlers::get_exchange_metrics,
    ),
    components(
        schemas(
            Order,
            OrderSide,
            OrderType,
            OrderStatus,
            SubmitOrderRequest,
            SubmitOrderResponse,
            OrderResponse,
            CancelOrderResponse,
            TradeResponse,
            PriceLevelResponse,
            OrderBookResponse,
            SpreadMetrics,
            TradeListResponse,
            ExchangeMetricsResponse,
            ErrorResponse,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Orders", description = "Order management endpoints"),
        (name = "Order Book", description = "Order book and market data endpoints"),
        (name = "Trades", description = "Trade history endpoints"),
        (name = "Metrics", description = "Exchange metrics endpoints"),
    )
)]
pub struct ApiDocV2;
