use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Tick entity - represents a market data tick from FIX feed
///
/// Stored in TimescaleDB hypertable partitioned by tick_time
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::ticks)]
#[diesel(primary_key(id))]
pub struct Tick {
    /// Auto-incrementing ID
    pub id: i64,

    /// Symbol ID (foreign key to symbols table)
    pub symbol_id: i64,

    /// Symbol name for easier querying
    pub symbol_name: String,

    /// Timestamp of the tick (partition key for TimescaleDB)
    pub tick_time: DateTime<Utc>,

    /// Bid price
    pub bid_price: Decimal,

    /// Ask price
    pub ask_price: Decimal,

    /// Bid volume/size
    pub bid_volume: Decimal,

    /// Ask volume/size
    pub ask_volume: Decimal,

    /// When this record was inserted into database
    pub created_at: DateTime<Utc>,
}

/// New tick for batch insertion
#[derive(Debug, Clone, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::ticks)]
pub struct NewTick {
    pub symbol_id: i64,
    pub symbol_name: String,
    pub tick_time: DateTime<Utc>,
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
}

impl NewTick {
    /// Create a new tick
    pub fn new(
        symbol_id: i64,
        symbol_name: String,
        tick_time: DateTime<Utc>,
        bid_price: Decimal,
        ask_price: Decimal,
        bid_volume: Decimal,
        ask_volume: Decimal,
    ) -> Self {
        Self {
            symbol_id,
            symbol_name,
            tick_time,
            bid_price,
            ask_price,
            bid_volume,
            ask_volume,
        }
    }

    /// Calculate spread (ask - bid)
    pub fn spread(&self) -> Decimal {
        self.ask_price - self.bid_price
    }

    /// Calculate mid price ((ask + bid) / 2)
    pub fn mid_price(&self) -> Decimal {
        (self.ask_price + self.bid_price) / Decimal::TWO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_tick_spread() {
        let tick = NewTick::new(
            1,
            "EURUSD".to_string(),
            Utc::now(),
            dec!(1.1000),
            dec!(1.1005),
            dec!(1000000),
            dec!(1000000),
        );

        assert_eq!(tick.spread(), dec!(0.0005));
    }

    #[test]
    fn test_tick_mid_price() {
        let tick = NewTick::new(
            1,
            "EURUSD".to_string(),
            Utc::now(),
            dec!(1.1000),
            dec!(1.1004),
            dec!(1000000),
            dec!(1000000),
        );

        assert_eq!(tick.mid_price(), dec!(1.1002));
    }
}
