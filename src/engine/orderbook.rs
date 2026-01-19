//! Order Book Engine
//!
//! This module provides the main `OrderBookEngine` struct which manages
//! multiple order books (one per trading symbol) and handles order
//! submission, cancellation, and matching.

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::models::{Order, OrderBook, OrderSide, OrderStatus, OrderType, PriceLevel, Trade, StopOrder};

use super::errors::OrderBookError;
use super::matching::match_order;
use super::trigger::TriggerEngine;
use super::validation::validate_order;

// ============================================================================
// Order Book Helper Functions
// ============================================================================

/// Remove an order from its price level in the order book
fn remove_order_from_price_level(
    book: &mut OrderBook,
    order_id: Uuid,
    price: Decimal,
    side: &OrderSide,
    remaining_qty: Decimal,
) {
    let levels = match side {
        OrderSide::Buy => &mut book.bids,
        OrderSide::Sell => &mut book.asks,
    };

    if let Some(level) = levels.get_mut(&price) {
        level.remove_order(order_id, remaining_qty);
        if level.is_empty() {
            levels.remove(&price);
        }
    }
}

/// Add an order to its price level in the order book
fn add_order_to_price_level(
    book: &mut OrderBook,
    order_id: Uuid,
    price: Decimal,
    side: &OrderSide,
    quantity: Decimal,
) {
    let levels = match side {
        OrderSide::Buy => &mut book.bids,
        OrderSide::Sell => &mut book.asks,
    };

    let level = levels.entry(price).or_insert_with(|| PriceLevel::new(price));
    level.add_order(order_id, quantity);
}

/// Thread-safe order book engine
pub struct OrderBookEngine {
    books: Arc<RwLock<HashMap<String, OrderBook>>>,
    trigger_engine: Arc<RwLock<TriggerEngine>>,
}

impl OrderBookEngine {
    /// Create a new order book engine
    pub fn new() -> Self {
        Self {
            books: Arc::new(RwLock::new(HashMap::new())),
            trigger_engine: Arc::new(RwLock::new(TriggerEngine::new())),
        }
    }

    /// Get or create an order book for a symbol
    fn  get_or_create_book(&self, symbol: &str) -> OrderBook {
        let mut books = self.books.write().unwrap();
        books
            .entry(symbol.to_string())
            .or_insert_with(|| OrderBook::new(symbol.to_string()))
            .clone()
    }

    /// Update an order book
    fn update_book(&self, book: OrderBook) {
        let mut books = self.books.write().unwrap();
        books.insert(book.symbol.clone(), book);
    }

    /// Add an order to the order book and attempt to match it
    pub fn add_order(&self, mut order: Order) -> Result<(Order, Vec<Trade>), OrderBookError> {
        // Validate order using centralized validation

        /*
          What ? Does:

          validate_order(&order)?;

          Is equivalent to:

          match validate_order(&order) {
              Ok(value) => value,      // Continue execution (value is () here)z
              Err(e) => return Err(e), // Return error immediately
        }
        */
        validate_order(&order)?;

        let mut book = self.get_or_create_book(&order.symbol);

        // Check for duplicate order
        if book.orders.contains_key(&order.id) {
            return Err(OrderBookError::DuplicateOrder(order.id));
        }

        // Attempt to match the order
        let (trades, cancelled_order_ids) = match_order(&mut book, &mut order)?;

        // Cancelled orders is STP cancellation
        // Remove cancelled orders from the book (STP cancellations)
        // First, collect the data we need to avoid borrow checker issues
        let cancellation_data: Vec<_> = cancelled_order_ids
            .iter()
            // |&id| pattern destructures the reference, giving the actual value instead of the reference to it
            // Different from *, &id equals to match a reference and give me the results, this is just pattern matching
            // let &value = r; value is i32 (5)
            // let value = *r value is i32 (5)
            .filter_map(|&id| {
                book.orders.get(&id).map(|order| {
                    (id, order.price, order.side.clone(), order.remaining_quantity())
                })
            })
            .collect();

        // Now perform the removals
        for (cancelled_id, price_opt, side, remaining_qty) in cancellation_data {
            if let Some(price) = price_opt {
                remove_order_from_price_level(&mut book, cancelled_id, price, &side, remaining_qty);
            }
            book.orders.remove(&cancelled_id);
        }

        // We use &trades because we don't want to MOVE the trades into the for loop
        // Add trades to book history
        for trade in &trades {
            book.add_trade(trade.clone());
        }

        // Add order to book if it should rest (based on TIF and fill status)
        if order.should_rest_in_book() && order.order_type == OrderType::Limit {
            let price = order.price.expect("Limit order must have price");
            add_order_to_price_level(&mut book, order.id, price, &order.side, order.remaining_quantity());
            book.orders.insert(order.id, order.clone());
        }

        // Update the book
        self.update_book(book);

        // Check for triggered stop orders if any trades occurred
        if !trades.is_empty() {
            let last_trade_price = trades.last().unwrap().price;
            let triggered_orders = {
                let mut trigger_engine = self.trigger_engine.write().unwrap();
                trigger_engine.on_trade(last_trade_price)
            };

            // Recursively submit triggered orders
            for triggered_order in triggered_orders {
                // Submit triggered order (ignore errors to prevent cascading failures)
                let _ = self.add_order(triggered_order);
            }
        }

        Ok((order, trades))
    }

