use chrono::Utc;
use std::collections::HashMap;

/// FIX message builder for cTrader FIX 4.4 protocol
pub struct FixMessage {
    fields: HashMap<u32, String>,
    body_fields: Vec<(u32, String)>,      // Body fields in insertion order
    repeating_groups: Vec<(u32, String)>, // For repeating fields like 269
}

impl FixMessage {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            body_fields: Vec::new(),
            repeating_groups: Vec::new(),
        }
    }

    /// Add a field to the FIX message (preserves insertion order for body fields)
    pub fn add_field(&mut self, tag: u32, value: impl ToString) {
        let value_str = value.to_string();
        self.fields.insert(tag, value_str.clone());

        // Track body fields (not header fields) in insertion order
        if tag != 49 && tag != 56 && tag != 34 && tag != 52 && tag != 50 && tag != 57 {
            self.body_fields.push((tag, value_str));
        }
    }

    /// Add a repeating field (can have multiple values for same tag)
    pub fn add_repeating_field(&mut self, tag: u32, value: impl ToString) {
        self.repeating_groups.push((tag, value.to_string()));
    }

    /// Build the FIX message with proper formatting and field ordering
    pub fn build(&self, msg_type: &str) -> String {
        let mut message = String::new();

        // Start with BeginString (tag 8)
        message.push_str("8=FIX.4.4\x01");

        // Build body with STRICT field ordering per FIX 4.4 specification
        let mut body = String::new();

        // MsgType MUST be first in body (tag 35)
        body.push_str(&format!("35={}\x01", msg_type));

        // Standard Header fields in required order:
        // 49=SenderCompID, 56=TargetCompID, 34=MsgSeqNum, 52=SendingTime
        // Then optional: 50=SenderSubID, 57=TargetSubID

        let header_order = [49, 56, 34, 52, 50, 57]; // Strict order for header
        for tag in header_order {
            if let Some(value) = self.fields.get(&tag) {
                body.push_str(&format!("{}={}\x01", tag, value));
            }
        }

        // Add body fields in INSERTION ORDER (critical for FIX repeating groups!)
        for (tag, value) in &self.body_fields {
            body.push_str(&format!("{}={}\x01", tag, value));
        }

        // Add repeating groups immediately after their count field
        for (tag, value) in &self.repeating_groups {
            body.push_str(&format!("{}={}\x01", tag, value));
        }

        // Add BodyLength (tag 9)
        message.push_str(&format!("9={}\x01", body.len()));
        message.push_str(&body);

        // Calculate and add checksum (tag 10)
        let checksum = calculate_checksum(&message);
        message.push_str(&format!("10={:03}\x01", checksum));

        message
    }
}

/// Calculate FIX checksum (sum of all bytes modulo 256)
fn calculate_checksum(message: &str) -> u8 {
    message.bytes().fold(0u32, |acc, b| acc + b as u32) as u8
}

/// Parse incoming FIX message into a HashMap
pub fn parse_fix_message(raw_message: &str) -> HashMap<u32, String> {
    let mut fields = HashMap::new();

    for field in raw_message.split('\x01') {
        if let Some((tag, value)) = field.split_once('=') {
            if let Ok(tag_num) = tag.parse::<u32>() {
                fields.insert(tag_num, value.to_string());
            }
        }
    }

    fields
}

/// Create a Logon message (MsgType=A)
pub fn create_logon_message(
    sender_comp_id: &str,
    target_comp_id: &str,
    sender_sub_id: &str,
    target_sub_id: &str,
    username: &str,
    password: &str,
) -> String {
    let mut msg = FixMessage::new();

    // Standard FIX fields
    msg.add_field(49, sender_comp_id);        // SenderCompID
    msg.add_field(56, target_comp_id);        // TargetCompID
    msg.add_field(50, sender_sub_id);         // SenderSubID
    msg.add_field(57, target_sub_id);         // TargetSubID (REQUIRED by cTrader!)
    msg.add_field(34, 1);                     // MsgSeqNum (start at 1)
    msg.add_field(52, Utc::now().format("%Y%m%d-%H:%M:%S%.3f").to_string()); // SendingTime

    // Logon specific fields
    msg.add_field(98, 0);                     // EncryptMethod (0 = None)
    msg.add_field(108, 30);                   // HeartBtInt (30 seconds)
    msg.add_field(141, "Y");                  // ResetSeqNumFlag
    msg.add_field(553, username);             // Username
    msg.add_field(554, password);             // Password

    msg.build("A")
}

