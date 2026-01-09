# Comprehensive Rust Closures Guide

## Table of Contents
1. [What Are Closures?](#what-are-closures)
2. [Closure Syntax](#closure-syntax)
3. [Closure Types and Traits](#closure-types-and-traits)
4. [Iterator Methods with Closures](#iterator-methods-with-closures)
5. [Beginner to Advanced Examples](#beginner-to-advanced-examples)
6. [Closure Chaining Patterns](#closure-chaining-patterns)
7. [Error Handling with Closures](#error-handling-with-closures)
8. [Advanced Patterns](#advanced-patterns)
9. [Performance Considerations](#performance-considerations)

---

## What Are Closures?

Closures are anonymous functions that can capture variables from their surrounding scope. They are similar to lambdas in other languages.

**Key characteristics:**
- Can capture variables from their environment
- Can be stored in variables
- Can be passed as arguments to functions
- Can be returned from functions
- Have three capture modes: by reference, by mutable reference, or by value

**Syntax comparison:**

```rust
// Regular function
fn add_one(x: i32) -> i32 {
    x + 1
}

// Closure with explicit types
let add_one = |x: i32| -> i32 { x + 1 };

// Closure with inferred types
let add_one = |x| x + 1;

// Closure with multiple statements
let add_and_print = |x| {
    let result = x + 1;
    println!("Result: {}", result);
    result
};

fn main() {
    println!("{}", add_one(5));
    println!("{}", add_and_print(5));
}
```

---

## Closure Syntax

### Basic Forms

```rust
fn main() {
    // No parameters
    let say_hello = || println!("Hello!");
    say_hello();

    // One parameter
    let double = |x| x * 2;
    println!("{}", double(5));

    // Multiple parameters
    let add = |x, y| x + y;
    println!("{}", add(3, 4));

    // With explicit types
    let multiply = |x: i32, y: i32| -> i32 { x * y };
    println!("{}", multiply(3, 4));

    // Multi-line body
    let calculate_fee = |price: f64, quantity: u32| {
        let total = price * quantity as f64;
        let fee = total * 0.001; // 0.1% fee
        fee
    };
    println!("Fee: ${:.2}", calculate_fee(150.0, 100));
}
```

### Capturing Variables

```rust
fn main() {
    let fee_rate = 0.001;
    let exchange_name = "NYSE";

    // Capture by immutable reference (&T)
    let calculate_fee = |amount: f64| amount * fee_rate;
    println!("Fee: ${:.2}", calculate_fee(1000.0));

    // Capture by mutable reference (&mut T)
    let mut total_volume = 0;
    let mut record_trade = |volume: u32| {
        total_volume += volume;
        println!("Total volume: {}", total_volume);
    };
    record_trade(100);
    record_trade(200);
    println!("Final volume: {}", total_volume);

    // Capture by value (move)
    let report_exchange = move || {
        println!("Trading on {}", exchange_name);
        // exchange_name moved into closure
    };
    report_exchange();
    // println!("{}", exchange_name); // Error: value moved
}
```

---

## Closure Types and Traits

Rust has three closure traits that determine how closures capture variables:

### 1. **Fn** - Borrows by Immutable Reference

Can be called multiple times without mutating captured variables.

```rust
fn apply_twice<F>(f: F, x: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(f(x))
}

fn main() {
    let add_five = |x| x + 5;
    let result = apply_twice(add_five, 10);
    println!("Result: {}", result); // 20

    // Practical example: calculate bid-ask spread
    let calculate_spread = |bid: f64, ask: f64| ask - bid;
    let spread1 = calculate_spread(99.5, 100.5);
    let spread2 = calculate_spread(150.0, 150.5);
    println!("Spreads: {}, {}", spread1, spread2);
}
```

### 2. **FnMut** - Borrows by Mutable Reference

Can modify captured variables.

```rust
fn apply_multiple<F>(mut f: F, x: i32, times: usize) -> i32
where
    F: FnMut(i32) -> i32,
{
    let mut result = x;
    for _ in 0..times {
        result = f(result);
    }
    result
}

fn main() {
    let mut call_count = 0;
    let increment = |x| {
        call_count += 1;
        x + 1
    };

    // Can't call directly multiple times due to mutable borrow
    // Instead, use in a context that expects FnMut
    println!("Call count: {}", call_count);

    // Practical example: order ID generator
    let mut order_counter = 0;
    let mut generate_order_id = || {
        order_counter += 1;
        format!("ORD-{:06}", order_counter)
    };

    println!("{}", generate_order_id()); // ORD-000001
    println!("{}", generate_order_id()); // ORD-000002
    println!("{}", generate_order_id()); // ORD-000003
}
```

### 3. **FnOnce** - Takes Ownership

Can only be called once, consumes captured variables.

```rust
fn call_once<F>(f: F)
where
    F: FnOnce(),
{
    f();
}

fn main() {
    let expensive_data = vec![1, 2, 3, 4, 5];

    let consume = move || {
        println!("Processing: {:?}", expensive_data);
        // expensive_data is moved into closure and consumed
    };

    call_once(consume);
    // call_once(consume); // Error: can't call twice
    // println!("{:?}", expensive_data); // Error: moved

    // Practical example: send order to exchange (one-time operation)
    let order_data = "BUY AAPL 100 @ 150.00".to_string();
    let send_order = move || {
        println!("Sending order: {}", order_data);
        // Simulate sending to exchange
        // order_data is consumed
    };
    send_order();
}
```

---

## Iterator Methods with Closures

Iterator methods heavily use closures. Here's a comprehensive list:

### 1. **map** - Transform Each Element

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Basic map
    let prices_with_fee: Vec<f64> = prices
        .iter()
        .map(|p| p * 1.001)
        .collect();
    println!("With fee: {:?}", prices_with_fee);

    // Multiple maps
    let order_ids = vec![1, 2, 3];
    let formatted: Vec<String> = order_ids
        .iter()
        .map(|id| format!("ORD-{:03}", id))
        .collect();
    println!("IDs: {:?}", formatted);
}
```

### 2. **filter** - Keep Elements Matching Condition

```rust
fn main() {
    let prices = vec![50.0, 150.0, 250.0, 300.0];

    // Filter expensive stocks
    let expensive: Vec<f64> = prices
        .iter()
        .filter(|&&p| p > 200.0)
        .copied()
        .collect();
    println!("Expensive: {:?}", expensive);

    // Practical: filter valid orders
    let quantities = vec![0, 100, -50, 500, 1000];
    let valid: Vec<i32> = quantities
        .iter()
        .filter(|&&q| q > 0)
        .copied()
        .collect();
    println!("Valid quantities: {:?}", valid);
}
```

### 3. **filter_map** - Filter and Map Combined

```rust
fn main() {
    let inputs = vec!["100", "abc", "200", "xyz", "300"];

    let valid_numbers: Vec<u32> = inputs
        .iter()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    println!("Valid numbers: {:?}", valid_numbers);

    // Practical: parse order quantities
    let order_strings = vec!["BUY 100", "SELL abc", "BUY 200"];
    let quantities: Vec<u32> = order_strings
        .iter()
        .filter_map(|s| s.split_whitespace().nth(1))
        .filter_map(|q| q.parse::<u32>().ok())
        .collect();
    println!("Quantities: {:?}", quantities);
}
```

### 4. **fold** - Reduce to Single Value

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Sum
    let total: f64 = prices.iter().fold(0.0, |acc, &p| acc + p);
    println!("Total: {}", total);

    // Calculate VWAP (Volume Weighted Average Price)
    let trades = vec![
        (100.0, 50),  // (price, volume)
        (101.0, 30),
        (99.5, 20),
    ];

    let (total_value, total_volume) = trades
        .iter()
        .fold((0.0, 0), |(value, vol), &(price, quantity)| {
            (value + price * quantity as f64, vol + quantity)
        });

    let vwap = total_value / total_volume as f64;
    println!("VWAP: {:.2}", vwap);
}
```

### 5. **reduce** - Like fold but Returns Option

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Sum using reduce
    let total = prices.iter().reduce(|acc, p| acc + p);
    println!("Total: {:?}", total);

    // Find max price
    let max_price = prices.iter().reduce(|a, b| if a > b { a } else { b });
    println!("Max: {:?}", max_price);

    // Empty vec returns None
    let empty: Vec<f64> = vec![];
    let result = empty.iter().reduce(|a, b| a + b);
    println!("Empty result: {:?}", result); // None
}
```

### 6. **for_each** - Execute for Each Element

```rust
fn main() {
    let orders = vec!["ORD-001", "ORD-002", "ORD-003"];

    // Print each order
    orders.iter().for_each(|order| {
        println!("Processing order: {}", order);
    });

    // With side effects
    let mut total_volume = 0;
    let volumes = vec![100, 200, 300];
    volumes.iter().for_each(|&vol| {
        total_volume += vol;
        println!("Running total: {}", total_volume);
    });
}
```

### 7. **find** - Find First Matching Element

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0];

    let first_expensive = prices.iter().find(|&&p| p > 180.0);
    println!("First expensive: {:?}", first_expensive); // Some(200.0)

    // Practical: find order by ID
    let orders = vec![
        ("ORD-001", 100),
        ("ORD-002", 200),
        ("ORD-003", 300),
    ];

    let found = orders.iter().find(|(id, _)| *id == "ORD-002");
    match found {
        Some((id, qty)) => println!("Found: {} with quantity {}", id, qty),
        None => println!("Not found"),
    }
}
```

### 8. **find_map** - Find and Map

```rust
fn main() {
    let data = vec!["abc", "100", "xyz", "200"];

    let first_number = data
        .iter()
        .find_map(|s| s.parse::<u32>().ok());
    println!("First number: {:?}", first_number); // Some(100)
}
```

### 9. **position** - Find Index of Element

```rust
fn main() {
    let symbols = vec!["AAPL", "GOOGL", "MSFT", "TSLA"];

    let index = symbols.iter().position(|&s| s == "MSFT");
    println!("Index of MSFT: {:?}", index); // Some(2)

    let not_found = symbols.iter().position(|&s| s == "NVDA");
    println!("Index of NVDA: {:?}", not_found); // None
}
```

### 10. **any / all** - Check Conditions

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Check if any price is above 180
    let has_expensive = prices.iter().any(|&p| p > 180.0);
    println!("Has expensive: {}", has_expensive); // true

    // Check if all prices are positive
    let all_positive = prices.iter().all(|&p| p > 0.0);
    println!("All positive: {}", all_positive); // true

    // Practical: validate order quantities
    let quantities = vec![100, 200, 300];
    let all_valid = quantities.iter().all(|&q| q > 0 && q <= 1000);
    println!("All valid: {}", all_valid);
}
```

### 11. **skip / take** - Slice Iterator

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0, 300.0];

    // Skip first 2, take next 2
    let middle: Vec<f64> = prices
        .iter()
        .skip(2)
        .take(2)
        .copied()
        .collect();
    println!("Middle: {:?}", middle); // [200.0, 250.0]

    // Practical: pagination
    let page_size = 2;
    let page_num = 1; // 0-indexed
    let page: Vec<f64> = prices
        .iter()
        .skip(page_num * page_size)
        .take(page_size)
        .copied()
        .collect();
    println!("Page {}: {:?}", page_num, page);
}
```

### 12. **skip_while / take_while** - Conditional Skip/Take

```rust
fn main() {
    let prices = vec![50.0, 75.0, 150.0, 200.0, 100.0];

    // Take while below 150
    let cheap: Vec<f64> = prices
        .iter()
        .take_while(|&&p| p < 150.0)
        .copied()
        .collect();
    println!("Cheap: {:?}", cheap); // [50.0, 75.0]

    // Skip while below 150
    let expensive: Vec<f64> = prices
        .iter()
        .skip_while(|&&p| p < 150.0)
        .copied()
        .collect();
    println!("Expensive: {:?}", expensive); // [150.0, 200.0, 100.0]
}
```

### 13. **enumerate** - Add Index

```rust
fn main() {
    let symbols = vec!["AAPL", "GOOGL", "MSFT"];

    symbols
        .iter()
        .enumerate()
        .for_each(|(i, symbol)| {
            println!("{}. {}", i + 1, symbol);
        });

    // With filtering
    let indexed: Vec<(usize, &str)> = symbols
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(i, &s)| (i, s))
        .collect();
    println!("Even indices: {:?}", indexed);
}
```

### 14. **zip** - Combine Two Iterators

```rust
fn main() {
    let symbols = vec!["AAPL", "GOOGL", "MSFT"];
    let prices = vec![150.0, 2800.0, 300.0];

    let combined: Vec<(&str, f64)> = symbols
        .iter()
        .zip(prices.iter())
        .map(|(&s, &p)| (s, p))
        .collect();

    println!("Combined: {:?}", combined);

    // Practical: calculate total portfolio value
    let quantities = vec![100, 50, 200];
    let total_value: f64 = prices
        .iter()
        .zip(quantities.iter())
        .map(|(&price, &qty)| price * qty as f64)
        .sum();
    println!("Portfolio value: ${:.2}", total_value);
}
```

### 15. **flat_map / flatten** - Flatten Nested Structures

```rust
fn main() {
    let orders = vec![
        vec![100, 200],
        vec![300, 400, 500],
        vec![600],
    ];

    // Flatten
    let all_orders: Vec<i32> = orders
        .iter()
        .flatten()
        .copied()
        .collect();
    println!("All orders: {:?}", all_orders);

    // flat_map
    let symbols = vec!["AAPL", "GOOGL"];
    let order_types: Vec<String> = symbols
        .iter()
        .flat_map(|&s| vec![
            format!("{} BUY", s),
            format!("{} SELL", s),
        ])
        .collect();
    println!("Order types: {:?}", order_types);
}
```

### 16. **partition** - Split Into Two Collections

```rust
fn main() {
    let prices = vec![50.0, 150.0, 250.0, 100.0, 300.0];

    let (cheap, expensive): (Vec<f64>, Vec<f64>) = prices
        .iter()
        .partition(|&&p| p < 200.0);

    println!("Cheap: {:?}", cheap);
    println!("Expensive: {:?}", expensive);

    // Practical: separate buy and sell orders
    let orders = vec![
        ("BUY", 100),
        ("SELL", 200),
        ("BUY", 300),
        ("SELL", 400),
    ];

    let (buys, sells): (Vec<_>, Vec<_>) = orders
        .iter()
        .partition(|(side, _)| *side == "BUY");

    println!("Buys: {:?}", buys);
    println!("Sells: {:?}", sells);
}
```

### 17. **collect** - Build Collections

```rust
use std::collections::{HashMap, HashSet};

fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Collect to Vec
    let vec: Vec<f64> = prices.iter().map(|&p| p * 1.1).collect();

    // Collect to HashSet
    let set: HashSet<&str> = vec!["AAPL", "GOOGL", "AAPL"].into_iter().collect();
    println!("Unique symbols: {:?}", set);

    // Collect to HashMap
    let symbols = vec!["AAPL", "GOOGL", "MSFT"];
    let prices = vec![150.0, 2800.0, 300.0];
    let map: HashMap<&str, f64> = symbols
        .iter()
        .zip(prices.iter())
        .map(|(&s, &p)| (s, p))
        .collect();
    println!("Price map: {:?}", map);

    // Collect to String
    let text: String = vec!["AAPL", "GOOGL", "MSFT"]
        .iter()
        .map(|&s| s)
        .collect::<Vec<_>>()
        .join(", ");
    println!("Symbols: {}", text);
}
```

### 18. **sum / product** - Aggregate Operations

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Sum
    let total: f64 = prices.iter().sum();
    println!("Total: {}", total);

    // Count
    let count = prices.iter().count();
    println!("Count: {}", count);

    // Average
    let average = total / count as f64;
    println!("Average: {:.2}", average);

    // Min/Max
    let min = prices.iter().min_by(|a, b| a.partial_cmp(b).unwrap());
    let max = prices.iter().max_by(|a, b| a.partial_cmp(b).unwrap());
    println!("Min: {:?}, Max: {:?}", min, max);
}
```

### 19. **scan** - Stateful Map

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0];

    // Calculate running total
    let running_total: Vec<f64> = prices
        .iter()
        .scan(0.0, |state, &price| {
            *state += price;
            Some(*state)
        })
        .collect();

    println!("Running total: {:?}", running_total);
    // [100.0, 250.0, 450.0, 700.0]

    // Practical: calculate cumulative profit
    let pnl = vec![10.0, -5.0, 15.0, -3.0, 8.0];
    let cumulative: Vec<f64> = pnl
        .iter()
        .scan(0.0, |total, &profit| {
            *total += profit;
            Some(*total)
        })
        .collect();

    println!("Cumulative P&L: {:?}", cumulative);
}
```

### 20. **inspect** - Debug Iterator Chain

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    let result: Vec<f64> = prices
        .iter()
        .inspect(|p| println!("Before map: {}", p))
        .map(|p| p * 1.1)
        .inspect(|p| println!("After map: {}", p))
        .collect();

    println!("Final: {:?}", result);
}
```

---

## Beginner to Advanced Examples

### Beginner Level

#### Example 1: Simple Transformation

```rust
fn main() {
    // Basic: Calculate fees for each trade
    let trade_values = vec![1000.0, 2500.0, 750.0];
    let fee_rate = 0.001;

    let fees: Vec<f64> = trade_values
        .iter()
        .map(|value| value * fee_rate)
        .collect();

    println!("Fees: {:?}", fees);
}
```

#### Example 2: Filtering Data

```rust
fn main() {
    // Filter valid orders
    let order_quantities = vec![0, 100, -50, 200, 500, -10, 1000];

    let valid_orders: Vec<i32> = order_quantities
        .iter()
        .filter(|&&qty| qty > 0)
        .copied()
        .collect();

    println!("Valid orders: {:?}", valid_orders);
}
```

### Intermediate Level

#### Example 3: Chaining Multiple Operations

```rust
fn main() {
    // Process orders: filter valid, calculate total value, apply fee
    let orders = vec![
        ("AAPL", 100, 150.0),
        ("GOOGL", -50, 2800.0),  // Invalid (negative)
        ("MSFT", 200, 300.0),
        ("TSLA", 0, 250.0),      // Invalid (zero)
        ("NVDA", 150, 500.0),
    ];

    let fee_rate = 0.001;

    let total_with_fees: f64 = orders
        .iter()
        .filter(|(_, qty, _)| *qty > 0)                    // Valid quantities
        .map(|(symbol, qty, price)| {
            let value = *qty as f64 * price;
            let fee = value * fee_rate;
            (symbol, value, fee)
        })
        .inspect(|(symbol, value, fee)| {
            println!("{}: ${:.2} (fee: ${:.2})", symbol, value, fee);
        })
        .map(|(_, value, fee)| value + fee)
        .sum();

    println!("Total with fees: ${:.2}", total_with_fees);
}
```

#### Example 4: Grouping and Aggregation

```rust
use std::collections::HashMap;

fn main() {
    // Calculate total volume per symbol
    let trades = vec![
        ("AAPL", 100),
        ("GOOGL", 50),
        ("AAPL", 200),
        ("MSFT", 150),
        ("AAPL", 50),
        ("GOOGL", 75),
    ];

    let volume_by_symbol: HashMap<&str, u32> = trades
        .iter()
        .fold(HashMap::new(), |mut acc, &(symbol, volume)| {
            *acc.entry(symbol).or_insert(0) += volume;
            acc
        });

    println!("Volume by symbol: {:?}", volume_by_symbol);

    // Find symbol with highest volume
    let top_symbol = volume_by_symbol
        .iter()
        .max_by_key(|(_, &volume)| volume)
        .map(|(symbol, volume)| (*symbol, *volume));

    println!("Top symbol: {:?}", top_symbol);
}
```

### Advanced Level

#### Example 5: Complex Trading Analytics

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Trade {
    symbol: String,
    price: f64,
    volume: u32,
    side: Side,
}

#[derive(Debug, Clone, PartialEq)]
enum Side {
    Buy,
    Sell,
}

fn main() {
    let trades = vec![
        Trade { symbol: "AAPL".to_string(), price: 150.0, volume: 100, side: Side::Buy },
        Trade { symbol: "AAPL".to_string(), price: 151.0, volume: 50, side: Side::Sell },
        Trade { symbol: "GOOGL".to_string(), price: 2800.0, volume: 20, side: Side::Buy },
        Trade { symbol: "AAPL".to_string(), price: 149.5, volume: 200, side: Side::Buy },
        Trade { symbol: "GOOGL".to_string(), price: 2805.0, volume: 15, side: Side::Sell },
    ];

    // Calculate VWAP per symbol
    let vwap_by_symbol: HashMap<String, f64> = trades
        .iter()
        .fold(HashMap::new(), |mut acc, trade| {
            let entry = acc.entry(trade.symbol.clone()).or_insert((0.0, 0u32));
            entry.0 += trade.price * trade.volume as f64;
            entry.1 += trade.volume;
            acc
        })
        .iter()
        .map(|(symbol, (total_value, total_volume))| {
            (symbol.clone(), total_value / *total_volume as f64)
        })
        .collect();

    println!("VWAP by symbol:");
    for (symbol, vwap) in &vwap_by_symbol {
        println!("  {}: ${:.2}", symbol, vwap);
    }

    // Calculate net position per symbol (buys - sells)
    let positions: HashMap<String, i32> = trades
        .iter()
        .fold(HashMap::new(), |mut acc, trade| {
            let entry = acc.entry(trade.symbol.clone()).or_insert(0);
            match trade.side {
                Side::Buy => *entry += trade.volume as i32,
                Side::Sell => *entry -= trade.volume as i32,
            }
            acc
        });

    println!("\nNet positions:");
    for (symbol, position) in &positions {
        println!("  {}: {} shares", symbol, position);
    }
}
```

#### Example 6: Real-Time Order Book Analytics

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Order {
    id: String,
    price: f64,
    quantity: u32,
}

fn main() {
    // Simulate order book (price -> orders at that price)
    let mut orders = vec![
        Order { id: "1".to_string(), price: 150.0, quantity: 100 },
        Order { id: "2".to_string(), price: 150.5, quantity: 200 },
        Order { id: "3".to_string(), price: 150.0, quantity: 150 },
        Order { id: "4".to_string(), price: 151.0, quantity: 50 },
        Order { id: "5".to_string(), price: 150.5, quantity: 75 },
    ];

    // Group orders by price level
    let price_levels: BTreeMap<String, Vec<Order>> = orders
        .iter()
        .fold(BTreeMap::new(), |mut acc, order| {
            let price_key = format!("{:.2}", order.price);
            acc.entry(price_key).or_insert_with(Vec::new).push(order.clone());
            acc
        });

    // Calculate total quantity at each price level
    println!("Order Book:");
    price_levels
        .iter()
        .rev()  // Show best prices first
        .for_each(|(price, orders)| {
            let total_qty: u32 = orders.iter().map(|o| o.quantity).sum();
            let order_count = orders.len();
            println!("  ${}: {} shares ({} orders)", price, total_qty, order_count);
        });

    // Calculate market depth (cumulative volume)
    let cumulative_depth: Vec<(String, u32)> = price_levels
        .iter()
        .rev()
        .scan(0u32, |state, (price, orders)| {
            let level_qty: u32 = orders.iter().map(|o| o.quantity).sum();
            *state += level_qty;
            Some((price.clone(), *state))
        })
        .collect();

    println!("\nCumulative Depth:");
    for (price, cum_qty) in cumulative_depth {
        println!("  ${}: {} shares cumulative", price, cum_qty);
    }
}
```

---

## Closure Chaining Patterns

### Pattern 1: Data Pipeline

```rust
fn main() {
    // Raw market data -> cleaned -> validated -> transformed -> stored
    let raw_prices = vec!["150.0", "abc", "200.5", "-10", "175.25"];

    let processed: Vec<f64> = raw_prices
        .iter()
        .filter_map(|s| s.parse::<f64>().ok())     // Parse
        .filter(|&p| p > 0.0)                       // Validate
        .map(|p| p * 1.001)                         // Add fee
        .collect();

    println!("Processed: {:?}", processed);
}
```

### Pattern 2: Extract-Transform-Load (ETL)

```rust
use std::collections::HashMap;

fn main() {
    let raw_data = vec![
        "AAPL,150.0,100",
        "GOOGL,2800.0,50",
        "INVALID",
        "MSFT,300.0,200",
    ];

    let trades: HashMap<String, (f64, u32)> = raw_data
        .iter()
        // Extract
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 3 {
                Some((parts[0], parts[1], parts[2]))
            } else {
                None
            }
        })
        // Transform
        .filter_map(|(symbol, price, qty)| {
            let price = price.parse::<f64>().ok()?;
            let qty = qty.parse::<u32>().ok()?;
            Some((symbol.to_string(), price, qty))
        })
        // Load
        .map(|(symbol, price, qty)| (symbol, (price, qty)))
        .collect();

    println!("Trades: {:?}", trades);
}
```

### Pattern 3: Multi-Stage Filtering

```rust
fn main() {
    let orders = vec![
        ("AAPL", 100, 150.0),
        ("GOOGL", -50, 2800.0),
        ("MSFT", 200, 300.0),
        ("TSLA", 5, 250.0),
        ("NVDA", 1000, 500.0),
    ];

    let threshold_value = 10000.0;

    let filtered: Vec<(String, u32, f64, f64)> = orders
        .iter()
        // Stage 1: Valid quantity
        .filter(|(_, qty, _)| *qty > 0)
        // Stage 2: Minimum quantity
        .filter(|(_, qty, _)| *qty >= 10)
        // Calculate value
        .map(|(symbol, qty, price)| {
            let value = *qty as f64 * price;
            (symbol.to_string(), *qty, *price, value)
        })
        // Stage 3: Minimum value
        .filter(|(_, _, _, value)| *value >= threshold_value)
        .collect();

    println!("Large orders:");
    for (symbol, qty, price, value) in filtered {
        println!("  {} x{} @ ${:.2} = ${:.2}", symbol, qty, price, value);
    }
}
```

### Pattern 4: Parallel Processing Simulation

```rust
fn main() {
    let symbols = vec!["AAPL", "GOOGL", "MSFT", "TSLA"];

    // Simulate processing each symbol with different operations
    let results: Vec<(String, f64, f64)> = symbols
        .iter()
        .map(|&symbol| {
            // Simulate fetching market data
            let bid = 100.0; // Mock
            let ask = 101.0; // Mock
            (symbol.to_string(), bid, ask)
        })
        .map(|(symbol, bid, ask)| {
            // Calculate spread
            let spread = ask - bid;
            (symbol, bid, ask, spread)
        })
        .filter(|(_, _, _, spread)| *spread > 0.0)
        .map(|(symbol, bid, ask, spread)| {
            // Calculate spread percentage
            let spread_pct = (spread / bid) * 100.0;
            (symbol, spread, spread_pct)
        })
        .collect();

    println!("Spread analysis:");
    for (symbol, spread, spread_pct) in results {
        println!("  {}: ${:.2} ({:.2}%)", symbol, spread, spread_pct);
    }
}
```

### Pattern 5: Nested Closures

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Closure that returns a closure
    let create_fee_calculator = |fee_rate: f64| {
        move |amount: f64| amount * fee_rate
    };

    let standard_fee = create_fee_calculator(0.001);
    let premium_fee = create_fee_calculator(0.0005);

    let with_standard_fees: Vec<f64> = prices
        .iter()
        .map(|&p| p + standard_fee(p))
        .collect();

    let with_premium_fees: Vec<f64> = prices
        .iter()
        .map(|&p| p + premium_fee(p))
        .collect();

    println!("Standard fees: {:?}", with_standard_fees);
    println!("Premium fees: {:?}", with_premium_fees);
}
```

---

## Error Handling with Closures

### Pattern 1: Option Handling

```rust
fn main() {
    let order_ids = vec!["ORD-001", "ORD-002", "INVALID", "ORD-003"];

    // Filter valid order IDs (simple validation)
    let valid_orders: Vec<&str> = order_ids
        .iter()
        .filter(|id| id.starts_with("ORD-"))
        .copied()
        .collect();

    println!("Valid orders: {:?}", valid_orders);

    // Using find
    let found = order_ids
        .iter()
        .find(|id| id.starts_with("ORD-002"));

    match found {
        Some(id) => println!("Found: {}", id),
        None => println!("Not found"),
    }
}
```

### Pattern 2: Result Handling with filter_map

```rust
fn parse_price(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|_| format!("Invalid price: {}", s))
}

fn main() {
    let price_strings = vec!["100.0", "abc", "200.5", "xyz", "150.25"];

    // Collect successful parses only
    let valid_prices: Vec<f64> = price_strings
        .iter()
        .filter_map(|s| parse_price(s).ok())
        .collect();

    println!("Valid prices: {:?}", valid_prices);

    // Collect both successes and errors
    let results: Vec<Result<f64, String>> = price_strings
        .iter()
        .map(|s| parse_price(s))
        .collect();

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(price) => println!("  [{}] Success: {:.2}", i, price),
            Err(e) => eprintln!("  [{}] Error: {}", i, e),
        }
    }
}
```

### Pattern 3: Collecting Results

```rust
fn validate_order(symbol: &str, qty: i32, price: f64) -> Result<(String, i32, f64), String> {
    if qty <= 0 {
        return Err(format!("Invalid quantity: {}", qty));
    }
    if price <= 0.0 {
        return Err(format!("Invalid price: {}", price));
    }
    Ok((symbol.to_string(), qty, price))
}

fn main() {
    let orders = vec![
        ("AAPL", 100, 150.0),
        ("GOOGL", -50, 2800.0),  // Invalid
        ("MSFT", 200, -10.0),     // Invalid
        ("TSLA", 150, 250.0),
    ];

    // Collect into Result<Vec<_>, String>
    let validated: Result<Vec<_>, String> = orders
        .iter()
        .map(|&(s, q, p)| validate_order(s, q, p))
        .collect();

    match validated {
        Ok(valid_orders) => {
            println!("All orders valid: {} orders", valid_orders.len());
            for order in valid_orders {
                println!("  {:?}", order);
            }
        }
        Err(e) => {
            eprintln!("Validation failed: {}", e);
        }
    }

    // Partition into successes and failures
    let (successes, failures): (Vec<_>, Vec<_>) = orders
        .iter()
        .map(|&(s, q, p)| validate_order(s, q, p))
        .partition(Result::is_ok);

    let valid_orders: Vec<_> = successes.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = failures.into_iter().map(Result::unwrap_err).collect();

    println!("\nValid: {}, Errors: {}", valid_orders.len(), errors.len());
    for error in errors {
        eprintln!("  Error: {}", error);
    }
}
```

### Pattern 4: Early Return with Try Operator

```rust
fn process_orders(order_strings: Vec<&str>) -> Result<Vec<(String, u32, f64)>, String> {
    order_strings
        .iter()
        .map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != 3 {
                return Err(format!("Invalid format: {}", line));
            }

            let symbol = parts[0].to_string();
            let qty = parts[1].parse::<u32>()
                .map_err(|_| format!("Invalid quantity: {}", parts[1]))?;
            let price = parts[2].parse::<f64>()
                .map_err(|_| format!("Invalid price: {}", parts[2]))?;

            Ok((symbol, qty, price))
        })
        .collect()
}

fn main() {
    let orders = vec![
        "AAPL,100,150.0",
        "GOOGL,50,2800.0",
        "MSFT,200,300.0",
    ];

    match process_orders(orders) {
        Ok(parsed) => {
            println!("Parsed {} orders:", parsed.len());
            for (symbol, qty, price) in parsed {
                println!("  {} x{} @ ${:.2}", symbol, qty, price);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // With invalid data
    let invalid_orders = vec![
        "AAPL,100,150.0",
        "INVALID",
        "MSFT,200,300.0",
    ];

    match process_orders(invalid_orders) {
        Ok(_) => println!("Success"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Pattern 5: Custom Error Accumulation

```rust
#[derive(Debug)]
struct ValidationError {
    field: String,
    message: String,
}

fn validate_order_comprehensive(
    symbol: &str,
    qty: i32,
    price: f64,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if symbol.is_empty() {
        errors.push(ValidationError {
            field: "symbol".to_string(),
            message: "Symbol cannot be empty".to_string(),
        });
    }

    if qty <= 0 {
        errors.push(ValidationError {
            field: "quantity".to_string(),
            message: format!("Quantity must be positive, got {}", qty),
        });
    }

    if price <= 0.0 {
        errors.push(ValidationError {
            field: "price".to_string(),
            message: format!("Price must be positive, got {}", price),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn main() {
    let orders = vec![
        ("AAPL", 100, 150.0),
        ("", -50, -10.0),
        ("GOOGL", 200, 2800.0),
    ];

    for (symbol, qty, price) in orders {
        match validate_order_comprehensive(symbol, qty, price) {
            Ok(_) => println!("✓ Order valid: {} x{} @ ${:.2}", symbol, qty, price),
            Err(errors) => {
                eprintln!("✗ Order invalid: {} x{} @ ${:.2}", symbol, qty, price);
                for error in errors {
                    eprintln!("    - {}: {}", error.field, error.message);
                }
            }
        }
    }
}
```

---

## Advanced Patterns

### Pattern 1: Closure as Function Parameter

```rust
fn apply_to_prices<F>(prices: &[f64], operation: F) -> Vec<f64>
where
    F: Fn(f64) -> f64,
{
    prices.iter().map(|&p| operation(p)).collect()
}

fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    let with_fee = apply_to_prices(&prices, |p| p * 1.001);
    let discounted = apply_to_prices(&prices, |p| p * 0.95);

    println!("With fee: {:?}", with_fee);
    println!("Discounted: {:?}", discounted);
}
```

### Pattern 2: Closure as Return Value

```rust
fn create_fee_calculator(base_rate: f64, volume_discount: bool) -> Box<dyn Fn(f64) -> f64> {
    if volume_discount {
        Box::new(move |amount| {
            let rate = if amount > 10000.0 { base_rate * 0.5 } else { base_rate };
            amount * rate
        })
    } else {
        Box::new(move |amount| amount * base_rate)
    }
}

fn main() {
    let standard_calc = create_fee_calculator(0.001, false);
    let discount_calc = create_fee_calculator(0.001, true);

    let amount = 15000.0;
    println!("Standard fee: ${:.2}", standard_calc(amount));
    println!("Discount fee: ${:.2}", discount_calc(amount));
}
```

### Pattern 3: Memoization with Closures

```rust
use std::collections::HashMap;

fn main() {
    let mut cache: HashMap<u32, f64> = HashMap::new();

    let mut calculate_expensive = |qty: u32| -> f64 {
        if let Some(&result) = cache.get(&qty) {
            println!("Cache hit for {}", qty);
            return result;
        }

        println!("Computing for {}...", qty);
        let result = (qty as f64 * 1.5).sqrt(); // Expensive calculation
        cache.insert(qty, result);
        result
    };

    println!("{}", calculate_expensive(100));
    println!("{}", calculate_expensive(200));
    println!("{}", calculate_expensive(100)); // Cache hit
    println!("{}", calculate_expensive(200)); // Cache hit
}
```

### Pattern 4: Strategy Pattern with Closures

```rust
enum OrderType {
    Market,
    Limit,
    Stop,
}

fn execute_order<F>(order_type: OrderType, execution_strategy: F)
where
    F: Fn(&str) -> String,
{
    let strategy_name = match order_type {
        OrderType::Market => "market",
        OrderType::Limit => "limit",
        OrderType::Stop => "stop",
    };

    let result = execution_strategy(strategy_name);
    println!("Execution result: {}", result);
}

fn main() {
    let aggressive_strategy = |order_type: &str| {
        format!("Executing {} order aggressively with minimal delay", order_type)
    };

    let conservative_strategy = |order_type: &str| {
        format!("Executing {} order conservatively with price checks", order_type)
    };

    execute_order(OrderType::Market, aggressive_strategy);
    execute_order(OrderType::Limit, conservative_strategy);
}
```

### Pattern 5: Combinator Pattern

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0, 300.0];

    // Chain of transformations with custom combinators
    let result: f64 = prices
        .iter()
        .filter(|&&p| p > 150.0)                    // Filter
        .map(|&p| p * 1.001)                        // Add fee
        .take(3)                                     // Limit results
        .sum();                                      // Aggregate

    println!("Result: {:.2}", result);

    // Custom combinator
    fn pipe<A, B, C, F, G>(f: F, g: G) -> impl Fn(A) -> C
    where
        F: Fn(A) -> B,
        G: Fn(B) -> C,
    {
        move |x| g(f(x))
    }

    let add_fee = |price: f64| price * 1.001;
    let round = |price: f64| (price * 100.0).round() / 100.0;

    let process_price = pipe(add_fee, round);

    let processed = prices.iter().map(|&p| process_price(p)).collect::<Vec<_>>();
    println!("Processed: {:?}", processed);
}
```

---

## Performance Considerations

### 1. Iterator Lazy Evaluation

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0, 300.0];

    // This doesn't execute until collect()
    let iter = prices
        .iter()
        .inspect(|p| println!("Inspecting: {}", p))
        .filter(|&&p| p > 150.0)
        .map(|p| p * 1.001);

    println!("Iterator created, but not executed yet");

    // Now it executes
    let result: Vec<f64> = iter.collect();
    println!("Result: {:?}", result);
}
```

### 2. Avoid Unnecessary Collections

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0, 250.0, 300.0];

    // Bad: Creates intermediate Vec
    let filtered: Vec<f64> = prices.iter().filter(|&&p| p > 150.0).copied().collect();
    let total: f64 = filtered.iter().sum();

    // Good: Direct chain
    let total: f64 = prices
        .iter()
        .filter(|&&p| p > 150.0)
        .sum();

    println!("Total: {}", total);
}
```

### 3. Use `copied()` or `cloned()` Wisely

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // For Copy types, use copied()
    let doubled: Vec<f64> = prices
        .iter()
        .copied()
        .map(|p| p * 2.0)
        .collect();

    let symbols = vec!["AAPL".to_string(), "GOOGL".to_string()];

    // For non-Copy types, use cloned()
    let symbols_upper: Vec<String> = symbols
        .iter()
        .cloned()
        .map(|s| s.to_uppercase())
        .collect();

    println!("Doubled: {:?}", doubled);
    println!("Upper: {:?}", symbols_upper);
}
```

### 4. Parallel Iteration (with rayon)

```rust
// Add to Cargo.toml: rayon = "1.5"
// use rayon::prelude::*;

// fn main() {
//     let prices: Vec<f64> = (0..1_000_000).map(|i| i as f64).collect();
//
//     // Sequential
//     let sum: f64 = prices.iter().map(|&p| p * 1.001).sum();
//
//     // Parallel (much faster for large datasets)
//     let sum_parallel: f64 = prices.par_iter().map(|&p| p * 1.001).sum();
//
//     println!("Sum: {}", sum);
//     println!("Sum parallel: {}", sum_parallel);
// }
```

---

## Understanding Capture Modes in Closure Chains

### How to Know What's Being Captured

When working with closure chains, understanding what gets borrowed, mutably borrowed, or moved is crucial for avoiding compiler errors and writing efficient code.

#### Visual Guide to Iterator Methods and Borrowing

```rust
fn main() {
    let data = vec![1, 2, 3, 4, 5];

    // .iter() → borrows &T
    data.iter()        // Iterator<Item = &i32>
        .map(|&x| x)   // x is i32 (destructured from &i32)
        .collect::<Vec<i32>>();

    // .iter_mut() → borrows &mut T
    let mut data_mut = vec![1, 2, 3];
    data_mut.iter_mut()     // Iterator<Item = &mut i32>
        .for_each(|x| *x *= 2); // x is &mut i32

    // .into_iter() → takes ownership T
    let data_owned = vec![1, 2, 3];
    data_owned.into_iter()  // Iterator<Item = i32>
        .map(|x| x * 2)     // x is i32 (owned)
        .collect::<Vec<i32>>();
    // data_owned is no longer accessible here
}
```

#### Pattern Matching in Closures

```rust
fn main() {
    let prices = vec![100.0, 150.0, 200.0];

    // Different ways to handle references in closures

    // Method 1: Pattern match to dereference
    prices.iter()
        .filter(|&&p| p > 150.0)  // &&f64 → f64
        .for_each(|&p| println!("{}", p));

    // Method 2: Explicit dereference
    prices.iter()
        .filter(|p| **p > 150.0)  // p is &&f64, **p is f64
        .for_each(|p| println!("{}", *p));

    // Method 3: Use reference in closure
    prices.iter()
        .filter(|p: &&f64| *p > &150.0)  // Compare references
        .for_each(|p| println!("{}", p));

    // Method 4: Use copied() adapter
    prices.iter()
        .copied()                  // Converts &T to T for Copy types
        .filter(|&p| p > 150.0)
        .for_each(|p| println!("{}", p));
}
```

#### Rules for Determining Capture Mode

```rust
fn demonstrate_capture_rules() {
    let immutable_data = vec![1, 2, 3];
    let mut mutable_data = vec![4, 5, 6];
    let expensive_data = vec![vec![1; 1000]; 100];

    // Rule 1: Rust prefers borrowing (&T) if possible
    let borrow_closure = || {
        println!("{:?}", immutable_data);  // Borrows immutable_data
    };
    borrow_closure();
    println!("{:?}", immutable_data);  // Still accessible

    // Rule 2: Mutable borrow (&mut T) when mutation needed
    let mut mutate_closure = || {
        mutable_data.push(7);  // Mutably borrows mutable_data
    };
    mutate_closure();
    // Can't use mutable_data here until mutate_closure is dropped

    // Rule 3: Move (T) when explicitly requested or required
    let move_closure = move || {
        println!("{:?}", expensive_data);  // Moves expensive_data
    };
    move_closure();
    // expensive_data is no longer accessible

    // Rule 4: Move when closure outlives the value
    fn return_closure() -> impl Fn() {
        let local_value = 42;
        move || println!("{}", local_value)  // Must move, not borrow
    }
}
```

#### Common Iterator Chain Patterns and Their Types

```rust
use std::collections::HashMap;

fn iterator_chain_types() {
    let orders = vec![
        ("AAPL", 100, 150.0),
        ("GOOGL", 50, 2800.0),
        ("MSFT", 200, 300.0),
    ];

    // Chain 1: Reference throughout
    let total: f64 = orders
        .iter()                           // &(str, i32, f64)
        .map(|(_, qty, price)| {          // Destructure reference
            *qty as f64 * price           // Dereference as needed
        })
        .sum();

    // Chain 2: Convert to owned with cloned()
    let symbols: Vec<String> = orders
        .iter()
        .map(|(symbol, _, _)| symbol)    // symbol is &&str
        .cloned()                         // &&str → &str
        .map(|s| s.to_string())          // &str → String
        .collect();

    // Chain 3: Move ownership with into_iter()
    let processed: Vec<(String, f64)> = orders
        .into_iter()                     // Moves orders
        .filter(|(_, qty, _)| *qty > 75)
        .map(|(symbol, qty, price)| {
            (symbol.to_string(), qty as f64 * price)
        })
        .collect();
    // orders no longer accessible
}
```

#### Debugging Capture Types

```rust
fn debug_capture_types() {
    let data = vec![1, 2, 3];

    // Trick 1: Use compiler errors to reveal types
    let closure = || {
        let _: () = data;  // Error will show the actual type
        // error: expected `()`, found `Vec<i32>`
    };

    // Trick 2: Use std::mem::size_of_val to check closure size
    let x = 42u8;
    let y = vec![1, 2, 3];

    let closure1 = || x;           // Captures u8 by value
    let closure2 = || &y;          // Captures &Vec
    let closure3 = move || y;      // Moves Vec

    println!("Sizes: {} {} {}",
        std::mem::size_of_val(&closure1),  // 1 byte
        std::mem::size_of_val(&closure2),  // 8 bytes (pointer)
        std::mem::size_of_val(&closure3)); // 24 bytes (Vec)
}
```

#### Practical Examples: Order Book Processing

```rust
#[derive(Clone, Debug)]
struct Order {
    id: String,
    symbol: String,
    quantity: u32,
    price: f64,
}

fn process_order_book() {
    let orders = vec![
        Order { id: "1".to_string(), symbol: "AAPL".to_string(), quantity: 100, price: 150.0 },
        Order { id: "2".to_string(), symbol: "GOOGL".to_string(), quantity: 50, price: 2800.0 },
    ];

    // Example 1: Borrowing for read-only operations
    let total_value: f64 = orders
        .iter()  // Borrows orders
        .map(|order| order.quantity as f64 * order.price)
        .sum();

    println!("Total value: {}", total_value);
    println!("Orders still accessible: {:?}", orders);

    // Example 2: Cloning when needed
    let high_value_orders: Vec<Order> = orders
        .iter()
        .filter(|order| order.quantity as f64 * order.price > 10000.0)
        .cloned()  // Clone the filtered orders
        .collect();

    // Example 3: Moving when transferring ownership
    let symbols: Vec<String> = orders
        .into_iter()  // Moves orders
        .map(|order| order.symbol)  // Takes owned symbol
        .collect();

    // orders is no longer accessible
    // println!("{:?}", orders);  // Error: value moved
}
```

---

## Advanced Closure Features

### 1. Higher-Rank Trait Bounds (HRTBs) with Closures

HRTBs allow you to accept closures that work with any lifetime, crucial for generic APIs.

```rust
// The for<'a> syntax - accepts closures with any lifetime
fn apply_to_ref<F>(f: F) -> i32
where
    F: for<'a> Fn(&'a i32) -> i32,
{
    let x = 42;
    f(&x)
}

// Practical: Processing borrowed data with flexible lifetimes
fn process_orders<F>(orders: &[Order], processor: F)
where
    F: for<'a> Fn(&'a Order) -> bool,
{
    for order in orders {
        if processor(order) {
            println!("Processing order: {}", order.id);
        }
    }
}

