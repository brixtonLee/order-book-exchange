# Quick Guide to Understanding Rust Codebases

A systematic approach to quickly understand any Rust project or open-source tool.

---

## Overview: The 5-Phase Approach

| Phase | Time | Focus |
|-------|------|-------|
| 1. Bird's Eye View | 15 min | README, Cargo.toml, project type |
| 2. Architecture | 20-30 min | Entry point, module structure |
| 3. Data Flow | 30-45 min | Core types, trace one operation |
| 4. Abstractions | 20-30 min | Traits, error handling |
| 5. Tests | 15-20 min | Usage patterns, examples |
| **Total** | **~90-120 min** | **Solid understanding** |

---

## Phase 1: Bird's Eye View (15 minutes)

### Step 1: Read Documentation First

```bash
# Read in this order:
1. README.md          # What the project does, quick start
2. Cargo.toml         # Dependencies = technology stack
3. CONTRIBUTING.md    # Coding standards, architecture notes
4. docs/ folder       # Design docs, architecture diagrams
5. CHANGELOG.md       # Recent changes, version history
```

**What to extract:**
- ✅ Project purpose and goals
- ✅ Build/test/run commands
- ✅ Key features and capabilities
- ✅ Technology choices and why

### Step 2: Analyze Dependencies (Cargo.toml)

```bash
cat Cargo.toml

# Look for these sections:
[dependencies]        # Runtime dependencies
[dev-dependencies]    # Test/benchmark dependencies
[features]            # Optional features
[workspace]           # Multi-crate project
```

**Dependencies reveal the stack:**
```toml
tokio = "1.0"              # → Async runtime
axum = "0.7"               # → Web framework
serde = "1.0"              # → Serialization
diesel = "2.0"             # → Database ORM
sqlx = "0.7"               # → Async SQL
redis = "0.23"             # → Caching
rdkafka = "0.34"           # → Message queue
reqwest = "0.11"           # → HTTP client
tonic = "0.10"             # → gRPC
rust_decimal = "1.33"      # → Precise decimals (finance)
uuid = "1.0"               # → Unique IDs
chrono = "0.4"             # → Date/time
tracing = "0.1"            # → Structured logging
criterion = "0.5"          # → Benchmarking
```

### Step 3: Identify Project Type

```bash
# Check project structure:
ls -la src/

# Binary (application):
src/main.rs → Entry point is here

# Library:
src/lib.rs → Public API defined here

# Both (binary + library):
src/main.rs + src/lib.rs → Binary uses library

# Workspace (monorepo):
Cargo.toml has [workspace] → Multiple sub-projects
```

---

## Phase 2: Understand Architecture (20-30 minutes)

### Step 4: Read the Entry Point

**For applications** (`src/main.rs`):
```rust
// Look for these patterns:
fn main() {
    // 1. Initialization sequence
    // 2. Configuration loading
    // 3. Server/service startup
    // 4. Main loop or event processing
}

#[tokio::main]
async fn main() {
    // Async application
}
```

**For libraries** (`src/lib.rs`):
```rust
// Look for:
pub mod module1;          // What modules exist
pub mod module2;

pub use module1::Type;    // What's exported (public API)
pub use module2::Trait;

// This shows the high-level structure
```

**What to extract:**
- ✅ What gets initialized and in what order?
- ✅ What are the main components?
- ✅ What modules are exposed?

### Step 5: Map Module Structure

