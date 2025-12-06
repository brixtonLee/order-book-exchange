use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;
use crate::models::{
    Order, OrderBook, OrderSide, OrderStatus, SelfTradePreventionMode, TimeInForce, Trade,
};

// Use super:: to access parents then access siblings
use super::fees::{calculate_maker_fee, calculate_taker_fee};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during order matching
#[derive(Debug, Error)]
pub enum MatchingError {
    #[error("Insufficient liquidity to fill order")]
    InsufficientLiquidity,

    #[error("Invalid order price")]
    InvalidPrice,

    #[error("Invalid order quantity")]
    InvalidQuantity,

    #[error("Self-trade detected")]
    SelfTrade,

    #[error("Post-only order would match immediately")]
    PostOnlyWouldMatch,

    #[error("Fill-or-kill order cannot be completely filled")]
    FillOrKillRejected,

    #[error("Order not found: {0}")]
    OrderNotFound(Uuid),
}

// ============================================================================
// Self-Trade Prevention
// ============================================================================

/// Result of self-trade prevention check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfTradeAction {
    Allow,
    CancelResting,
    CancelIncoming,
    CancelBoth,
    Skip,
    DecrementBoth,
}

/// Determine what action to take for self-trade prevention
fn check_self_trade(incoming: &Order, resting: &Order) -> SelfTradeAction {
    if incoming.user_id != resting.user_id {
        return SelfTradeAction::Allow;
    }

    match incoming.stp_mode {
        SelfTradePreventionMode::None => SelfTradeAction::Skip,
        SelfTradePreventionMode::CancelResting => SelfTradeAction::CancelResting,
        SelfTradePreventionMode::CancelIncoming => SelfTradeAction::CancelIncoming,
        SelfTradePreventionMode::CancelBoth => SelfTradeAction::CancelBoth,
        SelfTradePreventionMode::CancelSmallest => {
            if incoming.remaining_quantity() < resting.remaining_quantity() {
                SelfTradeAction::CancelIncoming
            } else {
                SelfTradeAction::CancelResting
            }
        }
        SelfTradePreventionMode::DecrementBoth => SelfTradeAction::DecrementBoth,
    }
}

// ============================================================================
// Shared Helper: Create Trade
// ============================================================================

/// Create a trade between two orders and update their fill quantities
fn create_trade(
    symbol: &str,
    price: Decimal,
    quantity: Decimal,
    buyer_order: &mut Order,
    seller_order: &mut Order,
) -> Trade {
    let trade_value = price * quantity;
    let maker_fee = calculate_maker_fee(trade_value);
    let taker_fee = calculate_taker_fee(trade_value);

    let trade = Trade::new(
        symbol.to_string(),
        price,
        quantity,
        buyer_order.id,
        seller_order.id,
        buyer_order.user_id.clone(),
        seller_order.user_id.clone(),
        maker_fee,
        taker_fee,
    );

    buyer_order.fill(quantity);
    seller_order.fill(quantity);

    trade
}

// ============================================================================
// Shared Helper: Process Single Resting Order
// ============================================================================

/// Result of processing a resting order
enum ProcessResult {
    /// A trade was executed
    Trade(Trade),
    /// Skip this order (STP or decrement)
    Skip,
    /// Cancel incoming order and stop matching
    CancelIncoming,
}

