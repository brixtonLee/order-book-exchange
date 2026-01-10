/// Database module for PostgreSQL and TimescaleDB integration
///
/// This module provides:
/// - Connection pooling for both PostgreSQL (metadata) and TimescaleDB (time-series)
/// - Repository pattern implementations adhering to SOLID principles
/// - Database models and schema
/// - Diesel ORM integration

pub mod connection;
pub mod enums;
pub mod models;
pub mod repositories;
pub mod schema;
pub mod tick_persister;

pub use connection::{DatabasePools, establish_connection_pools};
pub use tick_persister::TickPersister;