```bash
# Visualize directory structure:
tree src/ -L 2 -d

# Or use fd:
fd -t d . src/

# Common Rust patterns:
src/
├── main.rs           # Binary entry point
├── lib.rs            # Library root
├── api/              # HTTP handlers, routes
│   ├── mod.rs        # Module exports
│   ├── handlers.rs   # Request handlers
│   └── routes.rs     # Route definitions
├── domain/           # Core business logic
│   ├── mod.rs
│   ├── entities.rs   # Domain models
│   └── services.rs   # Business services
├── models/           # Data structures
│   ├── mod.rs
│   ├── user.rs
│   └── order.rs
├── engine/           # Core algorithms
│   ├── mod.rs
│   ├── matching.rs
│   └── validation.rs
├── db/ or storage/   # Persistence layer
│   ├── mod.rs
│   └── repository.rs
├── utils/ or common/ # Shared utilities
│   ├── mod.rs
│   └── helpers.rs
├── config/           # Configuration
│   └── mod.rs
├── errors/           # Error types
│   └── mod.rs
└── tests/            # Integration tests
    └── integration_test.rs
```

### Step 6: Read Module Exports (`mod.rs`)

```bash
# Find all mod.rs files:
fd "mod.rs" src/

# Read each one to understand module structure:
bat src/api/mod.rs
```

```rust
// Example mod.rs:
pub mod handlers;
pub mod routes;
pub mod middleware;

// Re-exports (public API):
pub use handlers::{submit_order, cancel_order};
pub use routes::create_router;

// This tells you:
// - Submodules: handlers, routes, middleware
// - Public API: submit_order, cancel_order, create_router
```

---

## Phase 3: Follow the Data Flow (30-45 minutes)

### Step 7: Find Core Data Types

```bash
# Find all structs:
rg "^pub struct" src/

# Find all enums:
rg "^pub enum" src/

# Find all traits:
rg "^pub trait" src/
```

**Categorize them:**
```rust
// 1. Configuration types
pub struct Config { ... }
pub struct Settings { ... }

// 2. Domain entities (business objects)
pub struct User { ... }
pub struct Order { ... }
pub struct Trade { ... }

// 3. Request/Response types (API boundaries)
pub struct CreateOrderRequest { ... }
pub struct OrderResponse { ... }

// 4. Internal state
pub struct OrderBook { ... }
pub struct Engine { ... }

// 5. Error types
pub enum OrderBookError { ... }
```

### Step 8: Understand Key Structs

Pick the most important struct and analyze it:

```bash
# Find struct definition:
rg "^pub struct OrderBook" src/

# See full definition (20 lines after):
rg "^pub struct OrderBook" -A 20 src/

# Find implementations:
rg "impl OrderBook" src/

# Find trait implementations:
rg "impl.*for OrderBook" src/
```

**What to look for:**
```rust
pub struct OrderBook {
    // 1. What data does it hold?
    bids: BTreeMap<Price, PriceLevel>,
    asks: BTreeMap<Price, PriceLevel>,
    orders: HashMap<Uuid, Order>,

    // 2. What are the field types?
    // BTreeMap → sorted data structure
    // HashMap → fast lookups
    // Uuid → unique identifiers
}

impl OrderBook {
    // 3. What methods does it have?
    pub fn new() -> Self { ... }           // Constructor
    pub fn add_order(&mut self, ...) { ... } // Mutating
    pub fn get_spread(&self) -> ... { ... }  // Read-only
}
```

### Step 9: Trace One Operation End-to-End

Pick a simple operation (e.g., "submit order") and trace it:

```bash
# 1. Find the API handler:
rg "submit_order" src/

# 2. Read the handler:
rg "fn submit_order" -A 30 src/api/handlers.rs

# 3. Follow the call chain:
# Handler → Validation → Engine → Storage → Response
```

