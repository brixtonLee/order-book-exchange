use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::{interval, Duration};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use super::messages::{
    create_logon_message, create_market_data_request, create_heartbeat,
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
                    while let Some(msg) = self.extract_message(&mut accumulated_data) {
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
                // Logon response received, send Market Data Request
                println!("✅ Logon successful! Sending Market Data Request...\n");

                let seq = {
                    let mut s = self.msg_seq_num.lock().await;
                    let current = *s;
                    *s += 1;
                    current
                };

                // Request data for XAUUSD (Gold) - symbol ID "41"
                let md_request = create_market_data_request(
                    &self.sender_comp_id,
                    &self.target_comp_id,
                    &self.sender_sub_id,
                    &self.target_sub_id,
                    seq,
                    &["41"], // Symbol ID 41 = XAUUSD (Gold)
                );

                println!("📤 Market Data Request: {}", format_for_display(&md_request));
                let mut w = writer.lock().await;
                w.write_all(md_request.as_bytes()).await?;
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
                // Market Data Snapshot - this contains the price data!
                println!("💰 Market Data Snapshot received!");
                self.process_market_data(raw_message);
            }
            "X" => {
                // Market Data Incremental Refresh - streaming updates!
                println!("⚡ Market Data Incremental Refresh!");
                self.process_market_data(raw_message);
            }
            _ => {}
        }

        Ok(())
    }

    /// Process market data using optimized parser and stream to channel
    fn process_market_data(&self, raw_message: &str) {
        // Use optimized parser
        if let Some((symbol_id, entries)) = self.parser.parse_market_data(raw_message) {
            let tick = self.parser.build_tick(symbol_id.clone(), entries);

            // Display tick information
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║ 📊 MARKET TICK - Symbol ID: {:<35}║", symbol_id);
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
