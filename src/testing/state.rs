use crate::models::order::{OrderSide, OrderType, SelfTradePreventionMode, TimeInForce};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Shared testing state accessible across handlers and background tasks
#[derive(Clone)]
pub struct TestingState {
    pub producer_state: Arc<RwLock<ProducerState>>,
    pub metrics: Arc<RwLock<TestingMetrics>>,
}

impl TestingState {
    pub fn new() -> Self {
        Self {
            producer_state: Arc::new(RwLock::new(ProducerState::default())),
            metrics: Arc::new(RwLock::new(TestingMetrics::default())),
        }
    }
}

/// Producer configuration and runtime state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerState {
    pub running: bool,
    pub rate_per_second: u32,
    pub symbols: Vec<String>,
    pub enabled_order_types: Vec<OrderType>,
    pub orders_generated: u64,
    pub errors: u64,
    pub started_at: Option<DateTime<Utc>>,
}

impl Default for ProducerState {
    fn default() -> Self {
        Self {
            running: false,
            rate_per_second: 10,
            symbols: vec!["AAPL".to_string(), "GOOGL".to_string(), "MSFT".to_string()],
            enabled_order_types: vec![OrderType::Limit, OrderType::Market],
            orders_generated: 0,
            errors: 0,
            started_at: None,
        }
    }
}

/// Comprehensive testing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingMetrics {
    // Order type distribution
    pub total_orders: u64,
    pub market_orders: u64,
    pub limit_orders: u64,
    pub iceberg_orders: u64,
    pub post_only_orders: u64,

    // Time-in-Force distribution
    pub gtc_orders: u64,
    pub ioc_orders: u64,
    pub fok_orders: u64,
    pub gtd_orders: u64,
    pub day_orders: u64,

    // Self-Trade Prevention distribution
    pub stp_none: u64,
    pub stp_cancel_resting: u64,
    pub stp_cancel_incoming: u64,
    pub stp_cancel_both: u64,
    pub stp_cancel_smallest: u64,
    pub stp_decrement_both: u64,

    // Stop orders
    pub stop_market_orders: u64,
    pub stop_limit_orders: u64,
    pub trailing_stop_orders: u64,
    pub stop_orders_triggered: u64,

    // Algorithms
    pub twap_algorithms: u64,
    pub vwap_algorithms: u64,
    pub algorithm_fills: u64,

    // Execution results
    pub fills: u64,
    pub partial_fills: u64,
    pub cancellations: u64,
    pub rejections: u64,
    pub self_trade_preventions: u64,

    // Order side distribution
    pub buy_orders: u64,
    pub sell_orders: u64,

    // Per-symbol metrics
    pub orders_by_symbol: HashMap<String, u64>,

    // Timestamp
    pub last_updated: DateTime<Utc>,
}

impl Default for TestingMetrics {
    fn default() -> Self {
        Self {
            total_orders: 0,
            market_orders: 0,
            limit_orders: 0,
            iceberg_orders: 0,
            post_only_orders: 0,
            gtc_orders: 0,
            ioc_orders: 0,
            fok_orders: 0,
            gtd_orders: 0,
            day_orders: 0,
            stp_none: 0,
            stp_cancel_resting: 0,
            stp_cancel_incoming: 0,
            stp_cancel_both: 0,
            stp_cancel_smallest: 0,
            stp_decrement_both: 0,
            stop_market_orders: 0,
            stop_limit_orders: 0,
            trailing_stop_orders: 0,
            stop_orders_triggered: 0,
            twap_algorithms: 0,
            vwap_algorithms: 0,
            algorithm_fills: 0,
            fills: 0,
            partial_fills: 0,
            cancellations: 0,
            rejections: 0,
            self_trade_preventions: 0,
            buy_orders: 0,
            sell_orders: 0,
            orders_by_symbol: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

impl TestingMetrics {
    /// Increment order type counter
    pub fn increment_order_type(&mut self, order_type: &OrderType) {
        self.total_orders += 1;
        match order_type {
            OrderType::Market => self.market_orders += 1,
            OrderType::Limit => self.limit_orders += 1,
        }
        self.last_updated = Utc::now();
    }

    /// Increment Time-in-Force counter
    pub fn increment_tif(&mut self, tif: &TimeInForce) {
        match tif {
            TimeInForce::GTC => self.gtc_orders += 1,
            TimeInForce::IOC => self.ioc_orders += 1,
            TimeInForce::FOK => self.fok_orders += 1,
            TimeInForce::GTD => self.gtd_orders += 1,
            TimeInForce::DAY => self.day_orders += 1,
        }
    }

    /// Increment STP mode counter
    pub fn increment_stp(&mut self, stp: &SelfTradePreventionMode) {
        match stp {
            SelfTradePreventionMode::None => self.stp_none += 1,
            SelfTradePreventionMode::CancelResting => self.stp_cancel_resting += 1,
            SelfTradePreventionMode::CancelIncoming => self.stp_cancel_incoming += 1,
            SelfTradePreventionMode::CancelBoth => self.stp_cancel_both += 1,
            SelfTradePreventionMode::CancelSmallest => self.stp_cancel_smallest += 1,
            SelfTradePreventionMode::DecrementBoth => self.stp_decrement_both += 1,
        }
    }

    /// Increment order side counter
    pub fn increment_side(&mut self, side: &OrderSide) {
        match side {
            OrderSide::Buy => self.buy_orders += 1,
            OrderSide::Sell => self.sell_orders += 1,
        }
    }

    /// Increment symbol counter
    pub fn increment_symbol(&mut self, symbol: &str) {
        *self.orders_by_symbol.entry(symbol.to_string()).or_insert(0) += 1;
    }

    /// Increment iceberg counter
    pub fn increment_iceberg(&mut self) {
        self.iceberg_orders += 1;
    }

    /// Increment post-only counter
    pub fn increment_post_only(&mut self) {
        self.post_only_orders += 1;
    }

    /// Increment fill counter
    pub fn increment_fill(&mut self) {
        self.fills += 1;
        self.last_updated = Utc::now();
    }

    /// Increment rejection counter
    pub fn increment_rejection(&mut self) {
        self.rejections += 1;
        self.last_updated = Utc::now();
    }
}