**Example flow:**
```rust
// 1. API Layer (src/api/handlers.rs)
pub async fn submit_order(
    Json(payload): Json<CreateOrderRequest>
) -> Result<Json<OrderResponse>, AppError> {
    // ↓ Calls validation
    validate_order(&payload)?;

    // ↓ Calls engine
    let (order, trades) = engine.add_order(order)?;

    // ↓ Returns response
    Ok(Json(OrderResponse { order, trades }))
}

// 2. Validation Layer (src/validation.rs)
fn validate_order(order: &Order) -> Result<(), ValidationError> {
    // Business rules
}

// 3. Engine Layer (src/engine/orderbook.rs)
impl OrderBookEngine {
    pub fn add_order(&self, order: Order) -> Result<...> {
        // ↓ Calls matching logic
        let trades = match_order(&mut book, &order)?;

        // ↓ Updates storage
        book.orders.insert(order.id, order);

        Ok((order, trades))
    }
}

// 4. Matching Logic (src/engine/matching.rs)
fn match_order(book: &mut OrderBook, order: &Order) -> Vec<Trade> {
    // Core algorithm
}
```

### Step 10: Understand Concurrency Patterns

```bash
# Find concurrency primitives:
rg "Arc<" src/
rg "Mutex<" src/
rg "RwLock<" src/
rg "tokio::spawn" src/
rg "mpsc::" src/
```

**Common patterns:**
```rust
// Shared state with Arc + Mutex/RwLock:
Arc<Mutex<HashMap<...>>>      // Thread-safe mutable map
Arc<RwLock<OrderBook>>         // Read-heavy workload

// Async task spawning:
tokio::spawn(async move { ... })  // Background task

// Message passing:
mpsc::channel()                // Producer-consumer
broadcast::channel()           // Pub-sub

// Concurrent iteration:
rayon::par_iter()              // Data parallelism
```

---

## Phase 4: Understand Abstractions (20-30 minutes)

### Step 11: Read Trait Definitions

```bash
# Find all traits:
rg "^pub trait" src/

# See trait with methods:
rg "^pub trait" -A 15 src/
```

**Traits define contracts:**
```rust
// Example: Repository pattern
pub trait OrderRepository {
    fn save(&self, order: &Order) -> Result<()>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Order>>;
    fn delete(&self, id: Uuid) -> Result<()>;
}

// Multiple implementations:
impl OrderRepository for PostgresRepository { ... }
impl OrderRepository for InMemoryRepository { ... }
```

**Find trait implementations:**
```bash
rg "impl.*Repository.*for" src/
```

### Step 12: Understand Error Handling

```bash
# Find error types:
rg "^pub enum.*Error" src/

# Common patterns:
rg "thiserror" Cargo.toml      # Using thiserror derive
rg "anyhow" Cargo.toml         # Using anyhow for app errors
```

**Error type example:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderBookError {
    #[error("Order not found: {0}")]
    OrderNotFound(Uuid),

    #[error("Invalid price: {0}")]
    InvalidPrice(Decimal),

    #[error("Insufficient quantity")]
    InsufficientQuantity,
}

// Usage:
fn get_order(id: Uuid) -> Result<Order, OrderBookError> {
    orders.get(&id)
        .cloned()
        .ok_or(OrderBookError::OrderNotFound(id))
}
```

### Step 13: Check Type Conversions

```bash
# Find From/Into implementations:
rg "impl From" src/
rg "impl Into" src/
rg "impl TryFrom" src/
```

**Conversion patterns:**
```rust
// Domain to DTO:
impl From<Order> for OrderDto {
    fn from(order: Order) -> Self {
        OrderDto { ... }
    }
}

// Request to domain:
impl TryFrom<CreateOrderRequest> for Order {
    type Error = ValidationError;

    fn try_from(req: CreateOrderRequest) -> Result<Self, Self::Error> {
        // Validation + conversion
    }
}
```

---

## Phase 5: Learn from Tests (15-20 minutes)

### Step 14: Find and Read Tests

```bash
# Find test files:
fd test src/
fd -e rs . tests/

# Find test functions:
rg "#\[test\]" src/
rg "#\[tokio::test\]" src/
```

**Test locations:**
```bash
# Unit tests (inline):
src/engine/orderbook.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_add_order() { ... }
}

