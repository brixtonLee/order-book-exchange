use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Iceberg order configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IcebergConfig {
    /// Total quantity of the entire order (visible + hidden)
    pub total_quantity: Decimal,

    /// Quantity to display in the order book
    pub display_quantity: Decimal,

    /// Current hidden quantity remaining
    pub hidden_quantity: Decimal,

    /// Minimum visible quantity before replenishing
    /// Some exchanges replenish when visible hits 0, others at a threshold
    pub replenish_threshold: Decimal,

    /// Variance to add to display quantity on replenish (anti-detection)
    /// e.g., 0.1 means ±10% randomization
    pub display_variance: Option<Decimal>,
}

impl IcebergConfig {
    pub fn new(total: Decimal, display: Decimal) -> Self {
        let display_clamped = display.min(total);
        Self {
            total_quantity: total,
            display_quantity: display_clamped,
            hidden_quantity: (total - display_clamped).max(Decimal::ZERO),
            replenish_threshold: Decimal::ZERO,
            display_variance: None,
        }
    }

    /// Process a fill against the visible portion
    /// Returns: IcebergFillResult
    pub fn process_fill(&mut self, fill_qty: Decimal) -> IcebergFillResult {
        let actual_fill = fill_qty.min(self.display_quantity);
        self.display_quantity -= actual_fill;
        self.total_quantity -= actual_fill;

        // Check if we need to replenish from hidden
        if self.display_quantity <= self.replenish_threshold && self.hidden_quantity > Decimal::ZERO {
            let replenish_amount = self.calculate_replenish_amount();
            self.hidden_quantity -= replenish_amount;
            self.display_quantity += replenish_amount;

            IcebergFillResult {
                filled_quantity: actual_fill,
                replenished: true,
                new_display_quantity: self.display_quantity,
                remaining_hidden: self.hidden_quantity,
            }
        } else {
            IcebergFillResult {
                filled_quantity: actual_fill,
                replenished: false,
                new_display_quantity: self.display_quantity,
                remaining_hidden: self.hidden_quantity,
            }
        }
    }

    fn calculate_replenish_amount(&self) -> Decimal {
        let base_amount = self.display_quantity;

        // Apply variance if configured (helps avoid detection)
        if let Some(_variance) = self.display_variance {
            // In production, use actual randomness
            // For now, just use base amount
            let factor = Decimal::ONE;
            (base_amount * factor).min(self.hidden_quantity)
        } else {
            base_amount.min(self.hidden_quantity)
        }
    }

    /// Check if the entire iceberg is complete
    pub fn is_complete(&self) -> bool {
        self.total_quantity.is_zero()
    }

    /// Get remaining total quantity
    pub fn remaining_quantity(&self) -> Decimal {
        self.total_quantity
    }

    /// Get visible quantity
    pub fn visible_quantity(&self) -> Decimal {
        self.display_quantity
    }

    /// Get hidden quantity
    pub fn hidden_quantity(&self) -> Decimal {
        self.hidden_quantity
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergFillResult {
    pub filled_quantity: Decimal,
    pub replenished: bool,
    pub new_display_quantity: Decimal,
    pub remaining_hidden: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_iceberg_creation() {
        let config = IcebergConfig::new(dec!(1000), dec!(100));

        assert_eq!(config.total_quantity, dec!(1000));
        assert_eq!(config.display_quantity, dec!(100));
        assert_eq!(config.hidden_quantity, dec!(900));
    }

    #[test]
    fn test_iceberg_fill_without_replenish() {
        let mut config = IcebergConfig::new(dec!(1000), dec!(100));

        // Fill 50 - should not replenish
        let result = config.process_fill(dec!(50));

        assert_eq!(result.filled_quantity, dec!(50));
        assert!(!result.replenished);
        assert_eq!(result.new_display_quantity, dec!(50));
        assert_eq!(config.total_quantity, dec!(950));
        assert_eq!(config.hidden_quantity, dec!(900));
    }

    #[test]
    fn test_iceberg_fill_with_replenish() {
        let mut config = IcebergConfig::new(dec!(1000), dec!(100));

        // Fill entire visible portion - should replenish
        let result = config.process_fill(dec!(100));

        assert_eq!(result.filled_quantity, dec!(100));
        assert!(result.replenished);
        assert_eq!(result.new_display_quantity, dec!(100)); // Replenished from hidden
        assert_eq!(config.total_quantity, dec!(900));
        assert_eq!(config.hidden_quantity, dec!(800));
    }

    #[test]
    fn test_iceberg_partial_replenish() {
        let mut config = IcebergConfig::new(dec!(150), dec!(100));

        // Fill entire visible portion
        let result = config.process_fill(dec!(100));

        assert!(result.replenished);
        // Only 50 left in hidden, so can only replenish 50
        assert_eq!(result.new_display_quantity, dec!(50));
        assert_eq!(config.hidden_quantity, dec!(0));
    }

    #[test]
    fn test_iceberg_complete() {
        let mut config = IcebergConfig::new(dec!(100), dec!(100));

        assert!(!config.is_complete());

        config.process_fill(dec!(100));

        assert!(config.is_complete());
        assert_eq!(config.remaining_quantity(), Decimal::ZERO);
    }

    #[test]
    fn test_iceberg_overfill_protection() {
        let mut config = IcebergConfig::new(dec!(1000), dec!(100));

        // Try to fill more than visible
        let result = config.process_fill(dec!(200));

        // Should only fill up to visible quantity
        assert_eq!(result.filled_quantity, dec!(100));
    }

    #[test]
    fn test_iceberg_clamping() {
        // Display quantity larger than total
        let config = IcebergConfig::new(dec!(100), dec!(200));

        assert_eq!(config.display_quantity, dec!(100));
        assert_eq!(config.hidden_quantity, Decimal::ZERO);
    }
}
