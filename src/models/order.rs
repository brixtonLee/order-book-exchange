use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::iceberg::IcebergConfig;

/// Represents a trading order in the order book
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Order {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    /// Time-in-force specifies order lifecycle behavior
    #[serde(default)]
    pub time_in_force: TimeInForce,
    /// Self-trade prevention mode
    #[serde(default)]
    pub stp_mode: SelfTradePreventionMode,
    /// Post-only ensures order only adds liquidity (maker-only)
    #[serde(default)]
    pub post_only: bool,
    /// Expiration time for GTD orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
    /// Iceberg configuration (None for regular orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg: Option<IcebergConfig>,
}

/// Order side: Buy or Sell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type: Limit or Market
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
}

/// Order status throughout its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

/// Time-in-force options for order lifecycle management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    /// Good-Till-Cancelled: remains active until filled or cancelled
    #[default]
    GTC,
    /// Immediate-Or-Cancel: execute immediately, cancel unfilled portion
    IOC,
    /// Fill-Or-Kill: execute completely or cancel entirely
    FOK,
    /// Good-Till-Date: expires at specified time
    GTD,
    /// Day order: expires at end of trading day
    DAY,
}

/// Self-trade prevention mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelfTradePreventionMode {
    /// No self-trade prevention (skip matching)
    #[default]
    None,
    /// Cancel the resting order in the book
    CancelResting,
    /// Cancel the incoming order
    CancelIncoming,
    /// Cancel both orders
    CancelBoth,
    /// Cancel the order with smaller quantity
    CancelSmallest,
    /// Decrement both orders by matched quantity
    DecrementBoth,
}

impl Order {
    /// Create a new order with default options (GTC, no STP, not post-only)
    pub fn new(
        symbol: String,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Decimal>,
        quantity: Decimal,
        user_id: String,
    ) -> Self {
        Self::new_with_options(
            symbol,
            side,
            order_type,
            price,
            quantity,
            user_id,
            TimeInForce::default(),
            SelfTradePreventionMode::default(),
            false,
            None,
        )
    }

    /// Create a new order with all options
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_options(
        symbol: String,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Decimal>,
        quantity: Decimal,
        user_id: String,
        time_in_force: TimeInForce,
        stp_mode: SelfTradePreventionMode,
        post_only: bool,
        expire_time: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id,
            timestamp: Utc::now(),
            time_in_force,
            stp_mode,
            post_only,
            expire_time,
            iceberg: None,
        }
    }

    /// Get the remaining unfilled quantity
    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }

    /// Get the quantity visible in the order book
    pub fn visible_quantity(&self) -> Decimal {
        match &self.iceberg {
            Some(config) => config.visible_quantity(),
            None => self.remaining_quantity(),
        }
    }

    /// Process a fill, handling iceberg replenishment
    /// Returns true if the order was replenished (timestamp should be updated)
    pub fn apply_fill(&mut self, fill_qty: Decimal) -> bool {
        self.filled_quantity += fill_qty;

        if let Some(ref mut iceberg) = self.iceberg {
            let result = iceberg.process_fill(fill_qty);

            if result.replenished {
                // IMPORTANT: Update timestamp - order loses time priority!
                self.timestamp = Utc::now();
                self.update_status();
                return true; // Signal that order was modified
            }
        }

        self.update_status();
        false
    }

    /// Check if the order is fully filled
    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }

    /// Update order status based on filled quantity
    pub fn update_status(&mut self) {
        if self.is_filled() {
            self.status = OrderStatus::Filled;
        } else if self.filled_quantity > Decimal::ZERO {
            self.status = OrderStatus::PartiallyFilled;
        }
    }

    /// Fill the order with a given quantity
    pub fn fill(&mut self, quantity: Decimal) {
        self.filled_quantity += quantity;
        self.update_status();
    }

    /// Check if order has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expire_time) = self.expire_time {
            Utc::now() > expire_time
        } else {
            false
        }
    }

    /// Check if order should be added to the book (based on TIF)
    pub fn should_rest_in_book(&self) -> bool {
        match self.time_in_force {
            TimeInForce::GTC | TimeInForce::GTD | TimeInForce::DAY => !self.is_filled(),
            TimeInForce::IOC | TimeInForce::FOK => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_creation() {
        let order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.50)),
            dec!(100),
            "user123".to_string(),
        );

        assert_eq!(order.symbol, "AAPL");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.quantity, dec!(100));
        assert_eq!(order.filled_quantity, Decimal::ZERO);
        assert_eq!(order.status, OrderStatus::New);
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new(
            "AAPL".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(dec!(150.50)),
            dec!(100),
            "user123".to_string(),
        );

        order.fill(dec!(50));
        assert_eq!(order.filled_quantity, dec!(50));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining_quantity(), dec!(50));

        order.fill(dec!(50));
        assert_eq!(order.filled_quantity, dec!(100));
        assert_eq!(order.status, OrderStatus::Filled);
        assert!(order.is_filled());
    }
}
