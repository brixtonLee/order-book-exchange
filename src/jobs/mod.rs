/// Cron jobs and scheduled tasks module
///
/// Contains background jobs that run on a schedule:
/// - Symbol synchronization from FIX feed
/// - Tick persistence (every 5 minutes)
/// - Database cleanup tasks
/// - Metrics aggregation

pub mod symbol_sync_job;
pub mod tick_persistence_job;

pub use symbol_sync_job::SymbolSyncJob;
pub use tick_persistence_job::create_tick_persistence_job;
