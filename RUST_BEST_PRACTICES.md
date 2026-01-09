# Rust Best Practices: The Complete Guide

**Production-ready patterns and principles for building robust Rust applications**

---

## Table of Contents

1. [Project Structure & Organization](#1-project-structure--organization)
2. [Module System & Code Organization](#2-module-system--code-organization)
3. [SOLID Principles in Rust](#3-solid-principles-in-rust)
4. [Error Handling Patterns](#4-error-handling-patterns)
5. [Testing Strategies](#5-testing-strategies)
6. [Type System Best Practices](#6-type-system-best-practices)
7. [Memory & Performance](#7-memory--performance)
8. [Concurrency Patterns](#8-concurrency-patterns)
9. [API Design](#9-api-design)
10. [Documentation & Code Quality](#10-documentation--code-quality)
11. [Common Anti-Patterns to Avoid](#11-common-anti-patterns-to-avoid)
12. [Production Checklist](#12-production-checklist)

---

## 1. Project Structure & Organization

### Standard Cargo Project Layout

```
my-exchange/
├── Cargo.toml                 # Package manifest
├── Cargo.lock                 # Dependency lock file
├── rust-toolchain.toml        # Rust version pinning
├── .rustfmt.toml             # Formatting config
├── .clippy.toml              # Linter config
├── build.rs                   # Build script (if needed)
├── benches/                   # Benchmarks
│   └── matching_bench.rs
├── examples/                  # Example programs
│   └── simple_client.rs
├── src/
│   ├── main.rs               # Binary entry point
│   ├── lib.rs                # Library root (if hybrid crate)
│   ├── bin/                  # Additional binaries
│   │   └── admin_tool.rs
│   └── [modules...]          # Your code modules
├── tests/                     # Integration tests
│   └── integration_test.rs
├── migrations/                # Database migrations
│   └── 001_initial.sql
├── docs/                      # Documentation
│   ├── architecture/
│   ├── api/
│   └── deployment/
├── scripts/                   # Build/deployment scripts
│   └── deploy.sh
└── .github/                   # GitHub specific
    └── workflows/
        └── ci.yml
```

### Workspace Organization (Multi-crate)

```
exchange-workspace/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   ├── exchange-core/         # Core business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── exchange-api/          # HTTP API
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── exchange-websocket/    # WebSocket server
│   │   ├── Cargo.toml
│   │   └── src/
│   └── exchange-common/       # Shared types
│       ├── Cargo.toml
│       └── src/
└── services/                   # Deployable services
    ├── trading-engine/
    └── market-data-feed/
```

**Workspace Cargo.toml:**
```toml
[workspace]
members = [
    "crates/exchange-core",
    "crates/exchange-api",
    "crates/exchange-websocket",
    "crates/exchange-common",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies with consistent versions
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
rust_decimal = "1.33"
```

### Module File Organization

```
src/
├── lib.rs                     # Public API surface
├── models/                    # Domain models
│   ├── mod.rs                # Module declaration
│   ├── order.rs              # Order types
│   ├── trade.rs              # Trade types
│   └── book.rs               # OrderBook types
├── engine/                    # Business logic
│   ├── mod.rs
│   ├── matching.rs           # Core matching
│   ├── validation.rs         # Validation logic
│   └── fees.rs               # Fee calculation
├── adapters/                  # External integrations
│   ├── mod.rs
│   ├── postgres.rs           # Database adapter
│   ├── redis.rs              # Cache adapter
│   └── kafka.rs              # Message queue
├── api/                       # API layer
│   ├── mod.rs
│   ├── routes.rs             # Route definitions
│   ├── handlers.rs           # Request handlers
│   └── middleware.rs         # Custom middleware
└── utils/                     # Utilities
    ├── mod.rs
    └── metrics.rs            # Metrics helpers
```

### Best Practices

✅ **DO:**
- Keep `main.rs` thin - just initialization and wiring
- Use workspaces for multi-service projects
- Separate concerns into distinct modules
- Keep test files next to implementation for unit tests
- Use `tests/` directory for integration tests

❌ **DON'T:**
- Mix business logic with I/O operations
- Create deeply nested module hierarchies (> 3 levels)
- Put all code in `main.rs` or `lib.rs`
- Use `mod.rs` for implementation (only for re-exports)

---

## 2. Module System & Code Organization

### Module Declaration Patterns

#### Modern Style (Preferred since Rust 2018)

```rust
// src/engine/mod.rs
pub mod matching;
pub mod validation;
pub mod fees;

// Re-export commonly used items
pub use matching::match_order;
pub use validation::validate_order;

// src/engine/matching.rs
pub fn match_order() { /* ... */ }
```

#### Module Organization Best Practices

```rust
// src/models/mod.rs - Clean re-exports
mod order;
mod trade;
mod book;

// Re-export all public items
pub use order::{Order, OrderSide, OrderType, OrderStatus};
pub use trade::{Trade, TradeSide};
pub use book::{OrderBook, PriceLevel};

// Or selective re-exports with rename
pub use order::Order as ExchangeOrder;
```

### Visibility Rules

```rust
// src/engine/internal.rs
pub(crate) fn internal_helper() { }  // Visible within crate
pub(super) fn parent_only() { }      // Visible to parent module
pub(in crate::engine) fn engine_only() { } // Visible within engine module

pub struct Order {
    pub id: String,               // Public field
    pub(crate) internal_id: u64,  // Crate-visible field
    timestamp: DateTime<Utc>,     // Private field
}

impl Order {
    pub fn new() -> Self { }      // Public method
    pub(crate) fn internal() { }  // Crate-only method
    fn helper() { }                // Private method
}
```

### Module Boundaries

```rust
// ✅ GOOD: Clear module boundaries
// src/engine/mod.rs
pub struct Engine {
    // Private implementation
}

impl Engine {
    pub fn submit_order(&mut self, order: Order) -> Result<Trade> {
        // Public API
    }
}

// ❌ BAD: Exposing internals
pub struct Engine {
    pub order_book: OrderBook,  // Don't expose internals!
    pub pending_orders: Vec<Order>,
}
```

### Feature Flags

```toml
# Cargo.toml
[features]
default = ["postgres", "metrics"]
postgres = ["sqlx/postgres"]
metrics = ["prometheus", "metrics-exporter-prometheus"]
test-utils = []  # Test-only utilities

[dependencies]
sqlx = { version = "0.7", optional = true }
prometheus = { version = "0.13", optional = true }
```

```rust
// Conditional compilation
#[cfg(feature = "metrics")]
pub mod metrics {
    pub fn record_latency(duration: Duration) { }
}

#[cfg(not(feature = "metrics"))]
pub mod metrics {
    pub fn record_latency(_duration: Duration) { }  // No-op
}

// In code
metrics::record_latency(start.elapsed());  // Works with or without feature
```

---

## 3. SOLID Principles in Rust

### Single Responsibility Principle (SRP)

```rust
// ✅ GOOD: Each struct has one responsibility
pub struct OrderBook {
    bids: BTreeMap<Decimal, Vec<Order>>,
    asks: BTreeMap<Decimal, Vec<Order>>,
}

impl OrderBook {
    pub fn add_order(&mut self, order: Order) { }
    pub fn remove_order(&mut self, id: &str) { }
    pub fn get_best_bid(&self) -> Option<&Order> { }
}

pub struct OrderMatcher {
    // Only responsible for matching logic
}

impl OrderMatcher {
    pub fn match_order(&self, order: &Order, book: &mut OrderBook) -> Vec<Trade> { }
}

pub struct FeeCalculator {
    // Only responsible for fee calculation
}

impl FeeCalculator {
    pub fn calculate_fee(&self, trade: &Trade) -> Decimal { }
}

// ❌ BAD: Multiple responsibilities
pub struct TradingEngine {
    order_book: OrderBook,

    // Too many responsibilities in one struct!
    pub fn add_order(&mut self, order: Order) { }
    pub fn match_orders(&mut self) { }
    pub fn calculate_fees(&self, trade: &Trade) -> Decimal { }
    pub fn send_notification(&self, user_id: &str, message: &str) { }
    pub fn save_to_database(&self) { }
}
```

### Open/Closed Principle (OCP)

```rust
// ✅ GOOD: Open for extension, closed for modification

// Define trait for extensibility
pub trait PriceStrategy: Send + Sync {
    fn calculate_price(&self, order: &Order) -> Decimal;
}

// Implementations can be added without modifying existing code
pub struct MarketPriceStrategy;
impl PriceStrategy for MarketPriceStrategy {
    fn calculate_price(&self, _order: &Order) -> Decimal {
        // Get current market price
        Decimal::from(100)
    }
}

pub struct LimitPriceStrategy;
impl PriceStrategy for LimitPriceStrategy {
    fn calculate_price(&self, order: &Order) -> Decimal {
        order.price
    }
}

pub struct PeggedPriceStrategy {
    offset: Decimal,
}
impl PriceStrategy for PeggedPriceStrategy {
    fn calculate_price(&self, order: &Order) -> Decimal {
        // Calculate based on reference price + offset
        order.price + self.offset
    }
}

// Engine uses trait - no modification needed for new strategies
pub struct OrderEngine {
    price_strategy: Box<dyn PriceStrategy>,
}

impl OrderEngine {
    pub fn process_order(&self, order: &Order) -> Decimal {
        self.price_strategy.calculate_price(order)
    }
}
```

### Liskov Substitution Principle (LSP)

```rust
// ✅ GOOD: Subtypes maintain parent contract

pub trait OrderValidator {
    fn validate(&self, order: &Order) -> Result<(), ValidationError>;
}

pub struct BasicValidator;
impl OrderValidator for BasicValidator {
    fn validate(&self, order: &Order) -> Result<(), ValidationError> {
        if order.quantity <= Decimal::ZERO {
            return Err(ValidationError::InvalidQuantity);
        }
        Ok(())
    }
}

pub struct StrictValidator;
impl OrderValidator for StrictValidator {
    fn validate(&self, order: &Order) -> Result<(), ValidationError> {
        // Stricter validation, but same contract
        if order.quantity <= Decimal::ZERO {
            return Err(ValidationError::InvalidQuantity);
        }
        if order.price <= Decimal::ZERO {
            return Err(ValidationError::InvalidPrice);
        }
        Ok(())
    }
}

// Both validators can be used interchangeably
fn process_with_validation(
    validator: &dyn OrderValidator,
    order: &Order
) -> Result<(), ValidationError> {
    validator.validate(order)?;
    // Process order
    Ok(())
}

// ❌ BAD: Breaking LSP
pub struct BrokenValidator;
impl OrderValidator for BrokenValidator {
    fn validate(&self, order: &Order) -> Result<(), ValidationError> {
        // This panics instead of returning error - breaks contract!
        assert!(order.quantity > Decimal::ZERO);
        Ok(())
    }
}
```

### Interface Segregation Principle (ISP)

```rust
// ✅ GOOD: Small, focused traits

pub trait OrderReader {
    fn get_order(&self, id: &str) -> Option<Order>;
    fn list_orders(&self, limit: usize) -> Vec<Order>;
}

pub trait OrderWriter {
    fn save_order(&mut self, order: Order) -> Result<(), Error>;
    fn delete_order(&mut self, id: &str) -> Result<(), Error>;
}

pub trait OrderNotifier {
    fn notify_order_created(&self, order: &Order);
    fn notify_order_cancelled(&self, order_id: &str);
}

// Clients only depend on what they need
struct OrderQueryService<R: OrderReader> {
    reader: R,
}

struct OrderCommandService<W: OrderWriter, N: OrderNotifier> {
    writer: W,
    notifier: N,
}

// ❌ BAD: Fat interface
pub trait OrderRepository {
    fn get_order(&self, id: &str) -> Option<Order>;
    fn save_order(&mut self, order: Order) -> Result<(), Error>;
    fn delete_order(&mut self, id: &str) -> Result<(), Error>;
    fn list_orders(&self, limit: usize) -> Vec<Order>;
    fn notify_order_created(&self, order: &Order);
    fn notify_order_cancelled(&self, order_id: &str);
    fn calculate_statistics(&self) -> Statistics;  // Not everyone needs this!
}
```

### Dependency Inversion Principle (DIP)

```rust
// ✅ GOOD: Depend on abstractions, not concretions

// Define abstraction (trait)
pub trait OrderRepository: Send + Sync {
    async fn save(&self, order: &Order) -> Result<(), Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Order>, Error>;
}

pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: OrderEvent) -> Result<(), Error>;
}

// High-level module depends on abstractions
pub struct OrderService<R: OrderRepository, P: EventPublisher> {
    repository: R,
    publisher: P,
}

impl<R: OrderRepository, P: EventPublisher> OrderService<R, P> {
    pub async fn create_order(&self, order: Order) -> Result<(), Error> {
        self.repository.save(&order).await?;
        self.publisher.publish(OrderEvent::Created(order)).await?;
        Ok(())
    }
}

// Low-level modules implement abstractions
pub struct PostgresOrderRepository {
    pool: PgPool,
}

impl OrderRepository for PostgresOrderRepository {
    async fn save(&self, order: &Order) -> Result<(), Error> {
        // PostgreSQL implementation
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Order>, Error> {
        // PostgreSQL implementation
        Ok(None)
    }
}

pub struct KafkaEventPublisher {
    producer: KafkaProducer,
}

impl EventPublisher for KafkaEventPublisher {
    async fn publish(&self, event: OrderEvent) -> Result<(), Error> {
        // Kafka implementation
        Ok(())
    }
}

// ❌ BAD: High-level module depends on concrete implementation
pub struct BadOrderService {
    // Direct dependency on concrete types
    repository: PostgresOrderRepository,
    publisher: KafkaEventPublisher,
}
```

---

## 4. Error Handling Patterns

### Error Type Design

```rust
use thiserror::Error;

// ✅ GOOD: Rich error types with context
#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order not found: {id}")]
    NotFound { id: String },

    #[error("Invalid quantity: {quantity} (must be > 0)")]
    InvalidQuantity { quantity: Decimal },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: Decimal,
        available: Decimal,
    },

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),

    #[error("Unexpected error: {0}")]
    Internal(String),
}

// Custom Result type alias
pub type Result<T> = std::result::Result<T, OrderError>;

// Usage
pub fn validate_order(order: &Order) -> Result<()> {
    if order.quantity <= Decimal::ZERO {
        return Err(OrderError::InvalidQuantity {
            quantity: order.quantity,
        });
    }
    Ok(())
}
```

### Error Propagation Patterns

```rust
// ✅ GOOD: Use ? operator for clean propagation
pub async fn process_order(order: Order) -> Result<Trade> {
    validate_order(&order)?;
    let account = get_account(&order.account_id).await?;
    check_balance(&account, &order)?;
    let trade = execute_order(order).await?;
    Ok(trade)
}

// ✅ GOOD: Add context when propagating
use anyhow::Context;

pub async fn load_config() -> anyhow::Result<Config> {
    let contents = std::fs::read_to_string("config.toml")
        .context("Failed to read config file")?;

    let config: Config = toml::from_str(&contents)
        .context("Failed to parse config file")?;

    Ok(config)
}

// ✅ GOOD: Map errors with more context
pub async fn get_order(id: &str) -> Result<Order> {
    repository::find_order(id)
        .await
        .map_err(|e| OrderError::Database(e))?
        .ok_or_else(|| OrderError::NotFound { id: id.to_string() })
}

// ❌ BAD: Using unwrap in production code
pub fn bad_example(order: Order) -> Trade {
    let validated = validate_order(&order).unwrap();  // Can panic!
    let trade = execute_order(order).unwrap();  // Can panic!
    trade
}

// ❌ BAD: Ignoring errors
pub async fn fire_and_forget(order: Order) {
    let _ = process_order(order).await;  // Error ignored!
}
```

### Recovery Patterns

```rust
// ✅ GOOD: Graceful fallbacks
pub async fn get_price(symbol: &str) -> Decimal {
    match fetch_live_price(symbol).await {
        Ok(price) => price,
        Err(e) => {
            tracing::warn!("Failed to fetch live price: {}", e);
            get_cached_price(symbol).unwrap_or(Decimal::ZERO)
        }
    }
}

// ✅ GOOD: Retry with exponential backoff
use tokio_retry::{Retry, strategy::ExponentialBackoff};

pub async fn reliable_send(message: Message) -> Result<()> {
    let retry_strategy = ExponentialBackoff::from_millis(100)
        .max_delay(Duration::from_secs(10))
        .take(5);  // Max 5 retries

    Retry::spawn(retry_strategy, || async {
        send_message(&message).await
    })
    .await
}

// ✅ GOOD: Circuit breaker pattern
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: RwLock<Option<Instant>>,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        // Check if circuit is open
        if self.is_open() {
            return Err(OrderError::Internal("Circuit breaker open".into()));
        }

        match f.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(e)
            }
        }
    }
}
```

---

## 5. Testing Strategies

### Unit Testing Best Practices

```rust
// src/engine/matching.rs
pub fn calculate_match_quantity(
    buy_order: &Order,
    sell_order: &Order
) -> Decimal {
    buy_order.quantity.min(sell_order.quantity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ✅ GOOD: Descriptive test names
    #[test]
    fn calculate_match_quantity_returns_smaller_quantity() {
        let buy = Order {
            quantity: dec!(100),
            ..Default::default()
        };
        let sell = Order {
            quantity: dec!(50),
            ..Default::default()
        };

        assert_eq!(calculate_match_quantity(&buy, &sell), dec!(50));
    }

    // ✅ GOOD: Test edge cases
    #[test]
    fn calculate_match_quantity_handles_zero_quantity() {
        let buy = Order {
            quantity: dec!(0),
            ..Default::default()
        };
        let sell = Order {
            quantity: dec!(100),
            ..Default::default()
        };

        assert_eq!(calculate_match_quantity(&buy, &sell), dec!(0));
    }

    // ✅ GOOD: Property-based testing
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn match_quantity_never_exceeds_orders(
            buy_qty in 0.0f64..10000.0,
            sell_qty in 0.0f64..10000.0
        ) {
            let buy = Order {
                quantity: Decimal::from_f64(buy_qty).unwrap(),
                ..Default::default()
            };
            let sell = Order {
                quantity: Decimal::from_f64(sell_qty).unwrap(),
                ..Default::default()
            };

            let matched = calculate_match_quantity(&buy, &sell);
            prop_assert!(matched <= buy.quantity);
            prop_assert!(matched <= sell.quantity);
        }
    }
}
```

### Integration Testing

```rust
// tests/integration_test.rs
use order_book_exchange::{OrderBookEngine, Order, OrderSide};

// ✅ GOOD: Test complete workflows
#[tokio::test]
async fn test_order_matching_workflow() {
    // Setup
    let engine = OrderBookEngine::new();

    // Submit buy order
    let buy_order = Order {
        id: "buy-1".into(),
        symbol: "XAUUSD".into(),
        side: OrderSide::Buy,
        price: dec!(2000),
        quantity: dec!(10),
        ..Default::default()
    };

    engine.submit_order(buy_order).await.unwrap();

    // Submit matching sell order
    let sell_order = Order {
        id: "sell-1".into(),
        symbol: "XAUUSD".into(),
        side: OrderSide::Sell,
        price: dec!(2000),
        quantity: dec!(5),
        ..Default::default()
    };

    let trades = engine.submit_order(sell_order).await.unwrap();

    // Verify
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, dec!(5));
    assert_eq!(trades[0].price, dec!(2000));
}
```

### Test Organization

```rust
// ✅ GOOD: Organize tests by behavior
#[cfg(test)]
mod tests {
    use super::*;

    mod order_validation {
        use super::*;

        #[test]
        fn rejects_negative_quantity() { }

        #[test]
        fn rejects_zero_price() { }

        #[test]
        fn accepts_valid_order() { }
    }

    mod order_matching {
        use super::*;

        #[test]
        fn matches_at_same_price() { }

        #[test]
        fn no_match_when_prices_cross() { }
    }

    mod fee_calculation {
        use super::*;

        #[test]
        fn calculates_maker_fee() { }

        #[test]
        fn calculates_taker_fee() { }
    }
}
```

### Test Fixtures and Builders

```rust
// ✅ GOOD: Test data builders
#[cfg(test)]
mod test_helpers {
    use super::*;

    pub struct OrderBuilder {
        order: Order,
    }

    impl OrderBuilder {
        pub fn new() -> Self {
            Self {
                order: Order {
                    id: uuid::Uuid::new_v4().to_string(),
                    symbol: "TEST".into(),
                    side: OrderSide::Buy,
                    price: dec!(100),
                    quantity: dec!(10),
                    timestamp: Utc::now(),
                    ..Default::default()
                },
            }
        }

        pub fn with_symbol(mut self, symbol: &str) -> Self {
            self.order.symbol = symbol.into();
            self
        }

        pub fn with_price(mut self, price: Decimal) -> Self {
            self.order.price = price;
            self
        }

        pub fn with_side(mut self, side: OrderSide) -> Self {
            self.order.side = side;
            self
        }

        pub fn build(self) -> Order {
            self.order
        }
    }

    // Usage in tests
    #[test]
    fn test_with_builder() {
        let order = OrderBuilder::new()
            .with_symbol("XAUUSD")
            .with_price(dec!(2000))
            .with_side(OrderSide::Sell)
            .build();

        assert_eq!(order.symbol, "XAUUSD");
    }
}
```

### Mocking and Dependency Injection

```rust
// ✅ GOOD: Use traits for mockable dependencies
#[cfg_attr(test, mockall::automock)]
pub trait PriceProvider: Send + Sync {
    async fn get_price(&self, symbol: &str) -> Result<Decimal>;
}

pub struct OrderValidator<P: PriceProvider> {
    price_provider: P,
}

impl<P: PriceProvider> OrderValidator<P> {
    pub async fn validate_limit_price(&self, order: &Order) -> Result<()> {
        let market_price = self.price_provider.get_price(&order.symbol).await?;

        // Validate limit price is reasonable (within 10% of market)
        let diff_pct = ((order.price - market_price).abs() / market_price) * dec!(100);

        if diff_pct > dec!(10) {
            return Err(OrderError::PriceTooFarFromMarket);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_validate_limit_price() {
        let mut mock_provider = MockPriceProvider::new();

        mock_provider
            .expect_get_price()
            .with(eq("XAUUSD"))
            .times(1)
            .returning(|_| Ok(dec!(2000)));

        let validator = OrderValidator {
            price_provider: mock_provider,
        };

        let order = Order {
            symbol: "XAUUSD".into(),
            price: dec!(2050),  // Within 10%
            ..Default::default()
        };

        assert!(validator.validate_limit_price(&order).await.is_ok());
    }
}
```

### Test Coverage and Quality

```bash
# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage

# Run mutation testing
cargo install cargo-mutants
cargo mutants

# Property-based testing
# Add to Cargo.toml:
[dev-dependencies]
proptest = "1.0"
quickcheck = "1.0"

# Fuzzing
cargo install cargo-fuzz
cargo fuzz init
cargo fuzz run fuzz_target
```

---

## 6. Type System Best Practices

### NewType Pattern

```rust
// ✅ GOOD: Type safety through NewType pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(String);

impl OrderId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(Decimal);

impl Price {
    pub fn new(value: Decimal) -> Result<Self, ValidationError> {
        if value <= Decimal::ZERO {
            return Err(ValidationError::InvalidPrice);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> Decimal {
        self.0
    }
}

// Now impossible to mix up parameters
fn submit_order(id: OrderId, price: Price) {
    // Type safe!
}

// ❌ BAD: Primitive obsession
fn bad_submit_order(id: String, price: Decimal) {
    // Easy to mix up parameters
}
```

### Type State Pattern

```rust
// ✅ GOOD: Compile-time state machine
use std::marker::PhantomData;

// State markers
struct Draft;
struct Validated;
struct Submitted;

pub struct Order<State> {
    id: String,
    symbol: String,
    price: Decimal,
    quantity: Decimal,
    _state: PhantomData<State>,
}

impl Order<Draft> {
    pub fn new(symbol: String, price: Decimal, quantity: Decimal) -> Self {
        Order {
            id: uuid::Uuid::new_v4().to_string(),
            symbol,
            price,
            quantity,
            _state: PhantomData,
        }
    }

    pub fn validate(self) -> Result<Order<Validated>, ValidationError> {
        if self.price <= Decimal::ZERO {
            return Err(ValidationError::InvalidPrice);
        }
        if self.quantity <= Decimal::ZERO {
            return Err(ValidationError::InvalidQuantity);
        }

        Ok(Order {
            id: self.id,
            symbol: self.symbol,
            price: self.price,
            quantity: self.quantity,
            _state: PhantomData,
        })
    }
}

impl Order<Validated> {
    pub fn submit(self) -> Order<Submitted> {
        // Can only submit validated orders
        Order {
            id: self.id,
            symbol: self.symbol,
            price: self.price,
            quantity: self.quantity,
            _state: PhantomData,
        }
    }
}

// Usage - compile-time guarantees!
let order = Order::<Draft>::new("XAUUSD".into(), dec!(2000), dec!(10));
let validated = order.validate()?;  // Must validate
let submitted = validated.submit(); // Can only submit after validation
```

### Builder Pattern with Types

```rust
// ✅ GOOD: Type-safe builder
pub struct OrderBuilder<Symbol = (), Price = (), Quantity = ()> {
    symbol: Symbol,
    price: Price,
    quantity: Quantity,
}

impl OrderBuilder {
    pub fn new() -> Self {
        OrderBuilder {
            symbol: (),
            price: (),
            quantity: (),
        }
    }
}

impl<P, Q> OrderBuilder<(), P, Q> {
    pub fn symbol(self, symbol: String) -> OrderBuilder<String, P, Q> {
        OrderBuilder {
            symbol,
            price: self.price,
            quantity: self.quantity,
        }
    }
}

impl<S, Q> OrderBuilder<S, (), Q> {
    pub fn price(self, price: Decimal) -> OrderBuilder<S, Decimal, Q> {
        OrderBuilder {
            symbol: self.symbol,
            price,
            quantity: self.quantity,
        }
    }
}

impl<S, P> OrderBuilder<S, P, ()> {
    pub fn quantity(self, quantity: Decimal) -> OrderBuilder<S, P, Decimal> {
        OrderBuilder {
            symbol: self.symbol,
            price: self.price,
            quantity,
        }
    }
}

// Can only build when all required fields are set
impl OrderBuilder<String, Decimal, Decimal> {
    pub fn build(self) -> Order {
        Order {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: self.symbol,
            price: self.price,
            quantity: self.quantity,
        }
    }
}

// Usage - won't compile without all fields
let order = OrderBuilder::new()
    .symbol("XAUUSD".into())
    .price(dec!(2000))
    .quantity(dec!(10))
    .build();  // Only available when all fields set
```

### Smart Use of Option and Result

```rust
// ✅ GOOD: Express intent through types
pub struct OrderBook {
    bids: BTreeMap<Price, Vec<Order>>,
    asks: BTreeMap<Price, Vec<Order>>,
}

impl OrderBook {
    // Option indicates order might not exist
    pub fn get_order(&self, id: &OrderId) -> Option<&Order> {
        self.bids
            .values()
            .chain(self.asks.values())
            .flat_map(|orders| orders.iter())
            .find(|order| order.id == *id)
    }

    // Result indicates operation can fail
    pub fn cancel_order(&mut self, id: &OrderId) -> Result<Order, CancelError> {
        self.remove_order(id)
            .ok_or(CancelError::OrderNotFound)
    }

    // No Option/Result means operation always succeeds
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }
}
```

---

## 7. Memory & Performance

### Zero-Copy Patterns

```rust
// ✅ GOOD: Avoid unnecessary allocations
use bytes::Bytes;

pub struct Message {
    // Use Bytes for zero-copy cloning
    payload: Bytes,
}

impl Message {
    pub fn parse(data: Bytes) -> Self {
        // No allocation, just reference counting
        Message { payload: data }
    }

    pub fn get_field(&self, start: usize, end: usize) -> Bytes {
        // Zero-copy slice
        self.payload.slice(start..end)
    }
}

// ✅ GOOD: Borrow instead of clone
fn process_orders(orders: &[Order]) {  // Borrow slice
    for order in orders {
        process_single(order);  // Pass reference
    }
}

// ❌ BAD: Unnecessary cloning
fn bad_process_orders(orders: Vec<Order>) {  // Takes ownership
    for order in orders {
        process_single(order.clone());  // Unnecessary clone!
    }
}
```

### Memory Layout Optimization

```rust
// ✅ GOOD: Optimize struct layout
#[repr(C)]  // Predictable layout
pub struct Order {
    // Group by size for better packing (largest to smallest)
    price: Decimal,      // 16 bytes
    quantity: Decimal,   // 16 bytes
    timestamp: i64,      // 8 bytes
    id: u64,            // 8 bytes
    side: OrderSide,    // 1 byte
    order_type: OrderType, // 1 byte
    // Total: 50 bytes with no padding
}

// ❌ BAD: Poor memory layout
pub struct BadOrder {
    side: OrderSide,     // 1 byte
    price: Decimal,      // 16 bytes (padding before!)
    order_type: OrderType, // 1 byte
    quantity: Decimal,   // 16 bytes (padding before!)
    // Lots of wasted padding
}

// ✅ GOOD: Use Box for large, rarely-accessed fields
pub struct Trade {
    id: u64,
    price: Decimal,
    quantity: Decimal,
    metadata: Box<TradeMetadata>,  // Large struct on heap
}
```

### Collection Capacity Management

```rust
// ✅ GOOD: Pre-allocate collections
pub struct OrderBook {
    orders: Vec<Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            // Pre-allocate typical capacity
            orders: Vec::with_capacity(10_000),
        }
    }

    pub fn bulk_insert(&mut self, new_orders: &[Order]) {
        // Reserve exact additional capacity needed
        self.orders.reserve(new_orders.len());
        self.orders.extend_from_slice(new_orders);
    }
}

// ✅ GOOD: Reuse allocations
pub struct OrderProcessor {
    buffer: Vec<Order>,  // Reusable buffer
}

impl OrderProcessor {
    pub fn process_batch(&mut self, orders: Vec<Order>) {
        self.buffer.clear();  // Clear but keep capacity
        self.buffer.extend(orders);
        // Process buffer...
    }
}
```

### String Handling

```rust
// ✅ GOOD: Use &str when possible
pub fn validate_symbol(symbol: &str) -> bool {
    // No allocation needed
    symbol.len() <= 20 && symbol.chars().all(|c| c.is_ascii_alphanumeric())
}

// ✅ GOOD: Use Cow for conditional ownership
use std::borrow::Cow;

pub fn normalize_symbol(symbol: &str) -> Cow<str> {
    if symbol.chars().all(|c| c.is_ascii_uppercase()) {
        Cow::Borrowed(symbol)  // No allocation if already uppercase
    } else {
        Cow::Owned(symbol.to_uppercase())  // Allocate only if needed
    }
}

// ✅ GOOD: Intern strings for repeated values
use once_cell::sync::Lazy;
use std::collections::HashSet;

static SYMBOL_CACHE: Lazy<RwLock<HashSet<&'static str>>> = Lazy::new(Default::default);

pub fn intern_symbol(symbol: String) -> &'static str {
    let cache = SYMBOL_CACHE.read().unwrap();
    if let Some(&interned) = cache.get(symbol.as_str()) {
        return interned;
    }
    drop(cache);

    let mut cache = SYMBOL_CACHE.write().unwrap();
    let leaked: &'static str = Box::leak(symbol.into_boxed_str());
    cache.insert(leaked);
    leaked
}
```

---

## 8. Concurrency Patterns

### Shared State Management

```rust
// ✅ GOOD: Choose the right synchronization primitive
use std::sync::{Arc, RwLock, Mutex};
use parking_lot::RwLock as ParkingLotRwLock;
use dashmap::DashMap;

pub struct OrderBookEngine {
    // RwLock for read-heavy workloads
    order_books: Arc<RwLock<HashMap<String, OrderBook>>>,

    // DashMap for concurrent access without global lock
    active_orders: Arc<DashMap<OrderId, Order>>,

    // Mutex for write-heavy or short critical sections
    trade_counter: Arc<Mutex<u64>>,
}

// ✅ GOOD: Minimize lock scope
impl OrderBookEngine {
    pub fn get_order_book(&self, symbol: &str) -> Option<OrderBook> {
        // Hold read lock briefly
        let books = self.order_books.read().unwrap();
        books.get(symbol).cloned()  // Clone to release lock quickly
    }

    pub fn update_order_book(&self, symbol: String, updater: impl FnOnce(&mut OrderBook)) {
        // Clone, modify, replace pattern
        let mut book = {
            let books = self.order_books.read().unwrap();
            books.get(&symbol).cloned().unwrap_or_default()
        };

        updater(&mut book);

        let mut books = self.order_books.write().unwrap();
        books.insert(symbol, book);
    }
}
```

### Channel Patterns

```rust
// ✅ GOOD: Choose appropriate channel type
use tokio::sync::{mpsc, broadcast, watch, oneshot};

pub struct TradingSystem {
    // mpsc for single consumer (order processor)
    order_tx: mpsc::Sender<Order>,

    // broadcast for multiple consumers (multiple subscribers)
    trade_broadcast: broadcast::Sender<Trade>,

    // watch for shared state updates (latest price)
    price_watch: watch::Sender<Decimal>,

    // oneshot for request-response
    shutdown_tx: oneshot::Sender<()>,
}

// ✅ GOOD: Bounded channels for backpressure
pub async fn create_order_processor() -> mpsc::Sender<Order> {
    let (tx, mut rx) = mpsc::channel(1000);  // Bounded capacity

    tokio::spawn(async move {
        while let Some(order) = rx.recv().await {
            process_order(order).await;
        }
    });

    tx
}

// ✅ GOOD: Select for multiple channels
use tokio::select;

pub async fn event_loop(
    mut orders: mpsc::Receiver<Order>,
    mut commands: mpsc::Receiver<Command>,
    mut shutdown: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            Some(order) = orders.recv() => {
                handle_order(order).await;
            }
            Some(cmd) = commands.recv() => {
                handle_command(cmd).await;
            }
            _ = &mut shutdown => {
                info!("Shutting down");
                break;
            }
        }
    }
}
```

### Task Management

```rust
// ✅ GOOD: Structured concurrency
use tokio::task::JoinSet;

pub async fn process_orders_parallel(orders: Vec<Order>) -> Vec<Result<Trade, Error>> {
    let mut tasks = JoinSet::new();

    for order in orders {
        tasks.spawn(async move {
            process_order(order).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(trade_result) => results.push(trade_result),
            Err(join_err) => results.push(Err(Error::from(join_err))),
        }
    }

    results
}

// ✅ GOOD: Graceful shutdown
pub struct Server {
    shutdown: tokio::sync::broadcast::Sender<()>,
}

impl Server {
    pub async fn run(self) {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

        let server = axum::serve(listener, app);

        tokio::select! {
            result = server => {
                error!("Server error: {:?}", result);
            }
            _ = shutdown_rx.recv() => {
                info!("Graceful shutdown initiated");
            }
        }
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}
```

---

## 9. API Design

### REST API Best Practices

```rust
// ✅ GOOD: Type-safe API with proper status codes
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

// Request/Response types
#[derive(Deserialize)]
pub struct CreateOrderRequest {
    symbol: String,
    side: OrderSide,
    #[serde(deserialize_with = "deserialize_decimal")]
    price: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    quantity: Decimal,
}

#[derive(Serialize)]
pub struct OrderResponse {
    id: String,
    status: OrderStatus,
    #[serde(serialize_with = "serialize_decimal")]
    filled_quantity: Decimal,
}

// API Error type
#[derive(Serialize)]
pub struct ApiError {
    error: String,
    details: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

// Handler with proper error handling
pub async fn create_order(
    State(engine): State<Arc<OrderBookEngine>>,
    Json(request): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), ApiError> {
    let order = Order::try_from(request)
        .map_err(|e| ApiError {
            error: "Invalid request".into(),
            details: Some(e.to_string()),
        })?;

    let result = engine.submit_order(order).await
        .map_err(|e| ApiError {
            error: "Order submission failed".into(),
            details: Some(e.to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(result.into())))
}

// Pagination
#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
}

fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 50 }

pub async fn list_orders(
    Query(params): Query<PaginationParams>,
) -> Json<Vec<OrderResponse>> {
    // Implement pagination
    Json(vec![])
}
```

### GraphQL Alternative

```rust
use async_graphql::{Object, Schema, Context};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn order(&self, ctx: &Context<'_>, id: String) -> Result<Order> {
        let engine = ctx.data::<Arc<OrderBookEngine>>()?;
        engine.get_order(&id)
            .ok_or_else(|| Error::new("Order not found"))
    }

    async fn orders(
        &self,
        ctx: &Context<'_>,
        symbol: Option<String>,
        limit: Option<i32>,
    ) -> Result<Vec<Order>> {
        let engine = ctx.data::<Arc<OrderBookEngine>>()?;
        let limit = limit.unwrap_or(100).min(1000);

        Ok(engine.list_orders(symbol.as_deref(), limit as usize))
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn submit_order(
        &self,
        ctx: &Context<'_>,
        input: CreateOrderInput,
    ) -> Result<Order> {
        let engine = ctx.data::<Arc<OrderBookEngine>>()?;
        engine.submit_order(input.into()).await
    }
}
```

---

## 10. Documentation & Code Quality

### Documentation Best Practices

```rust
//! # Order Book Exchange
//!
//! High-performance order matching engine with WebSocket streaming.
//!
//! ## Example
//!
//! ```rust
//! use order_book_exchange::{OrderBookEngine, Order};
//!
//! let engine = OrderBookEngine::new();
//! let order = Order::new("XAUUSD", OrderSide::Buy, 2000.0, 10.0);
//! let trades = engine.submit_order(order)?;
//! ```

/// Represents a limit order in the order book.
///
/// Orders are matched using price-time priority where orders at the same
/// price level are matched in FIFO order.
///
/// # Examples
///
/// ```
/// let order = Order::builder()
///     .symbol("XAUUSD")
///     .side(OrderSide::Buy)
///     .price(2000.0)
///     .quantity(10.0)
///     .build()?;
/// ```
pub struct Order {
    /// Unique order identifier
    pub id: OrderId,

    /// Trading symbol (e.g., "XAUUSD", "BTCUSD")
    pub symbol: String,

    // ... other fields
}

impl Order {
    /// Creates a new order with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading symbol
    /// * `side` - Buy or Sell
    /// * `price` - Limit price (must be positive)
    /// * `quantity` - Order quantity (must be positive)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Order)` if validation passes, otherwise returns
    /// `Err(ValidationError)` with details about the validation failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Price is zero or negative
    /// - Quantity is zero or negative
    /// - Symbol is empty or invalid
    pub fn new(
        symbol: impl Into<String>,
        side: OrderSide,
        price: Decimal,
        quantity: Decimal,
    ) -> Result<Self, ValidationError> {
        // Implementation
    }
}
```

### Code Quality Tools

```toml
# .rustfmt.toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Module"
group_imports = "StdExternalCrate"

# .clippy.toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 7

# Cargo.toml dev dependencies
[dev-dependencies]
criterion = "0.5"  # Benchmarking
proptest = "1.0"   # Property testing
mockall = "0.11"   # Mocking

# Run quality checks
# cargo fmt --check
# cargo clippy -- -D warnings
# cargo test
# cargo doc --no-deps --open
```

### Continuous Integration

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  RUST_BACKTRACE: 1

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Format
        run: cargo fmt --check

      - name: Lint
        run: cargo clippy -- -D warnings

      - name: Test
        run: cargo test --all-features

      - name: Doc
        run: cargo doc --no-deps

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Coverage
        run: cargo tarpaulin --out Xml

      - uses: codecov/codecov-action@v3
```

---

## 11. Common Anti-Patterns to Avoid

### ❌ Clone Everywhere

```rust
// ❌ BAD: Excessive cloning
fn process_orders(orders: Vec<Order>) {
    for order in orders {
        validate(order.clone());     // Unnecessary clone
        match_order(order.clone());  // Another clone!
        save(order.clone());         // Yet another!
    }
}

// ✅ GOOD: Use references
fn process_orders(orders: Vec<Order>) {
    for order in orders {
        validate(&order);
        let trades = match_order(&order);
        save(&order, &trades);
    }
}
```

### ❌ Stringly Typed Code

```rust
// ❌ BAD: Strings for everything
fn process_message(msg_type: &str, payload: &str) {
    match msg_type {
        "ORDER" => { /* ... */ }
        "CANCEL" => { /* ... */ }
        _ => { /* ... */ }
    }
}

// ✅ GOOD: Use enums
enum MessageType {
    Order(Order),
    Cancel(CancelRequest),
}

fn process_message(message: MessageType) {
    match message {
        MessageType::Order(order) => { /* ... */ }
        MessageType::Cancel(req) => { /* ... */ }
    }
}
```

### ❌ God Objects

```rust
// ❌ BAD: One struct does everything
struct TradingEngine {
    orders: Vec<Order>,
    trades: Vec<Trade>,
    users: HashMap<UserId, User>,

    fn submit_order(&mut self) { }
    fn match_orders(&mut self) { }
    fn calculate_fees(&self) { }
    fn send_notifications(&self) { }
    fn generate_reports(&self) { }
    fn backup_database(&self) { }
}

// ✅ GOOD: Separate concerns
struct OrderBook { /* ... */ }
struct MatchingEngine { /* ... */ }
struct FeeCalculator { /* ... */ }
struct NotificationService { /* ... */ }
struct ReportGenerator { /* ... */ }
```

### ❌ Ignoring Errors

```rust
// ❌ BAD: Silently ignoring errors
async fn save_order(order: Order) {
    let _ = database.insert(order).await;  // Error ignored!
}

// ✅ GOOD: Handle or propagate errors
async fn save_order(order: Order) -> Result<(), DatabaseError> {
    database.insert(order).await?;
    Ok(())
}
```

---

## 12. Production Checklist

### Pre-Production Checklist

```markdown
## Code Quality
- [ ] All tests passing (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation complete (`cargo doc`)
- [ ] No TODO comments in production paths
- [ ] Security audit (`cargo audit`)

## Performance
- [ ] Benchmarks meet requirements
- [ ] No unnecessary allocations in hot paths
- [ ] Appropriate data structures chosen
- [ ] Database queries optimized with indices
- [ ] Connection pooling configured

## Error Handling
- [ ] All Results handled (no unwrap/expect)
- [ ] Errors have context
- [ ] Graceful degradation implemented
- [ ] Circuit breakers for external services
- [ ] Retry logic with backoff

## Observability
- [ ] Structured logging configured
- [ ] Metrics exposed (Prometheus)
- [ ] Tracing enabled for requests
- [ ] Health check endpoint
- [ ] Alerts configured

## Security
- [ ] Input validation on all endpoints
- [ ] SQL injection prevention (parameterized queries)
- [ ] Rate limiting implemented
- [ ] Authentication/authorization configured
- [ ] Secrets in environment variables
- [ ] TLS enabled

## Operations
- [ ] Graceful shutdown handling
- [ ] Configuration externalized
- [ ] Database migrations versioned
- [ ] Rollback plan documented
- [ ] Load testing completed
- [ ] Monitoring dashboards created
- [ ] Runbook documentation written
```

### Deployment Configuration

```toml
# Cargo.toml production settings
[profile.release]
opt-level = 3
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Strip symbols
panic = "abort"      # Smaller binary

[profile.release-with-debug]
inherits = "release"
debug = true         # Keep debug symbols for profiling
```

### Environment Variables

```bash
# .env.example
# Server
HOST=0.0.0.0
PORT=8080
WORKERS=4

# Database
DATABASE_URL=postgresql://user:pass@localhost/exchange
DATABASE_MAX_CONNECTIONS=100
DATABASE_MIN_CONNECTIONS=10

# Redis
REDIS_URL=redis://localhost:6379

# Monitoring
METRICS_PORT=9090
LOG_LEVEL=info
SENTRY_DSN=https://...

# Security
JWT_SECRET=your-secret-key
RATE_LIMIT_PER_SECOND=100
```

---

## Summary

### Key Takeaways

1. **Structure**: Organize code into clear modules with defined boundaries
2. **Types**: Use Rust's type system to make invalid states unrepresentable
3. **Errors**: Always handle errors explicitly, never panic in production
4. **Testing**: Test at multiple levels (unit, integration, property-based)
5. **Performance**: Measure first, optimize what matters
6. **Concurrency**: Choose the right synchronization primitive for the job
7. **Documentation**: Document why, not what - code should be self-explanatory
8. **Monitoring**: You can't fix what you can't measure

### Quick Reference

| Principle | Do | Don't |
|-----------|-----|-------|
| **Ownership** | Use references when possible | Clone unnecessarily |
| **Errors** | Use Result<T, E> | Use panic/unwrap |
| **Types** | Create domain types | Use primitives everywhere |
| **Concurrency** | Use channels for communication | Share memory carelessly |
| **Testing** | Test behaviors | Test implementation details |
| **API Design** | Version your APIs | Break backwards compatibility |
| **Performance** | Benchmark before optimizing | Premature optimization |

---

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)