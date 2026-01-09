# Comprehensive Rust Macros Guide

## Table of Contents
1. [What Are Macros?](#what-are-macros)
2. [Standard Library Macros](#standard-library-macros)
3. [Derive Macros](#derive-macros)
4. [Declarative Macros (macro_rules!)](#declarative-macros)
5. [Procedural Macros](#procedural-macros)

---

## What Are Macros?

Macros are code that writes code (metaprogramming). They are expanded at compile time before the compiler interprets the code. Macros are identified by a `!` suffix (e.g., `println!`, `vec!`) or as attributes (e.g., `#[derive(Debug)]`).

**Key differences from functions:**
- Macros can take variable number of arguments
- Macros expand before type checking
- Macros can generate repetitive code
- Macros are more complex to write but more flexible

---

## Standard Library Macros

### 1. **println! / print!** - Formatted Output
Prints formatted text to stdout with (`println!`) or without (`print!`) newline.

```rust
fn main() {
    println!("Hello, world!");
    println!("Formatted: {}", 42);
    println!("Named: {name}, Age: {age}", name = "Alice", age = 30);
    println!("Debug: {:?}", vec![1, 2, 3]);
    println!("Pretty Debug: {:#?}", vec![1, 2, 3]);

    // Practical example from order book
    let order_id = "ORD-123";
    let price = 1850.50;
    println!("Order {} executed at ${:.2}", order_id, price);
}
```

### 2. **eprintln! / eprint!** - Error Output
Same as `println!` but writes to stderr instead of stdout.

```rust
fn main() {
    eprintln!("Error: Invalid order quantity");
    eprintln!("Failed to match order: {}", "insufficient liquidity");
}
```

### 3. **format!** - String Formatting
Creates a formatted `String` without printing.

```rust
fn create_order_message(symbol: &str, qty: u32, price: f64) -> String {
    format!("Order: {} x{} @ ${:.2}", symbol, qty, price)
}

fn main() {
    let msg = format!("Trade executed: {} shares", 100);
    let detailed = format!("{symbol} - Bid: {bid:.2}, Ask: {ask:.2}",
                          symbol = "AAPL", bid = 150.10, ask = 150.15);
    println!("{}", detailed);
}
```

### 4. **vec!** - Vector Creation
Creates a `Vec<T>` with initial elements.

```rust
fn main() {
    // Empty vector
    let v1: Vec<i32> = vec![];

    // Vector with elements
    let v2 = vec![1, 2, 3, 4, 5];

    // Vector with repeated value (100 zeros)
    let v3 = vec![0; 100];

    // Practical: Initialize order IDs
    let order_ids = vec![
        "ORD-001".to_string(),
        "ORD-002".to_string(),
        "ORD-003".to_string(),
    ];

    // Practical: Price levels
    let bid_prices = vec![1850.50, 1850.25, 1850.00];
}
```

### 5. **panic!** - Program Termination
Terminates the program with an error message.

```rust
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("Division by zero!");
    }
    a / b
}

fn validate_order(qty: i32) {
    if qty <= 0 {
        panic!("Order quantity must be positive, got: {}", qty);
    }
}

fn main() {
    // This will panic
    // divide(10, 0);

    // Better approach: use Result instead
    println!("Use Result<T, E> instead of panic! in production");
}
```

### 6. **assert! / assert_eq! / assert_ne!** - Assertions
Runtime checks that panic if condition fails. Primarily used in tests.

```rust
fn main() {
    let x = 5;
    assert!(x > 0, "x must be positive");

    let price = 100.0;
    let expected = 100.0;
    assert_eq!(price, expected, "Price mismatch");

    let bid = 99.5;
    let ask = 100.5;
    assert_ne!(bid, ask, "Bid and ask should differ");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_validation() {
        let qty = 100;
        assert!(qty > 0);
        assert_eq!(qty, 100);
        assert_ne!(qty, 0);
    }
}
```

### 7. **debug_assert!** - Debug-Only Assertions
Like `assert!` but only runs in debug builds (removed in release).

```rust
fn process_order(qty: u32) {
    // Only checked in debug builds
    debug_assert!(qty > 0, "Quantity should be validated earlier");

    // Process order...
}
```

### 8. **unreachable!** - Mark Unreachable Code
Indicates code that should never execute. Panics if reached.

```rust
enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
}

fn handle_order(status: OrderStatus) {
    match status {
        OrderStatus::Pending => println!("Processing..."),
        OrderStatus::Filled => println!("Complete"),
        OrderStatus::Cancelled => println!("Cancelled"),
        // If we've handled all variants, this is unreachable
        // _ => unreachable!("All variants covered"),
    }
}
```

### 9. **unimplemented! / todo!** - Placeholder Code
Marks code that's not yet implemented.

```rust
fn calculate_vwap() -> f64 {
    todo!("Implement VWAP calculation")
}

fn calculate_microprice() -> f64 {
    unimplemented!("Microprice not yet implemented")
}
```

### 10. **matches!** - Pattern Matching Check
Returns `bool` if value matches pattern.

```rust
enum OrderType {
    Market,
    Limit,
    Stop,
}

fn main() {
    let order_type = OrderType::Limit;

    if matches!(order_type, OrderType::Limit | OrderType::Stop) {
        println!("Order requires price parameter");
    }

    // Practical example
    let status_code = 200;
    let is_success = matches!(status_code, 200..=299);
    println!("Success: {}", is_success);
}
```

### 11. **concat! / concat_ws!** - Compile-Time String Concatenation
Concatenates literals at compile time.

```rust
fn main() {
    const VERSION: &str = concat!("v", "1", ".", "0");
    println!("Version: {}", VERSION);

    const API_PATH: &str = concat!("/api/", "v1", "/orders");
    println!("API Path: {}", API_PATH);
}
```

### 12. **env! / option_env!** - Environment Variables
Reads environment variables at compile time.

```rust
fn main() {
    // Panics if not set
    // let api_key = env!("API_KEY");

    // Returns Option<&str>
    let api_key = option_env!("API_KEY");
    match api_key {
        Some(key) => println!("API Key: {}", key),
        None => println!("No API key set"),
    }

    // Common use: cargo environment variables
    println!("Cargo Package: {}", env!("CARGO_PKG_NAME"));
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
}
```

### 13. **include! / include_str! / include_bytes!** - Include Files
Includes file contents at compile time.

```rust
fn main() {
    // Include text file as &str
    // let license = include_str!("LICENSE");

    // Include binary file as &[u8]
    // let image_data = include_bytes!("logo.png");

    // Include and evaluate Rust code
    // include!("generated_code.rs");
}
```

### 14. **file! / line! / column!** - Source Location
Returns source code location information.

```rust
fn log_error(msg: &str) {
    eprintln!("[{}:{}] Error: {}", file!(), line!(), msg);
}

fn main() {
    println!("Current file: {}", file!());
    println!("Current line: {}", line!());
    println!("Current column: {}", column!());

    log_error("Order validation failed");
}
```

### 15. **cfg!** - Conditional Compilation Check
Checks configuration at compile time.

```rust
fn main() {
    if cfg!(target_os = "windows") {
        println!("Running on Windows");
    } else if cfg!(target_os = "linux") {
        println!("Running on Linux");
    }

    if cfg!(debug_assertions) {
        println!("Debug build");
    } else {
        println!("Release build");
    }
}
```

### 16. **write! / writeln!** - Write to Buffer
Like `format!` but writes to a buffer implementing `Write`.

```rust
use std::fmt::Write;

fn main() {
    let mut output = String::new();

    write!(output, "Order: ").unwrap();
    writeln!(output, "ID={}, Qty={}", "ORD-001", 100).unwrap();

    println!("{}", output);
}
```

### 17. **dbg!** - Debug Print and Return
Prints value with file/line info and returns the value.

```rust
fn calculate_spread(bid: f64, ask: f64) -> f64 {
    let spread = ask - bid;
    dbg!(spread)  // Prints: [src/main.rs:123] spread = 0.5
}

fn main() {
    let price = dbg!(100.0 + 50.0);  // Prints and assigns
    println!("Final price: {}", price);

    // Practical: debugging calculations
    let volume = 1000;
    let price = 150.50;
    let total = dbg!(volume as f64 * dbg!(price));
}
```

### 18. **compile_error!** - Compile-Time Error
Forces a compile error with a message.

```rust
#[cfg(not(target_pointer_width = "64"))]
compile_error!("This application requires 64-bit architecture");

// Conditional compilation error
macro_rules! require_feature {
    () => {
        #[cfg(not(feature = "advanced"))]
        compile_error!("This code requires the 'advanced' feature");
    };
}
```

---

## Derive Macros

Derive macros automatically implement traits for structs/enums.

### Standard Derivable Traits

```rust
use serde::{Serialize, Deserialize};
use std::fmt;

// Debug - enables {:?} formatting
#[derive(Debug)]
struct Order {
    id: String,
    quantity: u32,
}

// Clone - enables .clone()
#[derive(Clone)]
struct Position {
    symbol: String,
    shares: i32,
}

// Copy - enables implicit copying (only for simple types)
#[derive(Copy, Clone)]
struct Price {
    value: f64,
}

// PartialEq, Eq - enables ==, !=
#[derive(PartialEq, Eq)]
struct OrderId(String);

// PartialOrd, Ord - enables <, >, <=, >=
#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct PriceLevel {
    price: u64,  // Price in cents to avoid f64
}

// Hash - enables use in HashMap/HashSet
#[derive(Hash, PartialEq, Eq)]
struct Symbol(String);

// Default - enables Default::default()
#[derive(Default)]
struct OrderConfig {
    max_quantity: u32,
    allow_short: bool,
}

// Serialize, Deserialize - JSON serialization (from serde)
#[derive(Serialize, Deserialize, Debug)]
struct Trade {
    symbol: String,
    price: f64,
    quantity: u32,
}

// Multiple derives
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MarketData {
    symbol: String,
    bid: f64,
    ask: f64,
    timestamp: u64,
}

fn main() {
    let order = Order {
        id: "ORD-001".to_string(),
        quantity: 100,
    };
    println!("{:?}", order);  // Uses Debug

    let pos = Position {
        symbol: "AAPL".to_string(),
        shares: 100,
    };
    let pos_copy = pos.clone();  // Uses Clone

    let price1 = Price { value: 150.0 };
    let price2 = price1;  // Uses Copy (no .clone() needed)

    let id1 = OrderId("ORD-001".to_string());
    let id2 = OrderId("ORD-001".to_string());
    assert_eq!(id1, id2);  // Uses PartialEq

    let config = OrderConfig::default();  // Uses Default
    println!("Max qty: {}", config.max_quantity);
}
```

---

## Declarative Macros (macro_rules!)

Custom macros using pattern matching.

### Basic Syntax

```rust
// Simple macro
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
}

// Macro with arguments
macro_rules! create_order {
    ($symbol:expr, $qty:expr) => {
        Order {
            symbol: $symbol.to_string(),
            quantity: $qty,
            order_type: OrderType::Market,
        }
    };
}

// Macro with multiple patterns
macro_rules! calculate {
    (add $a:expr, $b:expr) => { $a + $b };
    (sub $a:expr, $b:expr) => { $a - $b };
    (mul $a:expr, $b:expr) => { $a * $b };
}

fn main() {
    say_hello!();

    // let order = create_order!("AAPL", 100);

    let sum = calculate!(add 10, 5);
    let diff = calculate!(sub 10, 5);
    println!("Sum: {}, Diff: {}", sum, diff);
}
```

### Repetition in Macros

```rust
// Repeat pattern with comma separator
macro_rules! vec_of_strings {
    ($($x:expr),*) => {
        vec![$($x.to_string()),*]
    };
}

// Create hash map easily
macro_rules! hash_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut map = std::collections::HashMap::new();
        $(
            map.insert($key, $value);
        )*
        map
    }};
}

fn main() {
    let symbols = vec_of_strings!("AAPL", "GOOGL", "MSFT");
    println!("{:?}", symbols);

    let prices = hash_map! {
        "AAPL" => 150.0,
        "GOOGL" => 2800.0,
        "MSFT" => 300.0,
    };
    println!("{:?}", prices);
}
```

### Practical Example: Logging Macro

```rust
macro_rules! log {
    (info $($arg:tt)*) => {
        println!("[INFO] {}", format!($($arg)*))
    };
    (error $($arg:tt)*) => {
        eprintln!("[ERROR] {}", format!($($arg)*))
    };
    (debug $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!("[DEBUG] {}", format!($($arg)*))
    };
}

fn main() {
    log!(info "Server started on port {}", 3000);
    log!(error "Failed to connect to database");
    log!(debug "Order book state: {} orders", 42);
}
```

### Practical Example: Builder Pattern Macro

```rust
macro_rules! builder {
    ($name:ident { $($field:ident: $type:ty),* $(,)? }) => {
        struct $name {
            $($field: $type),*
        }

        impl $name {
            fn new($($field: $type),*) -> Self {
                Self { $($field),* }
            }
        }
    };
}

builder! {
    Order {
        symbol: String,
        quantity: u32,
        price: f64,
    }
}

fn main() {
    let order = Order::new("AAPL".to_string(), 100, 150.50);
    println!("Order: {} x{} @ {}", order.symbol, order.quantity, order.price);
}
```

---

## Procedural Macros

More powerful macros that operate on token streams. These require a separate crate with `proc-macro = true`.

### Types of Procedural Macros

1. **Derive macros** - `#[derive(MyTrait)]`
2. **Attribute macros** - `#[my_attribute]`
3. **Function-like macros** - `my_macro!(...)`

### Example: Custom Derive Macro (Conceptual)

```rust
// In a proc-macro crate:
//
// #[proc_macro_derive(Builder)]
// pub fn derive_builder(input: TokenStream) -> TokenStream {
//     // Parse input and generate builder pattern code
// }

// Usage:
#[derive(Builder)]
struct Order {
    symbol: String,
    quantity: u32,
    price: Option<f64>,
}

// Generated code would allow:
// let order = Order::builder()
//     .symbol("AAPL".to_string())
//     .quantity(100)
//     .price(Some(150.0))
//     .build();
```

---

## Common Patterns and Best Practices

### 1. Error Handling with Macros

```rust
macro_rules! try_or_return {
    ($expr:expr, $err_msg:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                eprintln!("{}: {:?}", $err_msg, e);
                return;
            }
        }
    };
}

use std::fs::File;

fn read_config() {
    let file = try_or_return!(
        File::open("config.toml"),
        "Failed to open config"
    );
    // Use file...
}
```

### 2. Timing Code Execution

```rust
macro_rules! time_it {
    ($name:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        println!("{} took {:?}", $name, duration);
        result
    }};
}

fn main() {
    let result = time_it!("Order matching", {
        // Simulate order matching
        std::thread::sleep(std::time::Duration::from_millis(10));
        42
    });
    println!("Result: {}", result);
}
```

### 3. Conditional Compilation

```rust
macro_rules! feature_enabled {
    ($feature:expr) => {
        #[cfg(feature = $feature)]
    };
}

// Usage with features
#[cfg(feature = "advanced_trading")]
fn calculate_microprice() -> f64 {
    // Advanced calculation
    0.0
}

#[cfg(not(feature = "advanced_trading"))]
fn calculate_microprice() -> f64 {
    // Basic calculation
    0.0
}
```

---

## Macros in This Project

Based on the order-book-exchange codebase, here are macros you're already using:

```rust
// Derive macros (very common)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order { /* ... */ }

// Testing macros
#[cfg(test)]
mod tests {
    #[test]
    fn test_matching() {
        assert_eq!(result, expected);
    }
}

// Error handling
return Err(OrderBookError::InvalidOrder(format!(
    "Invalid quantity: {}", qty
)));

// Logging (with tracing crate)
tracing::info!("Order {} matched", order_id);
tracing::error!("Failed to process order: {:?}", error);

// OpenAPI documentation (utoipa)
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    request_body = SubmitOrderRequest,
)]
async fn submit_order() { /* ... */ }
```

---

## Summary

**Most frequently used macros:**
- `println!`, `format!` - String formatting
- `vec!` - Vector creation
- `#[derive(...)]` - Trait implementation
- `assert_eq!`, `assert!` - Testing
- `panic!`, `todo!`, `unimplemented!` - Error handling
- `matches!` - Pattern checking
- `dbg!` - Debugging

**When to write custom macros:**
- Reducing boilerplate code
- Creating DSLs (Domain Specific Languages)
- Compile-time code generation
- Advanced metaprogramming

**Tips:**
- Start with standard library macros
- Use `macro_rules!` for simple metaprogramming
- Procedural macros require separate crates but are more powerful
- Prefer functions over macros when possible (easier to debug)
- Use `cargo expand` to see macro expansions