// Generic callback that works with any lifetime
fn with_buffer<F>(size: usize, callback: F)
where
    F: for<'r> FnOnce(&'r mut Vec<u8>),
{
    let mut buffer = vec![0u8; size];
    callback(&mut buffer);
}
```

### 2. Recursive Closures

Closures can't directly reference themselves, but we can work around this.

```rust
use std::rc::Rc;
use std::cell::RefCell;

fn recursive_closures() {
    // Method 1: Using Rc and RefCell
    let factorial: Rc<RefCell<Option<Box<dyn Fn(u32) -> u32>>>> =
        Rc::new(RefCell::new(None));

    let factorial_clone = factorial.clone();
    *factorial.borrow_mut() = Some(Box::new(move |n| {
        if n <= 1 {
            1
        } else {
            n * factorial_clone.borrow().as_ref().unwrap()(n - 1)
        }
    }));

    println!("5! = {}", factorial.borrow().as_ref().unwrap()(5));

    // Method 2: Using a helper function
    fn fibonacci_helper(n: u32, cache: &mut Vec<u32>) -> u32 {
        if n < cache.len() as u32 {
            return cache[n as usize];
        }

        let result = fibonacci_helper(n - 1, cache) + fibonacci_helper(n - 2, cache);
        cache.push(result);
        result
    }

    let mut fib = |n: u32| -> u32 {
        let mut cache = vec![0, 1];
        fibonacci_helper(n, &mut cache)
    };

    println!("fib(10) = {}", fib(10));
}
```

### 3. Async Closures

Working with async/await in closures requires special handling.

```rust
use futures::future::BoxFuture;
use std::future::Future;

