use crate::database::connection::{DatabaseError, PgPooledConnection};
use crate::database::enums::Timeframe;
use crate::database::models::OhlcCandle;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Numeric, Text, Timestamptz};
use std::sync::Arc;

/// OHLC repository trait - defines interface for querying OHLC candles
///
/// Queries TimescaleDB continuous aggregates for pre-computed candles
#[async_trait::async_trait]
pub trait OhlcRepository: Send + Sync {
    /// Get OHLC candles for a symbol and timeframe within time range
    fn get_candles(
        &self,
        symbol_id: i64,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<OhlcCandle>, DatabaseError>;

    /// Get latest candle for a symbol and timeframe
    fn get_latest_candle(
        &self,
        symbol_id: i64,
        timeframe: Timeframe,
    ) -> Result<Option<OhlcCandle>, DatabaseError>;

    /// Get latest candles for all symbols at a specific timeframe
    fn get_latest_candles_all(
        &self,
        timeframe: Timeframe,
    ) -> Result<Vec<OhlcCandle>, DatabaseError>;
}

/// Concrete implementation of OhlcRepository
///
/// Queries materialized views created by continuous aggregates
pub struct OhlcRepositoryImpl {
    get_conn: Arc<dyn Fn() -> Result<PgPooledConnection, DatabaseError> + Send + Sync>,
}

impl OhlcRepositoryImpl {
    /// Create new OHLC repository with connection provider
    pub fn new<F>(get_conn: F) -> Self
    where
        F: Fn() -> Result<PgPooledConnection, DatabaseError> + Send + Sync + 'static,
    {
        Self {
            get_conn: Arc::new(get_conn),
        }
    }

    /// Get the materialized view name for a timeframe
    fn get_view_name(timeframe: Timeframe) -> &'static str {
        match timeframe {
            Timeframe::OneMinute => "ohlc_1m",
            Timeframe::FiveMinutes => "ohlc_5m",
            Timeframe::FifteenMinutes => "ohlc_15m",
            Timeframe::ThirtyMinutes => "ohlc_30m",
            Timeframe::OneHour => "ohlc_1h",
            Timeframe::FourHours => "ohlc_4h",
            Timeframe::OneDay => "ohlc_1d",
        }
    }
}

// Diesel QueryableByName for reading from dynamic views
#[derive(QueryableByName, Debug)]
struct OhlcCandleRow {
    #[diesel(sql_type = BigInt)]
    symbol_id: i64,
    #[diesel(sql_type = Text)]
    symbol_name: String,
    #[diesel(sql_type = Text)]
    timeframe: String,
    #[diesel(sql_type = Timestamptz)]
    open_time: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    close_time: DateTime<Utc>,
    #[diesel(sql_type = Numeric)]
    open_price: rust_decimal::Decimal,
    #[diesel(sql_type = Numeric)]
    high_price: rust_decimal::Decimal,
    #[diesel(sql_type = Numeric)]
    low_price: rust_decimal::Decimal,
    #[diesel(sql_type = Numeric)]
    close_price: rust_decimal::Decimal,
    #[diesel(sql_type = Numeric)]
    volume: rust_decimal::Decimal,
    #[diesel(sql_type = BigInt)]
    tick_count: i64,
}

impl From<OhlcCandleRow> for OhlcCandle {
    fn from(row: OhlcCandleRow) -> Self {
        OhlcCandle {
            id: 0, // Continuous aggregates don't have IDs
            symbol_id: row.symbol_id,
            symbol_name: row.symbol_name,
            timeframe: Timeframe::from_str(&row.timeframe).unwrap_or(Timeframe::FiveMinutes),
            open_time: row.open_time,
            close_time: row.close_time,
            open_price: row.open_price,
            high_price: row.high_price,
            low_price: row.low_price,
            close_price: row.close_price,
            volume: row.volume,
            tick_count: row.tick_count,
            created_at: Utc::now(),
        }
    }
}

#[async_trait::async_trait]
impl OhlcRepository for OhlcRepositoryImpl {
    fn get_candles(
        &self,
        symbol_id: i64,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<OhlcCandle>, DatabaseError> {
        let mut conn = (self.get_conn)()?;
        let view_name = Self::get_view_name(timeframe);

        let limit_clause = limit
            .map(|l| format!("LIMIT {}", l))
            .unwrap_or_default();

        let query = format!(
            "SELECT symbol_id, symbol_name, timeframe, open_time, close_time, \
             open_price, high_price, low_price, close_price, volume, tick_count \
             FROM {} \
             WHERE symbol_id = $1 AND open_time >= $2 AND open_time <= $3 \
             ORDER BY open_time DESC {}",
            view_name, limit_clause
        );

        let rows = diesel::sql_query(query)
            .bind::<BigInt, _>(symbol_id)
            .bind::<Timestamptz, _>(from)
            .bind::<Timestamptz, _>(to)
            .load::<OhlcCandleRow>(&mut conn)?;

        Ok(rows.into_iter().map(OhlcCandle::from).collect())
    }

    fn get_latest_candle(
        &self,
        symbol_id: i64,
        timeframe: Timeframe,
    ) -> Result<Option<OhlcCandle>, DatabaseError> {
        let mut conn = (self.get_conn)()?;
        let view_name = Self::get_view_name(timeframe);

        let query = format!(
            "SELECT symbol_id, symbol_name, timeframe, open_time, close_time, \
             open_price, high_price, low_price, close_price, volume, tick_count \
             FROM {} \
             WHERE symbol_id = $1 \
             ORDER BY open_time DESC \
             LIMIT 1",
            view_name
        );

        let row = diesel::sql_query(query)
            .bind::<BigInt, _>(symbol_id)
            .get_result::<OhlcCandleRow>(&mut conn)
            .optional()?;

        Ok(row.map(OhlcCandle::from))
    }

    fn get_latest_candles_all(
        &self,
        timeframe: Timeframe,
    ) -> Result<Vec<OhlcCandle>, DatabaseError> {
        let mut conn = (self.get_conn)()?;
        let view_name = Self::get_view_name(timeframe);

        let query = format!(
            "SELECT DISTINCT ON (symbol_id) symbol_id, symbol_name, timeframe, open_time, close_time, \
             open_price, high_price, low_price, close_price, volume, tick_count \
             FROM {} \
             ORDER BY symbol_id, open_time DESC",
            view_name
        );

        let rows = diesel::sql_query(query).load::<OhlcCandleRow>(&mut conn)?;

        Ok(rows.into_iter().map(OhlcCandle::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_name_mapping() {
        assert_eq!(
            OhlcRepositoryImpl::get_view_name(Timeframe::OneMinute),
            "ohlc_1m"
        );
        assert_eq!(
            OhlcRepositoryImpl::get_view_name(Timeframe::FiveMinutes),
            "ohlc_5m"
        );
        assert_eq!(
            OhlcRepositoryImpl::get_view_name(Timeframe::OneDay),
            "ohlc_1d"
        );
    }
}
