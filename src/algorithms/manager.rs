use crate::algorithms::{AlgorithmStatus, TwapAlgorithm, VwapAlgorithm};
use crate::engine::OrderBookEngine;
use crate::websocket::Broadcaster;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info};
use uuid::Uuid;

/// Manages lifecycle and execution of trading algorithms
pub struct AlgorithmManager {
    twap_algos: Arc<RwLock<HashMap<Uuid, TwapAlgorithm>>>,
    vwap_algos: Arc<RwLock<HashMap<Uuid, VwapAlgorithm>>>,
    engine: Arc<OrderBookEngine>,
    broadcaster: Broadcaster,
}

impl AlgorithmManager {
    pub fn new(engine: Arc<OrderBookEngine>, broadcaster: Broadcaster) -> Self {
        Self {
            twap_algos: Arc::new(RwLock::new(HashMap::new())),
            vwap_algos: Arc::new(RwLock::new(HashMap::new())),
            engine,
            broadcaster,
        }
    }

    /// Submit a TWAP algorithm for execution
    pub fn submit_twap(&self, mut twap: TwapAlgorithm) -> Result<Uuid, String> {
        twap.start();
        let id = twap.id;
        let mut algos = self.twap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        algos.insert(id, twap);
        info!("TWAP algorithm {} submitted", id);
        Ok(id)
    }

    /// Submit a VWAP algorithm for execution
    pub fn submit_vwap(&self, mut vwap: VwapAlgorithm) -> Result<Uuid, String> {
        vwap.start();
        let id = vwap.id;
        let mut algos = self.vwap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        algos.insert(id, vwap);
        info!("VWAP algorithm {} submitted", id);
        Ok(id)
    }

    /// Pause an algorithm
    pub fn pause(&self, id: Uuid) -> Result<(), String> {
        // Try TWAP first
        {
            let mut algos = self.twap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.pause();
                return Ok(());
            }
        }