// Return type for async closures
fn create_async_processor() -> impl Fn(String) -> BoxFuture<'static, Result<(), String>> {
    |order_id| {
        Box::pin(async move {
            // Simulate async order processing
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            println!("Processed order: {}", order_id);
            Ok(())
        })
    }
}

// Using async blocks in iterator chains
async fn process_orders_async(order_ids: Vec<String>) {
    let futures: Vec<_> = order_ids
        .into_iter()
        .map(|id| async move {
            // Each creates a future
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            format!("Processed: {}", id)
        })
        .collect();

    // Await all futures concurrently
    let results = futures::future::join_all(futures).await;
    for result in results {
        println!("{}", result);
    }
}

// Async closure with error handling
async fn validate_orders<F, Fut>(orders: Vec<Order>, validator: F) -> Vec<Result<Order, String>>
where
    F: Fn(Order) -> Fut,
    Fut: Future<Output = Result<Order, String>>,
{
    let futures: Vec<_> = orders.into_iter().map(validator).collect();
    futures::future::join_all(futures).await
}
```

### 4. Non-Capturing Closures as Function Pointers

Non-capturing closures have zero size and can be converted to function pointers.

```rust
fn function_pointer_closures() {
    // Non-capturing closure
    let add_fee = |price: f64| price * 1.001;

    // Can be converted to function pointer
    let fn_ptr: fn(f64) -> f64 = add_fee;

    // Zero-cost abstraction
    println!("Closure size: {}", std::mem::size_of_val(&add_fee));  // 0
    println!("Fn ptr size: {}", std::mem::size_of_val(&fn_ptr));    // 8

    // Useful for C FFI
    extern "C" fn callback(x: i32) -> i32 {
        x * 2
    }

    // Array of function pointers
    let operations: [fn(i32) -> i32; 3] = [
        |x| x + 1,
        |x| x * 2,
        |x| x - 1,
    ];

    for (i, op) in operations.iter().enumerate() {
        println!("Operation {}: {}", i, op(10));
    }

    // Practical: Strategy pattern with zero cost
    enum TradingStrategy {
        Conservative,
        Aggressive,
        Neutral,
    }

    fn get_fee_calculator(strategy: TradingStrategy) -> fn(f64) -> f64 {
        match strategy {
            TradingStrategy::Conservative => |price| price * 0.002,
            TradingStrategy::Aggressive => |price| price * 0.001,
            TradingStrategy::Neutral => |price| price * 0.0015,
        }
    }
}
```

### 5. Clone Closures for Multi-threading

Sharing closures across threads requires careful handling.

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn multi_threaded_closures() {
    // Shared state
    let counter = Arc::new(Mutex::new(0));
    let results = Arc::new(Mutex::new(Vec::new()));

    // Closure factory for thread-safe closures
    let make_processor = {
        let counter = Arc::clone(&counter);
        let results = Arc::clone(&results);

        move || {
            let counter = Arc::clone(&counter);
            let results = Arc::clone(&results);

            move |order_id: String| {
                let mut count = counter.lock().unwrap();
                *count += 1;
                let order_num = *count;

                results.lock().unwrap().push(format!("Order {}: {}", order_num, order_id));
            }
        }
    };

    let mut handles = vec![];

    for i in 0..10 {
        let processor = make_processor();
        handles.push(thread::spawn(move || {
            processor(format!("ORD-{:03}", i));
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Processed {} orders", *counter.lock().unwrap());
    for result in results.lock().unwrap().iter() {
        println!("{}", result);
    }
}
```