    /// Cancel an order
    pub fn cancel_order(
        &self,
        symbol: &str,
        order_id: Uuid,
    ) -> Result<Order, OrderBookError> {
        let mut book = self.get_or_create_book(symbol);

        // Get the order
        let mut order = book
            .orders
            .remove(&order_id)
            .ok_or(OrderBookError::OrderNotFound(order_id))?;

        // Check if order can be cancelled
        if order.status == OrderStatus::Filled || order.status == OrderStatus::Cancelled {
            return Err(OrderBookError::OrderNotActive(order_id));
        }

        // Remove from price level using helper function
        if let Some(price) = order.price {
            remove_order_from_price_level(
                &mut book,
                order_id,
                price,
                &order.side,
                order.remaining_quantity(),
            );
        }

        // Update order status
        order.status = OrderStatus::Cancelled;

        // Update the book
        self.update_book(book);

        Ok(order)
    }

    /// Get order status
    pub fn get_order(&self, symbol: &str, order_id: Uuid) -> Result<Order, OrderBookError> {
        let book = self.get_or_create_book(symbol);
        book.orders
            .get(&order_id)
            .cloned()
            .ok_or(OrderBookError::OrderNotFound(order_id))
    }

    /// Get the order book for a symbol
    pub fn get_order_book(&self, symbol: &str) -> OrderBook {
        self.get_or_create_book(symbol)
    }

    /// Get recent trades for a symbol
    pub fn get_recent_trades(&self, symbol: &str, limit: usize) -> Vec<Trade> {
        let book = self.get_or_create_book(symbol);
        book.get_recent_trades(limit)
    }

    /// Get all active symbols
    pub fn get_symbols(&self) -> Vec<String> {
        let books = self.books.read().unwrap();
        books.keys().cloned().collect()
    }

    /// Get total number of active orders across all symbols
    pub fn get_total_active_orders(&self) -> usize {
        let books = self.books.read().unwrap();
        books.values().map(|book| book.orders.len()).sum()
    }

    /// Get total number of trades across all symbols
    pub fn get_total_trades(&self) -> usize {
        let books = self.books.read().unwrap();
        books.values().map(|book| book.trades.len()).sum()
    }

    /// Get total volume across all symbols
    pub fn get_total_volume(&self) -> Decimal {
        let books = self.books.read().unwrap();
        books
            .values()
            .flat_map(|book| &book.trades)
            .map(|trade| trade.value())
            .sum()
    }

    /// Get total fees collected across all symbols
    pub fn get_total_fees(&self) -> Decimal {
        let books = self.books.read().unwrap();
        books
            .values()
            // Flatten the trades from all books into one single iterator
            .flat_map(|book| &book.trades)
            .map(|trade| trade.total_fees())
            .sum()
    }

    // ============================================================================
    // Stop Order Management
    // ============================================================================

    /// Add a stop order
    pub fn add_stop_order(&self, stop: StopOrder) -> Result<(), OrderBookError> {
        let mut trigger_engine = self.trigger_engine.write().unwrap();
        trigger_engine.add_stop_order(stop);
        Ok(())
    }

    /// Cancel a stop order
    pub fn cancel_stop_order(&self, order_id: Uuid) -> Result<StopOrder, OrderBookError> {
        let mut trigger_engine = self.trigger_engine.write().unwrap();
        trigger_engine
            .cancel_stop_order(order_id)
            .ok_or(OrderBookError::OrderNotFound(order_id))
    }

    /// Get a stop order by ID
    pub fn get_stop_order(&self, order_id: Uuid) -> Option<StopOrder> {
        let trigger_engine = self.trigger_engine.read().unwrap();
        trigger_engine.get_stop_order(order_id).cloned()
    }

    /// Get all stop orders for a symbol
    pub fn get_stop_orders_by_symbol(&self, symbol: &str) -> Vec<StopOrder> {
        let trigger_engine = self.trigger_engine.read().unwrap();
        trigger_engine
            .get_stop_orders_by_symbol(symbol)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get total number of active stop orders
    pub fn get_total_stop_orders(&self) -> usize {
        let trigger_engine = self.trigger_engine.read().unwrap();
        trigger_engine.get_total_stop_orders()
    }
}

impl Default for OrderBookEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_add_and_match_order() {
        let engine = OrderBookEngine::new();

        // Add a sell order
        let sell_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(100),
            "seller1".to_string(),
        );

        let (order, trades) = engine.add_order(sell_order).unwrap();
        assert_eq!(trades.len(), 0);
        assert_eq!(order.status, OrderStatus::New);

        // Add a matching buy order
        let buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(50),
            "buyer1".to_string(),
        );

        let (order, trades) = engine.add_order(buy_order).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(50));
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_cancel_order() {
        let engine = OrderBookEngine::new();

        // Add an order
        let order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(100),
            "user1".to_string(),
        );

        let order_id = order.id;
        let (_, trades) = engine.add_order(order).unwrap();
        assert_eq!(trades.len(), 0);

        // Cancel the order
        let cancelled = engine.cancel_order("AAPL", order_id).unwrap();
        assert_eq!(cancelled.status, OrderStatus::Cancelled);

        // Try to get cancelled order
        let result = engine.get_order("AAPL", order_id);
        assert!(result.is_err());
    }
}
