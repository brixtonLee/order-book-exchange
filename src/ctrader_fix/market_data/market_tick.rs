use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
    /// Ask price
    pub ask_price: Option<Decimal>,
}

impl MarketTick {
    /// Create a new empty market tick
    pub fn new(symbol_id: String) -> Self {
        Self {
            symbol_id,
            timestamp: Utc::now(),
            bid_price: None,
            ask_price: None,
        }
    }

    /// Calculate mid-price if both bid and ask are available
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