### 6. Lifetime Interactions in Closures

Understanding how closures interact with lifetimes is crucial for complex APIs.

```rust
// Closures that return references
fn create_selector<'a, T>(data: &'a [T]) -> impl Fn(usize) -> Option<&'a T> {
    move |index| data.get(index)
}

// Multiple lifetime parameters
fn create_comparator<'a, 'b>() -> impl Fn(&'a str, &'b str) -> bool {
    |a, b| a.len() > b.len()
}

// Practical: Order book with lifetime-aware closures
struct OrderBook<'a> {
    orders: Vec<&'a Order>,
}

impl<'a> OrderBook<'a> {
    fn filter<F>(&self, predicate: F) -> Vec<&'a Order>
    where
        F: Fn(&&Order) -> bool,
    {
        self.orders.iter()
            .copied()
            .filter(predicate)
            .collect()
    }

    fn map_to_values<F, T>(&self, mapper: F) -> Vec<T>
    where
        F: Fn(&'a Order) -> T,
    {
        self.orders.iter()
            .map(|&order| mapper(order))
            .collect()
    }
}
```

### 7. Double Reference Patterns (&&T)

Understanding and working with double references in closures.

```rust
fn double_reference_patterns() {
    let data = vec![100, 200, 300];

    // Understanding &&T patterns
    data.iter()
        .filter(|&&x| x > 150)        // Pattern match &&i32 to i32
        .map(|&x| x * 2)              // Pattern match &i32 to i32
        .for_each(|x| println!("{}", x));

    // Complex nested structures
    let nested = vec![vec![1, 2], vec![3, 4], vec![5, 6]];

    let flattened: Vec<i32> = nested
        .iter()                       // &Vec<i32>
        .flat_map(|inner| inner.iter())  // &&i32
        .map(|&&x| x)                 // i32
        .collect();

    // Pattern matching in tuples
    let pairs = vec![("AAPL", 150.0), ("GOOGL", 2800.0)];

    pairs.iter()
        .map(|&(symbol, price)| (symbol, price * 1.01))  // Destructure &tuple
        .for_each(|(s, p)| println!("{}: {:.2}", s, p));

    // Using as_ref() to work with references
    let options = vec![Some(100), None, Some(200)];

    options.iter()
        .filter_map(|opt| opt.as_ref())  // Option<&i32>
        .map(|&x| x * 2)
        .for_each(|x| println!("{}", x));
}
```

