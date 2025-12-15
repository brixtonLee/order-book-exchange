use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Represents a trading symbol/instrument from cTrader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol ID (cTrader internal identifier)
    pub id: u32,
    /// Symbol name (e.g., "EURUSD", "GBPUSD")
    pub name: String,
    /// Number of decimal places for prices (0-5)
    pub digits: u8,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(id: u32, name: String, digits: u8) -> Self {
        Self { id, name, digits }
    }
}

/// High-performance market tick data structure
/// Optimized for real-time streaming with minimal allocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTick {
    /// Symbol identifier (cTrader symbol ID)
    pub symbol_id: String,
    /// Timestamp when tick was received
    pub timestamp: DateTime<Utc>,
    /// Bid price
    pub bid_price: Option<Decimal>,
    /// Bid size/volume
    pub bid_size: Option<Decimal>,
    /// Ask price
    pub ask_price: Option<Decimal>,
    /// Ask size/volume
    pub ask_size: Option<Decimal>,
}

impl MarketTick {
    /// Create a new empty market tick
    pub fn new(symbol_id: String) -> Self {
        Self {
            symbol_id,
            timestamp: Utc::now(),
            bid_price: None,
            bid_size: None,
            ask_price: None,
            ask_size: None,
        }
    }

    /// Calculate mid price if both bid and ask are available
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.bid_price, self.ask_price) {
            (Some(bid), Some(ask)) => Some((bid + ask) / Decimal::from(2)),
            _ => None,
        }
    }

    /// Calculate spread if both bid and ask are available
    pub fn spread(&self) -> Option<Decimal> {
        match (self.bid_price, self.ask_price) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Check if tick has complete bid/ask data
    pub fn is_complete(&self) -> bool {
        self.bid_price.is_some() && self.ask_price.is_some()
    }
}

/// Market data entry parsed from FIX message
/// This represents a single entry in the NoMDEntries repeating group
#[derive(Debug, Clone)]
pub struct MarketDataEntry {
    /// Entry type: 0=Bid, 1=Offer/Ask, 2=Trade
    pub entry_type: MDEntryType,
    /// Price for this entry
    pub price: Decimal,
    /// Size/volume for this entry
    pub size: Decimal,
}

/// MD Entry Type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MDEntryType {
    Bid = 0,
    Offer = 1,
    Trade = 2,
}

impl MDEntryType {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::Bid),
            '1' => Some(Self::Offer),
            '2' => Some(Self::Trade),
            _ => None,
        }
    }
}

/// Helper struct to manage the state of building a market data entry
/// This encapsulates the state machine for parsing repeating groups
struct EntryBuilder {
    entry_type: Option<MDEntryType>,
    price: Option<Decimal>,
    size: Option<Decimal>,
}

impl EntryBuilder {
    /// Create a new empty entry builder
    fn new() -> Self {
        Self {
            entry_type: None,
            price: None,
            size: None,
        }
    }

    /// Set the entry type from a FIX field value
    fn set_entry_type(&mut self, value: &str) {
        self.entry_type = value.chars().next().and_then(MDEntryType::from_char);
    }

    /// Set the price from a FIX field value
    fn set_price(&mut self, value: &str) {
        self.price = Decimal::from_str(value).ok();
    }

    /// Set the size from a FIX field value
    fn set_size(&mut self, value: &str) {
        self.size = Decimal::from_str(value).ok();
    }

    /// Try to build a complete MarketDataEntry if all fields are present
    /// Returns Some(entry) if complete, None if any field is missing
    fn try_build(&self) -> Option<MarketDataEntry> {
        match (self.entry_type, self.price, self.size) {
            (Some(et), Some(p), Some(s)) => Some(MarketDataEntry {
                entry_type: et,
                price: p,
                size: s,
            }),
            _ => None,
        }
    }

    /// Reset the builder state to start a new entry
    fn reset(&mut self) {
        self.entry_type = None;
        self.price = None;
        self.size = None;
    }
}

/// Optimized FIX message parser for market data
/// Uses zero-copy parsing where possible to minimize allocations
pub struct MarketDataParser {
    // Keeping this for potential future use with advanced parsing
    #[allow(dead_code)]
    buffer: Vec<u8>,
}

