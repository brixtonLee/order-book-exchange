use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a completed trade between two orders
/// You can understand it as Deal also
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: Uuid,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_id: String,
    pub seller_id: String,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

impl Trade {
    /// Create a new trade
    pub fn new(
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        buyer_order_id: Uuid,
        seller_order_id: Uuid,
        buyer_id: String,
        seller_id: String,
        maker_fee: Decimal,
        taker_fee: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol,
            price,
            quantity,
            buyer_order_id,
            seller_order_id,
            buyer_id,
            seller_id,
            maker_fee,
            taker_fee,
            timestamp: Utc::now(),
        }
    }

    /// Get the total trade value
    pub fn value(&self) -> Decimal {
        self.price * self.quantity
    }

    /// Get total fees collected from this trade
    pub fn total_fees(&self) -> Decimal {
        self.maker_fee + self.taker_fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            "AAPL".to_string(),
            dec!(150.50),
            dec!(100),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "buyer1".to_string(),
            "seller1".to_string(),
            dec!(0.15),
            dec!(0.30),
        );

        assert_eq!(trade.symbol, "AAPL");
        assert_eq!(trade.price, dec!(150.50));
        assert_eq!(trade.quantity, dec!(100));
        assert_eq!(trade.value(), dec!(15050.00));
        assert_eq!(trade.total_fees(), dec!(0.45));
    }
}