### 8. Custom Iterator Adapters

Creating your own iterator methods that accept closures.

```rust
trait IteratorExt: Iterator {
    // Group consecutive elements by a key function
    fn chunk_by_key<K, F>(self, key_fn: F) -> ChunkByKey<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: PartialEq,
    {
        ChunkByKey {
            iter: self,
            key_fn,
            current: None,
        }
    }

    // Apply transformation with index
    fn map_indexed<B, F>(self, f: F) -> MapIndexed<Self, F>
    where
        Self: Sized,
        F: FnMut(usize, Self::Item) -> B,
    {
        MapIndexed {
            iter: self,
            f,
            index: 0,
        }
    }
}

impl<I: Iterator> IteratorExt for I {}

// Implementation structs
struct ChunkByKey<I, F> {
    iter: I,
    key_fn: F,
    current: Option<(I::Item, )>,
}

struct MapIndexed<I, F> {
    iter: I,
    f: F,
    index: usize,
}

impl<I, F, B> Iterator for MapIndexed<I, F>
where
    I: Iterator,
    F: FnMut(usize, I::Item) -> B,
{
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| {
            let index = self.index;
            self.index += 1;
            (self.f)(index, item)
        })
    }
}

// Usage
fn use_custom_adapters() {
    let data = vec![1, 2, 3, 4, 5];

    let indexed: Vec<_> = data.iter()
        .map_indexed(|i, &x| format!("{}:{}", i, x))
        .collect();

    println!("{:?}", indexed);  // ["0:1", "1:2", "2:3", "3:4", "4:5"]
}
```

