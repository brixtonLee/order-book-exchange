use crate::testing::producer::ProducerConfig;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Pre-defined test scenarios with specialized configurations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestScenario {
    /// Basic testing: 70% limit, 30% market
    Basic,
    /// Self-trade prevention testing with same user
    SelfTradePrevention,
    /// Iceberg order focus (80% iceberg)
    IcebergOrders,
    /// Stop order cascade testing
    StopOrderCascade,
    /// Algorithm stress testing
    AlgorithmStress,
    /// Full random distribution (chaos mode)
    Chaos,
    /// Time-in-force expiration testing
    TimeInForce,
    /// Post-only maker orders
    PostOnlyMakers,
}

impl TestScenario {
    /// Convert scenario to producer configuration
    pub fn to_config(self) -> ProducerConfig {
        match self {
            TestScenario::Basic => ProducerConfig {
                // Basic order types
                market_percentage: 30,
                limit_percentage: 70,
                iceberg_percentage: 0,
                post_only_percentage: 0,

                // Simple TIF
                gtc_percentage: 100,
                ioc_percentage: 0,
                fok_percentage: 0,
                gtd_percentage: 0,
                day_percentage: 0,

                // No STP
                stp_none_percentage: 100,
                stp_cancel_resting_percentage: 0,
                stp_cancel_incoming_percentage: 0,
                stp_cancel_both_percentage: 0,
                stp_cancel_smallest_percentage: 0,
                stp_decrement_both_percentage: 0,

                ..ProducerConfig::default()
            },

            TestScenario::SelfTradePrevention => ProducerConfig {
                // All limit orders for better STP testing
                market_percentage: 0,
                limit_percentage: 100,
                iceberg_percentage: 0,
                post_only_percentage: 0,

                // GTC only
                gtc_percentage: 100,
                ioc_percentage: 0,
                fok_percentage: 0,
                gtd_percentage: 0,
                day_percentage: 0,

                // All STP modes evenly distributed
                stp_none_percentage: 0,
                stp_cancel_resting_percentage: 20,
                stp_cancel_incoming_percentage: 20,
                stp_cancel_both_percentage: 20,
                stp_cancel_smallest_percentage: 20,
                stp_decrement_both_percentage: 20,

                // Single user to maximize self-trade scenarios
                user_pool: vec!["test_user".to_string()],

                ..ProducerConfig::default()
            },

            TestScenario::IcebergOrders => ProducerConfig {
                // Mostly iceberg
                market_percentage: 10,
                limit_percentage: 10,
                iceberg_percentage: 80,
                post_only_percentage: 0,

                // GTC for icebergs
                gtc_percentage: 100,
                ioc_percentage: 0,
                fok_percentage: 0,
                gtd_percentage: 0,
                day_percentage: 0,

                // No STP
                stp_none_percentage: 100,
                stp_cancel_resting_percentage: 0,
                stp_cancel_incoming_percentage: 0,
                stp_cancel_both_percentage: 0,
                stp_cancel_smallest_percentage: 0,
                stp_decrement_both_percentage: 0,

                // Display only 10-15% of total
                iceberg_display_percentage: 12,

                ..ProducerConfig::default()
            },

            TestScenario::StopOrderCascade => {
                // This scenario would need special handling for stop orders
                // For now, use basic config with wide price range
                ProducerConfig {
                    market_percentage: 50,
                    limit_percentage: 50,
                    iceberg_percentage: 0,
                    post_only_percentage: 0,

                    gtc_percentage: 100,
                    ioc_percentage: 0,
                    fok_percentage: 0,
                    gtd_percentage: 0,
                    day_percentage: 0,

                    stp_none_percentage: 100,
                    stp_cancel_resting_percentage: 0,
                    stp_cancel_incoming_percentage: 0,
                    stp_cancel_both_percentage: 0,
                    stp_cancel_smallest_percentage: 0,
                    stp_decrement_both_percentage: 0,

                    // Wide price range for triggering
                    min_price: Decimal::new(5000, 2),  // 50.00
                    max_price: Decimal::new(30000, 2), // 300.00

                    ..ProducerConfig::default()
                }
            }

            TestScenario::AlgorithmStress => {
                // Continuous market activity for algorithms to execute against
                ProducerConfig {
                    market_percentage: 40,
                    limit_percentage: 60,
                    iceberg_percentage: 0,
                    post_only_percentage: 0,

                    // Mix of TIF for variety
                    gtc_percentage: 60,
                    ioc_percentage: 30,
                    fok_percentage: 10,
                    gtd_percentage: 0,
                    day_percentage: 0,

                    stp_none_percentage: 100,
                    stp_cancel_resting_percentage: 0,
                    stp_cancel_incoming_percentage: 0,
                    stp_cancel_both_percentage: 0,
                    stp_cancel_smallest_percentage: 0,
                    stp_decrement_both_percentage: 0,

                    // Many users for liquidity
                    user_pool: vec![
                        "user1".to_string(),
                        "user2".to_string(),
                        "user3".to_string(),
                        "user4".to_string(),
                        "user5".to_string(),
                        "user6".to_string(),
                        "user7".to_string(),
                        "user8".to_string(),
                    ],

                    ..ProducerConfig::default()
                }
            }

            TestScenario::Chaos => ProducerConfig::default(),

            TestScenario::TimeInForce => ProducerConfig {
                // All limit orders
                market_percentage: 0,
                limit_percentage: 100,
                iceberg_percentage: 0,
                post_only_percentage: 0,

                // Focus on expiring orders
                gtc_percentage: 20,
                ioc_percentage: 30,
                fok_percentage: 20,
                gtd_percentage: 30,
                day_percentage: 0,

                stp_none_percentage: 100,
                stp_cancel_resting_percentage: 0,
                stp_cancel_incoming_percentage: 0,
                stp_cancel_both_percentage: 0,
                stp_cancel_smallest_percentage: 0,
                stp_decrement_both_percentage: 0,

                ..ProducerConfig::default()
            },

            TestScenario::PostOnlyMakers => ProducerConfig {
                // All post-only limit orders
                market_percentage: 0,
                limit_percentage: 0,
                iceberg_percentage: 0,
                post_only_percentage: 100,

                // GTC only
                gtc_percentage: 100,
                ioc_percentage: 0,
                fok_percentage: 0,
                gtd_percentage: 0,
                day_percentage: 0,

                stp_none_percentage: 100,
                stp_cancel_resting_percentage: 0,
                stp_cancel_incoming_percentage: 0,
                stp_cancel_both_percentage: 0,
                stp_cancel_smallest_percentage: 0,
                stp_decrement_both_percentage: 0,

                ..ProducerConfig::default()
            },
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            TestScenario::Basic => "Basic testing with 70% limit and 30% market orders",
            TestScenario::SelfTradePrevention => {
                "Self-trade prevention testing with all STP modes"
            }
            TestScenario::IcebergOrders => "Iceberg order testing with 80% hidden orders",
            TestScenario::StopOrderCascade => "Stop order cascade with wide price ranges",
            TestScenario::AlgorithmStress => {
                "Algorithm stress testing with high liquidity"
            }
            TestScenario::Chaos => "Full random distribution (default config)",
            TestScenario::TimeInForce => {
                "Time-in-force testing with IOC, FOK, and GTD orders"
            }
            TestScenario::PostOnlyMakers => "Post-only maker order testing",
        }
    }
}

