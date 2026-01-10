use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::{Order, OrderSide, OrderType, OrderStatus, TimeInForce};
use crate::models::order::SelfTradePreventionMode;

/// TWAP execution algorithm
/// Divides order evenly across time intervals
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwapAlgorithm {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,

    /// Total quantity to execute
    pub total_quantity: Decimal,

    /// Quantity already executed
    pub executed_quantity: Decimal,

    /// Start time of execution window
    pub start_time: DateTime<Utc>,

    /// End time of execution window
    pub end_time: DateTime<Utc>,

    /// Interval between slices (seconds)
    pub slice_interval_seconds: i64,

    /// Number of slices completed
    pub slices_completed: u32,

    /// Limit price (None = market orders)
    pub limit_price: Option<Decimal>,

    /// Max participation rate (% of market volume)
    pub max_participation: Option<Decimal>,

    /// Algorithm status
    pub status: AlgorithmStatus,

    /// Urgency factor: 1.0 = normal, >1 = front-load, <1 = back-load
    pub urgency: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlgorithmStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Cancelled,
}

impl TwapAlgorithm {
    pub fn new(
        symbol: String,
        side: OrderSide,
        user_id: String,
        total_quantity: Decimal,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        slice_interval_seconds: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            user_id,
            total_quantity,
            executed_quantity: Decimal::ZERO,
            start_time,
            end_time,
            slice_interval_seconds,
            slices_completed: 0,
            limit_price: None,
            max_participation: None,
            status: AlgorithmStatus::Pending,
            urgency: Decimal::ONE,
        }
    }

    /// Calculate the next child order to submit
    pub fn next_slice(&mut self, current_time: DateTime<Utc>) -> Option<Order> {
        if self.status != AlgorithmStatus::Running {
            return None;
        }

        if current_time >= self.end_time {
            self.status = AlgorithmStatus::Completed;
            return None;
        }

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
            return None;
        }

        // Calculate target execution based on elapsed time
        let total_duration = (self.end_time - self.start_time).num_milliseconds() as f64;
        let elapsed = (current_time - self.start_time).num_milliseconds() as f64;
        let progress = (elapsed / total_duration).clamp(0.0, 1.0);

        // Apply urgency factor to the curve
        let adjusted_progress = progress.powf(1.0 / self.urgency.to_f64().unwrap_or(1.0));

        let target_quantity = self.total_quantity
            * Decimal::from_f64(adjusted_progress).unwrap_or(Decimal::ONE);

        // Calculate this slice's quantity
        let behind_by = target_quantity - self.executed_quantity;

        if behind_by <= Decimal::ZERO {
            return None; // Ahead of schedule
        }

        // Slice size = how much we're behind (with caps)
        let remaining = self.total_quantity - self.executed_quantity;
        let slice_quantity = behind_by.min(remaining);

        if slice_quantity <= Decimal::ZERO {
            return None;
        }

        self.slices_completed += 1;

        Some(Order {
            id: Uuid::new_v4(),
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: if self.limit_price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            price: self.limit_price,
            quantity: slice_quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: self.user_id.clone(),
            timestamp: current_time,
            time_in_force: TimeInForce::IOC, // Immediate-or-cancel for algo orders
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        })
    }

    /// Record execution of a child order
    pub fn record_fill(&mut self, filled_quantity: Decimal, _fill_price: Decimal) {
        self.executed_quantity += filled_quantity;

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
        }
    }

    /// Calculate execution statistics
    pub fn execution_stats(&self) -> TwapStats {
        let now = Utc::now();
        let expected_progress = if self.end_time > self.start_time {
            let total = (self.end_time - self.start_time).num_milliseconds() as f64;
            let elapsed = (now - self.start_time).num_milliseconds().max(0) as f64;
            (elapsed / total).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let actual_progress = (self.executed_quantity / self.total_quantity)
            .to_f64()
            .unwrap_or(0.0);

        TwapStats {
            expected_progress,
            actual_progress,
            behind_by: expected_progress - actual_progress,
            slices_completed: self.slices_completed,
            remaining_quantity: self.total_quantity - self.executed_quantity,
        }
    }

    /// Start the algorithm
    pub fn start(&mut self) {
        self.status = AlgorithmStatus::Running;
    }

    /// Pause the algorithm
    pub fn pause(&mut self) {
        self.status = AlgorithmStatus::Paused;
    }

    /// Cancel the algorithm
    pub fn cancel(&mut self) {
        self.status = AlgorithmStatus::Cancelled;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwapStats {
    pub expected_progress: f64,
    pub actual_progress: f64,
    pub behind_by: f64,
    pub slices_completed: u32,
    pub remaining_quantity: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rust_decimal_macros::dec;

    #[test]
    fn test_twap_creation() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let twap = TwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
            60, // 1 minute slices
        );

        assert_eq!(twap.total_quantity, dec!(1000));
        assert_eq!(twap.executed_quantity, Decimal::ZERO);
        assert_eq!(twap.status, AlgorithmStatus::Pending);
    }

    #[test]
    fn test_twap_slice_generation() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let mut twap = TwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
            60,
        );

        twap.start();

        // At start, should generate no slice (ahead of schedule)
        let slice = twap.next_slice(start);
        assert!(slice.is_none());

        // After some time
        let mid_time = start + Duration::minutes(30);
        let slice = twap.next_slice(mid_time);
        assert!(slice.is_some());

        if let Some(order) = slice {
            assert!(order.quantity > Decimal::ZERO);
            assert!(order.quantity <= dec!(1000));
        }
    }

    #[test]
    fn test_twap_completion() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let mut twap = TwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
            60,
        );

        twap.start();
        twap.record_fill(dec!(1000), dec!(100));

        assert_eq!(twap.status, AlgorithmStatus::Completed);
        assert_eq!(twap.executed_quantity, dec!(1000));
    }

    #[test]
    fn test_twap_stats() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let mut twap = TwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
            60,
        );

        twap.start();
        twap.record_fill(dec!(500), dec!(100));

        let stats = twap.execution_stats();
        assert_eq!(stats.actual_progress, 0.5);
        assert_eq!(stats.remaining_quantity, dec!(500));
    }
}
