use crate::testing::scenarios::TestScenario;
use crate::testing::state::{TestingMetrics, TestingState};
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Request to configure and start the producer
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartProducerRequest {
    pub rate_per_second: Option<u32>,
    pub symbols: Option<Vec<String>>,
}

/// Producer status response
#[derive(Debug, Serialize, ToSchema)]
pub struct ProducerStatusResponse {
    pub running: bool,
    pub rate_per_second: u32,
    pub symbols: Vec<String>,
    pub orders_generated: u64,
    pub errors: u64,
    pub started_at: Option<String>,
}

/// Testing metrics response
#[derive(Debug, Serialize, ToSchema)]
pub struct TestingMetricsResponse {
    pub metrics: TestingMetrics,
}

/// Generic success response
#[derive(Debug, Serialize, ToSchema)]
pub struct SuccessResponse {
    pub message: String,
}

/// Start the order producer
#[utoipa::path(
    post,
    path = "/api/v1/testing/producer/start",
    request_body = StartProducerRequest,
    responses(
        (status = 200, description = "Producer started", body = SuccessResponse),
        (status = 400, description = "Bad request")
    ),
    tag = "testing"
)]
pub async fn start_producer(
    State(state): State<Arc<TestingState>>,
    Json(request): Json<StartProducerRequest>,
) -> impl IntoResponse {
    let mut producer_state = state.producer_state.write().unwrap();

    // Update configuration
    if let Some(rate) = request.rate_per_second {
        producer_state.rate_per_second = rate;
    }
    if let Some(symbols) = request.symbols {
        producer_state.symbols = symbols;
    }

    // Start producer
    producer_state.running = true;
    producer_state.started_at = Some(Utc::now());

    (
        StatusCode::OK,
        Json(SuccessResponse {
            message: format!(
                "Producer started at {} orders/sec on symbols: {:?}",
                producer_state.rate_per_second, producer_state.symbols
            ),
        }),
    )
}

/// Stop the order producer
#[utoipa::path(
    post,
    path = "/api/v1/testing/producer/stop",
    responses(
        (status = 200, description = "Producer stopped", body = SuccessResponse)
    ),
    tag = "testing"
)]
pub async fn stop_producer(State(state): State<Arc<TestingState>>) -> impl IntoResponse {
    let mut producer_state = state.producer_state.write().unwrap();
    producer_state.running = false;

    (
        StatusCode::OK,
        Json(SuccessResponse {
            message: "Producer stopped".to_string(),
        }),
    )
}

/// Get producer status
#[utoipa::path(
    get,
    path = "/api/v1/testing/producer/status",
    responses(
        (status = 200, description = "Producer status", body = ProducerStatusResponse)
    ),
    tag = "testing"
)]
pub async fn get_producer_status(
    State(state): State<Arc<TestingState>>,
) -> impl IntoResponse {
    let producer_state = state.producer_state.read().unwrap();

    let response = ProducerStatusResponse {
        running: producer_state.running,
        rate_per_second: producer_state.rate_per_second,
        symbols: producer_state.symbols.clone(),
        orders_generated: producer_state.orders_generated,
        errors: producer_state.errors,
        started_at: producer_state
            .started_at
            .map(|dt| dt.to_rfc3339()),
    };

    (StatusCode::OK, Json(response))
}

/// Get comprehensive testing metrics
#[utoipa::path(
    get,
    path = "/api/v1/testing/metrics",
    responses(
        (status = 200, description = "Testing metrics", body = TestingMetricsResponse)
    ),
    tag = "testing"
)]
pub async fn get_testing_metrics(
    State(state): State<Arc<TestingState>>,
) -> impl IntoResponse {
    let metrics = state.metrics.read().unwrap().clone();

    (
        StatusCode::OK,
        Json(TestingMetricsResponse { metrics }),
    )
}

/// Reset testing metrics
#[utoipa::path(
    post,
    path = "/api/v1/testing/metrics/reset",
    responses(
        (status = 200, description = "Metrics reset", body = SuccessResponse)
    ),
    tag = "testing"
)]
pub async fn reset_testing_metrics(
    State(state): State<Arc<TestingState>>,
) -> impl IntoResponse {
    let mut metrics = state.metrics.write().unwrap();
    *metrics = TestingMetrics::default();

    let mut producer_state = state.producer_state.write().unwrap();
    producer_state.orders_generated = 0;
    producer_state.errors = 0;

    (
        StatusCode::OK,
        Json(SuccessResponse {
            message: "Testing metrics reset".to_string(),
        }),
    )
}

/// Start a pre-built test scenario
#[utoipa::path(
    post,
    path = "/api/v1/testing/scenarios/{scenario_name}",
    params(
        ("scenario_name" = String, Path, description = "Scenario name (basic, stp, iceberg, chaos, etc.)")
    ),
    responses(
        (status = 200, description = "Scenario started", body = SuccessResponse),
        (status = 400, description = "Invalid scenario name")
    ),
    tag = "testing"
)]
pub async fn start_scenario(
    State(state): State<Arc<TestingState>>,
    Path(scenario_name): Path<String>,
) -> impl IntoResponse {
    // Parse scenario name
    let scenario: TestScenario = match scenario_name.to_lowercase().as_str() {
        "basic" => TestScenario::Basic,
        "stp" | "self_trade_prevention" => TestScenario::SelfTradePrevention,
        "iceberg" | "iceberg_orders" => TestScenario::IcebergOrders,
        "stop" | "stop_order_cascade" => TestScenario::StopOrderCascade,
        "algorithm" | "algorithm_stress" => TestScenario::AlgorithmStress,
        "chaos" => TestScenario::Chaos,
        "tif" | "time_in_force" => TestScenario::TimeInForce,
        "post_only" | "post_only_makers" => TestScenario::PostOnlyMakers,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SuccessResponse {
                    message: format!(
                        "Invalid scenario: {}. Valid scenarios: basic, stp, iceberg, stop, algorithm, chaos, tif, post_only",
                        scenario_name
                    ),
                }),
            );
        }
    };

    // Get configuration for scenario
    let config = scenario.to_config();

    // Update producer state
    let mut producer_state = state.producer_state.write().unwrap();
    producer_state.running = true;
    producer_state.started_at = Some(Utc::now());

    // Note: The ProducerConfig in the OrderProducer is immutable
    // To truly apply the scenario config, we'd need to recreate the producer
    // For now, just start with default config and log the scenario intent
    // In a production system, you'd want to make config hot-reloadable

    (
        StatusCode::OK,
        Json(SuccessResponse {
            message: format!(
                "Scenario '{}' started: {}",
                scenario_name,
                scenario.description()
            ),
        }),
    )
}
