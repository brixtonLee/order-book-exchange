use rust_decimal::Decimal;
use std::str::FromStr;
use crate::ctrader_fix::{parse_fix_field, MarketTick};

/// Market data entry parsed from FIX message
/// This represents a single entry in the NoMDEntries repeating group
#[derive(Debug, Clone)]
pub struct MarketDataEntry {
    /// Entry type: 0=Bid, 1=Offer/Ask, 2=Trade
    pub entry_type: MDEntryType,
    /// Price for this entry
    pub price: Decimal
}

/// MD Entry Type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MDEntryType {
    Bid = 0,
    Offer = 1,
}

impl MDEntryType {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::Bid),
            '1' => Some(Self::Offer),
            _ => None,
        }
    }
}

/// Helper struct to manage the state of building a market data entry
/// This encapsulates the state machine for parsing repeating groups
struct EntryBuilder {
    entry_type: Option<MDEntryType>,
    price: Option<Decimal>,
}

impl EntryBuilder {
    /// Create a new empty entry builder
    fn new() -> Self {
        Self {
            entry_type: None,
            price: None,
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

    /// Try to build a complete MarketDataEntry if all fields are present
    /// Returns Some(entry) if complete, None if any field is missing
    fn try_build(&self) -> Option<MarketDataEntry> {
        match (self.entry_type, self.price) {
            (Some(et), Some(p)) => Some(MarketDataEntry {
                entry_type: et,
                price: p,
            }),
            _ => None,
        }
    }

    /// Reset the builder state to start a new entry
    fn reset(&mut self) {
        self.entry_type = None;
        self.price = None;
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
        let mut entries = vec![]; 
        let mut builder = EntryBuilder::new();

        // Parse each field in the FIX message
        for field in raw_message.split('\x01') {
            if let Some((tag, value)) = parse_fix_field(field) {
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
                }
                MDEntryType::Offer => {
                    tick.ask_price = Some(entry.price);
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