/// Create a Market Data Request message (MsgType=V)
///
/// CRITICAL: This message has complex repeating groups that must be in exact order.
/// We build it manually instead of using FixMessage to ensure correct field ordering.
pub fn create_market_data_request(
    sender_comp_id: &str,
    target_comp_id: &str,
    sender_sub_id: &str,
    target_sub_id: &str,
    msg_seq_num: u32,
    symbol_ids: &[&str],
) -> String {

    let sending_time = Utc::now().format("%Y%m%d-%H:%M:%S%.3f").to_string();
    let md_req_id = format!("REQ-{}", Utc::now().timestamp_millis());

    // Build message body with EXACT field order required by cTrader
    let mut body = String::new();

    // Header fields in required order
    body.push_str(&format!("35=V\x01"));                        // MsgType
    body.push_str(&format!("49={}\x01", sender_comp_id));       // SenderCompID
    body.push_str(&format!("56={}\x01", target_comp_id));       // TargetCompID
    body.push_str(&format!("34={}\x01", msg_seq_num));          // MsgSeqNum
    body.push_str(&format!("52={}\x01", sending_time));         // SendingTime
    body.push_str(&format!("50={}\x01", sender_sub_id));        // SenderSubID
    body.push_str(&format!("57={}\x01", target_sub_id));        // TargetSubID

    // Market Data Request fields in EXACT order
    body.push_str(&format!("262={}\x01", md_req_id));           // MDReqID
    body.push_str("263=1\x01");                                  // SubscriptionRequestType (Subscribe)
    body.push_str("264=1\x01");                                  // MarketDepth (Spot)
    body.push_str("265=1\x01");                                  // MDUpdateType (Incremental)

    // FIRST repeating group: NoRelatedSym + Symbol(s)
    body.push_str(&format!("146={}\x01", symbol_ids.len()));    // NoRelatedSym
    for symbol_id in symbol_ids {
        body.push_str(&format!("55={}\x01", symbol_id));        // Symbol (immediately after count!)
    }

    // SECOND repeating group: NoMDEntryTypes + MDEntryType(s)
    body.push_str("267=2\x01");                                  // NoMDEntryTypes
    body.push_str("269=0\x01");                                  // MDEntryType = Bid
    body.push_str("269=1\x01");                                  // MDEntryType = Offer

    // Build final message with header and trailer
    let mut message = String::new();
    message.push_str("8=FIX.4.4\x01");                          // BeginString
    message.push_str(&format!("9={}\x01", body.len()));         // BodyLength
    message.push_str(&body);

    // Calculate and add checksum
    let checksum = calculate_checksum(&message);
    message.push_str(&format!("10={:03}\x01", checksum));       // CheckSum

    message
}

/// Create a Heartbeat message (MsgType=0)
pub fn create_heartbeat(
    sender_comp_id: &str,
    target_comp_id: &str,
    sender_sub_id: &str,
    target_sub_id: &str,
    msg_seq_num: u32,
) -> String {
    let mut msg = FixMessage::new();

    msg.add_field(49, sender_comp_id);        // SenderCompID
    msg.add_field(56, target_comp_id);        // TargetCompID
    msg.add_field(50, sender_sub_id);         // SenderSubID
    msg.add_field(57, target_sub_id);         // TargetSubID
    msg.add_field(34, msg_seq_num);           // MsgSeqNum
    msg.add_field(52, Utc::now().format("%Y%m%d-%H:%M:%S%.3f").to_string()); // SendingTime

    msg.build("0")
}

/// Create a Security List Request message (MsgType=x)
/// Requests the list of available trading symbols from cTrader
pub fn create_security_list_request(
    sender_comp_id: &str,
    target_comp_id: &str,
    sender_sub_id: &str,
    target_sub_id: &str,
    msg_seq_num: u32,
    symbol_id: Option<&str>,
) -> String {
    let mut msg = FixMessage::new();

    // Standard FIX fields
    msg.add_field(49, sender_comp_id);        // SenderCompID
    msg.add_field(56, target_comp_id);        // TargetCompID
    msg.add_field(50, sender_sub_id);         // SenderSubID
    msg.add_field(57, target_sub_id);         // TargetSubID
    msg.add_field(34, msg_seq_num);           // MsgSeqNum
    msg.add_field(52, Utc::now().format("%Y%m%d-%H:%M:%S%.3f").to_string()); // SendingTime

    // Security List Request specific fields
    let req_id = format!("SECLST-{}", Utc::now().timestamp_millis());
    msg.add_field(320, req_id);               // SecurityReqID (unique ID)
    msg.add_field(559, 0);                    // SecurityListRequestType (0 = Symbol)

    // Optional: request specific symbol
    if let Some(sym_id) = symbol_id {
        msg.add_field(55, sym_id);            // Symbol
    }

    msg.build("x")
}

/// Extract metadata from Security List Response
/// Returns (request_id, result_code, num_symbols)
fn extract_response_metadata(fields: &HashMap<u32, String>) -> Option<(String, u32, usize)> {
    let req_id = fields.get(&320)?.clone();                    // SecurityReqID
    let result = fields.get(&560)?.parse::<u32>().ok()?;      // SecurityRequestResult
    let num_symbols = fields.get(&146)                         // NoRelatedSym
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    Some((req_id, result, num_symbols))
}


