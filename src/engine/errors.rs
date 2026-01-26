//! Error types for order book operations
//!
//! This module centralizes all error types used by the order book engine,
//! making error handling consistent and maintainable across the codebase.

use thiserror::Error;
use uuid::Uuid;

use super::matching::MatchingError;

/// Errors that can occur during order book operations
///
/// The `#[error]` attribute comes from the `thiserror` crate - Rust's most popular
/// library for creating custom error types. It eliminates boilerplate by automatically
/// implementing the `std::error::Error` and `Display` traits for your error enums.
///
/// The `#[error("...")]` attribute tells thiserror what message to display when the error is printed.
///
/// # Error Categories
///
/// - **Validation Errors**: `InvalidPrice`, `InvalidQuantity`, `InvalidExpireTime`, `InvalidSymbol`
/// - **State Errors**: `OrderNotFound`, `OrderNotActive`, `DuplicateOrder`
/// - **Trading Errors**: `InsufficientLiquidity`, `SelfTrade`
/// - **Internal Errors**: `MatchingError`
#[derive(Debug, Error)]
pub enum OrderBookError {
    /// Order with the specified ID was not found in the order book
    #[error("Order not found: {0}")]
    OrderNotFound(Uuid),

    /// Price validation failed (negative, zero, or missing for limit orders)
    #[error("Invalid price: {0}")]
    InvalidPrice(String),

    /// Quantity validation failed (negative or zero)
    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),

    /// Expire time validation failed (missing for GTD orders)
    #[error("Invalid expire time: {0}")]
    InvalidExpireTime(String),

    /// Not enough liquidity in the order book to fill the order
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    /// Self-trade was detected and prevented
    #[error("Self-trade detected")]
    SelfTrade,

    /// An order with the same ID already exists
    #[error("Duplicate order: {0}")]
    DuplicateOrder(Uuid),

    /// The trading symbol is invalid or not supported
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    /// The order is already filled or cancelled and cannot be modified
    #[error("Order already filled or cancelled: {0}")]
    OrderNotActive(Uuid),

    /// An error occurred during order matching
    #[error("Matching error: {0}")]
    MatchingError(#[from] MatchingError),

    /// Failed to acquire lock (RwLock poisoned)
    #[error("Lock error: {0}")]
    LockError(String),
}

impl OrderBookError {
    /// Returns true if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            OrderBookError::InvalidPrice(_)
                | OrderBookError::InvalidQuantity(_)
                | OrderBookError::InvalidExpireTime(_)
                | OrderBookError::InvalidSymbol(_)
        )
    }

    /// Returns true if this is a state error (order doesn't exist or wrong state)
    pub fn is_state_error(&self) -> bool {
        matches!(
            self,
            OrderBookError::OrderNotFound(_)
                | OrderBookError::OrderNotActive(_)
                | OrderBookError::DuplicateOrder(_)
        )
    }

    /// Returns true if this is a trading error
    pub fn is_trading_error(&self) -> bool {
        matches!(
            self,
            OrderBookError::InsufficientLiquidity | OrderBookError::SelfTrade
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = OrderBookError::InvalidPrice("Price must be positive".to_string());
        assert_eq!(err.to_string(), "Invalid price: Price must be positive");
    }

    #[test]
    fn test_error_categories() {
        assert!(OrderBookError::InvalidPrice("test".to_string()).is_validation_error());
        assert!(OrderBookError::OrderNotFound(Uuid::new_v4()).is_state_error());
        assert!(OrderBookError::InsufficientLiquidity.is_trading_error());
    }
}
