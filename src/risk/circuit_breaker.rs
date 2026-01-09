use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::VecDeque;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::Order;

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CircuitBreakerConfig {
    /// Maximum price change (%) before halt
    pub max_price_change_pct: Decimal,

    /// Time window for price change calculation (minutes)
    pub price_window_minutes: i64,

    /// Minimum trades before circuit breaker activates
    pub min_trades_for_activation: u32,

    /// How long to halt trading (minutes)
    pub halt_duration_minutes: i64,

    /// Maximum order size (quantity)
    pub max_order_size: Decimal,

    /// Maximum order value (price × quantity)
    pub max_order_value: Decimal,

    /// Maximum orders per second per user
    pub max_orders_per_second: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_price_change_pct: dec!(10),     // 10% price move
            price_window_minutes: 5,
            min_trades_for_activation: 10,
            halt_duration_minutes: 5,
            max_order_size: dec!(1_000_000),
            max_order_value: dec!(10_000_000),
            max_orders_per_second: 100,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CircuitState {
    /// Normal trading
    Normal,
    /// Trading halted
    Halted,
    /// Cooling off (limited trading)
    CoolingOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HaltReason {
    PriceVolatility,
    VolumeSpike,
    TechnicalIssue,
    Manual,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    halt_reason: Option<HaltReason>,
    halt_until: Option<DateTime<Utc>>,

    /// Recent prices for volatility calculation
    price_history: VecDeque<(DateTime<Utc>, Decimal)>,

    /// Reference price (usually opening or last clear price)
    reference_price: Option<Decimal>,

    /// Trade count in current window
    trade_count: u32,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Normal,
            halt_reason: None,
            halt_until: None,
            price_history: VecDeque::new(),
            reference_price: None,
            trade_count: 0,
        }
    }

    /// Check if trading is allowed
    pub fn is_trading_allowed(&mut self) -> bool {
        // Check if halt has expired
        if let Some(until) = self.halt_until {
            if Utc::now() >= until {
                self.resume();
            }
        }

        match self.state {
            CircuitState::Normal => true,
            CircuitState::Halted => false,
            CircuitState::CoolingOff => true, // Limited trading allowed
        }
    }

    /// Validate an order against risk limits
    pub fn validate_order(&self, order: &Order) -> Result<(), RiskError> {
        // Check circuit state
        if self.state == CircuitState::Halted {
            return Err(RiskError::TradingHalted {
                reason: self.halt_reason.unwrap_or(HaltReason::TechnicalIssue),
                resume_at: self.halt_until.unwrap_or_else(Utc::now),
            });
        }

        // Check order size
        if order.quantity > self.config.max_order_size {
            return Err(RiskError::OrderTooLarge {
                quantity: order.quantity,
                max: self.config.max_order_size,
            });
        }

        // Check order value
        if let Some(price) = order.price {
            let value = price * order.quantity;
            if value > self.config.max_order_value {
                return Err(RiskError::OrderValueTooHigh {
                    value,
                    max: self.config.max_order_value,
                });
            }
        }

        Ok(())
    }

    /// Process a trade and check for circuit breaker triggers
    pub fn on_trade(&mut self, trade_price: Decimal, timestamp: DateTime<Utc>) -> Option<HaltReason> {
        // Update price history
        self.price_history.push_back((timestamp, trade_price));

        // Remove old prices outside window
        let cutoff = timestamp - Duration::minutes(self.config.price_window_minutes);
        while let Some((ts, _)) = self.price_history.front() {
            if *ts < cutoff {
                self.price_history.pop_front();
            } else {
                break;
            }
        }

        self.trade_count += 1;

        // Set reference price if not set
        if self.reference_price.is_none() {
            self.reference_price = Some(trade_price);
        }

        // Check for trigger conditions
        if self.trade_count >= self.config.min_trades_for_activation {
            if let Some(reason) = self.check_triggers(trade_price) {
                self.trigger_halt(reason);
                return Some(reason);
            }
        }

        None
    }

    fn check_triggers(&self, current_price: Decimal) -> Option<HaltReason> {
        let ref_price = self.reference_price?;

        // Calculate price change percentage
        let change_pct = ((current_price - ref_price) / ref_price).abs() * dec!(100);

        if change_pct >= self.config.max_price_change_pct {
            return Some(HaltReason::PriceVolatility);
        }

        None
    }

    fn trigger_halt(&mut self, reason: HaltReason) {
        let until = Utc::now() + Duration::minutes(self.config.halt_duration_minutes);
        self.state = CircuitState::Halted;
        self.halt_reason = Some(reason);
        self.halt_until = Some(until);

        // Reset trade count
        self.trade_count = 0;

        // Update reference price to current for next period
        if let Some((_, price)) = self.price_history.back() {
            self.reference_price = Some(*price);
        }
    }

    /// Manually halt trading
    pub fn manual_halt(&mut self, duration_minutes: i64) {
        let until = Utc::now() + Duration::minutes(duration_minutes);
        self.state = CircuitState::Halted;
        self.halt_reason = Some(HaltReason::Manual);
        self.halt_until = Some(until);
    }

    /// Resume trading
    pub fn resume(&mut self) {
        self.state = CircuitState::Normal;
        self.halt_reason = None;
        self.halt_until = None;
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitState {
        self.state
    }

    /// Get halt reason (if halted)
    pub fn get_halt_reason(&self) -> Option<HaltReason> {
        self.halt_reason
    }

    /// Get halt end time (if halted)
    pub fn get_halt_until(&self) -> Option<DateTime<Utc>> {
        self.halt_until
    }

    /// Get circuit breaker status
    pub fn get_status(&self) -> CircuitBreakerStatus {
        CircuitBreakerStatus {
            state: self.state,
            halt_reason: self.halt_reason,
            halt_until: self.halt_until,
            reference_price: self.reference_price,
            trade_count: self.trade_count,
            price_history_size: self.price_history.len(),
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub halt_reason: Option<HaltReason>,
    pub halt_until: Option<DateTime<Utc>>,
    pub reference_price: Option<Decimal>,
    pub trade_count: u32,
    pub price_history_size: usize,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, ToSchema)]
#[serde(tag = "error_type")]
pub enum RiskError {
    #[error("Trading halted due to {reason:?}, resumes at {resume_at}")]
    TradingHalted {
        reason: HaltReason,
        resume_at: DateTime<Utc>,
    },

    #[error("Order quantity {quantity} exceeds maximum {max}")]
    OrderTooLarge {
        quantity: Decimal,
        max: Decimal,
    },

    #[error("Order value {value} exceeds maximum {max}")]
    OrderValueTooHigh {
        value: Decimal,
        max: Decimal,
    },

    #[error("Rate limit exceeded: {orders_per_second} orders/second")]
    RateLimitExceeded {
        orders_per_second: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OrderSide, OrderType, OrderStatus, TimeInForce};
    use crate::models::stp::SelfTradePreventionMode;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn create_test_order(price: Decimal, quantity: Decimal) -> Order {
        Order {
            id: Uuid::new_v4(),
            symbol: "TEST".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: "test_user".to_string(),
            timestamp: Utc::now(),
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        }
    }

    #[test]
    fn test_circuit_breaker_normal_operation() {
        let mut cb = CircuitBreaker::default();

        assert!(cb.is_trading_allowed());
        assert_eq!(cb.get_state(), CircuitState::Normal);
    }

    #[test]
    fn test_order_size_validation() {
        let config = CircuitBreakerConfig {
            max_order_size: dec!(1000),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Valid order
        let order = create_test_order(dec!(100), dec!(500));
        assert!(cb.validate_order(&order).is_ok());

        // Oversized order
        let order = create_test_order(dec!(100), dec!(2000));
        assert!(cb.validate_order(&order).is_err());
    }

    #[test]
    fn test_order_value_validation() {
        let config = CircuitBreakerConfig {
            max_order_value: dec!(100000),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Valid order
        let order = create_test_order(dec!(100), dec!(500));
        assert!(cb.validate_order(&order).is_ok());

        // High value order
        let order = create_test_order(dec!(1000), dec!(500));
        assert!(cb.validate_order(&order).is_err());
    }

    #[test]
    fn test_price_volatility_trigger() {
        let config = CircuitBreakerConfig {
            max_price_change_pct: dec!(10),
            min_trades_for_activation: 5,
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        let now = Utc::now();

        // Record trades within acceptable range
        for i in 1..=5 {
            cb.on_trade(dec!(100) + Decimal::from(i), now + Duration::seconds(i));
        }

        assert_eq!(cb.get_state(), CircuitState::Normal);

        // Large price move - should trigger
        let reason = cb.on_trade(dec!(120), now + Duration::seconds(6));
        assert!(reason.is_some());
        assert_eq!(cb.get_state(), CircuitState::Halted);
    }

    #[test]
    fn test_manual_halt() {
        let mut cb = CircuitBreaker::default();

        cb.manual_halt(5);

        assert_eq!(cb.get_state(), CircuitState::Halted);
        assert_eq!(cb.get_halt_reason(), Some(HaltReason::Manual));
    }

    #[test]
    fn test_resume() {
        let mut cb = CircuitBreaker::default();

        cb.manual_halt(5);
        assert_eq!(cb.get_state(), CircuitState::Halted);

        cb.resume();
        assert_eq!(cb.get_state(), CircuitState::Normal);
    }
}
