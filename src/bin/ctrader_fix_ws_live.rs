use order_book_api::{create_router, Broadcaster, OrderBookEngine};
use order_book_api::ctrader_fix::{CTraderFixClient, FixToWebSocketBridge};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_book_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   cTrader FIX → WebSocket Bridge (LIVE DATA)              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Your cTrader credentials (from ctrader_fix_test.rs)
    let host = "live-uk-eqx-01.p.c-trader.com".to_string();
    let port = 5201;
    let sender_comp_id = "live.fxpro.8244184".to_string();
    let target_comp_id = "cServer".to_string();
    let sender_sub_id = "QUOTE".to_string();
    let target_sub_id = "QUOTE".to_string();
    let username = "8244184".to_string();
    let password = "fixapibrixton".to_string();

    println!("📋 Configuration:");
    println!("   FIX Server: {}:{}", host, port);
    println!("   Account: {}\n", sender_comp_id);

    // Create the order book engine
    let engine = Arc::new(OrderBookEngine::new());

    // Create the WebSocket broadcaster
    let broadcaster = Broadcaster::new();

    // Create FIX client with tick channel
    println!("🔌 Creating FIX client with market data streaming...");
    let (client, tick_receiver) = CTraderFixClient::with_tick_channel(
        host,
        port,
        sender_comp_id,
        target_comp_id,
        sender_sub_id,
        target_sub_id,
        username,
        password,
    );

    // Create bridge to convert FIX ticks to WebSocket messages
    println!("🌉 Creating FIX → WebSocket bridge...");
    let bridge = FixToWebSocketBridge::new(broadcaster.clone());

    // Create datasource manager (not used in this binary, but required for router)
    use order_book_api::DatasourceManager;
    let datasource_manager = std::sync::Arc::new(DatasourceManager::new(broadcaster.clone()));

    // Spawn FIX client connection task
    println!("📡 Connecting to cTrader FIX server...\n");
    let client_handle = tokio::spawn(async move {
        let mut client = client;
        if let Err(e) = client.connect_and_run().await {
            eprintln!("❌ FIX client error: {}", e);
        }
    });

    // Spawn bridge task to consume ticks and broadcast to WebSocket
    let bridge_handle = tokio::spawn(async move {
        bridge.run(tick_receiver).await;
    });

    // Create the HTTP/WebSocket router
    let app = create_router(engine, broadcaster, datasource_manager);

    // Define the address
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("✅ Server Ready!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 HTTP Server:    http://{}", addr);
    println!("📚 Swagger UI:     http://{}/swagger-ui", addr);
    println!("🔌 WebSocket:      ws://{}/ws", addr);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("💡 Connect in Postman:");
    println!("   1. WebSocket URL: ws://localhost:3000/ws");
    println!("   2. Subscribe to ticker:");
    println!("      {{");
    println!("        \"action\": \"subscribe\",");
    println!("        \"channel\": \"ticker\",");
    println!("        \"symbol\": \"XAUUSD\"");
    println!("      }}");
    println!("   3. You'll receive LIVE market data from cTrader!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Start the server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for all tasks
    tokio::select! {
        _ = client_handle => println!("🔴 FIX client stopped"),
        _ = bridge_handle => println!("🔴 Bridge stopped"),
        _ = server_handle => println!("🔴 Server stopped"),
    }

    Ok(())
}
