use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// High-precision latency tracker using HDR Histogram
/// HDR Histograms provide accurate percentile calculations with minimal memory
pub struct LatencyTracker {
    /// Matching engine latency (most critical metric)
    matching_latency_ns: Histogram<u64>,

    /// Lock acquisition wait time
    lock_wait_ns: Histogram<u64>,

    /// End-to-end order processing
    total_latency_ns: Histogram<u64>,

    /// WebSocket broadcast latency
    broadcast_latency_ns: Histogram<u64>,

    /// Counter for total measurements
    sample_count: AtomicU64,
}

impl LatencyTracker {
    pub fn new() -> Self {
        // Configure histogram: 1ns to 10 seconds, 3 significant figures
        Self {
            matching_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            lock_wait_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            total_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            broadcast_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            sample_count: AtomicU64::new(0),
        }
    }

    /// Record matching engine latency
    #[inline]
    pub fn record_matching(&mut self, start: Instant) {
        let nanos = start.elapsed().as_nanos() as u64;
        let _ = self.matching_latency_ns.record(nanos);
        self.sample_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record lock wait latency
    #[inline]
    pub fn record_lock_wait(&mut self, start: Instant) {
        let nanos = start.elapsed().as_nanos() as u64;
        let _ = self.lock_wait_ns.record(nanos);
    }

    /// Record total latency
    #[inline]
    pub fn record_total(&mut self, start: Instant) {
        let nanos = start.elapsed().as_nanos() as u64;
        let _ = self.total_latency_ns.record(nanos);
    }

    /// Record broadcast latency
    #[inline]
    pub fn record_broadcast(&mut self, start: Instant) {
        let nanos = start.elapsed().as_nanos() as u64;
        let _ = self.broadcast_latency_ns.record(nanos);
    }

    /// Get latency statistics for matching engine
    pub fn matching_stats(&self) -> LatencyStats {
        LatencyStats {
            metric_name: "matching".to_string(),
            p50_ns: self.matching_latency_ns.value_at_percentile(50.0),
            p95_ns: self.matching_latency_ns.value_at_percentile(95.0),
            p99_ns: self.matching_latency_ns.value_at_percentile(99.0),
            p999_ns: self.matching_latency_ns.value_at_percentile(99.9),
            max_ns: self.matching_latency_ns.max(),
            min_ns: self.matching_latency_ns.min(),
            mean_ns: self.matching_latency_ns.mean(),
            sample_count: self.sample_count.load(Ordering::Relaxed),
        }
    }

    /// Get latency statistics for lock wait
    pub fn lock_wait_stats(&self) -> LatencyStats {
        LatencyStats {
            metric_name: "lock_wait".to_string(),
            p50_ns: self.lock_wait_ns.value_at_percentile(50.0),
            p95_ns: self.lock_wait_ns.value_at_percentile(95.0),
            p99_ns: self.lock_wait_ns.value_at_percentile(99.0),
            p999_ns: self.lock_wait_ns.value_at_percentile(99.9),
            max_ns: self.lock_wait_ns.max(),
            min_ns: self.lock_wait_ns.min(),
            mean_ns: self.lock_wait_ns.mean(),
            sample_count: self.lock_wait_ns.len(),
        }
    }

    /// Get latency statistics for total processing
    pub fn total_stats(&self) -> LatencyStats {
        LatencyStats {
            metric_name: "total".to_string(),
            p50_ns: self.total_latency_ns.value_at_percentile(50.0),
            p95_ns: self.total_latency_ns.value_at_percentile(95.0),
            p99_ns: self.total_latency_ns.value_at_percentile(99.0),
            p999_ns: self.total_latency_ns.value_at_percentile(99.9),
            max_ns: self.total_latency_ns.max(),
            min_ns: self.total_latency_ns.min(),
            mean_ns: self.total_latency_ns.mean(),
            sample_count: self.total_latency_ns.len(),
        }
    }

    /// Get latency statistics for broadcast
    pub fn broadcast_stats(&self) -> LatencyStats {
        LatencyStats {
            metric_name: "broadcast".to_string(),
            p50_ns: self.broadcast_latency_ns.value_at_percentile(50.0),
            p95_ns: self.broadcast_latency_ns.value_at_percentile(95.0),
            p99_ns: self.broadcast_latency_ns.value_at_percentile(99.0),
            p999_ns: self.broadcast_latency_ns.value_at_percentile(99.9),
            max_ns: self.broadcast_latency_ns.max(),
            min_ns: self.broadcast_latency_ns.min(),
            mean_ns: self.broadcast_latency_ns.mean(),
            sample_count: self.broadcast_latency_ns.len(),
        }
    }

    /// Get all latency statistics
    pub fn all_stats(&self) -> Vec<LatencyStats> {
        vec![
            self.matching_stats(),
            self.lock_wait_stats(),
            self.total_stats(),
            self.broadcast_stats(),
        ]
    }

    /// Reset all histograms
    pub fn reset(&mut self) {
        self.matching_latency_ns.clear();
        self.lock_wait_ns.clear();
        self.total_latency_ns.clear();
        self.broadcast_latency_ns.clear();
        self.sample_count.store(0, Ordering::Relaxed);
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub metric_name: String,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
    pub max_ns: u64,
    pub min_ns: u64,
    pub mean_ns: f64,
    pub sample_count: u64,
}

impl LatencyStats {
    /// Convert nanoseconds to microseconds for display
    pub fn p50_us(&self) -> f64 {
        self.p50_ns as f64 / 1000.0
    }

    pub fn p95_us(&self) -> f64 {
        self.p95_ns as f64 / 1000.0
    }

    pub fn p99_us(&self) -> f64 {
        self.p99_ns as f64 / 1000.0
    }

    pub fn p999_us(&self) -> f64 {
        self.p999_ns as f64 / 1000.0
    }

    pub fn max_us(&self) -> f64 {
        self.max_ns as f64 / 1000.0
    }

    pub fn min_us(&self) -> f64 {
        self.min_ns as f64 / 1000.0
    }

    pub fn mean_us(&self) -> f64 {
        self.mean_ns / 1000.0
    }
}

/// RAII guard for automatic latency measurement
pub struct LatencyGuard<'a> {
    tracker: &'a mut LatencyTracker,
    start: Instant,
    metric_type: MetricType,
}

#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Matching,
    LockWait,
    Total,
    Broadcast,
}

