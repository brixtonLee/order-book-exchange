use crate::database::enums::Timeframe;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// OHLC (Open-High-Low-Close) candle entity
///
/// Stored in TimescaleDB hypertable partitioned by open_time
/// Populated by TimescaleDB continuous aggregates
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::ohlc_candles)]
#[diesel(primary_key(symbol_id, timeframe, open_time))]
pub struct OhlcCandle {
    /// Auto-incrementing ID
    pub id: i64,

    /// Symbol ID (foreign key to symbols table)
    pub symbol_id: i64,

    /// Symbol name for easier querying
    pub symbol_name: String,

    /// Timeframe interval (1m, 5m, 15m, 30m, 1h, 4h, 1d)
    pub timeframe: Timeframe,

    /// Candle open time (partition key for TimescaleDB)
    pub open_time: DateTime<Utc>,

    /// Candle close time
    pub close_time: DateTime<Utc>,

    /// Opening price (first tick in interval)
    pub open_price: Decimal,

    /// Highest price in interval
    pub high_price: Decimal,

    /// Lowest price in interval
    pub low_price: Decimal,

    /// Closing price (last tick in interval)
    pub close_price: Decimal,

    /// Total volume traded in interval
    pub volume: Decimal,

    /// Number of ticks aggregated in this candle
    pub tick_count: i64,

    /// When this record was created
    pub created_at: DateTime<Utc>,
}

/// New OHLC candle for insertion
#[derive(Debug, Clone, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::ohlc_candles)]
pub struct NewOhlcCandle {
    pub symbol_id: i64,
    pub symbol_name: String,
    pub timeframe: Timeframe,
    pub open_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub open_price: Decimal,
    pub high_price: Decimal,
    pub low_price: Decimal,
    pub close_price: Decimal,
    pub volume: Decimal,
    pub tick_count: i64,
}

impl OhlcCandle {
    /// Calculate candle body size (abs(close - open))
    pub fn body_size(&self) -> Decimal {
        (self.close_price - self.open_price).abs()
    }

    /// Calculate candle range (high - low)
    pub fn range(&self) -> Decimal {
        self.high_price - self.low_price
    }

    /// Check if candle is bullish (close > open)
    pub fn is_bullish(&self) -> bool {
        self.close_price > self.open_price
    }

    /// Check if candle is bearish (close < open)
    pub fn is_bearish(&self) -> bool {
        self.close_price < self.open_price
    }

    /// Check if candle is doji (close ≈ open, within 0.01%)
    pub fn is_doji(&self) -> bool {
        if self.open_price.is_zero() {
            return false;
        }
        let percentage_diff = ((self.close_price - self.open_price).abs() / self.open_price)
            * Decimal::from(100);
        percentage_diff < Decimal::new(1, 2) // Less than 0.01%
    }

    /// Calculate upper shadow length
    pub fn upper_shadow(&self) -> Decimal {
        self.high_price - self.open_price.max(self.close_price)
    }

    /// Calculate lower shadow length
    pub fn lower_shadow(&self) -> Decimal {
        self.open_price.min(self.close_price) - self.low_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_candle(
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
    ) -> OhlcCandle {
        OhlcCandle {
            id: 1,
            symbol_id: 1,
            symbol_name: "EURUSD".to_string(),
            timeframe: Timeframe::FiveMinutes,
            open_time: Utc::now(),
            close_time: Utc::now(),
            open_price: open,
            high_price: high,
            low_price: low,
            close_price: close,
            volume: dec!(1000000),
            tick_count: 100,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_bullish_candle() {
        let candle = create_test_candle(dec!(1.1000), dec!(1.1010), dec!(1.0995), dec!(1.1008));
        assert!(candle.is_bullish());
        assert!(!candle.is_bearish());
        assert_eq!(candle.body_size(), dec!(0.0008));
    }

    #[test]
    fn test_bearish_candle() {
        let candle = create_test_candle(dec!(1.1000), dec!(1.1005), dec!(1.0990), dec!(1.0992));
        assert!(!candle.is_bullish());
        assert!(candle.is_bearish());
        assert_eq!(candle.body_size(), dec!(0.0008));
    }

    #[test]
    fn test_candle_range() {
        let candle = create_test_candle(dec!(1.1000), dec!(1.1010), dec!(1.0990), dec!(1.1005));
        assert_eq!(candle.range(), dec!(0.0020));
    }

    #[test]
    fn test_shadows() {
        let candle = create_test_candle(dec!(1.1000), dec!(1.1015), dec!(1.0985), dec!(1.1010));
        // Upper shadow: high - max(open, close) = 1.1015 - 1.1010 = 0.0005
        assert_eq!(candle.upper_shadow(), dec!(0.0005));
        // Lower shadow: min(open, close) - low = 1.1000 - 1.0985 = 0.0015
        assert_eq!(candle.lower_shadow(), dec!(0.0015));
    }
}
