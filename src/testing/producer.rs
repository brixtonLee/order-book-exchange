use crate::engine::orderbook::OrderBookEngine;
use crate::models::iceberg::IcebergConfig;
use crate::models::order::{
    Order, OrderSide, OrderType, SelfTradePreventionMode, TimeInForce,
};
use crate::testing::state::TestingState;
use chrono::{Duration, Utc};
use rand::prelude::*;
use rand::thread_rng;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info};
use uuid::Uuid;

/// Configuration for order generation with weighted distributions
#[derive(Debug, Clone)]
pub struct ProducerConfig {
    // Order type distribution (percentages should sum to 100)
    pub market_percentage: u32,      // 20%
    pub limit_percentage: u32,       // 60%
    pub iceberg_percentage: u32,     // 10%
    pub post_only_percentage: u32,   // 10%

    // Time-in-Force distribution (percentages should sum to 100)
    pub gtc_percentage: u32,   // 70%
    pub ioc_percentage: u32,   // 15%
    pub fok_percentage: u32,   // 10%
    pub gtd_percentage: u32,   // 5%
    pub day_percentage: u32,   // 0% (can be adjusted)

    // Self-Trade Prevention distribution (percentages should sum to 100)
    pub stp_none_percentage: u32,              // 60%
    pub stp_cancel_resting_percentage: u32,    // 10%
    pub stp_cancel_incoming_percentage: u32,   // 10%
    pub stp_cancel_both_percentage: u32,       // 5%
    pub stp_cancel_smallest_percentage: u32,   // 10%
    pub stp_decrement_both_percentage: u32,    // 5%

    // Price and quantity ranges
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub min_quantity: Decimal,
    pub max_quantity: Decimal,

    // Iceberg configuration
    pub iceberg_display_percentage: u32, // Display 20-30% of total quantity

    // User pool for STP testing
    pub user_pool: Vec<String>,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            // Order types
            market_percentage: 20,
            limit_percentage: 60,
            iceberg_percentage: 10,
            post_only_percentage: 10,

            // Time-in-Force
            gtc_percentage: 70,
            ioc_percentage: 15,
            fok_percentage: 10,
            gtd_percentage: 5,
            day_percentage: 0,

            // Self-Trade Prevention
            stp_none_percentage: 60,
            stp_cancel_resting_percentage: 10,
            stp_cancel_incoming_percentage: 10,
            stp_cancel_both_percentage: 5,
            stp_cancel_smallest_percentage: 10,
            stp_decrement_both_percentage: 5,

            // Price/quantity ranges
            min_price: Decimal::new(10000, 2),  // 100.00
            max_price: Decimal::new(20000, 2),  // 200.00
            min_quantity: Decimal::new(1, 0),
            max_quantity: Decimal::new(100, 0),

            // Iceberg
            iceberg_display_percentage: 25, // Display 25% of total

            // Users
            user_pool: vec![
                "trader1".to_string(),
                "trader2".to_string(),
                "trader3".to_string(),
                "trader4".to_string(),
                "trader5".to_string(),
            ],
        }
    }
}

/// Order producer that generates random orders based on configuration
pub struct OrderProducer {
    engine: Arc<OrderBookEngine>,
    testing_state: Arc<TestingState>,
    config: ProducerConfig,
}

impl OrderProducer {
    pub fn new(
        engine: Arc<OrderBookEngine>,
        testing_state: Arc<TestingState>,
        config: ProducerConfig,
    ) -> Self {
        Self {
            engine,
            testing_state,
            config,
        }
    }

    /// Start the producer background task
    pub async fn run(self: Arc<Self>) {
        info!("Order producer starting");

        loop {
            // Check if producer is running
            let (running, rate) = {
                let state = self.testing_state.producer_state.read().unwrap();
                (state.running, state.rate_per_second)
            };

            if !running {
                // Sleep for 1 second when not running
                tokio::time::sleep(TokioDuration::from_secs(1)).await;
                continue;
            }

            // Calculate interval between orders
            let interval_ms = if rate > 0 {
                1000 / rate as u64
            } else {
                1000
            };

            let mut interval = interval(TokioDuration::from_millis(interval_ms));

            // Generate orders at specified rate
            while {
                let state = self.testing_state.producer_state.read().unwrap();
                state.running
            } {
                interval.tick().await;

                // Generate and submit random order
                if let Err(e) = self.generate_and_submit_order().await {
                    error!("Failed to generate order: {}", e);
                    let mut state = self.testing_state.producer_state.write().unwrap();
                    state.errors += 1;
                }
            }

            info!("Order producer paused");
        }
    }

    /// Generate and submit a random order
    async fn generate_and_submit_order(&self) -> Result<(), String> {
        let order = self.generate_random_order();
        let symbol = order.symbol.clone();

        // Submit to engine
        match self.engine.add_order(order.clone()) {
            Ok(_) => {
                // Update producer state
                {
                    let mut state = self.testing_state.producer_state.write().unwrap();
                    state.orders_generated += 1;
                }

                // Update metrics
                {
                    let mut metrics = self.testing_state.metrics.write().unwrap();
                    metrics.increment_order_type(&order.order_type);
                    metrics.increment_tif(&order.time_in_force);
                    metrics.increment_stp(&order.stp_mode);
                    metrics.increment_side(&order.side);
                    metrics.increment_symbol(&symbol);

                    if order.iceberg.is_some() {
                        metrics.increment_iceberg();
                    }
                    if order.post_only {
                        metrics.increment_post_only();
                    }
                }

                Ok(())
            }
            Err(e) => {
                let mut metrics = self.testing_state.metrics.write().unwrap();
                metrics.increment_rejection();
                Err(format!("Order submission failed: {}", e))
            }
        }
    }