        // Try VWAP
        {
            let mut algos = self.vwap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.pause();
                return Ok(());
            }
        }

        Err(format!("Algorithm {} not found", id))
    }

    /// Resume a paused algorithm
    pub fn resume(&self, id: Uuid) -> Result<(), String> {
        // Try TWAP first
        {
            let mut algos = self.twap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.start();
                return Ok(());
            }
        }

        // Try VWAP
        {
            let mut algos = self.vwap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.start();
                return Ok(());
            }
        }

        Err(format!("Algorithm {} not found", id))
    }

    /// Cancel an algorithm
    pub fn cancel(&self, id: Uuid) -> Result<(), String> {
        // Try TWAP first
        {
            let mut algos = self.twap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.cancel();
                return Ok(());
            }
        }

        // Try VWAP
        {
            let mut algos = self.vwap_algos.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(algo) = algos.get_mut(&id) {
                algo.cancel();
                return Ok(());
            }
        }

        Err(format!("Algorithm {} not found", id))
    }

    /// Get TWAP algorithm status
    pub fn get_twap(&self, id: Uuid) -> Result<Option<TwapAlgorithm>, String> {
        let algos = self.twap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(algos.get(&id).cloned())
    }

    /// Get VWAP algorithm status
    pub fn get_vwap(&self, id: Uuid) -> Result<Option<VwapAlgorithm>, String> {
        let algos = self.vwap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(algos.get(&id).cloned())
    }

    /// Get all active TWAP algorithms
    pub fn get_all_twap(&self) -> Result<Vec<TwapAlgorithm>, String> {
        let algos = self.twap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(algos.values().cloned().collect())
    }

    /// Get all active VWAP algorithms
    pub fn get_all_vwap(&self) -> Result<Vec<VwapAlgorithm>, String> {
        let algos = self.vwap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(algos.values().cloned().collect())
    }

    /// Run the executor background task
    pub async fn run_executor(self: Arc<Self>) {
        info!("Algorithm executor starting");
        let mut tick_interval = interval(Duration::from_secs(1));

        loop {
            tick_interval.tick().await;

            let current_time = Utc::now();

            // Execute TWAP algorithms
            self.execute_twap_slice(current_time).await;

            // Execute VWAP algorithms
            self.execute_vwap_slice(current_time).await;

            // Cleanup completed/cancelled algorithms
            self.cleanup_finished_algorithms();
        }
    }

    /// Execute next slice for all running TWAP algorithms
    async fn execute_twap_slice(&self, current_time: chrono::DateTime<Utc>) {
        let mut algos = match self.twap_algos.write() {
            Ok(algos) => algos,
            Err(e) => {
                error!("Failed to acquire write lock for TWAP algorithms: {}", e);
                return;
            }
        };

        for (id, algo) in algos.iter_mut() {
            if algo.status != AlgorithmStatus::Running {
                continue;
            }

            // Generate next child order
            if let Some(child_order) = algo.next_slice(current_time) {
                let symbol = child_order.symbol.clone();
                let quantity = child_order.quantity;

                // Submit to engine
                match self.engine.add_order(child_order) {
                    Ok((order, _trades)) => {
                        // Record the fill
                        let filled = order.filled_quantity;
                        if filled > rust_decimal::Decimal::ZERO {
                            algo.record_fill(filled, order.price.unwrap_or_default());
                            info!(
                                "TWAP {} executed {} {} on {}",
                                id, filled, symbol, current_time
                            );
                        }
                    }
                    Err(e) => {
                        error!("TWAP {} child order failed: {}", id, e);
                    }
                }
            }
        }
    }

    /// Execute next slice for all running VWAP algorithms
    async fn execute_vwap_slice(&self, current_time: chrono::DateTime<Utc>) {
        let mut algos = match self.vwap_algos.write() {
            Ok(algos) => algos,
            Err(e) => {
                error!("Failed to acquire write lock for VWAP algorithms: {}", e);
                return;
            }
        };

        for (id, algo) in algos.iter_mut() {
            if algo.status != AlgorithmStatus::Running {
                continue;
            }

            // Generate next child order
            if let Some(child_order) = algo.next_slice(current_time) {
                let symbol = child_order.symbol.clone();
                let quantity = child_order.quantity;

                // Submit to engine
                match self.engine.add_order(child_order) {
                    Ok((order, _trades)) => {
                        // Record the fill
                        let filled = order.filled_quantity;
                        if filled > rust_decimal::Decimal::ZERO {
                            algo.record_fill(filled, order.price.unwrap_or_default());
                            info!(
                                "VWAP {} executed {} {} on {}",
                                id, filled, symbol, current_time
                            );
                        }
                    }
                    Err(e) => {
                        error!("VWAP {} child order failed: {}", id, e);
                    }
                }
            }
        }
    }

    /// Remove completed or cancelled algorithms to free memory
    fn cleanup_finished_algorithms(&self) {
        // Cleanup TWAP
        {
            match self.twap_algos.write() {
                Ok(mut algos) => {
                    algos.retain(|_, algo| {
                        algo.status != AlgorithmStatus::Completed
                            && algo.status != AlgorithmStatus::Cancelled
                    });
                }
                Err(e) => {
                    error!("Failed to acquire write lock for TWAP cleanup: {}", e);
                }
            }
        }

        // Cleanup VWAP
        {
            match self.vwap_algos.write() {
                Ok(mut algos) => {
                    algos.retain(|_, algo| {
                        algo.status != AlgorithmStatus::Completed
                            && algo.status != AlgorithmStatus::Cancelled
                    });
                }
                Err(e) => {
                    error!("Failed to acquire write lock for VWAP cleanup: {}", e);
                }
            }
        }
    }

    /// Get total number of active algorithms
    pub fn get_total_algorithms(&self) -> Result<usize, String> {
        let twap_count = self.twap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?.len();
        let vwap_count = self.vwap_algos.read().map_err(|e| format!("Failed to acquire read lock: {}", e))?.len();
        Ok(twap_count + vwap_count)
    }
}
