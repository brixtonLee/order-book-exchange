use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Symbol entity - represents a trading instrument
///
/// Stored in regular PostgreSQL database (not TimescaleDB)
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::symbols)]
#[diesel(primary_key(symbol_id))]
pub struct Symbol {
    /// Unique symbol ID from cTrader FIX
    pub symbol_id: i64,

    /// Human-readable symbol name (e.g., "EURUSD", "XAUUSD")
    pub symbol_name: String,

    /// Symbol description (optional)
    pub description: Option<String>,

    /// Number of decimal places for price precision
    pub digits: i32,

    /// Minimum price increment (tick size)
    pub tick_size: Decimal,

    /// Contract size (optional, for CFDs/futures)
    pub contract_size: Option<Decimal>,

    /// Timestamp when record was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when record was last updated
    pub updated_at: DateTime<Utc>,

    /// Timestamp of last successful sync from FIX
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// New symbol for insertion
#[derive(Debug, Clone, Insertable, AsChangeset, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::database::schema::symbols)]
pub struct NewSymbol {
    pub symbol_id: i64,
    pub symbol_name: String,
    pub description: Option<String>,
    pub digits: i32,
    pub tick_size: Decimal,
    pub contract_size: Option<Decimal>,
}

impl NewSymbol {
    /// Create a new symbol builder
    pub fn new(symbol_id: i64, symbol_name: String, digits: i32, tick_size: Decimal) -> Self {
        Self {
            symbol_id,
            symbol_name,
            description: None,
            digits,
            tick_size,
            contract_size: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, contract_size: Decimal) -> Self {
        self.contract_size = Some(contract_size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_symbol_builder() {
        let symbol = NewSymbol::new(1, "EURUSD".to_string(), 5, dec!(0.00001))
            .with_description("Euro vs US Dollar".to_string())
            .with_contract_size(dec!(100000));

        assert_eq!(symbol.symbol_id, 1);
        assert_eq!(symbol.symbol_name, "EURUSD");
        assert_eq!(symbol.digits, 5);
        assert_eq!(symbol.tick_size, dec!(0.00001));
        assert_eq!(symbol.description, Some("Euro vs US Dollar".to_string()));
        assert_eq!(symbol.contract_size, Some(dec!(100000)));
    }
}
