//! Order Book Engine Module
//!
//! This module contains the core order book functionality:
//! - `errors` - Error types for order book operations
//! - `validation` - Order validation functions
//! - `fees` - Fee calculation utilities
//! - `matching` - Order matching engine
//! - `orderbook` - Main order book engine

pub mod errors;
pub mod fees;
pub mod matching;
pub mod orderbook;
pub mod validation;

// Re-export commonly used types for convenience
pub use errors::OrderBookError;
pub use fees::{calculate_exchange_profit, calculate_maker_fee, calculate_taker_fee};
pub use matching::{match_order, MatchingError};
pub use orderbook::OrderBookEngine;
pub use validation::validate_order;
