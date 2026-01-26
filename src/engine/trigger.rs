use std::collections::{BTreeMap, HashMap};
use rust_decimal::Decimal;
use uuid::Uuid;
use chrono::Utc;

use crate::models::{Order, OrderType, OrderStatus, StopOrder, StopOrderType, StopOrderStatus};

/// Engine that monitors prices and triggers stop orders
pub struct TriggerEngine {
    /// Stop orders indexed by trigger price for efficient scanning
    /// Key: trigger price, Value: orders at that trigger level
    buy_stops: BTreeMap<Decimal, Vec<StopOrder>>,   // Trigger at or above
    sell_stops: BTreeMap<Decimal, Vec<StopOrder>>,  // Trigger at or below

    /// Index for O(1) lookup by order ID
    order_index: HashMap<Uuid, (Decimal, bool)>,  // order_id -> (trigger_price, is_buy)

    /// Last known trade price
    last_trade_price: Option<Decimal>,
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self {
            buy_stops: BTreeMap::new(),
            sell_stops: BTreeMap::new(),
            order_index: HashMap::new(),
            last_trade_price: None,
        }
    }

    /// Add a new stop order
    pub fn add_stop_order(&mut self, stop: StopOrder) {
        let is_buy = matches!(stop.side, crate::models::OrderSide::Buy);
        self.order_index.insert(stop.id, (stop.trigger_price, is_buy));

        let map = if is_buy {
            &mut self.buy_stops
        } else {
            &mut self.sell_stops
        };

        map.entry(stop.trigger_price)
            .or_insert_with(Vec::new)
            .push(stop);
    }

    /// Cancel a stop order
    pub fn cancel_stop_order(&mut self, order_id: Uuid) -> Option<StopOrder> {
        if let Some((trigger_price, is_buy)) = self.order_index.remove(&order_id) {
            
            let map = if is_buy {
                &mut self.buy_stops
            } else {
                &mut self.sell_stops
            };

            if let Some(orders) = map.get_mut(&trigger_price) {
                if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                    let mut order = orders.remove(pos);
                    order.status = StopOrderStatus::Cancelled;

                    // Clean up empty price levels
                    if orders.is_empty() {
                        map.remove(&trigger_price);
                    }

                    return Some(order);
                }
            }
        }
        None
    }

    /// Process a new trade and return any triggered orders
    ///
    /// This is called after every trade execution in the matching engine.
    /// Returns a Vec of Orders ready to be submitted to the main order book.
    pub fn on_trade(&mut self, trade_price: Decimal) -> Vec<Order> {
        let mut triggered_orders = Vec::new();
        let current_time = Utc::now();

        // Update trailing stops first
        self.update_trailing_stops(trade_price);

        // Check buy stops (trigger at or above)
        // Use range to efficiently get all stops at or below current price
        let triggered_buy_prices: Vec<Decimal> = self.buy_stops
            .range(..=trade_price)
            .map(|(price, _)| *price)
            .collect();

        for price in triggered_buy_prices {
            if let Some(stops) = self.buy_stops.remove(&price) {
                for mut stop in stops {
                    // Check expiration
                    if stop.is_expired(current_time) {
                        stop.status = StopOrderStatus::Expired;
                        continue;
                    }

                    if stop.should_trigger(trade_price) {
                        stop.status = StopOrderStatus::Triggered;
                        triggered_orders.push(self.convert_to_order(&stop));
                        self.order_index.remove(&stop.id);
                    } else {
                        // Put back if not triggered
                        self.buy_stops
                            .entry(stop.trigger_price)
                            .or_insert_with(Vec::new)
                            .push(stop);
                    }
                }
            }
        }

        // Check sell stops (trigger at or below)
        let triggered_sell_prices: Vec<Decimal> = self.sell_stops
            .range(trade_price..)
            .map(|(price, _)| *price)
            .collect();

        for price in triggered_sell_prices {
            if let Some(stops) = self.sell_stops.remove(&price) {
                for mut stop in stops {
                    // Check expiration
                    if stop.is_expired(current_time) {
                        stop.status = StopOrderStatus::Expired;
                        continue;
                    }

                    if stop.should_trigger(trade_price) {
                        stop.status = StopOrderStatus::Triggered;
                        triggered_orders.push(self.convert_to_order(&stop));
                        self.order_index.remove(&stop.id);
                    } else {
                        // Put back if not triggered
                        self.sell_stops
                            .entry(stop.trigger_price)
                            .or_insert_with(Vec::new)
                            .push(stop);
                    }
                }
            }
        }

        self.last_trade_price = Some(trade_price);
        triggered_orders
    }

    /// Convert a triggered stop order into a regular order
    fn convert_to_order(&self, stop: &StopOrder) -> Order {
        Order {
            id: Uuid::new_v4(), // New ID for the actual order
            symbol: stop.symbol.clone(),
            side: stop.side,
            order_type: match stop.stop_type {
                StopOrderType::StopMarket => OrderType::Market,
                StopOrderType::StopLimit | StopOrderType::TrailingStop => OrderType::Limit,
            },
            price: stop.limit_price,
            quantity: stop.quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: stop.user_id.clone(),
            timestamp: Utc::now(),
            time_in_force: stop.time_in_force,
            stp_mode: stop.stp_mode,
            post_only: stop.post_only,
            expire_time: stop.expire_time,
            iceberg: None,
        }
    }

    fn update_trailing_stops(&mut self, price: Decimal) {
        // Update all trailing stops with new price
        for stops in self.buy_stops.values_mut() {
            for stop in stops.iter_mut() {
                if stop.stop_type == StopOrderType::TrailingStop {
                    stop.update_trailing(price);
                }
            }
        }
        for stops in self.sell_stops.values_mut() {
            for stop in stops.iter_mut() {
                if stop.stop_type == StopOrderType::TrailingStop {
                    stop.update_trailing(price);
                }
            }
        }
    }

    /// Get a stop order by ID
    pub fn get_stop_order(&self, order_id: Uuid) -> Option<&StopOrder> {
        if let Some((trigger_price, is_buy)) = self.order_index.get(&order_id) {
            let map = if *is_buy {
                &self.buy_stops
            } else {
                &self.sell_stops
            };

            if let Some(orders) = map.get(trigger_price) {
                return orders.iter().find(|o| o.id == order_id);
            }
        }
        None
    }

    /// Get all active stop orders for a symbol
    pub fn get_stop_orders_by_symbol(&self, symbol: &str) -> Vec<&StopOrder> {
        let mut result = Vec::new();

        for stops in self.buy_stops.values() {
            for stop in stops {
                if stop.symbol == symbol && stop.is_active() {
                    result.push(stop);
                }
            }
        }

        for stops in self.sell_stops.values() {
            for stop in stops {
                if stop.symbol == symbol && stop.is_active() {
                    result.push(stop);
                }
            }
        }

        result
    }

    /// Get total number of active stop orders
    pub fn get_total_stop_orders(&self) -> usize {
        let buy_count: usize = self.buy_stops.values().map(|v| v.len()).sum();
        let sell_count: usize = self.sell_stops.values().map(|v| v.len()).sum();
        buy_count + sell_count
    }

    /// Get last trade price
    pub fn get_last_trade_price(&self) -> Option<Decimal> {
        self.last_trade_price
    }

    /// Clean up expired stop orders
    pub fn cleanup_expired(&mut self) -> usize {
        let current_time = Utc::now();
        let mut expired_count = 0;

        // Clean buy stops
        let mut empty_levels = Vec::new();
        for (price, stops) in self.buy_stops.iter_mut() {
            stops.retain(|stop| {
                if stop.is_expired(current_time) {
                    self.order_index.remove(&stop.id);
                    expired_count += 1;
                    false
                } else {
                    true
                }
            });
            if stops.is_empty() {
                empty_levels.push(*price);
            }
        }
        for price in empty_levels {
            self.buy_stops.remove(&price);
        }

        // Clean sell stops
        let mut empty_levels = Vec::new();
        for (price, stops) in self.sell_stops.iter_mut() {
            stops.retain(|stop| {
                if stop.is_expired(current_time) {
                    self.order_index.remove(&stop.id);
                    expired_count += 1;
                    false
                } else {
                    true
                }
            });
            if stops.is_empty() {
                empty_levels.push(*price);
            }
        }
        for price in empty_levels {
            self.sell_stops.remove(&price);
        }

        expired_count
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::models::{OrderSide, TimeInForce, TriggerCondition};
    use crate::models::order::SelfTradePreventionMode;

    fn create_test_stop(
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
    fn test_add_and_get_stop_order() {
        let mut engine = TriggerEngine::new();
        let stop = create_test_stop(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);
        let stop_id = stop.id;

        engine.add_stop_order(stop);

        assert_eq!(engine.get_total_stop_orders(), 1);
        assert!(engine.get_stop_order(stop_id).is_some());
    }

    #[test]
    fn test_cancel_stop_order() {
        let mut engine = TriggerEngine::new();
        let stop = create_test_stop(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);
        let stop_id = stop.id;

        engine.add_stop_order(stop);
        assert_eq!(engine.get_total_stop_orders(), 1);

        let cancelled = engine.cancel_stop_order(stop_id);
        assert!(cancelled.is_some());
        assert_eq!(cancelled.unwrap().status, StopOrderStatus::Cancelled);
        assert_eq!(engine.get_total_stop_orders(), 0);
    }

    #[test]
    fn test_trigger_buy_stop() {
        let mut engine = TriggerEngine::new();
        let stop = create_test_stop(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);

        engine.add_stop_order(stop);

        // Price below trigger - no trigger
        let triggered = engine.on_trade(dec!(99));
        assert_eq!(triggered.len(), 0);
        assert_eq!(engine.get_total_stop_orders(), 1);

        // Price at trigger - should trigger
        let triggered = engine.on_trade(dec!(100));
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].side, OrderSide::Buy);
        assert_eq!(engine.get_total_stop_orders(), 0);
    }

    #[test]
    fn test_trigger_sell_stop() {
        let mut engine = TriggerEngine::new();
        let stop = create_test_stop(OrderSide::Sell, dec!(100), TriggerCondition::AtOrBelow);

        engine.add_stop_order(stop);

        // Price above trigger - no trigger
        let triggered = engine.on_trade(dec!(101));
        assert_eq!(triggered.len(), 0);

        // Price at trigger - should trigger
        let triggered = engine.on_trade(dec!(100));
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].side, OrderSide::Sell);
    }

    #[test]
    fn test_get_stop_orders_by_symbol() {
        let mut engine = TriggerEngine::new();

        let stop1 = create_test_stop(OrderSide::Buy, dec!(100), TriggerCondition::AtOrAbove);
        let mut stop2 = create_test_stop(OrderSide::Sell, dec!(100), TriggerCondition::AtOrBelow);
        stop2.symbol = "OTHER".to_string();

        engine.add_stop_order(stop1);
        engine.add_stop_order(stop2);

        let test_stops = engine.get_stop_orders_by_symbol("TEST");
        assert_eq!(test_stops.len(), 1);

        let other_stops = engine.get_stop_orders_by_symbol("OTHER");
        assert_eq!(other_stops.len(), 1);
    }
}
