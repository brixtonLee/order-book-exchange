use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::datasource::DatasourceManager;
use crate::engine::OrderBookEngine;
use crate::websocket::{websocket_handler, Broadcaster, WsState};

use super::handlers::*;
use super::datasource_handlers::*;
use super::openapi::{ApiDocV1, ApiDocV2};

/// Create the API router with Swagger UI and WebSocket support
pub fn create_router(
    engine: Arc<OrderBookEngine>,
    broadcaster: Broadcaster,
    datasource_manager: Arc<DatasourceManager>,
) -> Router {
    // Create WebSocket state
    let ws_state = Arc::new(WsState {
        broadcaster: broadcaster.clone(),
        engine: engine.clone(),
    });

    Router::new()
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
        .route("/api/v1/health", get(get_health))
        .with_state(datasource_manager.clone())
        // Datasource control endpoints
        .route("/api/v1/datasource/start", post(start_datasource))
        .route("/api/v1/datasource/stop", post(stop_datasource))
        .route("/api/v1/datasource/status", get(get_datasource_status))
        .with_state(datasource_manager.clone())
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
        .with_state(engine)
}