# Integration tests:
tests/integration_test.rs
tests/api_test.rs
```

**What tests reveal:**
```rust
#[test]
fn test_order_matching() {
    // 1. How to create instances:
    let engine = OrderBookEngine::new();

    // 2. How to construct data:
    let order = Order::new(
        "AAPL".to_string(),
        OrderSide::Buy,
        OrderType::Limit,
        Some(dec!(150.00)),
        dec!(100),
        "user1".to_string(),
    );

    // 3. How to call methods:
    let (order, trades) = engine.add_order(order).unwrap();

    // 4. Expected behavior:
    assert_eq!(trades.len(), 0);
    assert_eq!(order.status, OrderStatus::New);
}
```

### Step 15: Check Examples

```bash
# Find example files:
ls examples/

# Read them:
bat examples/basic_usage.rs
```

Examples show real-world usage patterns.

---

## Essential Tools for Codebase Exploration

### Installation (Ubuntu/WSL)

```bash
# Essential tools:
sudo apt update
sudo apt install ripgrep fd-find bat tree

# Aliases (Ubuntu uses different names):
echo "alias fd='fdfind'" >> ~/.bashrc
echo "alias bat='batcat'" >> ~/.bashrc
source ~/.bashrc

# Optional (via cargo):
cargo install tokei      # Lines of code counter
cargo install sd         # Find & replace
```

### Quick Commands Reference

```bash
# See structure:
tree src/ -L 2

# Count code:
tokei

# Find structs:
rg "^pub struct" src/

# Find functions:
rg "^pub fn" src/

# Find implementations:
rg "^impl" src/

# Read file with syntax highlighting:
bat src/main.rs

# Find files:
fd .rs src/

# Search with context:
rg "add_order" -A 10 src/
```

---

## Complete Workflow Example: Understanding `tokio`

Let's walk through understanding the `tokio` async runtime:

```bash
# 1. Clone and enter:
git clone https://github.com/tokio-rs/tokio
cd tokio

# 2. Read overview:
bat README.md

# 3. Check dependencies and features:
bat tokio/Cargo.toml

# 4. See structure:
tree tokio/src -L 2 -d

# 5. Read library root:
bat tokio/src/lib.rs
# Output shows main modules:
# pub mod runtime;
# pub mod task;
# pub mod sync;
# pub mod io;

# 6. Find key struct (Runtime):
rg "^pub struct Runtime" tokio/src/

# 7. See Runtime implementation:
rg "impl Runtime" -A 20 tokio/src/runtime/

# 8. Understand how tasks spawn:
rg "pub fn spawn" tokio/src/
rg "pub fn spawn" -A 15 tokio/src/task/

# 9. Read tests for usage patterns:
rg "#\[tokio::test\]" tokio/tests/ -A 10

# 10. Check examples:
ls tokio/examples/
bat tokio/examples/hello_world.rs
```

**Time invested:** ~90 minutes
**Result:** Solid understanding of tokio's architecture

---

## Rust-Specific Search Patterns

### Find Definitions

```bash
# Structs:
rg "^pub struct \w+" -t rust

# Enums:
rg "^pub enum \w+" -t rust

# Traits:
rg "^pub trait \w+" -t rust

# Type aliases:
rg "^pub type \w+" -t rust

# Constants:
rg "^pub const \w+" -t rust
```

### Find Implementations

```bash
# All impl blocks:
rg "^impl" -t rust

# Trait implementations:
rg "impl.*for" -t rust

# Specific struct:
rg "impl OrderBook" -t rust

# Generic implementations:
rg "impl<.*>" -t rust
```

### Find Usage Patterns

```bash
# Find method calls:
rg "\.add_order\(" -t rust

# Find struct construction:
rg "OrderBook::new" -t rust
rg "OrderBook \{" -t rust

# Find macro usage:
rg "println!" -t rust
rg "#\[derive" -t rust

# Find async/await:
rg "async fn" -t rust
rg "\.await" -t rust
```

### Find Unsafe Code

```bash
# Unsafe blocks:
rg "unsafe" -t rust

