use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::models::Trade;

/// Maker fee rate (0.10%)
const MAKER_FEE_RATE: Decimal = dec!(0.001);

/// Taker fee rate (0.20%)
const TAKER_FEE_RATE: Decimal = dec!(0.002);

/// Calculate maker fee for a given trade value
pub fn calculate_maker_fee(trade_value: Decimal) -> Decimal {
    trade_value * MAKER_FEE_RATE
}

/// Calculate taker fee for a given trade value
pub fn calculate_taker_fee(trade_value: Decimal) -> Decimal {
    trade_value * TAKER_FEE_RATE
}

/// Calculate total exchange profit from a list of trades
pub fn calculate_exchange_profit(trades: &[Trade]) -> Decimal {
    trades.iter().map(|trade| trade.total_fees()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_maker_fee() {
        let trade_value = dec!(10000);
        let fee = calculate_maker_fee(trade_value);
        assert_eq!(fee, dec!(10.00));
    }

    #[test]
    fn test_taker_fee() {
        let trade_value = dec!(10000);
        let fee = calculate_taker_fee(trade_value);
        assert_eq!(fee, dec!(20.00));
    }

    #[test]
    fn test_fee_calculation() {
        // Example: Trade of 100 shares at $150.50
        let quantity = dec!(100);
        let price = dec!(150.50);
        let trade_value = quantity * price;

        let maker_fee = calculate_maker_fee(trade_value);
        let taker_fee = calculate_taker_fee(trade_value);

        assert_eq!(maker_fee, dec!(15.05));
        assert_eq!(taker_fee, dec!(30.10));
    }
}
