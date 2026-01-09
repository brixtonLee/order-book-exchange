use utoipa::OpenApi;

use crate::api::handlers;
use crate::api::datasource_handlers;
use crate::api::rabbitmq_handlers;
use crate::api::responses::*;
use crate::metrics::{SpreadMetrics, MicrostructureMetrics, TradingSignal};
use crate::models::{Order, OrderSide, OrderStatus, OrderType};
use crate::models::datasource::*;
use crate::rabbitmq::{RabbitMQConfig, ReconnectConfig, PublisherStats};

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
        handlers::get_microstructure_metrics,
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
            MicrostructureMetrics,
            TradingSignal,
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

/// OpenAPI v2 specification (with datasource control endpoints)
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Order Book API",
        version = "2.0.0",
        description = "A high-performance order matching engine and REST API built in Rust - Version 2.0 with FIX datasource control",
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
        handlers::get_microstructure_metrics,
        // Datasource control endpoints
        datasource_handlers::start_datasource,
        datasource_handlers::stop_datasource,
        datasource_handlers::get_datasource_status,
        datasource_handlers::get_health,
        // RabbitMQ control endpoints
        rabbitmq_handlers::connect_rabbitmq,
        rabbitmq_handlers::get_rabbitmq_status,
        rabbitmq_handlers::disconnect_rabbitmq,
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
            // Datasource models
            StartDatasourceRequest,
            StartDatasourceResponse,
            StopDatasourceResponse,
            DatasourceStatus,
            DatasourceMode,
            SymbolInfo,
            ConnectionInfo,
            HealthStatus,
            HealthState,
            ConnectionState,
            HeartbeatState,
            FixCredentials,
            MicrostructureMetrics,
            TradingSignal,
            // RabbitMQ models
            RabbitMQConfig,
            ReconnectConfig,
            PublisherStats,
            rabbitmq_handlers::RabbitMQConnectRequest,
            rabbitmq_handlers::RabbitMQConnectResponse,
            rabbitmq_handlers::RabbitMQStatusResponse,
        )
    ),
    tags(
        (name = "health", description = "System health monitoring"),
        (name = "datasource", description = "FIX datasource connection control"),
        (name = "RabbitMQ", description = "RabbitMQ messaging integration"),
        (name = "Health", description = "Health check endpoints"),
        (name = "Orders", description = "Order management endpoints"),
        (name = "Order Book", description = "Order book and market data endpoints"),
        (name = "Trades", description = "Trade history endpoints"),
        (name = "Metrics", description = "Exchange metrics endpoints"),
    )
)]
pub struct ApiDocV2;
