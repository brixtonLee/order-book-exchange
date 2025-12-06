// Utility functions and validation
// Can be extended with input validation, helper functions, etc.

pub mod validation {
    use rust_decimal::Decimal;

    /// Validate that a price is positive
    pub fn is_valid_price(price: Decimal) -> bool {
        price > Decimal::ZERO
    }

    /// Validate that a quantity is positive
    pub fn is_valid_quantity(quantity: Decimal) -> bool {
        quantity > Decimal::ZERO
    }

    /// Validate that a symbol is not empty
    pub fn is_valid_symbol(symbol: &str) -> bool {
        !symbol.is_empty() && symbol.len() <= 10
    }
}
