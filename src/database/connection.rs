use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager, Pool, PooledConnection};
use std::sync::Arc;
use thiserror::Error;

/// Type alias for PostgreSQL connection pool
pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Type alias for pooled connection
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Database pools container holding both PostgreSQL and TimescaleDB pools
#[derive(Clone)]
pub struct DatabasePools {
    /// Connection pool for regular PostgreSQL (symbols, metadata)
    pub metadata_pool: Arc<PgPool>,

    /// Connection pool for TimescaleDB (ticks, OHLC candles)
    pub timeseries_pool: Arc<PgPool>,
}

impl DatabasePools {
    /// Create new database pools from existing pool instances
    pub fn new(metadata_pool: PgPool, timeseries_pool: PgPool) -> Self {
        Self {
            metadata_pool: Arc::new(metadata_pool),
            timeseries_pool: Arc::new(timeseries_pool),
        }
    }

    /// Get a connection from the metadata pool
    pub fn get_metadata_conn(&self) -> Result<PgPooledConnection, DatabaseError> {
        self.metadata_pool
            .get()
            .map_err(|e| DatabaseError::ConnectionPoolError(e.to_string()))
    }

    /// Get a connection from the timeseries pool
    pub fn get_timeseries_conn(&self) -> Result<PgPooledConnection, DatabaseError> {
        self.timeseries_pool
            .get()
            .map_err(|e| DatabaseError::ConnectionPoolError(e.to_string()))
    }
}

/// Database-related errors
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection pool error: {0}")]
    ConnectionPoolError(String),

    #[error("Database query error: {0}")]
    QueryError(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
}

/// Establish connection pools for both databases
///
/// # Arguments
/// * `metadata_url` - PostgreSQL connection URL for metadata database
/// * `timeseries_url` - TimescaleDB connection URL for timeseries database
/// * `pool_size` - Maximum number of connections per pool
///
/// # Returns
/// * `Result<DatabasePools, DatabaseError>` - Database pools or error
pub fn establish_connection_pools(
    metadata_url: &str,
    timeseries_url: &str,
    pool_size: u32,
) -> Result<DatabasePools, DatabaseError> {
    tracing::info!("Establishing database connection pools...");

    // Create metadata database pool
    let metadata_manager = ConnectionManager::<PgConnection>::new(metadata_url);
    let metadata_pool = r2d2::Pool::builder()
        .max_size(pool_size)
        .build(metadata_manager)
        .map_err(|e| DatabaseError::ConnectionPoolError(format!("Metadata pool: {}", e)))?;

    tracing::info!("Metadata database pool created with max size: {}", pool_size);

    // Test metadata connection
    let _ = metadata_pool
        .get()
        .map_err(|e| DatabaseError::ConnectionFailed(format!("Metadata database: {}", e)))?;

    tracing::info!("Metadata database connection successful");

    // Create timeseries database pool
    let timeseries_manager = ConnectionManager::<PgConnection>::new(timeseries_url);
    let timeseries_pool = r2d2::Pool::builder()
        .max_size(pool_size)
        .build(timeseries_manager)
        .map_err(|e| DatabaseError::ConnectionPoolError(format!("Timeseries pool: {}", e)))?;

    tracing::info!("Timeseries database pool created with max size: {}", pool_size);

    // Test timeseries connection
    let _ = timeseries_pool
        .get()
        .map_err(|e| DatabaseError::ConnectionFailed(format!("Timeseries database: {}", e)))?;

    tracing::info!("Timeseries database connection successful");

    Ok(DatabasePools::new(metadata_pool, timeseries_pool))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_pools_creation() {
        // This test requires actual database connections
        // Skip in CI environments without databases
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let metadata_url = std::env::var("DATABASE_URL").unwrap();
        let timeseries_url = std::env::var("TIMESCALEDB_URL").unwrap();

        let result = establish_connection_pools(&metadata_url, &timeseries_url, 5);
        assert!(result.is_ok(), "Failed to create database pools");
    }
}
