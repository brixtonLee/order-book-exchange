use order_book_api::{create_router, Broadcaster, DatasourceManager, OrderBookEngine};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load environment variables from .env file (if present)
    dotenvy::dotenv().ok();

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

    // Initialize database (optional - only if DATABASE_URL is set)
    let database_state = initialize_database(datasource_manager.clone()).await;

    // Initialize cron scheduler (only if database is enabled)
    if database_state.is_some() {
        initialize_cron_scheduler(database_state.as_ref().unwrap(), datasource_manager.clone())
            .await;
    }

    // Create the router with WebSocket support, datasource control, and optional database
    let app = create_router(engine, broadcaster, datasource_manager, database_state);

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

/// Initialize database connection pools and repositories
async fn initialize_database(
    datasource_manager: Arc<DatasourceManager>,
) -> Option<order_book_api::api::DatabaseState> {
    use order_book_api::database::{
        establish_connection_pools, repositories::*, TickPersister,
    };

    // Check if database URLs are configured
    let metadata_url = std::env::var("DATABASE_URL").ok()?;
    let timeseries_url = std::env::var("TIMESCALEDB_URL").ok()?;

    tracing::info!("🗄️  Initializing PostgreSQL and TimescaleDB connections...");

    // Get pool configuration from environment (with defaults)
    let pool_size = std::env::var("DB_POOL_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);

    // Establish connection pools
    let pools = match establish_connection_pools(&metadata_url, &timeseries_url, pool_size) {
        Ok(pools) => {
            tracing::info!("✅ Database connections established successfully");
            pools
        }
        Err(e) => {
            tracing::error!("❌ Failed to establish database connections: {}", e);
            tracing::warn!("⚠️  Server will start without database functionality");
            return None;
        }
    };

    // Create repositories
    let pools_clone = pools.clone();
    let symbol_repository = Arc::new(SymbolRepositoryImpl::new(move || {
        pools_clone.get_metadata_conn()
    })) as Arc<dyn SymbolRepository>;

    let pools_clone = pools.clone();
    let tick_repository = Arc::new(TickRepositoryImpl::new(move || {
        pools_clone.get_timeseries_conn()
    })) as Arc<dyn TickRepository>;

    let pools_clone = pools.clone();
    let ohlc_repository = Arc::new(OhlcRepositoryImpl::new(move || {
        pools_clone.get_timeseries_conn()
    })) as Arc<dyn OhlcRepository>;

    // Create tick persister with batching
    let batch_size = std::env::var("TICK_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);

    let flush_interval_ms = std::env::var("TICK_FLUSH_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100);

    let tick_persister = TickPersister::new(tick_repository.clone(), batch_size, flush_interval_ms);
    let tick_persister_tx = tick_persister.start();

    // Attach tick persister to datasource manager
    datasource_manager
        .set_tick_persister(tick_persister_tx)
        .await;

    tracing::info!(
        "✅ Tick persister configured (batch_size={}, flush_interval={}ms)",
        batch_size,
        flush_interval_ms
    );

    // Create database state for API handlers
    let database_state = order_book_api::api::DatabaseState {
        symbol_repository: symbol_repository.clone(),
        tick_repository: tick_repository.clone(),
        ohlc_repository: ohlc_repository.clone(),
    };

    tracing::info!("✅ Database integration complete");
    tracing::info!("📊 New API endpoints available:");
    tracing::info!("   GET  /api/v1/symbols");
    tracing::info!("   GET  /api/v1/symbols/{{symbol_id}}");
    tracing::info!("   GET  /api/v1/ticks/{{symbol_id}}");
    tracing::info!("   GET  /api/v1/ohlc/{{symbol_id}}?timeframe=5m");

    Some(database_state)
}

/// Initialize cron scheduler for periodic jobs
async fn initialize_cron_scheduler(
    database_state: &order_book_api::api::DatabaseState,
    datasource_manager: Arc<DatasourceManager>,
) {
    use order_book_api::jobs::SymbolSyncJob;
    use tokio_cron_scheduler::JobScheduler;

    tracing::info!("⏰ Initializing cron scheduler...");

    // Create scheduler
    let scheduler = match JobScheduler::new().await {
        Ok(scheduler) => scheduler,
        Err(e) => {
            tracing::error!("❌ Failed to create cron scheduler: {}", e);
            return;
        }
    };

    // Create symbol sync job
    let symbol_sync_job = SymbolSyncJob::new(
        database_state.symbol_repository.clone(),
        Some(datasource_manager.clone()),
    );

    // Register job
    if let Err(e) = symbol_sync_job.register(&scheduler).await {
        tracing::error!("❌ Failed to register symbol sync job: {}", e);
        return;
    }

    // Start scheduler
    if let Err(e) = scheduler.start().await {
        tracing::error!("❌ Failed to start cron scheduler: {}", e);
        return;
    }

    tracing::info!("✅ Cron scheduler started successfully");
    tracing::info!("   • Symbol sync: Every 5 minutes");

    // Keep scheduler alive (it will run in the background)
    // The scheduler is automatically cleaned up when the program exits
    std::mem::forget(scheduler);
}