/// Process a single resting order against the incoming order
/// Handles STP checks and trade execution
fn process_resting_order(
    symbol: &str,
    price: Decimal,
    incoming_order: &mut Order,
    resting_order: &mut Order,
    resting_order_id: Uuid,
    orders_to_remove: &mut Vec<Uuid>,
    cancelled_orders: &mut Vec<Uuid>,
) -> ProcessResult {
    // Check self-trade prevention
    let stp_action = check_self_trade(incoming_order, resting_order);

    match stp_action {
        SelfTradeAction::Allow => {
            // Execute the trade
            let quantity = incoming_order
                .remaining_quantity()
                .min(resting_order.remaining_quantity());

            // Create trade based on order sides
            let trade = match incoming_order.side {
                OrderSide::Buy => {
                    create_trade(symbol, price, quantity, incoming_order, resting_order)
                }
                OrderSide::Sell => {
                    create_trade(symbol, price, quantity, resting_order, incoming_order)
                }
            };

            if resting_order.is_filled() {
                orders_to_remove.push(resting_order_id);
            }

            ProcessResult::Trade(trade)
        }

        SelfTradeAction::Skip => ProcessResult::Skip,

        SelfTradeAction::DecrementBoth => {
            let qty = incoming_order
                .remaining_quantity()
                .min(resting_order.remaining_quantity());
            incoming_order.fill(qty);
            resting_order.fill(qty);

            if resting_order.is_filled() {
                orders_to_remove.push(resting_order_id);
            }
            ProcessResult::Skip
        }

        SelfTradeAction::CancelResting => {
            resting_order.status = OrderStatus::Cancelled;
            orders_to_remove.push(resting_order_id);
            cancelled_orders.push(resting_order_id);
            ProcessResult::Skip
        }

        SelfTradeAction::CancelIncoming => {
            incoming_order.status = OrderStatus::Cancelled;
            cancelled_orders.push(incoming_order.id);
            ProcessResult::CancelIncoming
        }

        SelfTradeAction::CancelBoth => {
            incoming_order.status = OrderStatus::Cancelled;
            resting_order.status = OrderStatus::Cancelled;
            orders_to_remove.push(resting_order_id);
            cancelled_orders.push(incoming_order.id);
            cancelled_orders.push(resting_order_id);
            ProcessResult::CancelIncoming
        }
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Match an incoming order against the order book
/// Returns a vector of trades that were executed and IDs of cancelled orders
pub fn match_order(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
) -> Result<(Vec<Trade>, Vec<Uuid>), MatchingError> {
    // Validate order quantity
    if incoming_order.quantity <= Decimal::ZERO {
        return Err(MatchingError::InvalidQuantity);
    }

    // Check if order has expired
    if incoming_order.is_expired() {
        incoming_order.status = OrderStatus::Expired;
        return Ok((Vec::new(), Vec::new()));
    }

    // Post-only validation
    if incoming_order.post_only && would_match_immediately(orderbook, incoming_order) {
        incoming_order.status = OrderStatus::Rejected;
        return Err(MatchingError::PostOnlyWouldMatch);
    }

    // Match based on order side
    let (trades, cancelled_orders) = match incoming_order.side {
        OrderSide::Buy => match_buy_order(orderbook, incoming_order)?,
        OrderSide::Sell => match_sell_order(orderbook, incoming_order)?,
    };

    // Handle Fill-Or-Kill
    if incoming_order.time_in_force == TimeInForce::FOK && !incoming_order.is_filled() {
        incoming_order.status = OrderStatus::Rejected;
        return Err(MatchingError::FillOrKillRejected);
    }

    Ok((trades, cancelled_orders))
}

/// Check if an order would match immediately (for post-only validation)
fn would_match_immediately(orderbook: &OrderBook, order: &Order) -> bool {
    match order.side {
        OrderSide::Buy => {
            orderbook
                .get_best_ask()
                .map_or(false, |best_ask| order.price.map_or(true, |p| p >= best_ask))
        }
        OrderSide::Sell => {
            orderbook
                .get_best_bid()
                .map_or(false, |best_bid| order.price.map_or(true, |p| p <= best_bid))
        }
    }
}

// ============================================================================
// Match Buy Order (against asks)
// ============================================================================

/// Match a buy order against the ask side of the order book
fn match_buy_order(
    orderbook: &mut OrderBook,
    buy_order: &mut Order,
) -> Result<(Vec<Trade>, Vec<Uuid>), MatchingError> {
    let mut trades = Vec::new();
    let mut cancelled_orders = Vec::new();
    let mut empty_price_levels = Vec::new();

    let price_limit = buy_order.price.unwrap_or(Decimal::MAX);
    let ask_prices: Vec<Decimal> =
        // From 0 until price_limit (Inclusive because of =)
        orderbook.asks.range(..=price_limit)
            .map(|(price, _)| *price)
            .collect();

    for ask_price in ask_prices {
        if buy_order.is_filled() {
            break;
        }

        let should_stop = match_at_price_level(
            orderbook, buy_order,
            ask_price,
            &mut trades,
            &mut cancelled_orders,
            &mut empty_price_levels,
        )?;

        if should_stop {
            break;
        }
    }

    for price in empty_price_levels {
        orderbook.asks.remove(&price);
    }

    Ok((trades, cancelled_orders))
}

// Match at a single ask price level (for buy orders)
fn match_at_price_level(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
    price: Decimal,
    trades: &mut Vec<Trade>,
    cancelled_orders: &mut Vec<Uuid>,
    empty_price_levels: &mut Vec<Decimal>,
) -> Result<bool, MatchingError> {
    // Get order IDs at this price level
    let order_ids = orderbook.asks.get(&price)
        .map(|level| level.orders.iter().copied().collect())
        .unwrap_or_default();

    let mut orders_to_remove = Vec::new();

    // Process each resting order
    let should_stop = process_resting_orders(
        orderbook,
        incoming_order,
        price,
        order_ids,
        &mut orders_to_remove,
        trades,
        cancelled_orders,
    )?;

    // Cleanup price level
    cleanup_and_mark_empty(
        orderbook,
        price,
        orders_to_remove,
        empty_price_levels,
    );

    Ok(should_stop)
}

// Process all resting orders at price level
fn process_resting_orders(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
    price: Decimal,
    order_ids: Vec<Uuid>,
    orders_to_remove: &mut Vec<Uuid>,
    trades: &mut Vec<Trade>,
    cancelled_orders: &mut Vec<Uuid>,
) -> Result<bool, MatchingError> {
    for resting_order_id in order_ids {
        if incoming_order.is_filled() {
            return Ok(false);
        }

        let should_stop = process_single_resting_order(
            orderbook,
            incoming_order,
            price,
            resting_order_id,
            orders_to_remove,
            trades,
            cancelled_orders,
        )?;

        if should_stop {
            return Ok(true);
        }
    }

    Ok(false)
}

// Process a single resting order (works for both buy and sell sides)
fn process_single_resting_order(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
    price: Decimal,
    resting_order_id: Uuid,
    orders_to_remove: &mut Vec<Uuid>,
    trades: &mut Vec<Trade>,
    cancelled_orders: &mut Vec<Uuid>,
) -> Result<bool, MatchingError> {
    let resting_order = orderbook
        .orders
        .get_mut(&resting_order_id)
        .ok_or(MatchingError::OrderNotFound(resting_order_id))?;

    match process_resting_order(
        &orderbook.symbol,
        price,
        incoming_order,
        resting_order,
        resting_order_id,
        orders_to_remove,
        cancelled_orders,
    ) {
        ProcessResult::Trade(trade) => {
            trades.push(trade);
            Ok(false)
        }
        ProcessResult::Skip => Ok(false),
        ProcessResult::CancelIncoming => Ok(true),
    }
}

// Extract 6: Cleanup and mark empty levels
fn cleanup_and_mark_empty(
    orderbook: &mut OrderBook,
    price: Decimal,
    orders_to_remove: Vec<Uuid>,
    empty_price_levels: &mut Vec<Decimal>,
) {
    if let Some(price_level) = orderbook.asks.get_mut(&price) {
        // Remove orders from price level
        price_level.orders.retain(|id| !orders_to_remove.contains(id));

        // Update quantity and remove from main HashMap
        for order_id in orders_to_remove {
            if let Some(order) = orderbook.orders.remove(&order_id) {
                price_level.total_quantity -= order.quantity;
            }
        }

        if price_level.is_empty() {
            empty_price_levels.push(price);
        }
    }
}

// ============================================================================
// Match Sell Order (against bids)
// ============================================================================

/// Match a sell order against the bid side of the order book
fn match_sell_order(
    orderbook: &mut OrderBook,
    sell_order: &mut Order,
) -> Result<(Vec<Trade>, Vec<Uuid>), MatchingError> {
    let mut trades = Vec::new();
    let mut cancelled_orders = Vec::new();
    let mut empty_price_levels = Vec::new();

    let price_limit = sell_order.price.unwrap_or(Decimal::ZERO);
    let bid_prices: Vec<Decimal> =
        // From price_limit to max (descending - highest first)
        orderbook.bids.range(price_limit..)
            .rev()
            .map(|(price, _)| *price)
            .collect();

    for bid_price in bid_prices {
        if sell_order.is_filled() {
            break;
        }

        let should_stop = match_at_price_level_sell(
            orderbook,
            sell_order,
            bid_price,
            &mut trades,
            &mut cancelled_orders,
            &mut empty_price_levels,
        )?;

        if should_stop {
            break;
        }
    }

    for price in empty_price_levels {
        orderbook.bids.remove(&price);
    }

    Ok((trades, cancelled_orders))
}

// Match at a single bid price level (for sell orders)
fn match_at_price_level_sell(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
    price: Decimal,
    trades: &mut Vec<Trade>,
    cancelled_orders: &mut Vec<Uuid>,
    empty_price_levels: &mut Vec<Decimal>,
) -> Result<bool, MatchingError> {
    // Get order IDs at this price level
    let order_ids = orderbook.bids.get(&price)
        .map(|level| level.orders.iter().copied().collect())
        .unwrap_or_default();

    let mut orders_to_remove = Vec::new();

    // Process each resting order
    let should_stop = process_resting_orders_sell(
        orderbook,
        incoming_order,
        price,
        order_ids,
        &mut orders_to_remove,
        trades,
        cancelled_orders,
    )?;

    // Cleanup price level
    cleanup_and_mark_empty_sell(
        orderbook,
        price,
        orders_to_remove,
        empty_price_levels,
    );

    Ok(should_stop)
}

// Process all resting buy orders at a bid price level (for sell orders)
fn process_resting_orders_sell(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
    price: Decimal,
    order_ids: Vec<Uuid>,
    orders_to_remove: &mut Vec<Uuid>,
    trades: &mut Vec<Trade>,
    cancelled_orders: &mut Vec<Uuid>,
) -> Result<bool, MatchingError> {
    for resting_order_id in order_ids {
        if incoming_order.is_filled() {
            return Ok(false);
        }

        let should_stop = process_single_resting_order(
            orderbook,
            incoming_order,
            price,
            resting_order_id,
            orders_to_remove,
            trades,
            cancelled_orders,
        )?;

        if should_stop {
            return Ok(true);
        }
    }

    Ok(false)
}

// Cleanup and mark empty bid levels (for sell orders)
fn cleanup_and_mark_empty_sell(
    orderbook: &mut OrderBook,
    price: Decimal,
    orders_to_remove: Vec<Uuid>,
    empty_price_levels: &mut Vec<Decimal>,
) {
    if let Some(price_level) = orderbook.bids.get_mut(&price) {
        // Remove orders from price level
        price_level.orders.retain(|id| !orders_to_remove.contains(id));

        // Update quantity and remove from main HashMap
        for order_id in orders_to_remove {
            if let Some(order) = orderbook.orders.remove(&order_id) {
                price_level.total_quantity -= order.quantity;
            }
        }

        if price_level.is_empty() {
            empty_price_levels.push(price);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Order, OrderType, PriceLevel};
    use rust_decimal_macros::dec;

    /// Setup an orderbook with a single resting order at the given price and quantity
    fn setup_orderbook_with_order(
        side: OrderSide,
        price: Decimal,
        quantity: Decimal,
    ) -> (OrderBook, Uuid) {
        let mut orderbook = OrderBook::new("AAPL".to_string());
        let user_id = match side {
            OrderSide::Buy => "buyer1",
            OrderSide::Sell => "seller1",
        };

        let order = Order::new(
            "AAPL".to_string(),
            side,
            OrderType::Limit,
            Some(price),
            quantity,
            user_id.to_string(),
        );
        let order_id = order.id;

        orderbook.orders.insert(order.id, order.clone());
        let mut level = PriceLevel::new(price);
        level.add_order(order.id, quantity);

        match side {
            OrderSide::Buy => orderbook.bids.insert(price, level),
            OrderSide::Sell => orderbook.asks.insert(price, level),
        };

        (orderbook, order_id)
    }

    /// Convenience wrapper for setting up an orderbook with an ask (sell) order
    fn setup_orderbook_with_ask(price: Decimal, quantity: Decimal) -> (OrderBook, Uuid) {
        setup_orderbook_with_order(OrderSide::Sell, price, quantity)
    }

    /// Convenience wrapper for setting up an orderbook with a bid (buy) order
    fn setup_orderbook_with_bid(price: Decimal, quantity: Decimal) -> (OrderBook, Uuid) {
        setup_orderbook_with_order(OrderSide::Buy, price, quantity)
    }

    #[test]
    fn test_simple_match() {
        let (mut orderbook, _) = setup_orderbook_with_ask(dec!(150.00), dec!(100));

        let mut buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(50),
            "buyer1".to_string(),
        );

        let (trades, cancelled_orders) = match_order(&mut orderbook, &mut buy_order).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(50));
        assert_eq!(trades[0].price, dec!(150.00));
        assert!(buy_order.is_filled());
        assert_eq!(cancelled_orders.len(), 0);
    }

    #[test]
    fn test_full_fill_removes_from_orders_map() {
        let (mut orderbook, resting_order_id) = setup_orderbook_with_ask(dec!(150.00), dec!(100));

        let mut buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(100),
            "buyer1".to_string(),
        );

        let (trades, _) = match_order(&mut orderbook, &mut buy_order).unwrap();

        assert_eq!(trades.len(), 1);
        assert!(orderbook.orders.get(&resting_order_id).is_none());
        assert!(orderbook.asks.is_empty());
    }

    #[test]
    fn test_partial_fill_keeps_order_in_map() {
        let (mut orderbook, resting_order_id) = setup_orderbook_with_ask(dec!(150.00), dec!(100));

        let mut buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(30),
            "buyer1".to_string(),
        );

        let (trades, _) = match_order(&mut orderbook, &mut buy_order).unwrap();

        assert_eq!(trades.len(), 1);
        let remaining = orderbook.orders.get(&resting_order_id).unwrap();
        assert_eq!(remaining.remaining_quantity(), dec!(70));
        assert!(!orderbook.asks.is_empty());
    }

    #[test]
    fn test_sell_order_matching() {
        let (mut orderbook, _) = setup_orderbook_with_bid(dec!(150.00), dec!(100));

        let mut sell_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(50),
            "seller1".to_string(),
        );

        let (trades, _) = match_order(&mut orderbook, &mut sell_order).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(50));
        assert!(sell_order.is_filled());
    }

    #[test]
    fn test_price_improvement() {
        let (mut orderbook, _) = setup_orderbook_with_ask(dec!(148.00), dec!(100));

        let mut buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.00)),
            dec!(50),
            "buyer1".to_string(),
        );

        let (trades, _) = match_order(&mut orderbook, &mut buy_order).unwrap();

        assert_eq!(trades[0].price, dec!(148.00));
    }

    #[test]
    fn test_no_match_when_prices_dont_cross() {
        let (mut orderbook, _) = setup_orderbook_with_ask(dec!(150.00), dec!(100));

        let mut buy_order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(140.00)),
            dec!(50),
            "buyer1".to_string(),
        );

        let (trades, _) = match_order(&mut orderbook, &mut buy_order).unwrap();

        assert!(trades.is_empty());
        assert!(!buy_order.is_filled());
    }
}
