//! Order validation functions
//!
//! This module provides centralized validation for orders before they are
//! processed by the order book engine. All validation logic is contained here
//! to ensure consistency and make it easy to add new validation rules.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::models::{Order, OrderType, TimeInForce};

use super::errors::OrderBookError;

// ============================================================================
// Individual Validation Functions
// ============================================================================

/// Validate that order quantity is positive
///
/// # Arguments
/// * `quantity` - The order quantity to validate
///
/// # Returns
/// * `Ok(())` if quantity is valid (greater than zero)
/// * `Err(OrderBookError::InvalidQuantity)` if quantity is zero or negative
///
/// # Example
/// ```ignore
/// use rust_decimal_macros::dec;
/// assert!(validate_quantity(dec!(100)).is_ok());
/// assert!(validate_quantity(dec!(0)).is_err());
/// assert!(validate_quantity(dec!(-10)).is_err());
/// ```
pub fn validate_quantity(quantity: Decimal) -> Result<(), OrderBookError> {
    if quantity <= Decimal::ZERO {
        return Err(OrderBookError::InvalidQuantity(format!(
            "Quantity must be positive, got: {}",
            quantity
        )));
    }
    Ok(())
}

/// Validate order price based on order type
///
/// # Rules
/// - Price must be positive if provided
/// - Limit orders MUST have a price
/// - Market orders may have no price (None is acceptable)
///
/// # Arguments
/// * `price` - Optional price for the order
/// * `order_type` - The type of order (Limit or Market)
///
/// # Returns
/// * `Ok(())` if price is valid for the given order type
/// * `Err(OrderBookError::InvalidPrice)` if validation fails
pub fn validate_price(price: Option<Decimal>, order_type: &OrderType) -> Result<(), OrderBookError> {
    match (price, order_type) {
        // Price is provided but not positive
        (Some(p), _) if p <= Decimal::ZERO => Err(OrderBookError::InvalidPrice(format!(
            "Price must be positive, got: {}",
            p
        ))),
        // Limit order without price
        (None, OrderType::Limit) => Err(OrderBookError::InvalidPrice(
            "Limit orders must have a price".to_string(),
        )),
        // All other cases are valid
        _ => Ok(()),
    }
}

/// Validate that GTD (Good-Till-Date) orders have an expire_time
///
/// # Arguments
/// * `time_in_force` - The time-in-force setting for the order
/// * `expire_time` - Optional expiration timestamp
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(OrderBookError::InvalidExpireTime)` if GTD order has no expire_time
pub fn validate_expire_time(
    time_in_force: &TimeInForce,
    expire_time: Option<DateTime<Utc>>,
) -> Result<(), OrderBookError> {
    if *time_in_force == TimeInForce::GTD && expire_time.is_none() {
        return Err(OrderBookError::InvalidExpireTime(
            "GTD (Good-Till-Date) orders must have an expire_time".to_string(),
        ));
    }
    Ok(())
}

// ============================================================================
// Composite Validation Function
// ============================================================================

/// Validate an order before processing
///
/// This is the **single entry point** for all order validation. It calls all
/// individual validation functions and returns the first error encountered.
///
/// # Validations Performed
/// 1. Quantity must be positive
/// 2. Price must be valid for the order type
/// 3. GTD orders must have an expire_time
///
/// # Arguments
/// * `order` - The order to validate
///
/// # Returns
/// * `Ok(())` if all validations pass
/// * `Err(OrderBookError)` with the specific validation failure
///
/// # Example
/// ```ignore
/// let order = Order::new(...);
/// validate_order(&order)?; // Will return early if validation fails
/// // Continue processing...
/// ```
pub fn validate_order(order: &Order) -> Result<(), OrderBookError> {
    validate_quantity(order.quantity)?;
    validate_price(order.price, &order.order_type)?;
    validate_expire_time(&order.time_in_force, order.expire_time)?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_validate_quantity_positive() {
        assert!(validate_quantity(dec!(100)).is_ok());
        assert!(validate_quantity(dec!(0.001)).is_ok());
    }

    #[test]
    fn test_validate_quantity_invalid() {
        assert!(validate_quantity(dec!(0)).is_err());
        assert!(validate_quantity(dec!(-10)).is_err());

        let err = validate_quantity(dec!(-5)).unwrap_err();
        assert!(err.to_string().contains("-5"));
    }

    #[test]
    fn test_validate_price_limit_order() {
        // Valid limit order with price
        assert!(validate_price(Some(dec!(100)), &OrderType::Limit).is_ok());

        // Invalid: limit order without price
        assert!(validate_price(None, &OrderType::Limit).is_err());

        // Invalid: negative price
        assert!(validate_price(Some(dec!(-50)), &OrderType::Limit).is_err());
    }

    #[test]
    fn test_validate_price_market_order() {
        // Market orders don't need a price
        assert!(validate_price(None, &OrderType::Market).is_ok());

        // But if provided, it must be positive
        assert!(validate_price(Some(dec!(100)), &OrderType::Market).is_ok());
        assert!(validate_price(Some(dec!(-50)), &OrderType::Market).is_err());
    }

    #[test]
    fn test_validate_expire_time_gtd() {
        // GTD requires expire_time
        assert!(validate_expire_time(&TimeInForce::GTD, None).is_err());

        // GTD with expire_time is valid
        assert!(validate_expire_time(&TimeInForce::GTD, Some(Utc::now())).is_ok());
    }

    #[test]
    fn test_validate_expire_time_other_tif() {
        // Other TIF types don't require expire_time
        assert!(validate_expire_time(&TimeInForce::GTC, None).is_ok());
        assert!(validate_expire_time(&TimeInForce::IOC, None).is_ok());
        assert!(validate_expire_time(&TimeInForce::FOK, None).is_ok());
        assert!(validate_expire_time(&TimeInForce::DAY, None).is_ok());
    }
}
