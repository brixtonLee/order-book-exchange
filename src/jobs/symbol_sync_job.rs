use crate::database::models::NewSymbol;
use crate::database::repositories::SymbolRepository;
use crate::datasource::DatasourceManager;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Symbol synchronization job
///
/// Runs every 5 minutes to sync symbols from cTrader FIX to PostgreSQL
pub struct SymbolSyncJob {
    symbol_repository: Arc<dyn SymbolRepository>,
    datasource_manager: Option<Arc<DatasourceManager>>,
}

impl SymbolSyncJob {
    /// Create a new symbol sync job
    pub fn new(
        symbol_repository: Arc<dyn SymbolRepository>,
        datasource_manager: Option<Arc<DatasourceManager>>,
    ) -> Self {
        Self {
            symbol_repository,
            datasource_manager,
        }
    }

    /// Perform symbol synchronization
    ///
    /// Fetches symbols from FIX datasource and upserts to database
    async fn sync_symbols(&self) -> Result<usize, Box<dyn std::error::Error>> {
        tracing::info!("Starting symbol synchronization job");

        // Get symbols from datasource
        let symbols = match &self.datasource_manager {
            Some(manager) => {
                // Get symbols from DatasourceManager
                // For now, we'll use the symbol mapping that's already in the manager
                let symbol_map = manager.get_symbol_map().await;

                symbol_map
                    .iter()
                    .map(|(symbol_id, symbol_name)| {
                        NewSymbol::new(
                            *symbol_id,
                            symbol_name.clone(),
                            5, // Default digits for FX pairs
                            dec!(0.00001), // Default tick size
                        )
                    })
                    .collect::<Vec<_>>()
            }
            None => {
                tracing::warn!("No datasource manager available, skipping symbol sync");
                return Ok(0);
            }
        };

        if symbols.is_empty() {
            tracing::info!("No symbols to sync");
            return Ok(0);
        }

        // Batch upsert symbols to database
        let count = self.symbol_repository.upsert_batch(symbols)?;

        tracing::info!("Symbol synchronization completed: {} symbols synced", count);

        Ok(count)
    }

    /// Register this job with the scheduler
    ///
    /// Schedule: Every 5 minutes (0 */5 * * * *)
    pub async fn register(
        self,
        scheduler: &JobScheduler,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let symbol_repo = self.symbol_repository.clone();
        let datasource_mgr = self.datasource_manager.clone();

        let job = Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
            let symbol_repo = symbol_repo.clone();
            let datasource_mgr = datasource_mgr.clone();

            Box::pin(async move {
                let job = SymbolSyncJob {
                    symbol_repository: symbol_repo,
                    datasource_manager: datasource_mgr,
                };

                if let Err(e) = job.sync_symbols().await {
                    tracing::error!("Symbol sync job failed: {}", e);
                } else {
                    tracing::debug!("Symbol sync job completed successfully");
                }
            })
        })?;

        scheduler.add(job).await?;

        tracing::info!("Symbol sync job registered (runs every 5 minutes)");

        Ok(())
    }

    /// Run symbol sync immediately (manual trigger)
    pub async fn run_now(&self) -> Result<usize, Box<dyn std::error::Error>> {
        self.sync_symbols().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_sync_job_creation() {
        // This test would require mocking the repository
        // Skipped for now
    }
}
