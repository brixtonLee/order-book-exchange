use order_book_api::ctrader_fix::CTraderFixClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logs
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         cTrader FIX API Connection Test                   ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Your cTrader credentials
    let host = "live-uk-eqx-01.p.c-trader.com".to_string();
    let port = 5201; // Plain text port
    let sender_comp_id = "live.fxpro.8244184".to_string();
    let target_comp_id = "cServer".to_string();
    let sender_sub_id = "QUOTE".to_string();
    let target_sub_id = "QUOTE".to_string(); // TargetSubID must match SenderSubID
    let username = "8244184".to_string();
    // ⚠️ SECURITY WARNING: Never hardcode passwords in production!
    // Read from environment variable or secure vault
    let password = "fixapibrixton".to_string();

    println!("🚀 Starting FIX connection...");

    // Create and run the client
    let mut client = CTraderFixClient::new(
        host,
        port,
        sender_comp_id,
        target_comp_id,
        sender_sub_id,
        target_sub_id,
        username,
        password,
    );

    // Connect and start receiving messages
    match client.connect_and_run().await {
        Ok(_) => println!("✅ Connection closed gracefully"),
        Err(e) => eprintln!("❌ Error: {}", e),
    }

    Ok(())
}
