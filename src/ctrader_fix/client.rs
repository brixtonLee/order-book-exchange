use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::{interval, Duration};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

use super::messages::{
    create_logon_message, create_market_data_request, create_heartbeat,
    create_security_list_request, parse_security_list_response,
    parse_fix_message, format_for_display,
};
use super::market_data::{MarketTick, MarketDataParser};

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
    /// Available trading symbols from security list response
    symbols: Arc<StdMutex<Vec<(u32, String, u8)>>>,
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
            symbols: Arc::new(StdMutex::new(Vec::new())),
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
            symbols: Arc::new(StdMutex::new(Vec::new())),
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
                    while let Some(msg) = self.
                        extract_message(&mut accumulated_data) {
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

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║ 📨 RECEIVED FIX MESSAGE                                      ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Message Type: {:48}║", format!("{} ({})",
            match msg_type {
                "A" => "Logon",
                "0" => "Heartbeat",
                "1" => "Test Request",
                "5" => "Logout",
                "W" => "Market Data Snapshot",
                "X" => "Market Data Incremental Refresh",
                "Y" => "Market Data Request Reject",
                "x" => "Security List Request",
                "y" => "Security List",
                _ => "Other",
            },
            msg_type
        ));
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Raw Message:                                                 ║");
        println!("║ {:<60} ║", format_for_display(raw_message));
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Parsed Fields:                                               ║");

        // Sort and display all fields
        let mut sorted_fields: Vec<(&u32, &String)> = fields.iter().collect();
        sorted_fields.sort_by_key(|(tag, _)| *tag);

        for (tag, value) in sorted_fields {
            let field_name = get_field_name(*tag);
            let display = if value.len() > 40 {
                format!("{}...", &value[..37])
            } else {
                value.clone()
            };
            println!("║ [{:>3}] {:<20} = {:<30} ║", tag, field_name, display);
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

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

    /// Handle Security List Response (MsgType=y)
    /// Parses and displays the list of available trading symbols
    /// Then sends a Market Data Request for the first few symbols
    async fn handle_security_list_response(
        &mut self,
        raw_message: &str,
        writer: &Arc<Mutex<OwnedWriteHalf>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((req_id, result, symbols)) = parse_security_list_response(raw_message) {
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║ 📋 SECURITY LIST RESPONSE                                    ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Request ID: {:<50}║", req_id);
            println!("║ Result: {:<54}║", match result {
                0 => "✅ Valid request",
                1 => "❌ Invalid/unsupported request",
                2 => "⚠️  No instruments found",
                3 => "🔒 Not authorized",
                4 => "⏳ Data temporarily unavailable",
                5 => "❌ Request not supported",
                _ => "❓ Unknown result",
            });
            println!("║ Total Symbols: {:<47}║", symbols.len());
            println!("╠══════════════════════════════════════════════════════════════╣");

            if !symbols.is_empty() {
                println!("║ Available Symbols:                                           ║");
                println!("║ {:<4} {:<20} {:<6}                           ║", "ID", "Name", "Digits");
                println!("╠══════════════════════════════════════════════════════════════╣");

                // Display up to 20 symbols (to avoid flooding console)
                let display_count = symbols.len().min(20);
                for (id, name, digits) in symbols.iter().take(display_count) {
                    println!("║ {:<4} {:<20} {:<6}                           ║", id, name, digits);
                }

                if symbols.len() > 20 {
                    println!("║ ... and {} more symbols                                   ║", symbols.len() - 20);
                }
            }

            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();

            // Store symbols for later use
            {
                let mut stored_symbols = self.symbols.lock().unwrap();
                *stored_symbols = symbols.clone();
            }

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
                    .take(5)  // Take first 5 symbols
                    .map(|(id, name, _)| {
                        println!("  ✓ Subscribing to: {} (ID: {})", name, id);
                        id.to_string()
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

    /// Process market data using optimized parser and stream to channel
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

        // Use optimized parser
        if let Some((symbol_id, entries)) = self.parser.parse_market_data(raw_message) {
            let tick = self.parser.build_tick(symbol_id.clone(), entries);

            // Display tick information with latency
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║ 📊 MARKET TICK - Symbol ID: {:<35}║", symbol_id);
            println!("║ ⏱️  Time since last message: {:<36}║", format!("{}ms", elapsed_ms));
            println!("╠══════════════════════════════════════════════════════════════╣");

            if let Some(bid) = tick.bid_price {
                println!("║ 💵 BID:  {:<51}║", format!("{} (size: {})", bid, tick.bid_size.unwrap_or_default()));
            }
            if let Some(ask) = tick.ask_price {
                println!("║ 💶 ASK:  {:<51}║", format!("{} (size: {})", ask, tick.ask_size.unwrap_or_default()));
            }
            if let Some(mid) = tick.mid_price() {
                println!("║ 📊 MID:  {:<51}║", mid);
            }
            if let Some(spread) = tick.spread() {
                println!("║ 📏 SPREAD: {:<49}║", spread);
            }

            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();

            // Send to channel if connected
            if let Some(ref tx) = self.tick_sender {
                if let Err(e) = tx.send(tick) {
                    eprintln!("⚠️  Failed to send tick to channel: {}", e);
                }
            }
        }
    }
}

fn get_field_name(tag: u32) -> &'static str {
    match tag {
        8 => "BeginString",
        9 => "BodyLength",
        10 => "CheckSum",
        35 => "MsgType",
        49 => "SenderCompID",
        56 => "TargetCompID",
        50 => "SenderSubID",
        57 => "TargetSubID",
        34 => "MsgSeqNum",
        52 => "SendingTime",
        98 => "EncryptMethod",
        108 => "HeartBtInt",
        141 => "ResetSeqNumFlag",
        553 => "Username",
        554 => "Password",
        55 => "Symbol",
        262 => "MDReqID",
        263 => "SubscriptionReqType",
        264 => "MarketDepth",
        265 => "MDUpdateType",
        146 => "NoRelatedSym",
        268 => "NoMDEntries",
        269 => "MDEntryType",
        270 => "MDEntryPx",
        271 => "MDEntrySize",
        _ => "Unknown",
    }
}
