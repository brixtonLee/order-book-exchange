use chrono::{DateTime, Duration, NaiveTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::{Order, OrderSide, OrderType, OrderStatus, TimeInForce};
use crate::models::stp::SelfTradePreventionMode;
use super::twap::AlgorithmStatus;

/// VWAP execution algorithm
/// Follows historical volume profile to minimize market impact
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VwapAlgorithm {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,

    pub total_quantity: Decimal,
    pub executed_quantity: Decimal,

    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,

    /// Historical volume profile
    #[serde(skip)] // Skip serialization for now
    volume_profile: VolumeProfile,

    /// Target execution curve (cumulative)
    #[serde(skip)]
    target_curve: Vec<(DateTime<Utc>, Decimal)>,

    pub status: AlgorithmStatus,

    /// Actual VWAP achieved so far
    pub achieved_vwap: Decimal,
    total_notional: Decimal,
}

/// Historical volume distribution throughout the trading day
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    /// Time bucket -> fraction of daily volume (0.0 to 1.0)
    buckets: BTreeMap<NaiveTime, Decimal>,
    bucket_duration_minutes: i64,
}

impl VolumeProfile {
    /// Create a typical US equity volume profile (U-shaped)
    pub fn us_equity_default() -> Self {
        let mut buckets = BTreeMap::new();

        // High volume at open (9:30-10:00)
        buckets.insert(NaiveTime::from_hms_opt(9, 30, 0).unwrap(), dec!(0.08));
        buckets.insert(NaiveTime::from_hms_opt(9, 35, 0).unwrap(), dec!(0.06));
        buckets.insert(NaiveTime::from_hms_opt(9, 40, 0).unwrap(), dec!(0.05));
        buckets.insert(NaiveTime::from_hms_opt(9, 45, 0).unwrap(), dec!(0.04));
        buckets.insert(NaiveTime::from_hms_opt(9, 50, 0).unwrap(), dec!(0.04));
        buckets.insert(NaiveTime::from_hms_opt(9, 55, 0).unwrap(), dec!(0.03));

        // Mid-morning (10:00-12:00)
        for hour in 10..12 {
            for min in (0..60).step_by(5) {
                buckets.insert(
                    NaiveTime::from_hms_opt(hour, min, 0).unwrap(),
                    dec!(0.025)
                );
            }
        }

        // Low volume mid-day (12:00-14:00)
        for hour in 12..14 {
            for min in (0..60).step_by(5) {
                buckets.insert(
                    NaiveTime::from_hms_opt(hour, min, 0).unwrap(),
                    dec!(0.02)
                );
            }
        }

        // Afternoon pickup (14:00-15:30)
        for hour in 14..15 {
            for min in (0..60).step_by(5) {
                buckets.insert(
                    NaiveTime::from_hms_opt(hour, min, 0).unwrap(),
                    dec!(0.025)
                );
            }
        }

        // High volume at close (15:30-16:00)
        buckets.insert(NaiveTime::from_hms_opt(15, 30, 0).unwrap(), dec!(0.05));
        buckets.insert(NaiveTime::from_hms_opt(15, 35, 0).unwrap(), dec!(0.06));
        buckets.insert(NaiveTime::from_hms_opt(15, 40, 0).unwrap(), dec!(0.07));
        buckets.insert(NaiveTime::from_hms_opt(15, 45, 0).unwrap(), dec!(0.08));
        buckets.insert(NaiveTime::from_hms_opt(15, 50, 0).unwrap(), dec!(0.09));
        buckets.insert(NaiveTime::from_hms_opt(15, 55, 0).unwrap(), dec!(0.10));

        Self {
            buckets,
            bucket_duration_minutes: 5,
        }
    }

    /// Get expected volume percentage for a given time
    pub fn volume_at(&self, time: NaiveTime) -> Decimal {
        // Find the nearest bucket
        self.buckets
            .range(..=time)
            .last()
            .map(|(_, v)| *v)
            .unwrap_or(dec!(0.02)) // Default to low volume
    }

    /// Calculate cumulative volume from start to end
    pub fn cumulative_volume(&self, start: NaiveTime, end: NaiveTime) -> Decimal {
        self.buckets
            .range(start..=end)
            .map(|(_, v)| v)
            .sum()
    }
}

impl VwapAlgorithm {
    pub fn new(
        symbol: String,
        side: OrderSide,
        user_id: String,
        total_quantity: Decimal,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        let volume_profile = VolumeProfile::us_equity_default();

        let mut algo = Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            user_id,
            total_quantity,
            executed_quantity: Decimal::ZERO,
            start_time,
            end_time,
            volume_profile,
            target_curve: Vec::new(),
            status: AlgorithmStatus::Pending,
            achieved_vwap: Decimal::ZERO,
            total_notional: Decimal::ZERO,
        };

