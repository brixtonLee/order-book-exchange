use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::{interval, Duration};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

use super::messages::{create_logon_message, create_market_data_request, create_heartbeat, create_security_list_request, parse_security_list_response, parse_fix_message, format_for_display, SymbolData};
use super::market_data::{MarketTick, MarketDataParser};

/// Lightweight message for async display
/// Contains only essential FIX fields for minimal output
#[derive(Debug, Clone)]
struct DisplayMessage {
    /// Symbol ID (Tag 55)
    symbol: String,
    /// Number of MD Entries (Tag 268)
    num_entries: String,
    /// MD Entry Type (Tag 269)
    entry_type: String,
    /// MD Entry Price (Tag 270)
    entry_price: String,
    /// Time elapsed since last message (ms)
    elapsed_ms: i64,
}

pub struct CTraderFixClient {
    host: String,
    port: u16,
    sender_comp_id: String,
    target_comp_id: String,
    sender_sub_id: String,
    target_sub_id: String,
    username: String,
    password: String,
    msg_seq_num: Arc<Mutex<u32>>,
    /// Channel for streaming market ticks to consumers
    tick_sender: Option<mpsc::UnboundedSender<MarketTick>>,
    /// Parser for market data messages
    parser: MarketDataParser,
    /// Timestamp of last received message (for latency tracking)
    last_message_time: Arc<StdMutex<Option<Instant>>>,
    /// Channel for async display (non-blocking)
    display_sender: Option<mpsc::UnboundedSender<DisplayMessage>>,
}

