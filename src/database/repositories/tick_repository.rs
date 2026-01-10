use crate::database::connection::{DatabaseError, PgPooledConnection};
use crate::database::models::{NewTick, Tick};
use crate::database::schema::ticks;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::sync::Arc;

/// Tick repository trait - defines interface for tick operations
///
/// Focused on high-throughput batch inserts and time-range queries
#[async_trait::async_trait]
pub trait TickRepository: Send + Sync {
    /// Insert a single tick
    fn insert(&self, new_tick: NewTick) -> Result<Tick, DatabaseError>;

    /// Batch insert ticks (optimized for high throughput)
    fn insert_batch(&self, new_ticks: Vec<NewTick>) -> Result<usize, DatabaseError>;

    /// Get ticks for a symbol within time range
    fn get_by_symbol_and_time_range(
        &self,
        symbol_id: i64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>, DatabaseError>;

    /// Get latest tick for a symbol
    fn get_latest(&self, symbol_id: i64) -> Result<Option<Tick>, DatabaseError>;

    /// Get latest ticks for all symbols
    fn get_latest_all(&self) -> Result<Vec<Tick>, DatabaseError>;

    /// Count ticks for a symbol
    fn count_by_symbol(&self, symbol_id: i64) -> Result<i64, DatabaseError>;

    /// Delete ticks older than specified date (for manual cleanup)
    fn delete_before(&self, before: DateTime<Utc>) -> Result<usize, DatabaseError>;
}

/// Concrete implementation of TickRepository
pub struct TickRepositoryImpl {
    get_conn: Arc<dyn Fn() -> Result<PgPooledConnection, DatabaseError> + Send + Sync>,
}

impl TickRepositoryImpl {
    /// Create new tick repository with connection provider
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
impl TickRepository for TickRepositoryImpl {
    fn insert(&self, new_tick: NewTick) -> Result<Tick, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        diesel::insert_into(ticks::table)
            .values(&new_tick)
            .on_conflict_do_nothing() // Ignore duplicate ticks (based on unique constraint)
            .get_result::<Tick>(&mut conn)
            .map_err(DatabaseError::from)
    }

    fn insert_batch(&self, new_ticks: Vec<NewTick>) -> Result<usize, DatabaseError> {
        if new_ticks.is_empty() {
            return Ok(0);
        }

        let mut conn = (self.get_conn)()?;

        // Use batch insert with ON CONFLICT DO NOTHING for high throughput
        // This prevents errors from duplicate ticks
        let inserted = diesel::insert_into(ticks::table)
            .values(&new_ticks)
            .on_conflict_do_nothing()
            .execute(&mut conn)?;

        tracing::debug!(
            "Batch inserted {} ticks (attempted {})",
            inserted,
            new_ticks.len()
        );

        Ok(inserted)
    }

    fn get_by_symbol_and_time_range(
        &self,
        symbol_id: i64,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        let mut query = ticks::table
            .filter(ticks::symbol_id.eq(symbol_id))
            .filter(ticks::tick_time.ge(from))
            .filter(ticks::tick_time.le(to))
            .order(ticks::tick_time.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<Tick>(&mut conn).map_err(DatabaseError::from)
    }

    fn get_latest(&self, symbol_id: i64) -> Result<Option<Tick>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        ticks::table
            .filter(ticks::symbol_id.eq(symbol_id))
            .order(ticks::tick_time.desc())
            .first::<Tick>(&mut conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    fn get_latest_all(&self) -> Result<Vec<Tick>, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        // Use DISTINCT ON to get latest tick per symbol
        // This is PostgreSQL-specific syntax
        diesel::sql_query(
            "SELECT DISTINCT ON (symbol_id) * FROM ticks ORDER BY symbol_id, tick_time DESC",
        )
        .load::<Tick>(&mut conn)
        .map_err(DatabaseError::from)
    }

    fn count_by_symbol(&self, symbol_id: i64) -> Result<i64, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        ticks::table
            .filter(ticks::symbol_id.eq(symbol_id))
            .count()
            .get_result::<i64>(&mut conn)
            .map_err(DatabaseError::from)
    }

    fn delete_before(&self, before: DateTime<Utc>) -> Result<usize, DatabaseError> {
        let mut conn = (self.get_conn)()?;

        let deleted = diesel::delete(ticks::table)
            .filter(ticks::tick_time.lt(before))
            .execute(&mut conn)?;

        tracing::info!("Deleted {} ticks before {}", deleted, before);

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_tick_repository() {
        // Tests require actual database connection - skip in CI
    }
}