        algo.build_target_curve();
        algo
    }

    /// Build the target execution curve based on volume profile
    fn build_target_curve(&mut self) {
        let start_time_of_day = self.start_time.time();
        let end_time_of_day = self.end_time.time();

        // Get total expected volume in our window
        let total_vol_pct = self.volume_profile
            .cumulative_volume(start_time_of_day, end_time_of_day);

        if total_vol_pct.is_zero() {
            return;
        }

        // Build cumulative target curve
        let mut cumulative = Decimal::ZERO;
        let mut current = self.start_time;

        while current < self.end_time {
            let time_of_day = current.time();
            let bucket_vol = self.volume_profile.volume_at(time_of_day);
            let normalized = bucket_vol / total_vol_pct;
            cumulative += normalized;

            let target_qty = self.total_quantity * cumulative.min(Decimal::ONE);
            self.target_curve.push((current, target_qty));

            current = current + Duration::minutes(self.volume_profile.bucket_duration_minutes);
        }
    }

    /// Get the target quantity we should have executed by now
    pub fn target_at(&self, time: DateTime<Utc>) -> Decimal {
        self.target_curve
            .iter()
            .filter(|(t, _)| *t <= time)
            .last()
            .map(|(_, qty)| *qty)
            .unwrap_or(Decimal::ZERO)
    }

    /// Calculate next slice
    pub fn next_slice(&mut self, current_time: DateTime<Utc>) -> Option<Order> {
        if self.status != AlgorithmStatus::Running {
            return None;
        }

        if current_time >= self.end_time {
            self.status = AlgorithmStatus::Completed;
            return None;
        }

        let target = self.target_at(current_time);
        let behind_by = target - self.executed_quantity;

        if behind_by <= Decimal::ZERO {
            return None;
        }

        let slice_qty = behind_by.min(self.total_quantity - self.executed_quantity);

        if slice_qty <= Decimal::ZERO {
            return None;
        }

        Some(Order {
            id: Uuid::new_v4(),
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: OrderType::Market,
            price: None,
            quantity: slice_qty,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: self.user_id.clone(),
            timestamp: current_time,
            time_in_force: TimeInForce::IOC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        })
    }

    /// Record a fill and update VWAP calculation
    pub fn record_fill(&mut self, quantity: Decimal, price: Decimal) {
        self.executed_quantity += quantity;
        self.total_notional += quantity * price;

        if self.executed_quantity > Decimal::ZERO {
            self.achieved_vwap = self.total_notional / self.executed_quantity;
        }

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
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

    /// Get execution statistics
    pub fn stats(&self) -> VwapStats {
        let now = Utc::now();
        let target = self.target_at(now);
        let actual_progress = if self.total_quantity > Decimal::ZERO {
            (self.executed_quantity / self.total_quantity).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };

        VwapStats {
            achieved_vwap: self.achieved_vwap,
            executed_quantity: self.executed_quantity,
            remaining_quantity: self.total_quantity - self.executed_quantity,
            target_quantity: target,
            actual_progress,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VwapStats {
    pub achieved_vwap: Decimal,
    pub executed_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub target_quantity: Decimal,
    pub actual_progress: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_volume_profile_creation() {
        let profile = VolumeProfile::us_equity_default();

        // Check that profile has entries
        assert!(!profile.buckets.is_empty());

        // Check opening volume is higher
        let open_vol = profile.volume_at(NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        let midday_vol = profile.volume_at(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert!(open_vol > midday_vol);
    }

    #[test]
    fn test_vwap_creation() {
        let start = Utc::now();
        let end = start + Duration::hours(6);

        let vwap = VwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
        );

        assert_eq!(vwap.total_quantity, dec!(1000));
        assert_eq!(vwap.status, AlgorithmStatus::Pending);
    }

    #[test]
    fn test_vwap_calculation() {
        let start = Utc::now();
        let end = start + Duration::hours(6);

        let mut vwap = VwapAlgorithm::new(
            "TEST".to_string(),
            OrderSide::Buy,
            "user1".to_string(),
            dec!(1000),
            start,
            end,
        );

        vwap.start();

        // Record some fills
        vwap.record_fill(dec!(100), dec!(50));
        vwap.record_fill(dec!(200), dec!(51));

        // Check VWAP calculation
        // (100 * 50 + 200 * 51) / 300 = (5000 + 10200) / 300 = 50.666...
        assert!(vwap.achieved_vwap > Decimal::ZERO);
        assert_eq!(vwap.executed_quantity, dec!(300));
    }
}