### 9. Closure Size and Memory Optimization

Understanding and optimizing closure memory usage.

```rust
use std::mem;

fn closure_size_optimization() {
    let small_data = 42u8;
    let medium_data = vec![1; 100];
    let large_data = vec![vec![1; 1000]; 100];

    // Measure closure sizes
    let closure_small = || small_data;
    let closure_medium = || medium_data.clone();
    let closure_large = move || large_data.clone();

    println!("Closure sizes:");
    println!("  Small: {} bytes", mem::size_of_val(&closure_small));
    println!("  Medium: {} bytes", mem::size_of_val(&closure_medium));
    println!("  Large: {} bytes", mem::size_of_val(&closure_large));

    // Optimization: Capture only what you need
    let data = vec![1; 1000];
    let len = data.len();

    // Bad: Captures entire Vec
    let bad_closure = move || {
        println!("Length: {}", data.len());
    };

    // Good: Captures only length
    let good_closure = move || {
        println!("Length: {}", len);
    };

    println!("Bad closure: {} bytes", mem::size_of_val(&bad_closure));
    println!("Good closure: {} bytes", mem::size_of_val(&good_closure));

    // Practical: Order processing optimization
    struct LargeOrder {
        id: String,
        data: Vec<u8>,  // Large payload
    }

    impl LargeOrder {
        fn process_efficiently<F>(&self, processor: F)
        where
            F: FnOnce(&str),
        {
            // Only pass the ID, not the entire struct
            processor(&self.id);
        }
    }
}
```

