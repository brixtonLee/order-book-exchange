use std::collections::HashMap;
use crate::ctrader_fix::messages::parse_fix_message;
use crate::ctrader_fix::parse_fix_field;

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

/// Parse a Security List Response message (MsgType=y)
/// Returns (request_id, result_code, symbols)
pub fn parse_security_list_response(raw_message: &str) -> Option<(String, u32, Vec<SymbolData>)> {
    let fields = parse_fix_message(raw_message);
    let (req_id, result, _num_symbols) = extract_response_metadata(&fields)?;
    let symbols = parse_symbol_fields(raw_message);

    Some((req_id, result, symbols))
}

fn extract_response_metadata(fields: &HashMap<u32, String>) -> Option<(String, u32, usize)> {
    let req_id = fields.get(&320)?.clone();                    // SecurityReqID
    let result = fields.get(&560)?.parse::<u32>().ok()?;      // SecurityRequestResult
    let num_symbols = fields.get(&146)                         // NoRelatedSym
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    Some((req_id, result, num_symbols))
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
