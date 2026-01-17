use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::database_handlers::DatabaseState;
use crate::datasource::DatasourceManager;
use crate::engine::OrderBookEngine;
use crate::rabbitmq::RabbitMQService;
use crate::websocket::{websocket_handler, Broadcaster, WsState};

use super::database_handlers::*;
use super::datasource_handlers::{self, DatasourceState};
use super::handlers::*;
use super::openapi::{ApiDocV1, ApiDocV2};
use super::rabbitmq_handlers::*;

/// Create the API router with Swagger UI and WebSocket support
pub fn create_router(
    engine: Arc<OrderBookEngine>,
    broadcaster: Broadcaster,
    datasource_manager: Arc<DatasourceManager>,
    rabbitmq_service: Option<Arc<RabbitMQService>>,
    database_state: Option<DatabaseState>,
) -> Router {
    // Create WebSocket state
    let ws_state = Arc::new(WsState {
        broadcaster: broadcaster.clone(),
        engine: engine.clone(),
    });

    // Create datasource state (includes optional RabbitMQ service)
    let datasource_state = DatasourceState {
        manager: datasource_manager.clone(),
        rabbitmq_service: rabbitmq_service.clone(),
    };

    let router = Router::new()
        // Swagger UI with version selection
        .merge(
            SwaggerUi::new("/swagger-ui")
                .urls(vec![
                    (
                        utoipa_swagger_ui::Url::new("v1.0", "/api-docs/v1/openapi.json"),
                        ApiDocV1::openapi(),
                    ),
                    (
                        utoipa_swagger_ui::Url::new("v2.0", "/api-docs/v2/openapi.json"),
                        ApiDocV2::openapi(),
                    ),
                ])
        )
        // WebSocket endpoint
        .route("/ws", get(websocket_handler))
        .with_state(ws_state.clone())
        // Health endpoint (uses datasource manager)
        .route("/api/v1/health", get(datasource_handlers::get_health))
        .with_state(datasource_state.clone())
        // Datasource control endpoints
        .route("/api/v1/datasource/start", post(datasource_handlers::start_datasource))
        .route("/api/v1/datasource/stop", post(datasource_handlers::stop_datasource))
        .route("/api/v1/datasource/status", get(datasource_handlers::get_datasource_status))
        .with_state(datasource_state)
        // Legacy health check (kept for backwards compatibility)
        .route("/health", get(health_check))
        // Order endpoints
        .route("/api/v1/orders", post(submit_order))
        .route("/api/v1/orders/:symbol/:order_id", get(get_order))
        .route("/api/v1/orders/:symbol/:order_id", delete(cancel_order))
        // Order book endpoints
        .route("/api/v1/orderbook/:symbol", get(get_order_book))
        .route("/api/v1/orderbook/:symbol/spread", get(get_spread_metrics))
        // Trade endpoints
        .route("/api/v1/trades/:symbol", get(get_trades))
        // Metrics endpoints
        .route("/api/v1/metrics/exchange", get(get_exchange_metrics))
        .route("/api/v1/orderbook/:symbol/microstructure", get(get_microstructure_metrics))
        // Add state for REST endpoints
        .with_state(engine);

    // Conditionally merge RabbitMQ routes if service is configured
    let router = if let Some(rmq_service) = rabbitmq_service {
        let rmq_router = Router::new()
            // RabbitMQ control endpoints
            .route("/api/v1/rabbitmq/connect", post(connect_rabbitmq))
            .route("/api/v1/rabbitmq/status", get(get_rabbitmq_status))
            .route("/api/v1/rabbitmq/disconnect", post(disconnect_rabbitmq))
            .with_state(rmq_service);

        router.merge(rmq_router)
    } else {
        router
    };

    // Conditionally merge database routes if database is configured
    if let Some(db_state) = database_state {
        let db_router = Router::new()
            // Symbol endpoints
            .route("/api/v1/symbols", get(get_symbols))
            .route("/api/v1/symbols/:symbol_id", get(get_symbol_by_id))
            .route("/api/v1/symbols/name/:symbol_name", get(get_symbol_by_name))
            // Tick endpoints
            .route("/api/v1/ticks/:symbol_id", get(get_ticks))
            .route("/api/v1/ticks/:symbol_id/latest", get(get_latest_tick))
            // OHLC endpoints
            .route("/api/v1/ohlc/:symbol_id", get(get_ohlc_candles))
            .route("/api/v1/ohlc/:symbol_id/latest", get(get_latest_ohlc_candle))
            // Tick queue monitoring
            .route("/api/v1/database/tick-queue/status", get(get_tick_queue_status))
            .with_state(db_state);

        router.merge(db_router)
    } else {
        router
    }
}