### 10. Type Inference Limitations and Solutions

Sometimes Rust can't infer types in closure chains.

```rust
fn type_inference_challenges() {
    // Problem 1: Can't infer collection type
    let numbers = vec!["1", "2", "3"];

    // Won't compile - ambiguous collection type
    // let parsed = numbers.iter().map(|s| s.parse().unwrap()).collect();

    // Solution 1: Type annotation
    let parsed: Vec<i32> = numbers.iter()
        .map(|s| s.parse().unwrap())
        .collect();

    // Solution 2: Turbofish
    let parsed = numbers.iter()
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<Vec<_>>();

    // Problem 2: Complex closure return types
    let processor = |x: i32| {
        if x > 0 {
            Ok(x as f64)
        } else {
            Err("Negative value")
        }
    };

    // Need to specify Result types sometimes
    let results: Vec<Result<f64, &str>> = vec![-1, 2, 3]
        .into_iter()
        .map(processor)
        .collect();

    // Problem 3: Higher-order functions
    fn apply_twice<F, T>(f: F, value: T) -> T
    where
        F: Fn(T) -> T,
        T: Copy,
    {
        f(f(value))
    }

    // Sometimes need explicit type parameters
    let result = apply_twice::<_, i32>(|x| x * 2, 5);
}
```