impl<'a> LatencyGuard<'a> {
    pub fn new(tracker: &'a mut LatencyTracker, metric_type: MetricType) -> Self {
        Self {
            tracker,
            start: Instant::now(),
            metric_type,
        }
    }
}

impl<'a> Drop for LatencyGuard<'a> {
    fn drop(&mut self) {
        let nanos = self.start.elapsed().as_nanos() as u64;
        match self.metric_type {
            MetricType::Matching => {
                let _ = self.tracker.matching_latency_ns.record(nanos);
            }
            MetricType::LockWait => {
                let _ = self.tracker.lock_wait_ns.record(nanos);
            }
            MetricType::Total => {
                let _ = self.tracker.total_latency_ns.record(nanos);
            }
            MetricType::Broadcast => {
                let _ = self.tracker.broadcast_latency_ns.record(nanos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_latency_tracker_basic() {
        let mut tracker = LatencyTracker::new();

        let start = Instant::now();
        thread::sleep(Duration::from_micros(100));
        tracker.record_matching(start);

        let stats = tracker.matching_stats();
        assert!(stats.p50_ns > 0);
        assert_eq!(stats.sample_count, 1);
    }

    #[test]
    fn test_latency_guard() {
        let mut tracker = LatencyTracker::new();

        {
            let _guard = LatencyGuard::new(&mut tracker, MetricType::Matching);
            thread::sleep(Duration::from_micros(50));
        } // Guard drops here and records

        let stats = tracker.matching_stats();
        assert!(stats.p50_ns > 0);
    }

    #[test]
    fn test_multiple_metrics() {
        let mut tracker = LatencyTracker::new();

        // Record some measurements
        for _ in 0..10 {
            let start = Instant::now();
            thread::sleep(Duration::from_micros(10));
            tracker.record_matching(start);

            let start = Instant::now();
            thread::sleep(Duration::from_micros(5));
            tracker.record_lock_wait(start);
        }

        let matching_stats = tracker.matching_stats();
        let lock_wait_stats = tracker.lock_wait_stats();

        assert_eq!(matching_stats.sample_count, 10);
        assert!(lock_wait_stats.sample_count > 0);
    }

    #[test]
    fn test_reset() {
        let mut tracker = LatencyTracker::new();

        let start = Instant::now();
        tracker.record_matching(start);

        assert_eq!(tracker.matching_stats().sample_count, 1);

        tracker.reset();

        assert_eq!(tracker.matching_stats().sample_count, 0);
    }
}