impl std::fmt::Display for TestScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_configs() {
        // Test that all scenarios produce valid configs
        for scenario in [
            TestScenario::Basic,
            TestScenario::SelfTradePrevention,
            TestScenario::IcebergOrders,
            TestScenario::StopOrderCascade,
            TestScenario::AlgorithmStress,
            TestScenario::Chaos,
            TestScenario::TimeInForce,
            TestScenario::PostOnlyMakers,
        ] {
            let config = scenario.to_config();

            // Verify order type percentages sum to 100
            let order_type_sum = config.market_percentage
                + config.limit_percentage
                + config.iceberg_percentage
                + config.post_only_percentage;
            assert_eq!(
                order_type_sum, 100,
                "Order type percentages must sum to 100 for {:?}",
                scenario
            );

            // Verify TIF percentages sum to 100
            let tif_sum = config.gtc_percentage
                + config.ioc_percentage
                + config.fok_percentage
                + config.gtd_percentage
                + config.day_percentage;
            assert_eq!(
                tif_sum, 100,
                "TIF percentages must sum to 100 for {:?}",
                scenario
            );

            // Verify STP percentages sum to 100
            let stp_sum = config.stp_none_percentage
                + config.stp_cancel_resting_percentage
                + config.stp_cancel_incoming_percentage
                + config.stp_cancel_both_percentage
                + config.stp_cancel_smallest_percentage
                + config.stp_decrement_both_percentage;
            assert_eq!(
                stp_sum, 100,
                "STP percentages must sum to 100 for {:?}",
                scenario
            );
        }
    }
}