    /// Generate a random order based on configuration
    fn generate_random_order(&self) -> Order {
        let mut rng = thread_rng();

        // Select random symbol
        let state = self.testing_state.producer_state.read().unwrap();
        let symbol = state.symbols.choose(&mut rng).unwrap().clone();
        drop(state);

        // Select random user
        let user_id = self.config.user_pool.choose(&mut rng).unwrap().clone();

        // Select random side
        let side = if rng.gen_bool(0.5) {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        // Select order type and configuration based on weighted distribution
        let order_type_roll: u32 = rng.gen_range(0..100);
        let (order_type, post_only, iceberg) = if order_type_roll < self.config.market_percentage {
            // Market order
            (OrderType::Market, false, None)
        } else if order_type_roll
            < self.config.market_percentage + self.config.limit_percentage
        {
            // Limit order
            (OrderType::Limit, false, None)
        } else if order_type_roll
            < self.config.market_percentage
                + self.config.limit_percentage
                + self.config.iceberg_percentage
        {
            // Iceberg limit order
            let total_quantity = self.random_decimal(
                self.config.min_quantity,
                self.config.max_quantity,
                0,
            );
            let display_quantity = total_quantity
                * Decimal::new(self.config.iceberg_display_percentage as i64, 2);
            let iceberg_config = IcebergConfig::new(total_quantity, display_quantity);
            (OrderType::Limit, false, Some(iceberg_config))
        } else {
            // Post-only limit order
            (OrderType::Limit, true, None)
        };

        // Select Time-in-Force and expiration
        let tif_roll: u32 = rng.gen_range(0..100);
        let (time_in_force, expire_time) = if tif_roll < self.config.gtc_percentage {
            (TimeInForce::GTC, None)
        } else if tif_roll < self.config.gtc_percentage + self.config.ioc_percentage {
            (TimeInForce::IOC, None)
        } else if tif_roll
            < self.config.gtc_percentage + self.config.ioc_percentage + self.config.fok_percentage
        {
            (TimeInForce::FOK, None)
        } else if tif_roll
            < self.config.gtc_percentage
                + self.config.ioc_percentage
                + self.config.fok_percentage
                + self.config.gtd_percentage
        {
            // Random expiration time 1-24 hours in future
            let hours = rng.gen_range(1..=24);
            let expire = Utc::now() + Duration::hours(hours);
            (TimeInForce::GTD, Some(expire))
        } else {
            (TimeInForce::DAY, None)
        };

        // Select STP mode
        let stp_roll: u32 = rng.gen_range(0..100);
        let stp_mode = if stp_roll < self.config.stp_none_percentage {
            SelfTradePreventionMode::None
        } else if stp_roll
            < self.config.stp_none_percentage + self.config.stp_cancel_resting_percentage
        {
            SelfTradePreventionMode::CancelResting
        } else if stp_roll
            < self.config.stp_none_percentage
                + self.config.stp_cancel_resting_percentage
                + self.config.stp_cancel_incoming_percentage
        {
            SelfTradePreventionMode::CancelIncoming
        } else if stp_roll
            < self.config.stp_none_percentage
                + self.config.stp_cancel_resting_percentage
                + self.config.stp_cancel_incoming_percentage
                + self.config.stp_cancel_both_percentage
        {
            SelfTradePreventionMode::CancelBoth
        } else if stp_roll
            < self.config.stp_none_percentage
                + self.config.stp_cancel_resting_percentage
                + self.config.stp_cancel_incoming_percentage
                + self.config.stp_cancel_both_percentage
                + self.config.stp_cancel_smallest_percentage
        {
            SelfTradePreventionMode::CancelSmallest
        } else {
            SelfTradePreventionMode::DecrementBoth
        };

        // Generate price and quantity
        let price = if order_type == OrderType::Market {
            None
        } else {
            Some(self.random_decimal(self.config.min_price, self.config.max_price, 2))
        };
        let quantity = if iceberg.is_some() {
            // Iceberg already has quantity
            iceberg.as_ref().unwrap().total_quantity
        } else {
            self.random_decimal(self.config.min_quantity, self.config.max_quantity, 0)
        };

        Order {
            id: Uuid::new_v4(),
            symbol,
            user_id,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: crate::models::OrderStatus::New,
            time_in_force,
            stp_mode,
            post_only,
            expire_time,
            iceberg,
            timestamp: Utc::now(),
        }
    }

    /// Generate random decimal in range
    fn random_decimal(&self, min: Decimal, max: Decimal, scale: u32) -> Decimal {
        let mut rng = thread_rng();
        let range = max - min;
        let random_factor = Decimal::from_f64_retain(rng.gen_range(0.0..1.0)).unwrap();
        let value = min + (range * random_factor);

        // Round to specified decimal places
        value.round_dp(scale)
    }
}