impl MarketDataParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(4096),
        }
    }

    /// Parse a single FIX field in the format "tag=value"
    /// Returns Some((tag, value)) if valid, None otherwise
    fn parse_fix_field(field: &str) -> Option<(u32, &str)> {
        // split_once splits the string at the first occurrence of '='
        // and_then will run the closure if the || equals to Some(T), if it is None, then skip the closure and return none
        field.split_once('=').and_then(|(tag_str, value)| {
            tag_str.parse::<u32>().ok().map(|tag| (tag, value))
        })
    }

    /// Handle a single FIX field and update parser state accordingly
    /// This processes the field based on its tag and delegates to appropriate handlers
    fn handle_field(
        tag: u32,
        value: &str,
        symbol_id: &mut Option<String>,
        builder: &mut EntryBuilder,
        entries: &mut Vec<MarketDataEntry>,
    ) {
        match tag {
            55 => {
                // Symbol - store the instrument identifier
                *symbol_id = Some(value.to_string());
            }
            269 => {
                // MDEntryType - signals start of new entry, finalize previous if complete
                if let Some(entry) = builder.try_build() {
                    entries.push(entry);
                }
                // Start new entry and reset state
                builder.reset();
                builder.set_entry_type(value);
            }
            270 => {
                // MDEntryPx - price for current entry
                builder.set_price(value);
            }
            271 => {
                // MDEntrySize - volume/size for current entry
                builder.set_size(value);
            }
            _ => {
                // Ignore unknown tags
            }
        }
    }

    /// Parse a market data snapshot or incremental refresh message
    /// Returns (symbol_id, entries)
    ///
    /// This function orchestrates the parsing by:
    /// 1. Splitting the FIX message into fields
    /// 2. Parsing each field into (tag, value) pairs
    /// 3. Delegating field handling to process each tag
    /// 4. Finalizing any incomplete entry at the end
    pub fn parse_market_data(&self, raw_message: &str) -> Option<(String, Vec<MarketDataEntry>)> {
        let mut symbol_id = None;
        let mut entries = Vec::with_capacity(4); // Pre-allocate for typical 2-4 entries
        let mut builder = EntryBuilder::new();

        // Parse each field in the FIX message
        for field in raw_message.split('\x01') {
            if let Some((tag, value)) = Self::parse_fix_field(field) {
                Self::handle_field(tag, value, &mut symbol_id, &mut builder, &mut entries);
            }
        }

        // Finalize the last entry if it's complete
        if let Some(entry) = builder.try_build() {
            entries.push(entry);
        }

        symbol_id.map(|sym| (sym, entries))
    }

    /// Build a MarketTick from parsed entries
    pub fn build_tick(&self, symbol_id: String, entries: Vec<MarketDataEntry>) -> MarketTick {
        let mut tick = MarketTick::new(symbol_id);

        for entry in entries {
            match entry.entry_type {
                MDEntryType::Bid => {
                    tick.bid_price = Some(entry.price);
                    tick.bid_size = Some(entry.size);
                }
                MDEntryType::Offer => {
                    tick.ask_price = Some(entry.price);
                    tick.ask_size = Some(entry.size);
                }
                MDEntryType::Trade => {
                    // For trades, we might want to update both bid and ask
                    // or handle differently depending on use case
                }
            }
        }

        tick
    }

    /// Quick check if message is a market data message
    pub fn is_market_data_message(raw_message: &str) -> bool {
        // Check for MsgType=W (Snapshot) or X (Incremental Refresh)
        raw_message.contains("35=W\x01") || raw_message.contains("35=X\x01")
    }
}

impl Default for MarketDataParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_tick_calculations() {
        let mut tick = MarketTick::new("41".to_string());
        tick.bid_price = Some(Decimal::from_str("2650.50").unwrap());
        tick.ask_price = Some(Decimal::from_str("2651.00").unwrap());

        let mid = tick.mid_price().unwrap();
        assert_eq!(mid.to_string(), "2650.75");

        let spread = tick.spread().unwrap();
        assert_eq!(spread.to_string(), "0.50");
    }

    #[test]
    fn test_parse_market_data() {
        let parser = MarketDataParser::new();

        // Simplified FIX message
        let msg = "8=FIX.4.4\x0135=W\x0155=41\x01268=2\x01269=0\x01270=2650.50\x01271=100\x01269=1\x01270=2651.00\x01271=150\x0110=123\x01";

        if let Some((symbol, entries)) = parser.parse_market_data(msg) {
            assert_eq!(symbol, "41");
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].entry_type, MDEntryType::Bid);
            assert_eq!(entries[1].entry_type, MDEntryType::Offer);
        } else {
            panic!("Failed to parse market data");
        }
    }
}
