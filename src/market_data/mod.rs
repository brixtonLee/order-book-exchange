/// Market data distribution module
///
/// This module provides centralized tick distribution using a pub/sub pattern.
/// The TickDistributor receives ticks from the FIX client and broadcasts them
/// to all registered consumers (WebSocket, RabbitMQ, TickQueue, etc.).

pub mod tick_distributor;

pub use tick_distributor::{TickDistributor, TickDistributorStats};
