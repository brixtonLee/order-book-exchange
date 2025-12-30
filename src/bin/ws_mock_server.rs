use order_book_api::{create_router, Broadcaster, OrderBookEngine};
use order_book_api::ctrader_fix::{FixToWebSocketBridge, MarketTick};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Generate mock market ticks for testing WebSocket
async fn generate_mock_ticks(bridge: FixToWebSocketBridge) {
    println!("📊 Mock market data generator started!\n");

    let mut price_xauusd = Decimal::from_str("2650.00").unwrap();
    let mut price_eurusd = Decimal::from_str("1.0850").unwrap();

    loop {
        // Simulate price movements
        price_xauusd += Decimal::from_str("0.10").unwrap();
        price_eurusd += Decimal::from_str("0.0001").unwrap();

        // Reset if price gets too high
        if price_xauusd > Decimal::from_str("2700.00").unwrap() {
            price_xauusd = Decimal::from_str("2650.00").unwrap();
        }
        if price_eurusd > Decimal::from_str("1.0900").unwrap() {
            price_eurusd = Decimal::from_str("1.0850").unwrap();
        }

        // Generate XAUUSD tick
        let mut tick_xauusd = MarketTick::new("41".to_string());
        tick_xauusd.bid_price = Some(price_xauusd);
        tick_xauusd.ask_price = Some(price_xauusd + Decimal::from_str("0.50").unwrap());
        // Generate EURUSD tick
        let mut tick_eurusd = MarketTick::new("1".to_string());
        tick_eurusd.bid_price = Some(price_eurusd);
        tick_eurusd.ask_price = Some(price_eurusd + Decimal::from_str("0.0002").unwrap());

        // Broadcast ticks
        bridge.process_tick(tick_xauusd);
        bridge.process_tick(tick_eurusd);

        // Update every 1 second
        sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_book_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🚀 Starting Mock WebSocket Server (for Testing)\n");

    // Create the order book engine
    let engine = Arc::new(OrderBookEngine::new());

    // Create the WebSocket broadcaster
    let broadcaster = Broadcaster::new();

    // Create bridge
    let bridge = FixToWebSocketBridge::new(broadcaster.clone());

    // Create datasource manager (not used in this binary, but required for router)
    use order_book_api::DatasourceManager;
    let datasource_manager = std::sync::Arc::new(DatasourceManager::new(broadcaster.clone()));

    // Spawn mock data generator
    println!("🎭 Starting mock market data generator...");
    let mock_handle = tokio::spawn(async move {
        generate_mock_ticks(bridge).await;
    });

    // Create the HTTP/WebSocket router
    let app = create_router(engine, broadcaster, datasource_manager);

    // Define the address
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("\n✅ Mock Server Ready!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 HTTP Server:    http://{}", addr);
    println!("📚 Swagger UI:     http://{}/swagger-ui", addr);
    println!("🔌 WebSocket:      ws://{}/ws", addr);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("💡 Test in Postman:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1️⃣  Create new WebSocket Request");
    println!("2️⃣  Connect to: ws://localhost:3000/ws");
    println!("3️⃣  Send subscribe message:\n");
    println!("    Subscribe to XAUUSD (Gold):");
    println!("    {{");
    println!("      \"action\": \"subscribe\",");
    println!("      \"channel\": \"ticker\",");
    println!("      \"symbol\": \"XAUUSD\"");
    println!("    }}\n");
    println!("    Subscribe to EURUSD:");
    println!("    {{");
    println!("      \"action\": \"subscribe\",");
    println!("      \"channel\": \"ticker\",");
    println!("      \"symbol\": \"EURUSD\"");
    println!("    }}\n");
    println!("    Subscribe to all tickers:");
    println!("    {{");
    println!("      \"action\": \"subscribe\",");
    println!("      \"channel\": \"ticker\",");
    println!("      \"symbol\": \"*\"");
    println!("    }}\n");
    println!("4️⃣  Watch real-time price updates stream in!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Generating mock ticks every 1 second for:");
    println!("   • XAUUSD (Gold): Starting at 2650.00");
    println!("   • EURUSD: Starting at 1.0850");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Start the server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for all tasks
    tokio::select! {
        _ = mock_handle => println!("Mock generator stopped"),
        _ = server_handle => println!("Server stopped"),
    }
}
