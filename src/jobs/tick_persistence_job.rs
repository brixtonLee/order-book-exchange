use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio::sync::mpsc;

use crate::database::{TickQueue, repositories::TickRepository};

/// Create a tick persistence job that flushes the queue every 5 minutes
///
/// This job:
/// - Runs every 5 minutes (cron: "0 */5 * * * *")
/// - Drains all ticks from the queue
/// - Batch-inserts them to TimescaleDB
/// - Handles emergency flush triggers
///
/// The job also listens for emergency flush signals from the queue
/// (when queue becomes full)
pub async fn create_tick_persistence_job(
    queue: Arc<TickQueue>,
    repository: Arc<dyn TickRepository>,
    scheduler: &JobScheduler,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create emergency flush channel
    let (emergency_tx, mut emergency_rx) = mpsc::unbounded_channel();
    queue.set_emergency_flush_trigger(emergency_tx);

    // Clone for emergency flush task
    let queue_clone = Arc::clone(&queue);
    let repository_clone = Arc::clone(&repository);

    // Spawn emergency flush listener
    tokio::spawn(async move {
        while let Some(()) = emergency_rx.recv().await {
            tracing::warn!("🚨 Emergency flush triggered!");
            flush_queue(&queue_clone, &repository_clone).await;
        }
    });

    // Create scheduled job (every 5 minutes)
    let queue_clone = Arc::clone(&queue);
    let repository_clone = Arc::clone(&repository);

    let job = Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
        let queue = Arc::clone(&queue_clone);
        let repository = Arc::clone(&repository_clone);

        Box::pin(async move {
            tracing::debug!("🕐 Tick persistence job triggered");
            flush_queue(&queue, &repository).await;
        })
    })?;

    scheduler.add(job).await?;

    tracing::info!("✅ Tick persistence job registered");
    tracing::info!("   Schedule: Every 5 minutes");

    Ok(())
}

/// Flush queue to database
async fn flush_queue(queue: &Arc<TickQueue>, repository: &Arc<dyn TickRepository>) {
    let start = std::time::Instant::now();

    // Drain queue
    let ticks = queue.drain_all();
    let tick_count = ticks.len();

    if tick_count == 0 {
        tracing::debug!("   No ticks to persist");
        return;
    }

    // Batch insert to database (blocking operation, run in spawn_blocking)
    let repository_clone = Arc::clone(repository);
    let ticks_clone = ticks.clone();

    match tokio::task::spawn_blocking(move || {
        repository_clone.insert_batch(&ticks_clone)
    })
    .await
    {
        Ok(Ok(inserted)) => {
            let duration = start.elapsed();
            tracing::info!(
                "📥 Persisted {} ticks to database in {:.2}ms ({:.0} ticks/sec)",
                inserted,
                duration.as_secs_f64() * 1000.0,
                inserted as f64 / duration.as_secs_f64()
            );

            if inserted != tick_count {
                tracing::warn!(
                    "⚠️  Some ticks were not inserted (duplicates): {} attempted, {} inserted",
                    tick_count,
                    inserted
                );
            }
        }
        Ok(Err(e)) => {
            tracing::error!("❌ Failed to persist ticks to database: {}", e);
            tracing::error!("   {} ticks lost", tick_count);
        }
        Err(e) => {
            tracing::error!("❌ Failed to spawn blocking task: {}", e);
            tracing::error!("   {} ticks lost", tick_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::NewTick;
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;

    struct MockRepository {
        pub insert_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl TickRepository for MockRepository {
        async fn insert_batch(&self, ticks: &[NewTick]) -> Result<usize, String> {
            let count = ticks.len();
            self.insert_count.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
            Ok(count)
        }

        async fn get_by_symbol_and_time_range(
            &self,
            _symbol_id: i64,
            _from: chrono::DateTime<Utc>,
            _to: chrono::DateTime<Utc>,
            _limit: i64,
        ) -> Result<Vec<crate::database::models::Tick>, String> {
            unimplemented!()
        }

        async fn get_latest(&self, _symbol_id: i64) -> Result<Option<crate::database::models::Tick>, String> {
            unimplemented!()
        }

        async fn get_latest_all(&self) -> Result<Vec<crate::database::models::Tick>, String> {
            unimplemented!()
        }

        async fn delete_before(&self, _before: chrono::DateTime<Utc>) -> Result<usize, String> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_flush_queue() {
        let queue = Arc::new(TickQueue::new(100));
        let insert_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let repository = Arc::new(MockRepository {
            insert_count: Arc::clone(&insert_count),
        }) as Arc<dyn TickRepository>;

        // Add some ticks to queue
        for i in 0..10 {
            let tick = crate::ctrader_fix::market_data::MarketTick {
                symbol_id: i.to_string(),
                bid_price: Some(Decimal::new(100, 0)),
                ask_price: Some(Decimal::new(101, 0)),
                timestamp: Utc::now(),
            };
            queue.enqueue(tick);
        }

        // Flush
        flush_queue(&queue, &repository).await;

        // Verify
        assert_eq!(insert_count.load(std::sync::atomic::Ordering::Relaxed), 10);
        assert_eq!(queue.stats().current_size, 0);
        assert_eq!(queue.stats().total_flushed, 10);
    }
}
