use order_book_api::{create_router, Broadcaster, OrderBookEngine};
use order_book_api::ctrader_fix::{CTraderFixClient, FixToWebSocketBridge};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    println!("🚀 Starting cTrader FIX → WebSocket Server\n");

    // Create the order book engine
    let engine = Arc::new(OrderBookEngine::new());

    // Create the WebSocket broadcaster
    let broadcaster = Broadcaster::new();

    // Create FIX client with tick channel
    let (client, tick_receiver) = CTraderFixClient::with_tick_channel(
        "h51.p.ctrader.com".to_string(),
        5201,
        "demo.icmarkets.16319805".to_string(),
        "QUOTE".to_string(),
        "QUOTE".to_string(),
        "QUOTE".to_string(),
        "16319805".to_string(),
        "4b6e0ec9-e8f6-416e-a91c-bd8b2b5e0796".to_string(),
    );

    // Create bridge
    let bridge = FixToWebSocketBridge::new(broadcaster.clone());

    // Spawn FIX client connection task
    println!("📡 Connecting to cTrader FIX server...");
    let client_handle = tokio::spawn(async move {
        let mut client = client;
        if let Err(e) = client.connect_and_run().await {
            eprintln!("❌ FIX client error: {}", e);
        }
    });

    // Spawn bridge task to consume ticks and broadcast to WebSocket
    println!("🌉 Starting FIX → WebSocket bridge...");
    let bridge_handle = tokio::spawn(async move {
        bridge.run(tick_receiver).await;
    });

    // Create the HTTP/WebSocket router
    let app = create_router(engine, broadcaster);

    // Define the address
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("\n✅ Server ready!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 HTTP Server:    http://{}", addr);
    println!("📚 Swagger UI:     http://{}/swagger-ui", addr);
    println!("🔌 WebSocket:      ws://{}/ws", addr);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("💡 To test WebSocket in Postman:");
    println!("   1. Create new WebSocket Request");
    println!("   2. Connect to: ws://localhost:3000/ws");
    println!("   3. Send subscribe message:");
    println!("      {{");
    println!("        \"action\": \"subscribe\",");
    println!("        \"channel\": \"ticker\",");
    println!("        \"symbol\": \"XAUUSD\"");
    println!("      }}");
    println!("\n🎯 Available symbols: XAUUSD, EURUSD, GBPUSD, USDJPY\n");

    // Start the server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for all tasks
    tokio::select! {
        _ = client_handle => println!("FIX client stopped"),
        _ = bridge_handle => println!("Bridge stopped"),
        _ = server_handle => println!("Server stopped"),
    }
}
