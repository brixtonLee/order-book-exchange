use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use utoipa::ToSchema;

use crate::models::OrderBook;

/// Market microstructure analytics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MicrostructureMetrics {
    /// Simple mid price
    pub mid_price: Decimal,

    /// Volume-weighted mid price (more accurate fair value)
    pub microprice: Decimal,

    /// Book imbalance at best bid/ask (-1 to +1)
    pub imbalance_l1: Decimal,

    /// Weighted multi-level imbalance
    pub imbalance_weighted: Decimal,

    /// Volume at best bid
    pub best_bid_volume: Decimal,

    /// Volume at best ask
    pub best_ask_volume: Decimal,

    /// Total bid depth (configurable levels)
    pub total_bid_depth: Decimal,

    /// Total ask depth (configurable levels)
    pub total_ask_depth: Decimal,

    /// Spread in basis points
    pub spread_bps: Decimal,

    /// Timestamp of calculation
    pub timestamp: DateTime<Utc>,
}

impl MicrostructureMetrics {
    /// Calculate all microstructure metrics from order book
    pub fn from_order_book(book: &OrderBook, depth_levels: usize) -> Option<Self> {
        let best_bid = book.get_best_bid()?;
        let best_ask = book.get_best_ask()?;

        // Get volumes at best prices
        let best_bid_vol = book.bids.get(&best_bid)
            .map(|level| level.total_quantity)
            .unwrap_or(Decimal::ZERO);
        let best_ask_vol = book.asks.get(&best_ask)
            .map(|level| level.total_quantity)
            .unwrap_or(Decimal::ZERO);

        // Mid price (simple average)
        let mid_price = (best_bid + best_ask) / dec!(2);

        // Microprice (volume-weighted)
        // Intuition: More volume on bid side → price more likely to go up
        // So we weight ask price MORE when there's more bid volume
        let microprice = if best_bid_vol + best_ask_vol > Decimal::ZERO {
            (best_bid * best_ask_vol + best_ask * best_bid_vol)
                / (best_bid_vol + best_ask_vol)
        } else {
            mid_price
        };

        // Level 1 imbalance
        let total_vol = best_bid_vol + best_ask_vol;
        let imbalance_l1 = if total_vol > Decimal::ZERO {
            (best_bid_vol - best_ask_vol) / total_vol
        } else {
            Decimal::ZERO
        };

        // Multi-level weighted imbalance
        let (bid_depth, ask_depth, imbalance_weighted) =
            Self::calculate_depth_imbalance(book, depth_levels);

        // Spread in basis points
        let spread_bps = if mid_price > Decimal::ZERO {
            ((best_ask - best_bid) / mid_price) * dec!(10000)
        } else {
            Decimal::ZERO
        };

        Some(Self {
            mid_price,
            microprice,
            imbalance_l1,
            imbalance_weighted,
            best_bid_volume: best_bid_vol,
            best_ask_volume: best_ask_vol,
            total_bid_depth: bid_depth,
            total_ask_depth: ask_depth,
            spread_bps,
            timestamp: Utc::now(),
        })
    }

    /// Calculate weighted imbalance across multiple price levels
    fn calculate_depth_imbalance(
        book: &OrderBook,
        levels: usize,
    ) -> (Decimal, Decimal, Decimal) {
        let mut bid_weighted = Decimal::ZERO;
        let mut ask_weighted = Decimal::ZERO;
        let mut total_bid = Decimal::ZERO;
        let mut total_ask = Decimal::ZERO;

        // Exponential decay weights: 1.0, 0.5, 0.25, 0.125, ...
        let decay = dec!(0.5);

        // Get top N bid levels
        let bid_levels: Vec<_> = book.bids.iter()
            .rev()
            .take(levels)
            .collect();

        // Get top N ask levels
        let ask_levels: Vec<_> = book.asks.iter()
            .take(levels)
            .collect();

        for (i, (_price, level)) in bid_levels.iter().enumerate() {
            let weight = Decimal::from_f64(
                decay.to_f64().unwrap_or(1.0).powi(i as i32)
            ).unwrap_or(Decimal::ONE);
            bid_weighted += level.total_quantity * weight;
            total_bid += level.total_quantity;
        }

        for (i, (_price, level)) in ask_levels.iter().enumerate() {
            let weight = Decimal::from_f64(
                decay.to_f64().unwrap_or(1.0).powi(i as i32)
            ).unwrap_or(Decimal::ONE);
            ask_weighted += level.total_quantity * weight;
            total_ask += level.total_quantity;
        }

        let total_weighted = bid_weighted + ask_weighted;
        let imbalance = if total_weighted > Decimal::ZERO {
            (bid_weighted - ask_weighted) / total_weighted
        } else {
            Decimal::ZERO
        };

        (total_bid, total_ask, imbalance)
    }

    /// Predict short-term price direction based on imbalance
    /// Returns expected price change as a multiple of spread
    pub fn predicted_price_move(&self) -> Decimal {
        // Academic research suggests: E[ΔP] ≈ λ × imbalance × spread
        // where λ is typically 0.5-1.0
        let lambda = dec!(0.7);
        let spread = self.spread_bps / dec!(10000);
        lambda * self.imbalance_weighted * spread * self.mid_price
    }