### 11. Impl Trait and Closure Return Types

Advanced patterns for returning closures from functions.

```rust
// Basic: Return closure with impl Trait
fn create_multiplier(factor: i32) -> impl Fn(i32) -> i32 {
    move |x| x * factor
}

// Can't use impl Trait in traits (before Rust 1.75)
trait OrderProcessor {
    // This requires nightly or newer Rust
    // fn create_validator(&self) -> impl Fn(&Order) -> bool;

    // Stable workaround
    fn create_validator(&self) -> Box<dyn Fn(&Order) -> bool>;
}

// Multiple closure return types
fn create_calculator(use_fee: bool) -> Box<dyn Fn(f64) -> f64> {
    if use_fee {
        Box::new(|price| price * 1.001)
    } else {
        Box::new(|price| price)
    }
}

// Generic closure return with lifetime
fn create_filter<'a>(threshold: f64) -> impl Fn(&'a Order) -> bool + 'a {
    move |order: &Order| order.price > threshold
}
```

### 12. Closure Coercion and Trait Hierarchy

Understanding how closure traits coerce.

```rust
fn closure_coercion() {
    // Hierarchy: Fn : FnMut : FnOnce

    fn takes_fn<F: Fn()>(f: F) {
        f();
        f();  // Can call multiple times
    }

    fn takes_fn_mut<F: FnMut()>(mut f: F) {
        f();
        f();  // Can call multiple times and mutate
    }

    fn takes_fn_once<F: FnOnce()>(f: F) {
        f();  // Can only call once
    }

    // Fn closure (most restrictive)
    let print_hello = || println!("Hello");
    takes_fn(print_hello);
    takes_fn_mut(print_hello);    // Fn coerces to FnMut
    takes_fn_once(print_hello);   // Fn coerces to FnOnce

    // FnMut closure
    let mut counter = 0;
    let mut increment = || counter += 1;
    // takes_fn(increment);       // Error: FnMut doesn't coerce to Fn
    takes_fn_mut(increment);
    takes_fn_once(increment);     // FnMut coerces to FnOnce

    // FnOnce closure
    let data = vec![1, 2, 3];
    let consume = move || drop(data);
    // takes_fn(consume);         // Error: FnOnce doesn't coerce to Fn
    // takes_fn_mut(consume);     // Error: FnOnce doesn't coerce to FnMut
    takes_fn_once(consume);
}
```

### 13. Performance: Iterator Fusion and Optimization

Advanced performance patterns with closures.

```rust
fn performance_optimizations() {
    let data = vec![1; 1_000_000];

    // Bad: Multiple allocations
    let bad_result: Vec<i32> = data.iter()
        .map(|&x| x * 2)
        .collect::<Vec<_>>()  // First allocation
        .into_iter()
        .filter(|&x| x > 1)
        .collect();           // Second allocation

    // Good: Single allocation (iterator fusion)
    let good_result: Vec<i32> = data.iter()
        .map(|&x| x * 2)
        .filter(|&x| x > 1)
        .collect();

    // Excellent: No allocation with fold
    let sum: i64 = data.iter()
        .map(|&x| x as i64)
        .filter(|&x| x > 0)
        .fold(0i64, |acc, x| acc + x);

    // Loop fusion with multiple operations
    let (sum, count, max) = data.iter()
        .fold((0i64, 0usize, 0i32), |(sum, count, max), &x| {
            (sum + x as i64, count + 1, max.max(x))
        });

    println!("Sum: {}, Count: {}, Max: {}", sum, count, max);

    // Using specialized methods for better performance
    use std::cmp;

    // Instead of: data.iter().map(|x| x * 2).max()
    // Use: data.iter().max().map(|x| x * 2)

    let max_doubled = data.iter()
        .max()
        .map(|&x| x * 2);
}
```

### 14. Const Generics with Closures

Using const generics to create specialized closure-based functions.

```rust
fn const_generic_closures() {
    // Apply operation N times
    fn apply_n_times<F, const N: usize>(mut f: F, initial: i32) -> i32
    where
        F: FnMut(i32) -> i32,
    {
        let mut result = initial;
        for _ in 0..N {
            result = f(result);
        }
        result
    }

    let double = |x| x * 2;
    let result = apply_n_times::<_, 3>(double, 5);  // 5 * 2 * 2 * 2 = 40
    println!("Result: {}", result);

    // Fixed-size processing
    fn process_batch<F, T, const SIZE: usize>(items: [T; SIZE], processor: F) -> [T; SIZE]
    where
        F: Fn(T) -> T,
        T: Copy,
    {
        let mut result = items;
        for i in 0..SIZE {
            result[i] = processor(result[i]);
        }
        result
    }

    let prices = [100.0, 150.0, 200.0];
    let with_fees = process_batch(prices, |p| p * 1.001);
    println!("With fees: {:?}", with_fees);
}
```

### 15. Advanced Error Handling Patterns

Sophisticated error handling with closures and combinators.

```rust
use std::error::Error;

fn advanced_error_handling() {
    // Custom error combinator
    fn try_with_fallback<T, E, F, G>(primary: F, fallback: G) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        G: FnOnce(E) -> Result<T, E>,
    {
        primary().or_else(fallback)
    }

    // Retry with exponential backoff
    fn retry_with_backoff<T, E, F>(mut operation: F, max_attempts: u32) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut attempt = 0;
        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) if attempt >= max_attempts - 1 => return Err(e),
                Err(_) => {
                    attempt += 1;
                    std::thread::sleep(
                        std::time::Duration::from_millis(100 * 2_u64.pow(attempt))
                    );
                }
            }
        }
    }

    // Transaction pattern
    fn with_transaction<T, E, F>(operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        println!("Starting transaction");
        let result = operation();
        match &result {
            Ok(_) => println!("Committing transaction"),
            Err(_) => println!("Rolling back transaction"),
        }
        result
    }

    // Chain of validators
    type Validator<T> = Box<dyn Fn(&T) -> Result<(), String>>;

    fn validate_with_chain<T>(value: &T, validators: Vec<Validator<T>>) -> Result<(), Vec<String>> {
        let errors: Vec<String> = validators
            .iter()
            .filter_map(|validator| validator(value).err())
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

---

## Summary

### Most Common Closure Patterns

1. **map** - Transform each element
2. **filter** - Keep matching elements
3. **fold/reduce** - Aggregate to single value
4. **for_each** - Execute side effects
5. **find** - Find first match
6. **any/all** - Boolean checks
7. **collect** - Build collections

### Closure Traits Hierarchy

```
FnOnce (least restrictive - can only call once)
  ↓
FnMut (can mutate captured variables)
  ↓
Fn (most restrictive, can be called multiple times)
```

### Understanding Capture Modes

1. **`.iter()`** → Borrows elements as `&T`
2. **`.iter_mut()`** → Borrows elements as `&mut T`
3. **`.into_iter()`** → Takes ownership of elements as `T`
4. **`move` keyword** → Forces closure to take ownership
5. **Pattern matching** → Use `|&x|` or `|&&x|` to destructure references

### Best Practices

1. **Prefer iterators over loops** - More expressive and potentially faster
2. **Chain operations** - Avoid intermediate collections
3. **Use type inference** - Let Rust infer closure types when possible
4. **Handle errors gracefully** - Use `filter_map`, `Result`, `Option`
5. **Be mindful of moves** - Understand when closures capture by value
6. **Profile performance** - Iterators are usually fast, but measure
7. **Optimize closure size** - Capture only what you need
8. **Use HRTBs** - For flexible lifetime handling in generic APIs

### Common Mistakes to Avoid

1. Unnecessary `.collect()` in chains
2. Not handling errors in `filter_map` chains
3. Capturing too much in closures (use specific captures)
4. Forgetting about lazy evaluation
5. Using `unwrap()` instead of proper error handling
6. Not understanding double reference patterns (`&&T`)
7. Fighting the borrow checker instead of using `.clone()` when appropriate

### When to Use What

- **Functions**: Reusable, named logic
- **Closures**: One-off operations, capturing environment
- **Iterators**: Data transformation pipelines
- **Loops**: When you need complex control flow or early breaks
- **Async closures**: For concurrent/parallel operations
- **Function pointers**: For FFI or when zero-size is critical

This guide covers closures from basics to advanced patterns, with practical examples from trading and order book domains!