# Unsafe functions:
rg "unsafe fn" -t rust

# Raw pointers:
rg "\*const|\*mut" -t rust
```

---

## Key Patterns to Recognize

### 1. Ownership & Borrowing

```rust
fn takes_ownership(s: String) { ... }        // Moves ownership
fn borrows(s: &String) { ... }               // Immutable borrow
fn borrows_mut(s: &mut String) { ... }       // Mutable borrow
```

### 2. Error Handling

```rust
Result<T, E>                  // Fallible operations
Option<T>                     // Nullable values
?                             // Error propagation
unwrap() / expect()           // Panic on error (avoid in prod)
```

### 3. Lifetimes

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str
// 'a means: returned reference lives as long as inputs
```

### 4. Traits & Generics

```rust
fn process<T: Display>(item: T) { ... }      // Generic with trait bound
fn process(item: impl Display) { ... }        // Trait syntax sugar
fn process(item: &dyn Display) { ... }        // Trait object (dynamic)
```

### 5. Smart Pointers

```rust
Box<T>                        // Heap allocation
Rc<T>                         // Reference counting (single-threaded)
Arc<T>                        // Atomic ref counting (multi-threaded)
RefCell<T>                    // Interior mutability
Mutex<T>                      // Thread-safe interior mutability
```

---

## Common Gotchas for Beginners

### 1. Module System

```rust
// In src/lib.rs or src/main.rs:
mod engine;              // Looks for src/engine.rs or src/engine/mod.rs

// In src/engine/mod.rs:
pub mod matching;        // Looks for src/engine/matching.rs

// Usage:
use crate::engine::matching::match_order;
```

### 2. Privacy

```rust
mod foo {
    pub struct Bar {
        pub public_field: i32,    // Accessible outside
        private_field: i32,        // Only in same module
    }
}
```

### 3. Prelude

```rust
// std::prelude::* is auto-imported:
// - Option, Result, Some, None, Ok, Err
// - Vec, String, Box
// - Clone, Copy, Send, Sync
```

---

## Time-Saving Tips

1. **Start with tests** - They show real usage without boilerplate
2. **Use `cargo doc --open`** - Auto-generated docs from `///` comments
3. **Follow types** - Use IDE "Go to Definition" (F12)
4. **Search bidirectionally** - Find definition, then find usages
5. **Don't understand everything** - Focus on the critical path
6. **Draw diagrams** - Visualize module/data relationships
7. **Run examples** - See it work before diving deep
8. **Use clippy** - `cargo clippy` shows idiomatic patterns

---

## Checklist: Are You Done?

After 90-120 minutes, you should be able to answer:

- ✅ What does this project do? (purpose, features)
- ✅ How do I build/test/run it? (commands)
- ✅ What's the high-level architecture? (modules, layers)
- ✅ What are the core data types? (main structs/enums)
- ✅ How does a request flow through the system? (trace one path)
- ✅ What concurrency patterns are used? (Arc, Mutex, async)
- ✅ How are errors handled? (error types, Result usage)
- ✅ What are the key abstractions? (important traits)
- ✅ How do I use the main APIs? (seen in tests/examples)

If yes → You have a solid understanding! 🎉

If no → Spend more time on the specific area you're unclear about.

---

## Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **Rust by Example**: https://doc.rust-lang.org/rust-by-example/
- **Rustlings**: https://github.com/rust-lang/rustlings (interactive exercises)
- **Crate docs**: https://docs.rs/
- **Awesome Rust**: https://github.com/rust-unofficial/awesome-rust (curated list)

---

## Next Steps

Once you understand the codebase:
1. **Make a small change** - Best way to verify understanding
2. **Write a test** - Ensures you can use the APIs
3. **Fix a bug** - Real-world experience
4. **Add a feature** - Deep dive into architecture

**Learning by doing beats reading every time!**
