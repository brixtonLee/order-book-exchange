use order_book_api::{create_router, Broadcaster, DatasourceManager, OrderBookEngine};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_book_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create the order book engine
    let engine = Arc::new(OrderBookEngine::new());

    // Create the WebSocket broadcaster
    let broadcaster = Broadcaster::new();

    // Create the datasource manager
    let datasource_manager = Arc::new(DatasourceManager::new(broadcaster.clone()));

    // Create the router with WebSocket support and datasource control
    let app = create_router(engine, broadcaster, datasource_manager);

    // Define the address
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    tracing::info!("🚀 Order Book API server running on http://{}", addr);
    tracing::info!("📊 Health check: http://{}/api/v1/health", addr);
    tracing::info!("📚 Swagger UI: http://{}/swagger-ui", addr);
    tracing::info!("🔌 WebSocket: ws://{}/ws", addr);
    tracing::info!("🔧 Datasource control: http://{}/api/v1/datasource/*", addr);
    tracing::info!("");
    tracing::info!("📡 WebSocket Subscription Examples:");
    tracing::info!("   Subscribe to XAUUSD ticks:");
    tracing::info!(r#"   {{"action":"subscribe","channel":"ticker","symbol":"XAUUSD"}}"#);
    tracing::info!("");
    tracing::info!("   Subscribe to EURUSD ticks:");
    tracing::info!(r#"   {{"action":"subscribe","channel":"ticker","symbol":"EURUSD"}}"#);
    tracing::info!("");
    tracing::info!("   Subscribe to order book:");
    tracing::info!(r#"   {{"action":"subscribe","channel":"orderbook","symbol":"AAPL"}}"#);
    tracing::info!("");
    tracing::info!("   Subscribe to all trades:");
    tracing::info!(r#"   {{"action":"subscribe","channel":"trades"}}"#);
    tracing::info!("");

    // Start the server
    axum::serve(listener, app).await.unwrap();
}
