use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;
use utoipa::ToSchema;

/// OHLC candle timeframe enumeration
///
/// Represents the time interval for aggregating tick data into OHLC candles.
/// Used in TimescaleDB continuous aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub enum Timeframe {
    #[serde(rename = "1m")]
    OneMinute,

    #[serde(rename = "5m")]
    FiveMinutes,

    #[serde(rename = "15m")]
    FifteenMinutes,

    #[serde(rename = "30m")]
    ThirtyMinutes,

    #[serde(rename = "1h")]
    OneHour,

    #[serde(rename = "4h")]
    FourHours,

    #[serde(rename = "1d")]
    OneDay,
}

impl Timeframe {
    /// Convert enum to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::OneMinute => "1m",
            Timeframe::FiveMinutes => "5m",
            Timeframe::FifteenMinutes => "15m",
            Timeframe::ThirtyMinutes => "30m",
            Timeframe::OneHour => "1h",
            Timeframe::FourHours => "4h",
            Timeframe::OneDay => "1d",
        }
    }

    /// Parse string to Timeframe enum
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "1m" => Some(Timeframe::OneMinute),
            "5m" => Some(Timeframe::FiveMinutes),
            "15m" => Some(Timeframe::FifteenMinutes),
            "30m" => Some(Timeframe::ThirtyMinutes),
            "1h" => Some(Timeframe::OneHour),
            "4h" => Some(Timeframe::FourHours),
            "1d" => Some(Timeframe::OneDay),
            _ => None,
        }
    }

    /// Get all timeframe variants
    pub fn all() -> Vec<Self> {
        vec![
            Timeframe::OneMinute,
            Timeframe::FiveMinutes,
            Timeframe::FifteenMinutes,
            Timeframe::ThirtyMinutes,
            Timeframe::OneHour,
            Timeframe::FourHours,
            Timeframe::OneDay,
        ]
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        match self {
            Timeframe::OneMinute => 60,
            Timeframe::FiveMinutes => 300,
            Timeframe::FifteenMinutes => 900,
            Timeframe::ThirtyMinutes => 1800,
            Timeframe::OneHour => 3600,
            Timeframe::FourHours => 14400,
            Timeframe::OneDay => 86400,
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Diesel ToSql implementation - convert Rust enum to SQL TEXT
impl ToSql<Text, Pg> for Timeframe {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(serialize::IsNull::No)
    }
}

// Diesel FromSql implementation - convert SQL TEXT to Rust enum
impl FromSql<Text, Pg> for Timeframe {
    fn from_sql(bytes: <Pg as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Timeframe::from_str(&text)
            .ok_or_else(|| format!("Invalid timeframe value: {}", text).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_as_str() {
        assert_eq!(Timeframe::OneMinute.as_str(), "1m");
        assert_eq!(Timeframe::FiveMinutes.as_str(), "5m");
        assert_eq!(Timeframe::OneHour.as_str(), "1h");
        assert_eq!(Timeframe::OneDay.as_str(), "1d");
    }

    #[test]
    fn test_timeframe_from_str() {
        assert_eq!(Timeframe::from_str("1m"), Some(Timeframe::OneMinute));
        assert_eq!(Timeframe::from_str("5m"), Some(Timeframe::FiveMinutes));
        assert_eq!(Timeframe::from_str("1h"), Some(Timeframe::OneHour));
        assert_eq!(Timeframe::from_str("invalid"), None);
    }

    #[test]
    fn test_timeframe_duration() {
        assert_eq!(Timeframe::OneMinute.duration_seconds(), 60);
        assert_eq!(Timeframe::FiveMinutes.duration_seconds(), 300);
        assert_eq!(Timeframe::OneHour.duration_seconds(), 3600);
        assert_eq!(Timeframe::OneDay.duration_seconds(), 86400);
    }

    #[test]
    fn test_timeframe_all() {
        let all = Timeframe::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&Timeframe::OneMinute));
        assert!(all.contains(&Timeframe::OneDay));
    }
}
