use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::OrderBook;

/// Spread metrics for an order book
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpreadMetrics {
    pub symbol: String,
    #[schema(value_type = Option<String>, example = "150.45")]
    pub best_bid: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "150.55")]
    pub best_ask: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "0.10")]
    pub spread_absolute: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "0.0006")]
    pub spread_percentage: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "6.64")]
    pub spread_bps: Option<Decimal>,
    #[schema(value_type = Option<String>, example = "150.50")]
    pub mid_price: Option<Decimal>,
    #[schema(value_type = String, example = "5000")]
    pub bid_depth: Decimal,
    #[schema(value_type = String, example = "3000")]
    pub ask_depth: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Calculate spread metrics for an order book
pub fn calculate_spread_metrics(orderbook: &OrderBook) -> SpreadMetrics {
    let best_bid = orderbook.get_best_bid();
    let best_ask = orderbook.get_best_ask();
    let spread = orderbook.get_spread();
    let mid_price = orderbook.get_mid_price();

    let spread_percentage = match (spread, mid_price) {
        (Some(s), Some(m)) if m > Decimal::ZERO => Some(s / m),
        _ => None,
    };

    let spread_bps = orderbook.get_spread_bps();

    SpreadMetrics {
        symbol: orderbook.symbol.clone(),
        best_bid,
        best_ask,
        spread_absolute: spread,
        spread_percentage,
        spread_bps,
        mid_price,
        bid_depth: orderbook.get_bid_depth(),
        ask_depth: orderbook.get_ask_depth(),
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PriceLevel;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    #[test]
    fn test_spread_metrics() {
        let mut book = OrderBook::new("AAPL".to_string());

        // Add bid level
        let mut bid_level = PriceLevel::new(dec!(100.00));
        bid_level.add_order(Uuid::new_v4(), dec!(50));
        book.bids.insert(dec!(100.00), bid_level);

        // Add ask level
        let mut ask_level = PriceLevel::new(dec!(100.50));
        ask_level.add_order(Uuid::new_v4(), dec!(30));
        book.asks.insert(dec!(100.50), ask_level);

        let metrics = calculate_spread_metrics(&book);

        assert_eq!(metrics.best_bid, Some(dec!(100.00)));
        assert_eq!(metrics.best_ask, Some(dec!(100.50)));
        assert_eq!(metrics.spread_absolute, Some(dec!(0.50)));
        assert_eq!(metrics.bid_depth, dec!(50));
        assert_eq!(metrics.ask_depth, dec!(30));
    }
}