impl CTraderFixClient {
    pub fn new(
        host: String,
        port: u16,
        sender_comp_id: String,
        target_comp_id: String,
        sender_sub_id: String,
        target_sub_id: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            host,
            port,
            sender_comp_id,
            target_comp_id,
            sender_sub_id,
            target_sub_id,
            username,
            password,
            msg_seq_num: Arc::new(Mutex::new(1)),
            tick_sender: None,
            parser: MarketDataParser::new(),
            last_message_time: Arc::new(StdMutex::new(None)),
            display_sender: None,
        }
    }

    /// Create a new client with tick streaming channel
    pub fn with_tick_channel(
        host: String,
        port: u16,
        sender_comp_id: String,
        target_comp_id: String,
        sender_sub_id: String,
        target_sub_id: String,
        username: String,
        password: String,
    ) -> (Self, mpsc::UnboundedReceiver<MarketTick>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let client = Self {
            host,
            port,
            sender_comp_id,
            target_comp_id,
            sender_sub_id,
            target_sub_id,
            username,
            password,
            msg_seq_num: Arc::new(Mutex::new(1)),
            tick_sender: Some(tx),
            parser: MarketDataParser::new(),
            last_message_time: Arc::new(StdMutex::new(None)),
            display_sender: None,
        };

        (client, rx)
    }

    pub async fn connect_and_run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔌 Connecting to cTrader FIX API...");
        println!("   Host: {}:{}", self.host, self.port);
        println!("   SenderCompID: {}", self.sender_comp_id);
        println!("   TargetCompID: {}", self.target_comp_id);
        println!();

        // Connect to cTrader
        let stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;
        println!("✅ TCP connection established!");

        // Spawn async display task for non-blocking output
        let (display_tx, mut display_rx) = mpsc::unbounded_channel::<DisplayMessage>();
        tokio::spawn(async move {
            println!("📺 Display task started (async, non-blocking)\n");
            while let Some(msg) = display_rx.recv().await {
                println!("║ [ 55] Symbol      = {:<30}", msg.symbol);
                println!("║ [268] NoMDEntries = {:<30}", msg.num_entries);
                println!("║ [269] MDEntryType = {:<30}", msg.entry_type);
                println!("║ [270] MDEntryPx   = {:<30}", msg.entry_price);
                println!("║ ⏱️  Elapsed      = {}ms", msg.elapsed_ms);
                println!();
            }
        });
        self.display_sender = Some(display_tx);

        // Why mut on reader?
        // Reading requires mutable access to track internal position

        // Writing doesn't require outer mutability
        let (mut reader, writer) = stream.into_split();

        // Wrap writer in Arc<Mutex> for sharing across tasks
        let writer = Arc::new(Mutex::new(writer));

        // Send Logon message
        println!("\n📤 Sending Logon message...");
        let logon_msg = create_logon_message(
            &self.sender_comp_id,
            &self.target_comp_id,
            &self.sender_sub_id,
            &self.target_sub_id,
            &self.username,
            &self.password,
        );

        println!("   Logon message: {}", format_for_display(&logon_msg));
        {
            let mut w = writer.lock().await;
            w.write_all(logon_msg.as_bytes()).await?;
            w.flush().await?;
        }

        // Increment sequence number
        {
            let mut seq = self.msg_seq_num.lock().await;
            *seq += 1;
        }

        println!("✅ Logon message sent!");

        // Spawn heartbeat task with shared writer
        let sender_comp_id = self.sender_comp_id.clone();
        let target_comp_id = self.target_comp_id.clone();
        let sender_sub_id = self.sender_sub_id.clone();
        let target_sub_id = self.target_sub_id.clone();
        let msg_seq_num = Arc::clone(&self.msg_seq_num);
        let writer_clone = Arc::clone(&writer);

        tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(30));
            loop {
                heartbeat_interval.tick().await;
                let seq = {
                    let mut s = msg_seq_num.lock().await;
                    let current = *s;
                    *s += 1;
                    current
                };
                let hb = create_heartbeat(&sender_comp_id, &target_comp_id, &sender_sub_id, &target_sub_id, seq);
                println!("\n💓 Sending Heartbeat (seq {})", seq);

                // Actually send the heartbeat!
                let mut w = writer_clone.lock().await;
                if let Err(e) = w.write_all(hb.as_bytes()).await {
                    eprintln!("❌ Failed to send heartbeat: {}", e);
                    break;
                }
                if let Err(e) = w.flush().await {
                    eprintln!("❌ Failed to flush heartbeat: {}", e);
                    break;
                }
            }
        });

        // Read responses
        println!("\n📥 Waiting for responses from cTrader...\n");
        let mut buffer = vec![0u8; 8192]; // Increased buffer size
        let mut accumulated_data = Vec::new();

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    println!("\n🔴 Connection closed by server");
                    break;
                }
                Ok(n) => {
                    accumulated_data.extend_from_slice(&buffer[..n]);

                    // Try to extract complete FIX messages (terminated by SOH after checksum)
                    while let Some(msg) = 
                        self.extract_message(&mut accumulated_data) {
                        self.handle_message(&msg, &writer).await?;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error reading from stream: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_message(&self, buffer: &mut Vec<u8>) -> Option<String> {
        // Look for complete FIX message (starts with "8=FIX" and ends with checksum)
        // This is a simplified implementation
        if let Ok(s) = String::from_utf8(buffer.clone()) {
            if s.contains("10=") && s.contains("\x01") {
                // Find the end of the first complete message
                if let Some(checksum_pos) = s.find("10=") {
                    if let Some(end_pos) = s[checksum_pos..].find("\x01") {
                        let full_end = checksum_pos + end_pos + 1;
                        let message = s[..full_end].to_string();
                        buffer.drain(..full_end);
                        return Some(message);
                    }
                }
            }
        }
        None
    }

    async fn handle_message(
        &mut self,
        raw_message: &str,
        writer: &Arc<Mutex<OwnedWriteHalf>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fields = parse_fix_message(raw_message);

        // Get message type
        let msg_type = fields.get(&35).map(|s| s.as_str()).unwrap_or("Unknown");

        // Verbose message display removed for performance
        // Market data display now handled by async display task

        // Handle specific message types
        match msg_type {
            "A" => {
                // Logon response received, send Security List Request
                println!("✅ Logon successful! Sending Security List Request...\n");

                let seq = {
                    let mut s = self.msg_seq_num.lock().await;
                    let current = *s;
                    *s += 1;
                    current
                };

                // Request list of all available symbols
                let sec_list_req = create_security_list_request(
                    &self.sender_comp_id,
                    &self.target_comp_id,
                    &self.sender_sub_id,
                    &self.target_sub_id,
                    seq,
                    None,  // None = request ALL symbols
                );

                println!("📤 Security List Request: {}", format_for_display(&sec_list_req));
                let mut w = writer.lock().await;
                w.write_all(sec_list_req.as_bytes()).await?;
                w.flush().await?;
            }
            "1" => {
                // Test Request - respond with Heartbeat
                let seq = {
                    let mut s = self.msg_seq_num.lock().await;
                    let current = *s;
                    *s += 1;
                    current
                };
                let hb = create_heartbeat(
                    &self.sender_comp_id,
                    &self.target_comp_id,
                    &self.sender_sub_id,
                    &self.target_sub_id,
                    seq,
                );
                let mut w = writer.lock().await;
                w.write_all(hb.as_bytes()).await?;
                w.flush().await?;
            }
            "W" => {
                self.process_market_data(raw_message);
            }
            "X" => {
                self.process_market_data(raw_message);
            }
            "y" => {
                // Security List Response - parse and display symbols
                self.handle_security_list_response(raw_message, writer).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn display_security_list_header(request_id: &str, result_code: u32, symbol_data: &[SymbolData]){
        println!("║ 📋 SECURITY LIST RESPONSE                                    ║");
        println!("║ Request ID: {:<50}║", request_id);
        println!("║ Result: {:<54}║", Self::format_security_result_code(result_code));
        println!("║ Total Symbols: {:<47}║", symbol_data.len());
        println!("╠══════════════════════════════════════════════════════════════╣");

        if !symbol_data.is_empty() {
            Self::display_symbol_data(symbol_data);
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
    }

    fn format_security_result_code(result_code: u32) -> &'static str {
        match result_code {
            0 => "✅ Valid request",
            1 => "❌ Invalid/unsupported request",
            2 => "⚠️  No instruments found",
            3 => "🔒 Not authorized",
            4 => "⏳ Data temporarily unavailable",
            5 => "❌ Request not supported",
            _ => "❓ Unknown result",
        }
    }

    fn display_symbol_data(symbol_data: &[SymbolData]){
        println!("║ Available Symbols:                                           ║");
        println!("║ {:<4} {:<20} {:<6}                           ║", "ID", "Name", "Digits");
        println!("╠══════════════════════════════════════════════════════════════╣");

        let display_count = symbol_data.len().min(20);
        for symbol_data in symbol_data.iter().take(display_count) {
            println!("║ {:<4} {:<20} {:<6}                           ║", symbol_data.symbol_id, symbol_data.symbol_name, symbol_data.symbol_digits);
        }

        if symbol_data.len() > 20 {
            println!("║ ... and {} more symbols                                   ║", symbol_data.len() - 20);
        }
    }




    /// Handle Security List Response (MsgType=y)
    /// Parses and displays the list of available trading symbols
    /// Then sends a Market Data Request for the first few symbols
    async fn handle_security_list_response(
        &mut self,
        raw_message: &str,
        writer: &Arc<Mutex<OwnedWriteHalf>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((req_id, result, symbols)) = parse_security_list_response(raw_message) {
            Self::display_security_list_header(&req_id, result, &symbols);

            // Send Market Data Request using received symbols
            if !symbols.is_empty() && result == 0 {
                println!("📊 Sending Market Data Request for symbols...\n");

                let seq = {
                    let mut s = self.msg_seq_num.lock().await;
                    let current = *s;
                    *s += 1;
                    current
                };

                // Request market data for first few symbols (limit to avoid overwhelming)
                let symbol_ids: Vec<String> = symbols
                    .iter()
                    .map(|symbol_data| {
                        println!("  ✓ Subscribing to: {} (ID: {})", symbol_data.symbol_name, symbol_data.symbol_id);
                        symbol_data.symbol_id.to_string()
                    })
                    .collect();

                let symbol_id_refs: Vec<&str> = symbol_ids.iter().map(|s| s.as_str()).collect();

                let md_request = create_market_data_request(
                    &self.sender_comp_id,
                    &self.target_comp_id,
                    &self.sender_sub_id,
                    &self.target_sub_id,
                    seq,
                    &symbol_id_refs,
                );

                println!("\n📤 Market Data Request: {}", format_for_display(&md_request));
                let mut w = writer.lock().await;
                w.write_all(md_request.as_bytes()).await?;
                w.flush().await?;
            }
        } else {
            eprintln!("⚠️  Failed to parse Security List Response");
        }

        Ok(())
    }

    /// Process market data - lightweight extraction and async display
    /// Non-blocking: sends to display channel instead of printing directly
    fn process_market_data(&self, raw_message: &str) {
        // Capture current time immediately for latency tracking
        let current_time = Instant::now();

        // Calculate elapsed time since last message
        let elapsed_ms = {
            let mut last_time = self.last_message_time.lock().unwrap();

            // Calculate elapsed using Option::map for clean code
            let elapsed = last_time
                .map(|prev| prev.elapsed().as_millis() as i64)
                .unwrap_or(0);

            // Update last message time for next calculation
            *last_time = Some(current_time);

            elapsed
        }; // Mutex lock is dropped here

        // Lightweight field extraction (no heavy tick building)
        let fields = parse_fix_message(raw_message);

        // Extract only the 4 essential fields
        // let display_msg = DisplayMessage {
        //     symbol: fields.get(&55).cloned().unwrap_or_else(|| "N/A".to_string()),
        //     num_entries: fields.get(&268).cloned().unwrap_or_else(|| "N/A".to_string()),
        //     entry_type: fields.get(&269).cloned().unwrap_or_else(|| "N/A".to_string()),
        //     entry_price: fields.get(&270).cloned().unwrap_or_else(|| "N/A".to_string()),
        //     elapsed_ms,
        // };

        // // Send to async display task (non-blocking!)
        // if let Some(ref tx) = self.display_sender {
        //     let _ = tx.send(display_msg);
        // }

        // Still build and send tick for other consumers if needed
        if let Some(ref tx) = self.tick_sender {
            if let Some((symbol_id, entries)) = self.parser.parse_market_data(raw_message) {
                let tick = self.parser.build_tick(symbol_id, entries);
                let _ = tx.send(tick);
            }
        }
    }
}
