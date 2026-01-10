use crate::database::connection::{DatabaseError, PgPooledConnection};
use crate::database::models::{NewSymbol, Symbol};
use crate::database::schema::symbols;
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;

/// Symbol repository trait - defines interface for symbol operations
///
/// Adheres to Interface Segregation Principle: focused on symbol-specific operations
#[async_trait::async_trait]
pub trait SymbolRepository: Send + Sync {
    /// Find symbol by ID
    fn find_by_id(&self, symbol_id: i64) -> Result<Option<Symbol>, DatabaseError>;

    /// Find symbol by name
    fn find_by_name(&self, symbol_name: &str) -> Result<Option<Symbol>, DatabaseError>;

    /// Get all symbols
    fn get_all(&self) -> Result<Vec<Symbol>, DatabaseError>;

    /// Insert a new symbol
    fn insert(&self, new_symbol: NewSymbol) -> Result<Symbol, DatabaseError>;

    /// Upsert (insert or update) a symbol
    /// Returns true if inserted, false if updated
    fn upsert(&self, new_symbol: NewSymbol) -> Result<(Symbol, bool), DatabaseError>;

    /// Batch upsert symbols (for sync job)
    fn upsert_batch(&self, new_symbols: Vec<NewSymbol>) -> Result<usize, DatabaseError>;

    /// Update last_synced_at timestamp
    fn update_sync_timestamp(&self, symbol_id: i64) -> Result<(), DatabaseError>;

    /// Get symbols not synced in last N hours (for monitoring stale data)
    fn get_stale_symbols(&self, hours: i64) -> Result<Vec<Symbol>, DatabaseError>;

    /// Delete symbol by ID
    fn delete(&self, symbol_id: i64) -> Result<bool, DatabaseError>;
}

/// Concrete implementation of SymbolRepository
///
/// Uses PostgreSQL connection pool from DatabasePools
pub struct SymbolRepositoryImpl {
    // Note: We store a function that provides connections to allow flexibility
    // This adheres to Dependency Inversion Principle
    get_conn: Arc<dyn Fn() -> Result<PgPooledConnection, DatabaseError> + Send + Sync>,
}

impl SymbolRepositoryImpl {
    /// Create new symbol repository with connection provider
    pub fn new<F>(get_conn: F) -> Self
    where
        F: Fn() -> Result<PgPooledConnection, DatabaseError> + Send + Sync + 'static,
    {
        Self {
            get_conn: Arc::new(get_conn),
        }
    }
}

#[async_trait::async_trait]
impl SymbolRepository for SymbolRepositoryImpl {
    fn find_by_id(&self, symbol_id: i64) -> Result<Option<Symbol>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        symbols::table
            .filter(symbols::symbol_id.eq(symbol_id))
            .first::<Symbol>(&mut conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    fn find_by_name(&self, symbol_name: &str) -> Result<Option<Symbol>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        symbols::table
            .filter(symbols::symbol_name.eq(symbol_name))
            .first::<Symbol>(&mut conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    fn get_all(&self) -> Result<Vec<Symbol>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        symbols::table
            .order(symbols::symbol_name.asc())
            .load::<Symbol>(&mut conn)
            .map_err(DatabaseError::from)
    }

    fn insert(&self, new_symbol: NewSymbol) -> Result<Symbol, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        diesel::insert_into(symbols::table)
            .values(&new_symbol)
            .get_result::<Symbol>(&mut conn)
            .map_err(DatabaseError::from)
    }

    fn upsert(&self, new_symbol: NewSymbol) -> Result<(Symbol, bool), DatabaseError> {
        let mut conn = (self.get_conn)()?;

        // Try to find existing symbol
        let existing = symbols::table
            .filter(symbols::symbol_id.eq(new_symbol.symbol_id))
            .first::<Symbol>(&mut conn)
            .optional()?;

        match existing {
            Some(_) => {
                // Update existing symbol
                let updated = diesel::update(symbols::table)
                    .filter(symbols::symbol_id.eq(new_symbol.symbol_id))
                    .set((
                        &new_symbol,
                        symbols::updated_at.eq(Utc::now()),
                        symbols::last_synced_at.eq(Some(Utc::now())),
                    ))
                    .get_result::<Symbol>(&mut conn)?;

                Ok((updated, false))
            }
            None => {
                // Insert new symbol
                let inserted = diesel::insert_into(symbols::table)
                    .values(&new_symbol)
                    .get_result::<Symbol>(&mut conn)?;

                Ok((inserted, true))
            }
        }
    }

    fn upsert_batch(&self, new_symbols: Vec<NewSymbol>) -> Result<usize, DatabaseError> {
        let mut conn = (self.get_conn)()?;
        let mut count = 0;

        // Use transaction for atomicity
        conn.transaction::<_, DatabaseError, _>(|conn| {
            for new_symbol in new_symbols {
                diesel::insert_into(symbols::table)
                    .values(&new_symbol)
                    .on_conflict(symbols::symbol_id)
                    .do_update()
                    .set((
                        symbols::symbol_name.eq(&new_symbol.symbol_name),
                        symbols::description.eq(&new_symbol.description),
                        symbols::digits.eq(new_symbol.digits),
                        symbols::tick_size.eq(new_symbol.tick_size),
                        symbols::contract_size.eq(&new_symbol.contract_size),
                        symbols::updated_at.eq(Utc::now()),
                        symbols::last_synced_at.eq(Some(Utc::now())),
                    ))
                    .execute(conn)?;

                count += 1;
            }

            Ok(count)
        })
    }

    fn update_sync_timestamp(&self, symbol_id: i64) -> Result<(), DatabaseError> {
        let mut conn = (self.get_conn)()?;

        diesel::update(symbols::table)
            .filter(symbols::symbol_id.eq(symbol_id))
            .set(symbols::last_synced_at.eq(Some(Utc::now())))
            .execute(&mut conn)?;

        Ok(())
    }

    fn get_stale_symbols(&self, hours: i64) -> Result<Vec<Symbol>, DatabaseError> {
        let mut conn = (self.get_conn)()?;
        let threshold = Utc::now() - chrono::Duration::hours(hours);

        symbols::table
            .filter(
                symbols::last_synced_at
                    .is_null()
                    .or(symbols::last_synced_at.lt(Some(threshold))),
            )
            .order(symbols::last_synced_at.asc().nulls_first())
            .load::<Symbol>(&mut conn)
            .map_err(DatabaseError::from)
    }

    fn delete(&self, symbol_id: i64) -> Result<bool, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        let deleted = diesel::delete(symbols::table)
            .filter(symbols::symbol_id.eq(symbol_id))
            .execute(&mut conn)?;

        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Tests require actual database connection - skip in CI
    #[test]
    #[ignore]
    fn test_symbol_repository() {
        // This would test the repository with a real database connection
        // Implementation depends on your test database setup
    }
}