#[derive(Debug, Clone)]
pub struct SymbolData{
    pub symbol_id: u32,
    pub symbol_name: String,
    pub symbol_digits: u8
}

struct SymbolEntryBuilder{
    symbol_id: Option<u32>,
    symbol_name: Option<String>,
    symbol_digits: Option<u8>,
}

impl SymbolEntryBuilder {
    fn new() -> Self {
        Self {
            symbol_id: None,
            symbol_name: None,
            symbol_digits: None,
        }
    }

    fn set_symbol_id(&mut self, value: u32) {
        self.symbol_id = Some(value);
    }

    fn set_symbol_name(&mut self, value: &str) {
        self.symbol_name = Some(value.to_string());
    }

    fn set_symbol_digits(&mut self, value: u8) {
        self.symbol_digits = Some(value);
    }

    // Since symbol_name is Option<String>,
    // when it is matched, Some(name) is returned where name is the reference (&String)
    // SymbolData needs to own the string, not a reference
    // Hence clone is required at here
    fn try_complete_symbol_data(
        &self
    ) -> Option<SymbolData> {
        match (self.symbol_id, self.symbol_name.clone(), self.symbol_digits) {
            (Some(id), Some(name), Some(digits)) => Some(SymbolData {
                symbol_id: id,
                symbol_name: name,
                symbol_digits: digits
            }),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.symbol_id = None;
        self.symbol_name = None;
        self.symbol_digits = None;
    }
}

/// Parse repeating group of symbols from FIX message
fn parse_symbol_fields(raw_message: &str) -> Vec<SymbolData> {
    let mut symbol_entries = Vec::new();
    let mut symbol_builder = SymbolEntryBuilder::new();

    // Parse field by field to handle repeating groups
    for field in raw_message.split('\x01') {
        if let Some((tag, value)) = parse_fix_field(field) {
            handle_field(tag, value, &mut symbol_builder, &mut symbol_entries);
            }
        }

    // Save last symbol if complete
    if let Some(symbol) = symbol_builder.try_complete_symbol_data() {
        symbol_entries.push(symbol);
    }

    symbol_entries
}

fn parse_fix_field(field: &str) -> Option<(u32, &str)> {
    // split_once splits the string at the first occurrence of '='
    // and_then will run the closure if the || equals to Some(T), if it is None, then skip the closure and return none
    field.split_once('=').and_then(|(tag_str, value)| {
        tag_str.parse::<u32>().ok().map(|tag| (tag, value))
    })
}
    
fn handle_field(tag: u32, value: &str, builder: &mut SymbolEntryBuilder, entries: &mut Vec<SymbolData>){
    match tag {
        55 => {
            // Use .ok() to safely parse - returns None if parse fails
            if let Some(symbol_id) = value.parse::<u32>().ok() {
                builder.set_symbol_id(symbol_id);

                // New symbol ID - save previous symbol if complete
                if let Some(symbol) = builder.try_complete_symbol_data() {
                    entries.push(symbol);
                }
            }
            builder.reset();
        }
        1007 => {
            // Symbol name
            builder.set_symbol_name(value);
        }
        1008 => {
            // Use .ok() to safely parse - returns None if parse fails
            if let Some(digits) = value.parse::<u8>().ok() {
                builder.set_symbol_digits(digits);
            }
        }
        _ => {}
    }
}

/// Parse a Security List Response message (MsgType=y)
/// Returns (request_id, result_code, symbols)
pub fn parse_security_list_response(raw_message: &str) -> Option<(String, u32, Vec<SymbolData>)> {
    let fields = parse_fix_message(raw_message);
    let (req_id, result, _num_symbols) = extract_response_metadata(&fields)?;
    let symbols = parse_symbol_fields(raw_message);

    Some((req_id, result, symbols))
}

/// Format FIX message for display (replace SOH with |)
pub fn format_for_display(message: &str) -> String {
    message.replace('\x01', " | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_calculation() {
        let test_msg = "8=FIX.4.4\x019=5\x0135=A\x01";
        let checksum = calculate_checksum(test_msg);
        // Checksum should be valid u8 (0-255)
        assert!(checksum <= 255);
    }

    #[test]
    fn test_parse_fix_message() {
        let raw = "8=FIX.4.4\x019=100\x0135=A\x0149=SENDER\x0156=TARGET\x0110=123\x01";
        let fields = parse_fix_message(raw);

        assert_eq!(fields.get(&8), Some(&"FIX.4.4".to_string()));
        assert_eq!(fields.get(&35), Some(&"A".to_string()));
        assert_eq!(fields.get(&49), Some(&"SENDER".to_string()));
    }
}
