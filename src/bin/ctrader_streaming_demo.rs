use order_book_api::ctrader_fix::{CTraderFixClient, FixToWebSocketBridge, MarketDataStats};
use order_book_api::websocket::broadcaster::Broadcaster;
use std::env;
use std::sync::Arc;

/// Demonstration of WebSocket-like tick streaming from cTrader FIX API
///
/// This binary shows:
/// 1. Connecting to cTrader FIX API
/// 2. Subscribing to market data (real-time ticks)
/// 3. Streaming ticks through channels (WebSocket-like)
/// 4. Broadcasting to WebSocket clients via your existing infrastructure
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║     cTrader FIX API → WebSocket Streaming Demo                ║");
    println!("║     Real-time Tick Data Pipeline                              ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Load credentials from environment variables
    let host = env::var("CTRADER_HOST")
        .unwrap_or_else(|_| "h51.p.ctrader.com".to_string());
    let port: u16 = env::var("CTRADER_PORT")
        .unwrap_or_else(|_| "5201".to_string())
        .parse()
        .expect("Invalid port");
    let sender_comp_id = env::var("CTRADER_SENDER_COMP_ID")
        .expect("Missing CTRADER_SENDER_COMP_ID env var");
    let target_comp_id = env::var("CTRADER_TARGET_COMP_ID")
        .unwrap_or_else(|_| "CSERVER".to_string());
    let sender_sub_id = env::var("CTRADER_SENDER_SUB_ID")
        .unwrap_or_else(|_| "QUOTE".to_string());
    let target_sub_id = env::var("CTRADER_TARGET_SUB_ID")
        .unwrap_or_else(|_| "QUOTE".to_string());
    let username = env::var("CTRADER_USERNAME")
        .expect("Missing CTRADER_USERNAME env var");
    let password = env::var("CTRADER_PASSWORD")
        .expect("Missing CTRADER_PASSWORD env var");

    println!("📋 Configuration:");
    println!("   Host: {}:{}", host, port);
    println!("   Sender: {}", sender_comp_id);
    println!("   Target: {}", target_comp_id);
    println!();

    // Create WebSocket broadcaster (your existing infrastructure)
    let broadcaster = Broadcaster::with_capacity(10000);
    println!("✅ WebSocket broadcaster initialized");

    // Create FIX client with tick streaming channel
    let (mut fix_client, tick_receiver) = CTraderFixClient::with_tick_channel(
        host,
        port,
        sender_comp_id,
        target_comp_id,
        sender_sub_id,
        target_sub_id,
        username,
        password,
    );
    println!("✅ FIX client created with streaming channel");

    // Create bridge to convert FIX ticks → WebSocket messages
    let mut bridge = FixToWebSocketBridge::new(broadcaster.clone());

    // Add any custom symbol mappings
    bridge.add_symbol_mapping("41".to_string(), "XAUUSD".to_string());

    println!("✅ FIX to WebSocket bridge created");
    println!();

    // Create stats tracker
    let stats = Arc::new(MarketDataStats::new());
    let stats_clone = Arc::clone(&stats);

    // Spawn stats reporter task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let count = stats_clone.get_tick_count();
            if count > 0 {
                println!("\n📊 Statistics: {} ticks processed", count);
                if let Some(last_tick) = stats_clone.get_last_tick_time().await {
                    println!("   Last tick: {}", last_tick.format("%H:%M:%S%.3f"));
                }
                println!();
            }
        }
    });

    // Spawn bridge task to consume ticks and broadcast to WebSocket
    let bridge_handle = tokio::spawn(async move {
        bridge.run(tick_receiver).await;
    });

    // Spawn FIX client task
    let fix_handle = tokio::spawn(async move {
        if let Err(e) = fix_client.connect_and_run().await {
            eprintln!("❌ FIX client error: {}", e);
        }
    });

    println!("🚀 All tasks started!");
    println!("   - FIX client: Connecting and subscribing to market data");
    println!("   - Bridge: Converting ticks to WebSocket messages");
    println!("   - Broadcaster: Ready to serve WebSocket clients");
    println!();
    println!("💡 Tip: Connect WebSocket clients to ws://localhost:3000/ws");
    println!("   Subscribe to channel: {{\"action\": \"subscribe\", \"channel\": \"ticker\", \"symbol\": \"XAUUSD\"}}");
    println!();
    println!("Press Ctrl+C to stop...");
    println!();

    // Wait for tasks (or Ctrl+C)
    tokio::select! {
        _ = fix_handle => {
            println!("FIX client task finished");
        }
        _ = bridge_handle => {
            println!("Bridge task finished");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n\n🛑 Received Ctrl+C, shutting down...");
        }
    }

    Ok(())
}
