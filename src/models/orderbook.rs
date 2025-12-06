use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use uuid::Uuid;

use super::{Order, Trade};

/// Represents a price level in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub total_quantity: Decimal,
    pub orders: VecDeque<Uuid>,
}

impl PriceLevel {
    /// Create a new price level
    pub fn new(price: Decimal) -> Self {
        Self {
            price,
            total_quantity: Decimal::ZERO,
            orders: VecDeque::new(),
        }
    }

    /// Add an order to this price level
    pub fn add_order(&mut self, order_id: Uuid, quantity: Decimal) {
        self.orders.push_back(order_id);
        self.total_quantity += quantity;
    }

    /// Remove an order from this price level
    pub fn remove_order(&mut self, order_id: Uuid, quantity: Decimal) -> bool {
        // .position find the index of the first element that matches the condition
        if let Some(pos) = self.orders.iter().position(|&id| id == order_id) {
            self.orders.remove(pos);
            self.total_quantity -= quantity;
            return true;
        }

        false
    }

    /// Check if this price level is empty
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
}

/// The main order book structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    #[serde(skip)]
    pub bids: BTreeMap<Decimal, PriceLevel>,
    #[serde(skip)]
    pub asks: BTreeMap<Decimal, PriceLevel>,
    #[serde(skip)]
    pub orders: HashMap<Uuid, Order>,
    pub trades: Vec<Trade>,
}

impl OrderBook {
    /// Create a new order book for a symbol
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            trades: Vec::new(),
        }
    }

    // These highest buy price and lowest sell price are beneficial to broker. Since we are acting as a market maker
    /// Get the best bid price (highest buy price)
    pub fn get_best_bid(&self) -> Option<Decimal> {
        // self.bids.keys().next_back().copied()
        self.bids.keys().next_back().cloned()
    }

    /// Get the best ask price (lowest sell price)
    pub fn get_best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    /// Get the spread (difference between best ask and best bid)
    pub fn get_spread(&self) -> Option<Decimal> {
        match (self.get_best_ask(), self.get_best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price (average of best bid and best ask)
    pub fn get_mid_price(&self) -> Option<Decimal> {
        match (self.get_best_ask(), self.get_best_bid()) {
            (Some(ask), Some(bid)) => Some((ask + bid) / Decimal::from(2)),
            _ => None,
        }
    }

    /// Get spread in basis points
    pub fn get_spread_bps(&self) -> Option<Decimal> {
        match (self.get_spread(), self.get_mid_price()) {
            (Some(spread), Some(mid)) if mid > Decimal::ZERO => {
                Some(spread / mid * Decimal::from(10000))
            }
            _ => None,
        }
    }

    /// Get an order by ID
    pub fn get_order(&self, order_id: Uuid) -> Option<&Order> {
        self.orders.get(&order_id)
    }

    /// Get a mutable reference to an order by ID
    pub fn get_order_mut(&mut self, order_id: Uuid) -> Option<&mut Order> {
        self.orders.get_mut(&order_id)
    }

    /// Add a trade to the history
    pub fn add_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
    }

    /// Get recent trades (last n trades)
    pub fn get_recent_trades(&self, limit: usize) -> Vec<Trade> {
        let start = if self.trades.len() > limit {
            self.trades.len() - limit
        } else {
            0
        };
        self.trades[start..].to_vec()
    }

    /// Get total depth on bid side
    pub fn get_bid_depth(&self) -> Decimal {
        Self::calculate_depth(&self.bids)
    }

    /// Get total depth on ask side
    pub fn get_ask_depth(&self) -> Decimal {
        Self::calculate_depth(&self.asks)
    }

    /// Calculate total depth for a given side of the book
    fn calculate_depth(levels: &BTreeMap<Decimal, PriceLevel>) -> Decimal {
        levels.values().map(|level| level.total_quantity).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_level() {
        let mut level = PriceLevel::new(dec!(100.00));
        let order_id = Uuid::new_v4();

        level.add_order(order_id, dec!(50));
        assert_eq!(level.total_quantity, dec!(50));
        assert_eq!(level.orders.len(), 1);

        level.remove_order(order_id, dec!(50));
        assert_eq!(level.total_quantity, Decimal::ZERO);
        assert!(level.is_empty());
    }

    #[test]
    fn test_orderbook_spread() {
        let mut book = OrderBook::new("AAPL".to_string());

        // Add some price levels manually for testing
        let mut bid_level = PriceLevel::new(dec!(100.00));
        bid_level.add_order(Uuid::new_v4(), dec!(10));
        book.bids.insert(dec!(100.00), bid_level);

        let mut ask_level = PriceLevel::new(dec!(100.50));
        ask_level.add_order(Uuid::new_v4(), dec!(10));
        book.asks.insert(dec!(100.50), ask_level);

        assert_eq!(book.get_best_bid(), Some(dec!(100.00)));
        assert_eq!(book.get_best_ask(), Some(dec!(100.50)));
        assert_eq!(book.get_spread(), Some(dec!(0.50)));
        assert_eq!(book.get_mid_price(), Some(dec!(100.25)));
    }
}
