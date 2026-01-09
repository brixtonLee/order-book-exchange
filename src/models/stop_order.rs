use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{OrderSide, OrderStatus, TimeInForce};
use super::stp::SelfTradePreventionMode;

/// Condition that triggers the stop order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TriggerCondition {
    /// Trigger when last trade price >= trigger_price (for buy stops)
    AtOrAbove,
    /// Trigger when last trade price <= trigger_price (for sell stops)
    AtOrBelow,
    /// Trigger when last trade price > trigger_price
    Above,
    /// Trigger when last trade price < trigger_price
    Below,
}

/// Type of order to submit when triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StopOrderType {
    /// Stop-Market: Submit market order when triggered
    StopMarket,
    /// Stop-Limit: Submit limit order at specified price when triggered
    StopLimit,
    /// Trailing Stop: Trigger price follows market by offset
    TrailingStop,
}

/// A stop order waiting to be triggered
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StopOrder {
    pub id: Uuid,
    pub symbol: String,
    pub user_id: String,

    // Trigger configuration
    pub trigger_price: Decimal,
    pub trigger_condition: TriggerCondition,
    pub stop_type: StopOrderType,

    // Order to submit when triggered
    pub side: OrderSide,
    pub quantity: Decimal,
    pub limit_price: Option<Decimal>, // For stop-limit orders

    // Trailing stop specific
    pub trail_amount: Option<Decimal>,     // Fixed offset
    pub trail_percent: Option<Decimal>,    // Percentage offset
    pub highest_price: Option<Decimal>,    // Tracked high (for sell trailing)
    pub lowest_price: Option<Decimal>,     // Tracked low (for buy trailing)

    // Metadata
    pub created_at: DateTime<Utc>,
    pub expire_time: Option<DateTime<Utc>>,
    pub status: StopOrderStatus,

    // For creating triggered order
    pub time_in_force: TimeInForce,
    pub stp_mode: SelfTradePreventionMode,
    pub post_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StopOrderStatus {
    Pending,    // Waiting for trigger
    Triggered,  // Trigger hit, order submitted
    Cancelled,  // User cancelled
    Expired,    // Time expired
    Rejected,   // Triggered order was rejected
}

impl StopOrder {
    /// Check if this stop should trigger given the last trade price
    pub fn should_trigger(&self, last_price: Decimal) -> bool {
        match self.trigger_condition {
            TriggerCondition::AtOrAbove => last_price >= self.trigger_price,
            TriggerCondition::AtOrBelow => last_price <= self.trigger_price,
            TriggerCondition::Above => last_price > self.trigger_price,
            TriggerCondition::Below => last_price < self.trigger_price,
        }
    }

    /// Update trailing stop trigger price based on market movement
    pub fn update_trailing(&mut self, last_price: Decimal) {
        match (self.side, self.trail_amount, self.trail_percent) {
            // Sell trailing stop: trigger follows price UP
            (OrderSide::Sell, Some(offset), _) => {
                let new_high = self.highest_price.unwrap_or(last_price).max(last_price);
                self.highest_price = Some(new_high);
                self.trigger_price = new_high - offset;
            }
            (OrderSide::Sell, _, Some(pct)) => {
                let new_high = self.highest_price.unwrap_or(last_price).max(last_price);
                self.highest_price = Some(new_high);
                self.trigger_price = new_high * (Decimal::ONE - pct / Decimal::from(100));
            }
            // Buy trailing stop: trigger follows price DOWN
            (OrderSide::Buy, Some(offset), _) => {
                let new_low = self.lowest_price.unwrap_or(last_price).min(last_price);
                self.lowest_price = Some(new_low);
                self.trigger_price = new_low + offset;
            }
            (OrderSide::Buy, _, Some(pct)) => {
                let new_low = self.lowest_price.unwrap_or(last_price).min(last_price);
                self.lowest_price = Some(new_low);
                self.trigger_price = new_low * (Decimal::ONE + pct / Decimal::from(100));
            }
            _ => {}
        }
    }

    /// Check if the stop order has expired
    pub fn is_expired(&self, current_time: DateTime<Utc>) -> bool {
        if let Some(expire_time) = self.expire_time {
            current_time >= expire_time
        } else {
            false
        }
    }

    /// Check if the stop order is active (can be triggered)
    pub fn is_active(&self) -> bool {
        self.status == StopOrderStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_stop_order(
        side: OrderSide,
        trigger_price: Decimal,
        trigger_condition: TriggerCondition,
    ) -> StopOrder {
        StopOrder {
            id: Uuid::new_v4(),
            symbol: "TEST".to_string(),
            user_id: "test_user".to_string(),
            trigger_price,
            trigger_condition,
            stop_type: StopOrderType::StopMarket,
            side,
            quantity: dec!(100),
            limit_price: None,
            trail_amount: None,
            trail_percent: None,
            highest_price: None,
            lowest_price: None,
            created_at: Utc::now(),
            expire_time: None,
            status: StopOrderStatus::Pending,
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
        }
    }

    #[test]
    fn test_trigger_at_or_above() {
        let stop = create_test_stop_order(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);

        assert!(!stop.should_trigger(dec!(99)));
        assert!(stop.should_trigger(dec!(100)));
        assert!(stop.should_trigger(dec!(101)));
    }

    #[test]
    fn test_trigger_at_or_below() {
        let stop = create_test_stop_order(OrderSide::Sell, dec!(100), TriggerCondition::AtOrBelow);

        assert!(stop.should_trigger(dec!(99)));
        assert!(stop.should_trigger(dec!(100)));
        assert!(!stop.should_trigger(dec!(101)));
    }

    #[test]
    fn test_trailing_stop_sell() {
        let mut stop = create_test_stop_order(OrderSide::Sell, dec!(95), TriggerCondition::AtOrBelow);
        stop.stop_type = StopOrderType::TrailingStop;
        stop.trail_amount = Some(dec!(5));

        // Price moves up - trigger price should follow
        stop.update_trailing(dec!(100));
        assert_eq!(stop.trigger_price, dec!(95)); // 100 - 5

        stop.update_trailing(dec!(105));
        assert_eq!(stop.trigger_price, dec!(100)); // 105 - 5

        // Price moves down - trigger price should stay
        stop.update_trailing(dec!(103));
        assert_eq!(stop.trigger_price, dec!(100)); // Still 105 - 5 (doesn't follow down)
    }

    #[test]
    fn test_trailing_stop_buy() {
        let mut stop = create_test_stop_order(OrderSide::Buy, dec!(105), TriggerCondition::AtOrAbove);
        stop.stop_type = StopOrderType::TrailingStop;
        stop.trail_amount = Some(dec!(5));

        // Price moves down - trigger price should follow
        stop.update_trailing(dec!(100));
        assert_eq!(stop.trigger_price, dec!(105)); // 100 + 5

        stop.update_trailing(dec!(95));
        assert_eq!(stop.trigger_price, dec!(100)); // 95 + 5

        // Price moves up - trigger price should stay
        stop.update_trailing(dec!(97));
        assert_eq!(stop.trigger_price, dec!(100)); // Still 95 + 5 (doesn't follow up)
    }

    #[test]
    fn test_trailing_stop_percentage() {
        let mut stop = create_test_stop_order(OrderSide::Sell, dec!(95), TriggerCondition::AtOrBelow);
        stop.stop_type = StopOrderType::TrailingStop;
        stop.trail_percent = Some(dec!(5)); // 5%

        stop.update_trailing(dec!(100));
        assert_eq!(stop.trigger_price, dec!(95)); // 100 * 0.95
    }

    #[test]
    fn test_expiration() {
        let mut stop = create_test_stop_order(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);

        assert!(!stop.is_expired(Utc::now()));

        stop.expire_time = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(stop.is_expired(Utc::now()));
    }
}
