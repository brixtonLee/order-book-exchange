// Library Crate Root
// lib.rs

// pub mod xxx declares xxx module exists in the same crate
// lib.rs is the public API contract for your library crate when other crates using it
// main.rs (if you have it) also imports through lib.rs like an external crate
pub mod algorithms;
pub mod api;
pub mod ctrader_fix;
pub mod database;
pub mod datasource;
pub mod disruptor;
pub mod engine;
pub mod jobs;
pub mod market_data;
pub mod metrics;
pub mod models;
pub mod persistence;
pub mod protocol;
pub mod rabbitmq;
pub mod risk;
pub mod testing;
pub mod utils;
pub mod websocket;

// pub use = re-export at crate root
pub use api::{create_router, AppState};
pub use datasource::DatasourceManager;
pub use engine::{OrderBookEngine, OrderBookError};
pub use models::{Order, OrderBook, OrderSide, OrderStatus, OrderType, Trade};
pub use websocket::Broadcaster;