    /// Get trading signal based on imbalance
    /// Returns: Bullish (>0), Bearish (<0), Neutral (0)
    pub fn trading_signal(&self) -> TradingSignal {
        let threshold = dec!(0.2); // 20% imbalance threshold

        if self.imbalance_weighted > threshold {
            TradingSignal::Bullish
        } else if self.imbalance_weighted < -threshold {
            TradingSignal::Bearish
        } else {
            TradingSignal::Neutral
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum TradingSignal {
    Bullish,
    Bearish,
    Neutral,
}

/// Time-weighted metrics for signal smoothing
pub struct SmoothedMetrics {
    history: VecDeque<MicrostructureMetrics>,
    window_size: usize,
}

impl SmoothedMetrics {
    pub fn new(window_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    pub fn update(&mut self, metrics: MicrostructureMetrics) {
        if self.history.len() >= self.window_size {
            self.history.pop_front();
        }
        self.history.push_back(metrics);
    }

    /// Simple moving average of imbalance
    pub fn sma_imbalance(&self) -> Decimal {
        if self.history.is_empty() {
            return Decimal::ZERO;
        }

        let sum: Decimal = self.history.iter()
            .map(|m| m.imbalance_l1)
            .sum();

        sum / Decimal::from(self.history.len())
    }

    /// Exponentially weighted moving average of imbalance
    pub fn ewma_imbalance(&self, alpha: f64) -> Decimal {
        if self.history.is_empty() {
            return Decimal::ZERO;
        }

        let alpha_dec = Decimal::from_f64(alpha).unwrap_or(dec!(0.2));
        let one_minus_alpha = Decimal::ONE - alpha_dec;
        let mut ewma = Decimal::ZERO;

        for metrics in &self.history {
            ewma = ewma * one_minus_alpha + metrics.imbalance_l1 * alpha_dec;
        }

        ewma
    }

    /// Get average microprice
    pub fn avg_microprice(&self) -> Decimal {
        if self.history.is_empty() {
            return Decimal::ZERO;
        }

        let sum: Decimal = self.history.iter()
            .map(|m| m.microprice)
            .sum();

        sum / Decimal::from(self.history.len())
    }

    /// Get average spread in basis points
    pub fn avg_spread_bps(&self) -> Decimal {
        if self.history.is_empty() {
            return Decimal::ZERO;
        }

        let sum: Decimal = self.history.iter()
            .map(|m| m.spread_bps)
            .sum();

        sum / Decimal::from(self.history.len())
    }

    /// Get latest metrics
    pub fn latest(&self) -> Option<&MicrostructureMetrics> {
        self.history.back()
    }

    /// Check if window is full
    pub fn is_full(&self) -> bool {
        self.history.len() >= self.window_size
    }

    /// Get number of metrics in history
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_book() -> OrderBook {
        use crate::models::orderbook::PriceLevel;
        let mut book = OrderBook::new("TEST".to_string());

        // Add bids manually
        let mut bid1 = PriceLevel::new(dec!(100.00));
        bid1.add_order(Uuid::new_v4(), dec!(5000));
        book.bids.insert(dec!(100.00), bid1);

        let mut bid2 = PriceLevel::new(dec!(99.95));
        bid2.add_order(Uuid::new_v4(), dec!(3000));
        book.bids.insert(dec!(99.95), bid2);

        let mut bid3 = PriceLevel::new(dec!(99.90));
        bid3.add_order(Uuid::new_v4(), dec!(2000));
        book.bids.insert(dec!(99.90), bid3);

        // Add asks manually
        let mut ask1 = PriceLevel::new(dec!(100.05));
        ask1.add_order(Uuid::new_v4(), dec!(2000));
        book.asks.insert(dec!(100.05), ask1);

        let mut ask2 = PriceLevel::new(dec!(100.10));
        ask2.add_order(Uuid::new_v4(), dec!(4000));
        book.asks.insert(dec!(100.10), ask2);

        let mut ask3 = PriceLevel::new(dec!(100.15));
        ask3.add_order(Uuid::new_v4(), dec!(3000));
        book.asks.insert(dec!(100.15), ask3);

        book
    }


    #[test]
    fn test_microstructure_metrics() {
        let book = create_test_book();
        let metrics = MicrostructureMetrics::from_order_book(&book, 3).unwrap();

        // Check basic calculations
        assert_eq!(metrics.mid_price, dec!(100.025));
        assert!(metrics.microprice > Decimal::ZERO);

        // Imbalance should be positive (more bid volume)
        assert!(metrics.imbalance_l1 > Decimal::ZERO);

        // Check spread
        assert!(metrics.spread_bps > Decimal::ZERO);
    }

    #[test]
    fn test_trading_signal() {
        let book = create_test_book();
        let metrics = MicrostructureMetrics::from_order_book(&book, 3).unwrap();

        let signal = metrics.trading_signal();
        // Should be bullish due to more bid volume
        assert_eq!(signal, TradingSignal::Bullish);
    }

    #[test]
    fn test_smoothed_metrics() {
        let mut smoothed = SmoothedMetrics::new(5);
        let book = create_test_book();

        for _ in 0..3 {
            let metrics = MicrostructureMetrics::from_order_book(&book, 3).unwrap();
            smoothed.update(metrics);
        }

        assert_eq!(smoothed.len(), 3);
        assert!(!smoothed.is_full());

        let sma = smoothed.sma_imbalance();
        assert!(sma > Decimal::ZERO);
    }

    #[test]
    fn test_predicted_price_move() {
        let book = create_test_book();
        let metrics = MicrostructureMetrics::from_order_book(&book, 3).unwrap();

        let predicted_move = metrics.predicted_price_move();
        // Should predict upward movement due to bid imbalance
        assert!(predicted_move > Decimal::ZERO);
    }
}
