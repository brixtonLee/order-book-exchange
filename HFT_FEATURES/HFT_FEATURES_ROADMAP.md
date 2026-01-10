# High-Frequency Trading Features Roadmap

> A comprehensive guide to advanced HFT features for the Rust Order Book, with architecture decisions, code patterns, and alternatives.

---

## Table of Contents

1. [Latency Measurement Infrastructure](#1-latency-measurement-infrastructure)
2. [Stop-Loss & Stop-Limit Orders](#2-stop-loss--stop-limit-orders)
3. [Iceberg / Hidden Orders](#3-iceberg--hidden-orders)
4. [Order Book Imbalance & Microprice](#4-order-book-imbalance--microprice)
5. [TWAP/VWAP Execution Algorithms](#5-twapvwap-execution-algorithms)
6. [Binary Protocol Implementation](#6-binary-protocol-implementation)
7. [Write-Ahead Log (WAL) Persistence](#7-write-ahead-log-wal-persistence)
8. [Circuit Breakers & Risk Controls](#8-circuit-breakers--risk-controls)
9. [Memory-Mapped Ring Buffer (Disruptor Pattern)](#9-memory-mapped-ring-buffer-disruptor-pattern)

---

## 1. Latency Measurement Infrastructure

### Why This Matters

In HFT, **latency is money**. A 1-microsecond advantage can mean millions in profit annually. You cannot optimize what you cannot measure. Professional trading systems track latency at multiple points:

- **Wire-to-wire**: Network packet arrival to response sent
- **Matching latency**: Time to execute the matching algorithm
- **Queue time**: Time spent waiting for lock acquisition
- **End-to-end**: Order submission to fill confirmation

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Latency Measurement Flow                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [Incoming Order] ──┬──> timestamp_t1 (arrival)                 │
│                     │                                            │
│                     ▼                                            │
│              ┌─────────────┐                                     │
│              │ Parse/Decode │ ──> timestamp_t2                   │
│              └─────────────┘                                     │
│                     │                                            │
│                     ▼                                            │
│              ┌─────────────┐                                     │
│              │ Acquire Lock │ ──> timestamp_t3 (lock acquired)  │
│              └─────────────┘                                     │
│                     │                                            │
│                     ▼                                            │
│              ┌─────────────┐                                     │
│              │   Matching   │ ──> timestamp_t4 (match complete) │
│              └─────────────┘                                     │
│                     │                                            │
│                     ▼                                            │
│              ┌─────────────┐                                     │
│              │  Response    │ ──> timestamp_t5 (sent)           │
│              └─────────────┘                                     │
│                                                                  │
│  Metrics:                                                        │
│    - Decode latency:   t2 - t1                                  │
│    - Lock wait:        t3 - t2                                  │
│    - Match latency:    t4 - t3  ← Most critical                 │
│    - Total latency:    t5 - t1                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/metrics/latency.rs`**

```rust
use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// High-precision latency tracker using HDR Histogram
/// HDR Histograms provide accurate percentile calculations with minimal memory
pub struct LatencyTracker {
    /// Matching engine latency (most critical metric)
    matching_latency_ns: Histogram<u64>,

    /// Lock acquisition wait time
    lock_wait_ns: Histogram<u64>,

    /// End-to-end order processing
    total_latency_ns: Histogram<u64>,

    /// WebSocket broadcast latency
    broadcast_latency_ns: Histogram<u64>,

    /// Counter for total measurements
    sample_count: AtomicU64,
}

impl LatencyTracker {
    pub fn new() -> Self {
        // Configure histogram: 1ns to 10 seconds, 3 significant figures
        Self {
            matching_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            lock_wait_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            total_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            broadcast_latency_ns: Histogram::new_with_bounds(1, 10_000_000_000, 3).unwrap(),
            sample_count: AtomicU64::new(0),
        }
    }

    /// Record matching engine latency
    #[inline]
    pub fn record_matching(&mut self, start: Instant) {
        let nanos = start.elapsed().as_nanos() as u64;
        let _ = self.matching_latency_ns.record(nanos);
        self.sample_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get latency statistics
    pub fn stats(&self) -> LatencyStats {
        LatencyStats {
            p50_ns: self.matching_latency_ns.value_at_percentile(50.0),
            p95_ns: self.matching_latency_ns.value_at_percentile(95.0),
            p99_ns: self.matching_latency_ns.value_at_percentile(99.0),
            p999_ns: self.matching_latency_ns.value_at_percentile(99.9),
            max_ns: self.matching_latency_ns.max(),
            min_ns: self.matching_latency_ns.min(),
            mean_ns: self.matching_latency_ns.mean(),
            sample_count: self.sample_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LatencyStats {
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,  // Critical for HFT - tail latency matters!
    pub max_ns: u64,
    pub min_ns: u64,
    pub mean_ns: f64,
    pub sample_count: u64,
}

/// RAII guard for automatic latency measurement
pub struct LatencyGuard<'a> {
    tracker: &'a mut LatencyTracker,
    start: Instant,
    metric_type: MetricType,
}

pub enum MetricType {
    Matching,
    LockWait,
    Total,
    Broadcast,
}

impl<'a> LatencyGuard<'a> {
    pub fn new(tracker: &'a mut LatencyTracker, metric_type: MetricType) -> Self {
        Self {
            tracker,
            start: Instant::now(),
            metric_type,
        }
    }
}

impl<'a> Drop for LatencyGuard<'a> {
    fn drop(&mut self) {
        let nanos = self.start.elapsed().as_nanos() as u64;
        match self.metric_type {
            MetricType::Matching => { let _ = self.tracker.matching_latency_ns.record(nanos); }
            MetricType::LockWait => { let _ = self.tracker.lock_wait_ns.record(nanos); }
            MetricType::Total => { let _ = self.tracker.total_latency_ns.record(nanos); }
            MetricType::Broadcast => { let _ = self.tracker.broadcast_latency_ns.record(nanos); }
        }
    }
}
```

### For Ultra-Low Latency: CPU Cycle Counting

```rust
/// Use RDTSC for sub-nanosecond precision (x86_64 only)
#[cfg(target_arch = "x86_64")]
pub fn rdtsc() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

/// Convert cycles to nanoseconds (calibrate at startup)
pub struct CycleCounter {
    cycles_per_ns: f64,
}

impl CycleCounter {
    pub fn calibrate() -> Self {
        let start_cycles = rdtsc();
        let start_time = Instant::now();

        // Spin for 100ms to calibrate
        std::thread::sleep(std::time::Duration::from_millis(100));

        let elapsed_ns = start_time.elapsed().as_nanos() as f64;
        let elapsed_cycles = (rdtsc() - start_cycles) as f64;

        Self {
            cycles_per_ns: elapsed_cycles / elapsed_ns,
        }
    }

    pub fn cycles_to_ns(&self, cycles: u64) -> f64 {
        cycles as f64 / self.cycles_per_ns
    }
}
```

### Dependencies Required

```toml
# Cargo.toml additions
[dependencies]
hdrhistogram = "7.5"          # Industry-standard latency histograms
```

### Alternatives Comparison

| Approach | Precision | Overhead | Use Case |
|----------|-----------|----------|----------|
| `Instant::now()` | ~20ns | Low | General measurement |
| `RDTSC` | ~1ns | Minimal | Ultra-low latency |
| `clock_gettime(MONOTONIC)` | ~20ns | Low | Cross-platform |
| `quanta` crate | ~10ns | Low | Best of both worlds |

**Recommended Alternative:** The `quanta` crate provides a nice abstraction:

```rust
use quanta::Clock;

let clock = Clock::new();
let start = clock.raw();
// ... operation ...
let elapsed_ns = clock.delta_as_nanos(start, clock.raw());
```

---

## 2. Stop-Loss & Stop-Limit Orders

### Why This Matters

Stop orders are **conditional orders** that become active when a trigger price is reached. They're essential for:
- Risk management (automatic loss limiting)
- Breakout trading strategies
- Trailing stop strategies

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Stop Order Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Main Order Book              Trigger Engine (Separate)        │
│  ┌─────────────────┐         ┌─────────────────────────┐        │
│  │                 │         │                         │        │
│  │  Active Orders  │         │  Stop Orders by Trigger │        │
│  │  (can match)    │         │  Price (dormant)        │        │
│  │                 │         │                         │        │
│  │  Bids │ Asks    │         │  BTreeMap<Price, Vec>   │        │
│  │       │         │         │                         │        │
│  └───────┼─────────┘         └───────────┬─────────────┘        │
│          │                               │                       │
│          │ Trade Executes                │                       │
│          ▼                               │                       │
│  ┌─────────────────┐                     │                       │
│  │  Trade Event    │ ────────────────────┘                       │
│  │  price: $100.50 │         │                                   │
│  └─────────────────┘         │ Check triggers                    │
│                              ▼                                   │
│                    ┌─────────────────────────┐                   │
│                    │ Triggered? Submit order │                   │
│                    │ to main book            │                   │
│                    └─────────────────────────┘                   │
│                                                                  │
│  Key Insight: Stop orders live in a SEPARATE data structure     │
│  and only enter the main book when triggered.                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/models/stop_order.rs`**

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Condition that triggers the stop order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TriggerCondition {
    /// Trigger when last trade price >= trigger_price (for buy stops)
    AtOrAbove,
    /// Trigger when last trade price <= trigger_price (for sell stops)
    AtOrBelow,
    /// Trigger when last trade price > trigger_price
    Above,
    /// Trigger when last trade price < trigger_price
    Below,
}

/// Type of order to submit when triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StopOrderType {
    /// Stop-Market: Submit market order when triggered
    StopMarket,
    /// Stop-Limit: Submit limit order at specified price when triggered
    StopLimit,
    /// Trailing Stop: Trigger price follows market by offset
    TrailingStop,
}

/// A stop order waiting to be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopOrder {
    pub id: Uuid,
    pub symbol: String,
    pub user_id: String,

    // Trigger configuration
    pub trigger_price: Decimal,
    pub trigger_condition: TriggerCondition,
    pub stop_type: StopOrderType,

    // Order to submit when triggered
    pub side: OrderSide,
    pub quantity: Decimal,
    pub limit_price: Option<Decimal>,  // For stop-limit orders

    // Trailing stop specific
    pub trail_amount: Option<Decimal>,      // Fixed offset
    pub trail_percent: Option<Decimal>,     // Percentage offset
    pub highest_price: Option<Decimal>,     // Tracked high (for sell trailing)
    pub lowest_price: Option<Decimal>,      // Tracked low (for buy trailing)

    // Metadata
    pub created_at: DateTime<Utc>,
    pub expire_time: Option<DateTime<Utc>>,
    pub status: StopOrderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopOrderStatus {
    Pending,    // Waiting for trigger
    Triggered,  // Trigger hit, order submitted
    Cancelled,  // User cancelled
    Expired,    // Time expired
    Rejected,   // Triggered order was rejected
}

impl StopOrder {
    /// Check if this stop should trigger given the last trade price
    pub fn should_trigger(&self, last_price: Decimal) -> bool {
        match self.trigger_condition {
            TriggerCondition::AtOrAbove => last_price >= self.trigger_price,
            TriggerCondition::AtOrBelow => last_price <= self.trigger_price,
            TriggerCondition::Above => last_price > self.trigger_price,
            TriggerCondition::Below => last_price < self.trigger_price,
        }
    }

    /// Update trailing stop trigger price based on market movement
    pub fn update_trailing(&mut self, last_price: Decimal) {
        match (self.side, self.trail_amount, self.trail_percent) {
            // Sell trailing stop: trigger follows price UP
            (OrderSide::Sell, Some(offset), _) => {
                let new_high = self.highest_price.unwrap_or(last_price).max(last_price);
                self.highest_price = Some(new_high);
                self.trigger_price = new_high - offset;
            }
            (OrderSide::Sell, _, Some(pct)) => {
                let new_high = self.highest_price.unwrap_or(last_price).max(last_price);
                self.highest_price = Some(new_high);
                self.trigger_price = new_high * (Decimal::ONE - pct / Decimal::ONE_HUNDRED);
            }
            // Buy trailing stop: trigger follows price DOWN
            (OrderSide::Buy, Some(offset), _) => {
                let new_low = self.lowest_price.unwrap_or(last_price).min(last_price);
                self.lowest_price = Some(new_low);
                self.trigger_price = new_low + offset;
            }
            (OrderSide::Buy, _, Some(pct)) => {
                let new_low = self.lowest_price.unwrap_or(last_price).min(last_price);
                self.lowest_price = Some(new_low);
                self.trigger_price = new_low * (Decimal::ONE + pct / Decimal::ONE_HUNDRED);
            }
            _ => {}
        }
    }
}
```

**File: `src/engine/trigger.rs`**

```rust
use std::collections::BTreeMap;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::{Order, OrderType, StopOrder, StopOrderType};

/// Engine that monitors prices and triggers stop orders
pub struct TriggerEngine {
    /// Stop orders indexed by trigger price for efficient scanning
    /// Key: trigger price, Value: orders at that trigger level
    buy_stops: BTreeMap<Decimal, Vec<StopOrder>>,   // Trigger at or above
    sell_stops: BTreeMap<Decimal, Vec<StopOrder>>,  // Trigger at or below

    /// Index for O(1) lookup by order ID
    order_index: HashMap<Uuid, Decimal>,  // order_id -> trigger_price

    /// Last known trade price
    last_trade_price: Option<Decimal>,
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self {
            buy_stops: BTreeMap::new(),
            sell_stops: BTreeMap::new(),
            order_index: HashMap::new(),
            last_trade_price: None,
        }
    }

    /// Add a new stop order
    pub fn add_stop_order(&mut self, stop: StopOrder) {
        self.order_index.insert(stop.id, stop.trigger_price);

        let map = match stop.side {
            OrderSide::Buy => &mut self.buy_stops,
            OrderSide::Sell => &mut self.sell_stops,
        };

        map.entry(stop.trigger_price)
            .or_insert_with(Vec::new)
            .push(stop);
    }

    /// Cancel a stop order
    pub fn cancel_stop_order(&mut self, order_id: Uuid) -> Option<StopOrder> {
        if let Some(trigger_price) = self.order_index.remove(&order_id) {
            // Search in both maps
            for map in [&mut self.buy_stops, &mut self.sell_stops] {
                if let Some(orders) = map.get_mut(&trigger_price) {
                    if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                        return Some(orders.remove(pos));
                    }
                }
            }
        }
        None
    }

    /// Process a new trade and return any triggered orders
    ///
    /// This is called after every trade execution in the matching engine.
    /// Returns a Vec of Orders ready to be submitted to the main order book.
    pub fn on_trade(&mut self, trade_price: Decimal) -> Vec<Order> {
        let mut triggered_orders = Vec::new();

        // Update trailing stops first
        self.update_trailing_stops(trade_price);

        // Check buy stops (trigger at or above)
        // Use range to efficiently get all stops at or below current price
        let triggered_buy_prices: Vec<Decimal> = self.buy_stops
            .range(..=trade_price)
            .map(|(price, _)| *price)
            .collect();

        for price in triggered_buy_prices {
            if let Some(stops) = self.buy_stops.remove(&price) {
                for stop in stops {
                    if stop.should_trigger(trade_price) {
                        triggered_orders.push(self.convert_to_order(stop));
                    }
                }
            }
        }

        // Check sell stops (trigger at or below)
        let triggered_sell_prices: Vec<Decimal> = self.sell_stops
            .range(trade_price..)
            .map(|(price, _)| *price)
            .collect();

        for price in triggered_sell_prices {
            if let Some(stops) = self.sell_stops.remove(&price) {
                for stop in stops {
                    if stop.should_trigger(trade_price) {
                        triggered_orders.push(self.convert_to_order(stop));
                    }
                }
            }
        }

        self.last_trade_price = Some(trade_price);
        triggered_orders
    }

    /// Convert a triggered stop order into a regular order
    fn convert_to_order(&self, stop: StopOrder) -> Order {
        Order {
            id: Uuid::new_v4(),  // New ID for the actual order
            symbol: stop.symbol,
            side: stop.side,
            order_type: match stop.stop_type {
                StopOrderType::StopMarket => OrderType::Market,
                StopOrderType::StopLimit | StopOrderType::TrailingStop => OrderType::Limit,
            },
            price: stop.limit_price,
            quantity: stop.quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: stop.user_id,
            timestamp: Utc::now(),
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            triggered_by: Some(stop.id),  // Link back to original stop
        }
    }

    fn update_trailing_stops(&mut self, price: Decimal) {
        // Update all trailing stops with new price
        for stops in self.buy_stops.values_mut() {
            for stop in stops.iter_mut() {
                if stop.stop_type == StopOrderType::TrailingStop {
                    stop.update_trailing(price);
                }
            }
        }
        for stops in self.sell_stops.values_mut() {
            for stop in stops.iter_mut() {
                if stop.stop_type == StopOrderType::TrailingStop {
                    stop.update_trailing(price);
                }
            }
        }
    }
}
```

### Integration with Matching Engine

```rust
// In src/engine/matching.rs - after a trade executes:

impl MatchingEngine {
    pub fn execute_order(&mut self, order: Order) -> MatchResult {
        let trades = self.match_order(order);

        // After matching, check for triggered stops
        for trade in &trades {
            let triggered = self.trigger_engine.on_trade(trade.price);

            // Submit triggered orders back to matching engine
            for triggered_order in triggered {
                // These get processed in the next matching cycle
                // to avoid recursive matching during a single order
                self.pending_triggered_orders.push(triggered_order);
            }
        }

        MatchResult { trades, triggered_count: triggered.len() }
    }
}
```

### Alternatives Comparison

| Approach | Pros | Cons |
|----------|------|------|
| **BTreeMap by trigger price** | O(log n) trigger check, efficient range scans | Re-indexing needed for trailing stops |
| **Priority Queue (BinaryHeap)** | O(1) next trigger access | Can't efficiently update trailing stops |
| **Interval Tree** | Great for range triggers | Complex implementation |
| **Linear scan** | Simple | O(n) on every trade - too slow |

**Recommendation:** BTreeMap is ideal for most cases. For high-volume trailing stops, consider a separate data structure that's optimized for frequent updates.

---

## 3. Iceberg / Hidden Orders

### Why This Matters

Large institutional orders can **move the market** if their full size is visible. Iceberg orders solve this by:
- Only showing a small "tip" of the order
- Automatically replenishing from hidden quantity
- Reducing information leakage to other market participants

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Iceberg Order Lifecycle                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Total Order: 10,000 shares                                     │
│  Display Quantity: 500 shares                                   │
│  Hidden Quantity: 9,500 shares                                  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Book View (what others see)     │ Reality                │   │
│  ├──────────────────────────────────┼───────────────────────┤   │
│  │ Price    │ Qty                   │ Total                 │   │
│  │ $100.05  │ 500 ◄─ visible tip    │ 10,000 total         │   │
│  │ $100.04  │ 1,200                 │ 1,200                 │   │
│  │ $100.03  │ 800                   │ 800                   │   │
│  └──────────────────────────────────┴───────────────────────┘   │
│                                                                  │
│  Fill Sequence:                                                  │
│                                                                  │
│  1. Market sell 300 shares arrives                              │
│     → Fills 300 from visible (500 → 200 visible)                │
│                                                                  │
│  2. Market sell 250 shares arrives                              │
│     → Fills 200 visible + 50 from hidden                        │
│     → Replenish: hidden (9,450) → visible (500)                 │
│     → NEW TIMESTAMP (loses time priority!)                      │
│                                                                  │
│  Key Insight: When hidden portion replenishes visible,          │
│  the order gets a NEW timestamp = loses queue priority          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/models/iceberg.rs`**

```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Iceberg order configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergConfig {
    /// Total quantity of the entire order (visible + hidden)
    pub total_quantity: Decimal,

    /// Quantity to display in the order book
    pub display_quantity: Decimal,

    /// Current hidden quantity remaining
    pub hidden_quantity: Decimal,

    /// Minimum visible quantity before replenishing
    /// Some exchanges replenish when visible hits 0, others at a threshold
    pub replenish_threshold: Decimal,

    /// Variance to add to display quantity on replenish (anti-detection)
    /// e.g., 0.1 means ±10% randomization
    pub display_variance: Option<Decimal>,
}

impl IcebergConfig {
    pub fn new(total: Decimal, display: Decimal) -> Self {
        Self {
            total_quantity: total,
            display_quantity: display.min(total),
            hidden_quantity: (total - display).max(Decimal::ZERO),
            replenish_threshold: Decimal::ZERO,
            display_variance: None,
        }
    }

    /// Process a fill against the visible portion
    /// Returns: (filled_qty, should_replenish, new_display_qty)
    pub fn process_fill(&mut self, fill_qty: Decimal) -> IcebergFillResult {
        let actual_fill = fill_qty.min(self.display_quantity);
        self.display_quantity -= actual_fill;
        self.total_quantity -= actual_fill;

        // Check if we need to replenish from hidden
        if self.display_quantity <= self.replenish_threshold && self.hidden_quantity > Decimal::ZERO {
            let replenish_amount = self.calculate_replenish_amount();
            self.hidden_quantity -= replenish_amount;
            self.display_quantity += replenish_amount;

            IcebergFillResult {
                filled_quantity: actual_fill,
                replenished: true,
                new_display_quantity: self.display_quantity,
                remaining_hidden: self.hidden_quantity,
            }
        } else {
            IcebergFillResult {
                filled_quantity: actual_fill,
                replenished: false,
                new_display_quantity: self.display_quantity,
                remaining_hidden: self.hidden_quantity,
            }
        }
    }

    fn calculate_replenish_amount(&self) -> Decimal {
        let base_amount = self.display_quantity;

        // Apply variance if configured (helps avoid detection)
        if let Some(variance) = self.display_variance {
            // In production, use actual randomness
            let factor = Decimal::ONE; // + random(-variance, +variance)
            (base_amount * factor).min(self.hidden_quantity)
        } else {
            base_amount.min(self.hidden_quantity)
        }
    }

    /// Check if the entire iceberg is complete
    pub fn is_complete(&self) -> bool {
        self.total_quantity.is_zero()
    }
}

#[derive(Debug, Clone)]
pub struct IcebergFillResult {
    pub filled_quantity: Decimal,
    pub replenished: bool,
    pub new_display_quantity: Decimal,
    pub remaining_hidden: Decimal,
}
```

**Integration with Order model:**

```rust
// In src/models/order.rs - extend the Order struct

pub struct Order {
    // ... existing fields ...

    /// Iceberg configuration (None for regular orders)
    pub iceberg: Option<IcebergConfig>,
}

impl Order {
    /// Get the quantity visible in the order book
    pub fn visible_quantity(&self) -> Decimal {
        match &self.iceberg {
            Some(config) => config.display_quantity,
            None => self.quantity - self.filled_quantity,
        }
    }

    /// Process a fill, handling iceberg replenishment
    pub fn apply_fill(&mut self, fill_qty: Decimal) -> bool {
        self.filled_quantity += fill_qty;

        if let Some(ref mut iceberg) = self.iceberg {
            let result = iceberg.process_fill(fill_qty);

            if result.replenished {
                // IMPORTANT: Update timestamp - order loses time priority!
                self.timestamp = Utc::now();
                return true; // Signal that order was modified
            }
        }

        false
    }
}
```

### Matching Engine Integration

```rust
// In matching.rs - handle iceberg orders specially

fn match_at_price_level(&mut self, incoming: &mut Order, level: &mut PriceLevel) -> Vec<Trade> {
    let mut trades = Vec::new();

    while incoming.remaining_quantity() > Decimal::ZERO && !level.orders.is_empty() {
        let resting_id = level.orders.front().unwrap();
        let resting = self.orders.get_mut(resting_id).unwrap();

        // Use VISIBLE quantity for matching, not total
        let match_qty = incoming.remaining_quantity().min(resting.visible_quantity());

        if match_qty > Decimal::ZERO {
            // Execute trade
            let trade = self.execute_trade(incoming, resting, match_qty);
            trades.push(trade);

            // Apply fill to resting order
            let was_replenished = resting.apply_fill(match_qty);

            if was_replenished {
                // Order was replenished - move to back of queue (new timestamp)
                let order_id = level.orders.pop_front().unwrap();
                level.orders.push_back(order_id);
            } else if resting.is_filled() {
                // Order complete - remove from queue
                level.orders.pop_front();
            }
        }
    }

    trades
}
```

### Alternatives Comparison

| Approach | Pros | Cons |
|----------|------|------|
| **Embedded in Order** | Simple, single data structure | Order struct grows for all orders |
| **Separate IcebergOrder type** | Clean separation | More complex type handling |
| **Wrapper pattern** | Flexible | Indirection overhead |

**Recommendation:** Embed `Option<IcebergConfig>` in Order. The memory overhead is minimal and the code is simpler.

---

## 4. Order Book Imbalance & Microprice

### Why This Matters

The simple "mid price" `(best_bid + best_ask) / 2` is a **poor estimate** of fair value. Professional traders use:

- **Order Book Imbalance**: Predicts short-term price direction
- **Microprice**: Volume-weighted fair value estimate
- **Depth Imbalance**: Multi-level analysis for stronger signals

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   Market Microstructure Metrics                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Order Book Snapshot:                                           │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Bids                    │ Asks                          │    │
│  │ $100.00  5,000 shares   │ $100.05  2,000 shares        │    │
│  │ $99.95   3,000 shares   │ $100.10  4,000 shares        │    │
│  │ $99.90   2,000 shares   │ $100.15  3,000 shares        │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  Calculations:                                                   │
│                                                                  │
│  1. Mid Price (naive):                                          │
│     (100.00 + 100.05) / 2 = $100.025                           │
│                                                                  │
│  2. Book Imbalance (Level 1):                                   │
│     bid_vol = 5,000   ask_vol = 2,000                          │
│     imbalance = (5000 - 2000) / (5000 + 2000) = +0.43          │
│     Interpretation: 43% more buying pressure → price likely ↑   │
│                                                                  │
│  3. Microprice:                                                  │
│     Uses OPPOSITE side volume as weights                        │
│     microprice = (bid × ask_vol + ask × bid_vol) / (bid_vol +  │
│                   ask_vol)                                      │
│     = (100.00 × 2000 + 100.05 × 5000) / 7000                   │
│     = $100.036                                                  │
│     Interpretation: Fair value slightly above mid               │
│                     (more bid volume pushes price up)           │
│                                                                  │
│  4. Multi-Level Depth Imbalance:                                │
│     Sum volumes at multiple levels with decay weights           │
│     Level 1: weight 1.0,  Level 2: weight 0.5,  Level 3: 0.25  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/metrics/microstructure.rs`**

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::models::OrderBook;

/// Market microstructure analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrostructureMetrics {
    /// Simple mid price
    pub mid_price: Decimal,

    /// Volume-weighted mid price (more accurate fair value)
    pub microprice: Decimal,

    /// Book imbalance at best bid/ask (-1 to +1)
    pub imbalance_l1: Decimal,

    /// Weighted multi-level imbalance
    pub imbalance_weighted: Decimal,

    /// Volume at best bid
    pub best_bid_volume: Decimal,

    /// Volume at best ask
    pub best_ask_volume: Decimal,

    /// Total bid depth (configurable levels)
    pub total_bid_depth: Decimal,

    /// Total ask depth (configurable levels)
    pub total_ask_depth: Decimal,

    /// Spread in basis points
    pub spread_bps: Decimal,
}

impl MicrostructureMetrics {
    /// Calculate all microstructure metrics from order book
    pub fn from_order_book(book: &OrderBook, depth_levels: usize) -> Option<Self> {
        let best_bid = book.best_bid()?;
        let best_ask = book.best_ask()?;
        let best_bid_vol = book.volume_at_price(OrderSide::Buy, best_bid)?;
        let best_ask_vol = book.volume_at_price(OrderSide::Sell, best_ask)?;

        // Mid price (simple average)
        let mid_price = (best_bid + best_ask) / dec!(2);

        // Microprice (volume-weighted)
        // Intuition: More volume on bid side → price more likely to go up
        // So we weight ask price MORE when there's more bid volume
        let microprice = if best_bid_vol + best_ask_vol > Decimal::ZERO {
            (best_bid * best_ask_vol + best_ask * best_bid_vol)
                / (best_bid_vol + best_ask_vol)
        } else {
            mid_price
        };

        // Level 1 imbalance
        let total_vol = best_bid_vol + best_ask_vol;
        let imbalance_l1 = if total_vol > Decimal::ZERO {
            (best_bid_vol - best_ask_vol) / total_vol
        } else {
            Decimal::ZERO
        };

        // Multi-level weighted imbalance
        let (bid_depth, ask_depth, imbalance_weighted) =
            Self::calculate_depth_imbalance(book, depth_levels);

        // Spread in basis points
        let spread_bps = if best_bid > Decimal::ZERO {
            ((best_ask - best_bid) / mid_price) * dec!(10000)
        } else {
            Decimal::ZERO
        };

        Some(Self {
            mid_price,
            microprice,
            imbalance_l1,
            imbalance_weighted,
            best_bid_volume: best_bid_vol,
            best_ask_volume: best_ask_vol,
            total_bid_depth: bid_depth,
            total_ask_depth: ask_depth,
            spread_bps,
        })
    }

    /// Calculate weighted imbalance across multiple price levels
    fn calculate_depth_imbalance(
        book: &OrderBook,
        levels: usize
    ) -> (Decimal, Decimal, Decimal) {
        let mut bid_weighted = Decimal::ZERO;
        let mut ask_weighted = Decimal::ZERO;
        let mut total_bid = Decimal::ZERO;
        let mut total_ask = Decimal::ZERO;

        // Exponential decay weights: 1.0, 0.5, 0.25, 0.125, ...
        let decay = dec!(0.5);

        let bid_levels = book.get_bid_levels(levels);
        let ask_levels = book.get_ask_levels(levels);

        for (i, level) in bid_levels.iter().enumerate() {
            let weight = decay.powi(i as i64);
            bid_weighted += level.total_quantity * weight;
            total_bid += level.total_quantity;
        }

        for (i, level) in ask_levels.iter().enumerate() {
            let weight = decay.powi(i as i64);
            ask_weighted += level.total_quantity * weight;
            total_ask += level.total_quantity;
        }

        let total_weighted = bid_weighted + ask_weighted;
        let imbalance = if total_weighted > Decimal::ZERO {
            (bid_weighted - ask_weighted) / total_weighted
        } else {
            Decimal::ZERO
        };

        (total_bid, total_ask, imbalance)
    }

    /// Predict short-term price direction based on imbalance
    /// Returns expected price change as a multiple of spread
    pub fn predicted_price_move(&self) -> Decimal {
        // Academic research suggests: E[ΔP] ≈ λ × imbalance × spread
        // where λ is typically 0.5-1.0
        let lambda = dec!(0.7);
        let spread = self.spread_bps / dec!(10000);
        lambda * self.imbalance_weighted * spread * self.mid_price
    }
}

/// Time-weighted metrics for signal smoothing
pub struct SmoothedMetrics {
    history: VecDeque<(DateTime<Utc>, MicrostructureMetrics)>,
    window_size: usize,
}

impl SmoothedMetrics {
    pub fn new(window_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    pub fn update(&mut self, metrics: MicrostructureMetrics) {
        let now = Utc::now();

        if self.history.len() >= self.window_size {
            self.history.pop_front();
        }
        self.history.push_back((now, metrics));
    }

    /// Exponentially weighted moving average of imbalance
    pub fn ewma_imbalance(&self, half_life: f64) -> Decimal {
        if self.history.is_empty() {
            return Decimal::ZERO;
        }

        let alpha = 1.0 - (-1.0 / half_life).exp();
        let mut ewma = Decimal::ZERO;

        for (_, metrics) in &self.history {
            ewma = ewma * Decimal::from_f64(1.0 - alpha).unwrap()
                 + metrics.imbalance_l1 * Decimal::from_f64(alpha).unwrap();
        }

        ewma
    }
}
```

### WebSocket API Extension

```rust
// Add to WebSocket messages

#[derive(Serialize)]
pub struct MicrostructureUpdate {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: MicrostructureMetrics,
}

// Broadcast on every book update or trade
```

---

## 5. TWAP/VWAP Execution Algorithms

### Why This Matters

When you need to execute a large order (say, $10M worth), submitting it all at once would:
- Move the market against you
- Signal your intent to other traders
- Result in poor average execution price

**TWAP** (Time-Weighted Average Price) and **VWAP** (Volume-Weighted Average Price) slice large orders over time.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Execution Algorithm Flow                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Parent Order: Buy 100,000 shares over 1 hour                   │
│                                                                  │
│  TWAP Strategy:                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Time       │ Slice Size │ Cumulative │ Target %          │   │
│  │ 09:30      │ 8,333      │ 8,333      │ 8.33%             │   │
│  │ 09:35      │ 8,333      │ 16,666     │ 16.67%            │   │
│  │ 09:40      │ 8,333      │ 25,000     │ 25.00%            │   │
│  │ ...        │ ...        │ ...        │ ...               │   │
│  │ 10:30      │ 8,341      │ 100,000    │ 100%              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  VWAP Strategy (follows historical volume profile):             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Time       │ Hist. Vol% │ Slice Size │ Notes             │   │
│  │ 09:30-09:35│ 15%        │ 15,000     │ High open volume  │   │
│  │ 09:35-09:40│ 5%         │ 5,000      │ Volume drops      │   │
│  │ 09:40-09:45│ 3%         │ 3,000      │ Mid-day lull      │   │
│  │ ...        │ ...        │ ...        │ ...               │   │
│  │ 15:55-16:00│ 12%        │ 12,000     │ Close spike       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Child Order Flow:                                               │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Parent Order                                               │ │
│  │ ┌──────────────────────────────────────────────────────┐  │ │
│  │ │ Algorithm Engine                                     │  │ │
│  │ │   ↓         ↓         ↓         ↓                   │  │ │
│  │ │ Child 1  Child 2  Child 3  Child 4  ...             │  │ │
│  │ │   ↓         ↓         ↓         ↓                   │  │ │
│  │ │ [Order Book] ← Each child submitted separately      │  │ │
│  │ └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/algorithms/mod.rs`**

```rust
pub mod twap;
pub mod vwap;
pub mod engine;
```

**File: `src/algorithms/twap.rs`**

```rust
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::{Order, OrderSide, OrderType};

/// TWAP execution algorithm
/// Divides order evenly across time intervals
#[derive(Debug, Clone)]
pub struct TwapAlgorithm {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,

    /// Total quantity to execute
    pub total_quantity: Decimal,

    /// Quantity already executed
    pub executed_quantity: Decimal,

    /// Start time of execution window
    pub start_time: DateTime<Utc>,

    /// End time of execution window
    pub end_time: DateTime<Utc>,

    /// Interval between slices
    pub slice_interval: Duration,

    /// Number of slices completed
    pub slices_completed: u32,

    /// Limit price (None = market orders)
    pub limit_price: Option<Decimal>,

    /// Max participation rate (% of market volume)
    pub max_participation: Option<Decimal>,

    /// Algorithm status
    pub status: AlgorithmStatus,

    /// Urgency factor: 1.0 = normal, >1 = front-load, <1 = back-load
    pub urgency: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Cancelled,
}

impl TwapAlgorithm {
    pub fn new(
        symbol: String,
        side: OrderSide,
        user_id: String,
        total_quantity: Decimal,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        slice_interval: Duration,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            user_id,
            total_quantity,
            executed_quantity: Decimal::ZERO,
            start_time,
            end_time,
            slice_interval,
            slices_completed: 0,
            limit_price: None,
            max_participation: None,
            status: AlgorithmStatus::Pending,
            urgency: Decimal::ONE,
        }
    }

    /// Calculate the next child order to submit
    pub fn next_slice(&mut self, current_time: DateTime<Utc>) -> Option<Order> {
        if self.status != AlgorithmStatus::Running {
            return None;
        }

        if current_time >= self.end_time {
            self.status = AlgorithmStatus::Completed;
            return None;
        }

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
            return None;
        }

        // Calculate target execution based on elapsed time
        let total_duration = (self.end_time - self.start_time).num_milliseconds() as f64;
        let elapsed = (current_time - self.start_time).num_milliseconds() as f64;
        let progress = (elapsed / total_duration).clamp(0.0, 1.0);

        // Apply urgency factor to the curve
        let adjusted_progress = progress.powf(1.0 / self.urgency.to_f64().unwrap_or(1.0));

        let target_quantity = self.total_quantity
            * Decimal::from_f64(adjusted_progress).unwrap_or(Decimal::ONE);

        // Calculate this slice's quantity
        let behind_by = target_quantity - self.executed_quantity;

        if behind_by <= Decimal::ZERO {
            return None; // Ahead of schedule
        }

        // Slice size = how much we're behind (with caps)
        let remaining = self.total_quantity - self.executed_quantity;
        let slice_quantity = behind_by.min(remaining);

        if slice_quantity <= Decimal::ZERO {
            return None;
        }

        self.slices_completed += 1;

        Some(Order {
            id: Uuid::new_v4(),
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: if self.limit_price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            price: self.limit_price,
            quantity: slice_quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: self.user_id.clone(),
            timestamp: current_time,
            time_in_force: TimeInForce::IOC, // Immediate-or-cancel for algo orders
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            parent_algo_id: Some(self.id), // Link to parent algorithm
            ..Default::default()
        })
    }

    /// Record execution of a child order
    pub fn record_fill(&mut self, filled_quantity: Decimal, fill_price: Decimal) {
        self.executed_quantity += filled_quantity;

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
        }
    }

    /// Calculate execution statistics
    pub fn execution_stats(&self) -> TwapStats {
        let expected_progress = if self.end_time > self.start_time {
            let total = (self.end_time - Utc::now()).num_milliseconds() as f64;
            let remaining = (self.end_time - self.start_time).num_milliseconds() as f64;
            1.0 - (remaining / total).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let actual_progress = (self.executed_quantity / self.total_quantity)
            .to_f64()
            .unwrap_or(0.0);

        TwapStats {
            expected_progress,
            actual_progress,
            behind_by: expected_progress - actual_progress,
            slices_completed: self.slices_completed,
            remaining_quantity: self.total_quantity - self.executed_quantity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TwapStats {
    pub expected_progress: f64,
    pub actual_progress: f64,
    pub behind_by: f64,
    pub slices_completed: u32,
    pub remaining_quantity: Decimal,
}
```

**File: `src/algorithms/vwap.rs`**

```rust
use chrono::{DateTime, Duration, NaiveTime, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use uuid::Uuid;

/// VWAP execution algorithm
/// Follows historical volume profile to minimize market impact
#[derive(Debug, Clone)]
pub struct VwapAlgorithm {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub user_id: String,

    pub total_quantity: Decimal,
    pub executed_quantity: Decimal,

    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,

    /// Historical volume profile: time bucket -> percentage of daily volume
    /// Sum of all percentages should be ~1.0 (for the execution window)
    pub volume_profile: VolumeProfile,

    /// Target execution curve (cumulative)
    target_curve: Vec<(DateTime<Utc>, Decimal)>,

    pub status: AlgorithmStatus,

    /// Actual VWAP achieved so far
    pub achieved_vwap: Decimal,
    total_notional: Decimal,
}

/// Historical volume distribution throughout the trading day
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    /// Time bucket -> fraction of daily volume (0.0 to 1.0)
    buckets: BTreeMap<NaiveTime, Decimal>,
    bucket_duration: Duration,
}

impl VolumeProfile {
    /// Create a typical US equity volume profile (U-shaped)
    pub fn us_equity_default() -> Self {
        let mut buckets = BTreeMap::new();

        // High volume at open
        buckets.insert(NaiveTime::from_hms_opt(9, 30, 0).unwrap(), dec!(0.08));
        buckets.insert(NaiveTime::from_hms_opt(9, 35, 0).unwrap(), dec!(0.06));
        buckets.insert(NaiveTime::from_hms_opt(9, 40, 0).unwrap(), dec!(0.05));
        buckets.insert(NaiveTime::from_hms_opt(9, 45, 0).unwrap(), dec!(0.04));

        // Low volume mid-day
        buckets.insert(NaiveTime::from_hms_opt(10, 0, 0).unwrap(), dec!(0.03));
        // ... more buckets ...
        buckets.insert(NaiveTime::from_hms_opt(12, 0, 0).unwrap(), dec!(0.02));

        // High volume at close
        buckets.insert(NaiveTime::from_hms_opt(15, 30, 0).unwrap(), dec!(0.05));
        buckets.insert(NaiveTime::from_hms_opt(15, 45, 0).unwrap(), dec!(0.07));
        buckets.insert(NaiveTime::from_hms_opt(15, 55, 0).unwrap(), dec!(0.10));

        Self {
            buckets,
            bucket_duration: Duration::minutes(5),
        }
    }

    /// Get expected volume percentage for a given time
    pub fn volume_at(&self, time: NaiveTime) -> Decimal {
        // Find the nearest bucket
        self.buckets
            .range(..=time)
            .last()
            .map(|(_, v)| *v)
            .unwrap_or(dec!(0.02)) // Default to low volume
    }

    /// Calculate cumulative volume from start to end
    pub fn cumulative_volume(&self, start: NaiveTime, end: NaiveTime) -> Decimal {
        self.buckets
            .range(start..=end)
            .map(|(_, v)| v)
            .sum()
    }
}

impl VwapAlgorithm {
    pub fn new(
        symbol: String,
        side: OrderSide,
        user_id: String,
        total_quantity: Decimal,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        volume_profile: VolumeProfile,
    ) -> Self {
        let mut algo = Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            user_id,
            total_quantity,
            executed_quantity: Decimal::ZERO,
            start_time,
            end_time,
            volume_profile,
            target_curve: Vec::new(),
            status: AlgorithmStatus::Pending,
            achieved_vwap: Decimal::ZERO,
            total_notional: Decimal::ZERO,
        };

        algo.build_target_curve();
        algo
    }

    /// Build the target execution curve based on volume profile
    fn build_target_curve(&mut self) {
        let start_time_of_day = self.start_time.time();
        let end_time_of_day = self.end_time.time();

        // Get total expected volume in our window
        let total_vol_pct = self.volume_profile
            .cumulative_volume(start_time_of_day, end_time_of_day);

        // Build cumulative target curve
        let mut cumulative = Decimal::ZERO;
        let mut current = self.start_time;

        while current < self.end_time {
            let time_of_day = current.time();
            let bucket_vol = self.volume_profile.volume_at(time_of_day);
            let normalized = bucket_vol / total_vol_pct;
            cumulative += normalized;

            let target_qty = self.total_quantity * cumulative.min(Decimal::ONE);
            self.target_curve.push((current, target_qty));

            current = current + self.volume_profile.bucket_duration;
        }
    }

    /// Get the target quantity we should have executed by now
    pub fn target_at(&self, time: DateTime<Utc>) -> Decimal {
        self.target_curve
            .iter()
            .filter(|(t, _)| *t <= time)
            .last()
            .map(|(_, qty)| *qty)
            .unwrap_or(Decimal::ZERO)
    }

    /// Calculate next slice
    pub fn next_slice(&mut self, current_time: DateTime<Utc>) -> Option<Order> {
        if self.status != AlgorithmStatus::Running {
            return None;
        }

        let target = self.target_at(current_time);
        let behind_by = target - self.executed_quantity;

        if behind_by <= Decimal::ZERO {
            return None;
        }

        let slice_qty = behind_by.min(self.total_quantity - self.executed_quantity);

        if slice_qty <= Decimal::ZERO {
            return None;
        }

        Some(Order {
            id: Uuid::new_v4(),
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: OrderType::Market,
            price: None,
            quantity: slice_qty,
            // ... rest of fields
            parent_algo_id: Some(self.id),
            ..Default::default()
        })
    }

    /// Record a fill and update VWAP calculation
    pub fn record_fill(&mut self, quantity: Decimal, price: Decimal) {
        self.executed_quantity += quantity;
        self.total_notional += quantity * price;

        if self.executed_quantity > Decimal::ZERO {
            self.achieved_vwap = self.total_notional / self.executed_quantity;
        }

        if self.executed_quantity >= self.total_quantity {
            self.status = AlgorithmStatus::Completed;
        }
    }
}
```

**File: `src/algorithms/engine.rs`**

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Manages all running execution algorithms
pub struct AlgorithmEngine {
    /// Running TWAP algorithms
    twap_algos: HashMap<Uuid, TwapAlgorithm>,

    /// Running VWAP algorithms
    vwap_algos: HashMap<Uuid, VwapAlgorithm>,

    /// Channel to submit child orders to matching engine
    order_sender: mpsc::Sender<Order>,

    /// Timer interval for checking algorithms
    tick_interval: Duration,
}

impl AlgorithmEngine {
    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.tick_interval);

        loop {
            interval.tick().await;
            let now = Utc::now();

            // Process all TWAP algorithms
            for algo in self.twap_algos.values_mut() {
                if let Some(order) = algo.next_slice(now) {
                    let _ = self.order_sender.send(order).await;
                }
            }

            // Process all VWAP algorithms
            for algo in self.vwap_algos.values_mut() {
                if let Some(order) = algo.next_slice(now) {
                    let _ = self.order_sender.send(order).await;
                }
            }

            // Clean up completed algorithms
            self.twap_algos.retain(|_, a| a.status != AlgorithmStatus::Completed);
            self.vwap_algos.retain(|_, a| a.status != AlgorithmStatus::Completed);
        }
    }
}
```

### Alternatives Comparison

| Algorithm | Best For | Market Impact | Complexity |
|-----------|----------|---------------|------------|
| **TWAP** | Predictable execution | Medium | Low |
| **VWAP** | Matching benchmark | Low | Medium |
| **POV (% of Volume)** | Stealth execution | Very Low | High |
| **Implementation Shortfall** | Balancing urgency vs impact | Optimal | Very High |
| **Arrival Price** | Aggressive execution | High | Medium |

---

## 6. Binary Protocol Implementation

### Why This Matters

JSON parsing is **extremely slow** compared to binary protocols:
- JSON: ~10-50μs to parse an order message
- Binary: ~100ns to decode the same message

HFT systems use binary protocols like:
- **FIX/FAST**: Industry standard (complex)
- **SBE (Simple Binary Encoding)**: Modern, efficient
- **Custom binary**: Fastest, exchange-specific

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Binary Protocol Layout                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  JSON Message (typical):                                         │
│  {"type":"order","side":"buy","price":"100.05","qty":"1000",    │
│   "symbol":"BTC-USD","user":"u123"}                             │
│  Size: ~120 bytes                                               │
│  Parse time: ~20μs                                              │
│                                                                  │
│  Binary Message:                                                 │
│  ┌─────┬─────┬─────┬──────────┬──────────┬────────────┬───────┐ │
│  │Type │Side │OType│ Price    │ Quantity │ Symbol     │UserID │ │
│  │1B   │1B   │1B   │ 8B       │ 8B       │ 8B         │8B     │ │
│  └─────┴─────┴─────┴──────────┴──────────┴────────────┴───────┘ │
│  Size: 35 bytes                                                 │
│  Parse time: ~100ns                                             │
│                                                                  │
│  Key Design Decisions:                                           │
│  - Fixed-width fields (no length prefixes)                      │
│  - Price as i64 (price × 10^8 for 8 decimal places)            │
│  - Symbol as fixed 8-byte array (padded)                        │
│  - Network byte order (big-endian)                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/protocol/binary.rs`**

```rust
use bytes::{Buf, BufMut, BytesMut};
use std::io::{self, Error, ErrorKind};

/// Price multiplier for fixed-point encoding
/// Price of 100.12345678 becomes 10012345678i64
const PRICE_SCALE: i64 = 100_000_000; // 10^8

/// Message types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    NewOrder = 1,
    CancelOrder = 2,
    ModifyOrder = 3,
    ExecutionReport = 4,
    OrderBookSnapshot = 5,
    Trade = 6,
    Heartbeat = 255,
}

impl TryFrom<u8> for MessageType {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::NewOrder),
            2 => Ok(MessageType::CancelOrder),
            3 => Ok(MessageType::ModifyOrder),
            4 => Ok(MessageType::ExecutionReport),
            5 => Ok(MessageType::OrderBookSnapshot),
            6 => Ok(MessageType::Trade),
            255 => Ok(MessageType::Heartbeat),
            _ => Err(Error::new(ErrorKind::InvalidData, "Unknown message type")),
        }
    }
}

/// Binary order message (35 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryOrderMessage {
    pub msg_type: u8,           // 1 byte
    pub side: u8,               // 1 byte: 0=Buy, 1=Sell
    pub order_type: u8,         // 1 byte: 0=Limit, 1=Market
    pub time_in_force: u8,      // 1 byte: 0=GTC, 1=IOC, 2=FOK, etc.
    pub price: i64,             // 8 bytes: Fixed-point (price × 10^8)
    pub quantity: i64,          // 8 bytes: Fixed-point (qty × 10^8)
    pub symbol: [u8; 8],        // 8 bytes: Null-padded ASCII
    pub order_id: [u8; 16],     // 16 bytes: UUID bytes
    pub timestamp_ns: u64,      // 8 bytes: Nanoseconds since epoch
}

impl BinaryOrderMessage {
    pub const SIZE: usize = 52;

    /// Encode to bytes (zero-copy into buffer)
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.msg_type);
        buf.put_u8(self.side);
        buf.put_u8(self.order_type);
        buf.put_u8(self.time_in_force);
        buf.put_i64(self.price);
        buf.put_i64(self.quantity);
        buf.put_slice(&self.symbol);
        buf.put_slice(&self.order_id);
        buf.put_u64(self.timestamp_ns);
    }

    /// Decode from bytes (zero-copy from buffer)
    pub fn decode(buf: &mut impl Buf) -> io::Result<Self> {
        if buf.remaining() < Self::SIZE {
            return Err(Error::new(ErrorKind::UnexpectedEof, "Incomplete message"));
        }

        let msg_type = buf.get_u8();
        let side = buf.get_u8();
        let order_type = buf.get_u8();
        let time_in_force = buf.get_u8();
        let price = buf.get_i64();
        let quantity = buf.get_i64();

        let mut symbol = [0u8; 8];
        buf.copy_to_slice(&mut symbol);

        let mut order_id = [0u8; 16];
        buf.copy_to_slice(&mut order_id);

        let timestamp_ns = buf.get_u64();

        Ok(Self {
            msg_type,
            side,
            order_type,
            time_in_force,
            price,
            quantity,
            symbol,
            order_id,
            timestamp_ns,
        })
    }

    /// Convert to domain Order type
    pub fn to_order(&self) -> Result<Order, io::Error> {
        let symbol = String::from_utf8_lossy(&self.symbol)
            .trim_end_matches('\0')
            .to_string();

        let price = if self.order_type == 0 {
            Some(Decimal::new(self.price, 8))
        } else {
            None
        };

        Ok(Order {
            id: Uuid::from_bytes(self.order_id),
            symbol,
            side: if self.side == 0 { OrderSide::Buy } else { OrderSide::Sell },
            order_type: if self.order_type == 0 { OrderType::Limit } else { OrderType::Market },
            price,
            quantity: Decimal::new(self.quantity, 8),
            // ... other fields with defaults
            ..Default::default()
        })
    }

    /// Create from domain Order type
    pub fn from_order(order: &Order) -> Self {
        let mut symbol = [0u8; 8];
        let symbol_bytes = order.symbol.as_bytes();
        symbol[..symbol_bytes.len().min(8)].copy_from_slice(&symbol_bytes[..symbol_bytes.len().min(8)]);

        let price = order.price
            .map(|p| (p * Decimal::new(PRICE_SCALE, 0)).to_i64().unwrap_or(0))
            .unwrap_or(0);

        let quantity = (order.quantity * Decimal::new(PRICE_SCALE, 0))
            .to_i64()
            .unwrap_or(0);

        Self {
            msg_type: MessageType::NewOrder as u8,
            side: if order.side == OrderSide::Buy { 0 } else { 1 },
            order_type: if order.order_type == OrderType::Limit { 0 } else { 1 },
            time_in_force: order.time_in_force as u8,
            price,
            quantity,
            symbol,
            order_id: *order.id.as_bytes(),
            timestamp_ns: order.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
        }
    }
}

/// Framed message with length prefix
pub struct FramedCodec;

impl FramedCodec {
    /// Encode a message with 2-byte length prefix
    pub fn encode_framed(msg: &BinaryOrderMessage, buf: &mut BytesMut) {
        buf.put_u16(BinaryOrderMessage::SIZE as u16);
        msg.encode(buf);
    }

    /// Decode a framed message
    pub fn decode_framed(buf: &mut impl Buf) -> io::Result<Option<BinaryOrderMessage>> {
        if buf.remaining() < 2 {
            return Ok(None); // Need more data
        }

        let len = buf.get_u16() as usize;

        if buf.remaining() < len {
            return Ok(None); // Need more data
        }

        BinaryOrderMessage::decode(buf).map(Some)
    }
}
```

### Benchmark Comparison

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_encoding() {
        let order = create_test_order();
        let iterations = 1_000_000;

        // Binary encoding
        let start = Instant::now();
        let mut buf = BytesMut::with_capacity(64);
        for _ in 0..iterations {
            buf.clear();
            let msg = BinaryOrderMessage::from_order(&order);
            msg.encode(&mut buf);
        }
        let binary_time = start.elapsed();

        // JSON encoding
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = serde_json::to_vec(&order);
        }
        let json_time = start.elapsed();

        println!("Binary: {:?} per op", binary_time / iterations as u32);
        println!("JSON: {:?} per op", json_time / iterations as u32);
        // Typical results: Binary ~50ns, JSON ~500ns (10x faster)
    }
}
```

### Alternatives Comparison

| Protocol | Parse Speed | Schema Evolution | Complexity |
|----------|-------------|------------------|------------|
| **Custom Binary** | Fastest (~50ns) | Poor | Low |
| **FlatBuffers** | Very Fast (~100ns) | Good | Medium |
| **Cap'n Proto** | Very Fast (~100ns) | Excellent | Medium |
| **Protocol Buffers** | Fast (~200ns) | Excellent | Medium |
| **SBE** | Very Fast (~80ns) | Good | Medium |
| **JSON** | Slow (~2000ns) | Excellent | Low |

**Recommendation:** Start with custom binary for learning, then consider SBE or FlatBuffers for production.

---

## 7. Write-Ahead Log (WAL) Persistence

### Why This Matters

Exchanges must **never lose orders** even if they crash. WAL provides:
- Durability: Orders survive crashes
- Recovery: Rebuild state by replaying log
- Audit trail: Complete history of all events

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Write-Ahead Log Architecture                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Write Path:                                                     │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │ Order    │───▶│ Serialize│───▶│ Append   │───▶│ fsync()  │  │
│  │ Event    │    │ to bytes │    │ to file  │    │ (durable)│  │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
│                                       │                         │
│                                       ▼                         │
│                              ┌────────────────┐                 │
│                              │ WAL File       │                 │
│                              │ ┌────────────┐ │                 │
│                              │ │ Seq 1: New │ │                 │
│                              │ │ Seq 2: Fill│ │                 │
│                              │ │ Seq 3: New │ │                 │
│                              │ │ ...        │ │                 │
│                              │ └────────────┘ │                 │
│                              └────────────────┘                 │
│                                                                  │
│  Recovery Path:                                                  │
│  ┌────────────────┐    ┌──────────┐    ┌──────────────────┐    │
│  │ Read WAL file  │───▶│ Replay   │───▶│ Rebuild state    │    │
│  │ from beginning │    │ events   │    │ (order book)     │    │
│  └────────────────┘    └──────────┘    └──────────────────┘    │
│                                                                  │
│  Checkpointing:                                                  │
│  - Periodically snapshot full state to disk                     │
│  - On recovery: load checkpoint + replay WAL from checkpoint    │
│  - Reduces recovery time for large logs                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/persistence/wal.rs`**

```rust
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use bincode;
use serde::{Deserialize, Serialize};

/// Events that get persisted to WAL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalEvent {
    /// New order submitted
    OrderSubmitted {
        sequence: u64,
        timestamp_ns: u64,
        order: Order,
    },

    /// Order was cancelled
    OrderCancelled {
        sequence: u64,
        timestamp_ns: u64,
        order_id: Uuid,
        symbol: String,
    },

    /// Trade executed
    TradeExecuted {
        sequence: u64,
        timestamp_ns: u64,
        trade: Trade,
    },

    /// Order modified
    OrderModified {
        sequence: u64,
        timestamp_ns: u64,
        order_id: Uuid,
        new_quantity: Option<Decimal>,
        new_price: Option<Decimal>,
    },

    /// Checkpoint marker (state was snapshotted)
    Checkpoint {
        sequence: u64,
        timestamp_ns: u64,
        checkpoint_path: String,
    },
}

/// Write-Ahead Log for durability
pub struct WriteAheadLog {
    /// Current WAL file
    file: BufWriter<File>,

    /// Current sequence number
    sequence: u64,

    /// Path to WAL directory
    wal_dir: PathBuf,

    /// Current WAL file index
    file_index: u64,

    /// Max size before rotation (default 100MB)
    max_file_size: u64,

    /// Current file size
    current_size: u64,

    /// Sync mode
    sync_mode: SyncMode,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncMode {
    /// fsync after every write (safest, slowest)
    EveryWrite,
    /// fsync every N writes
    Batched(u32),
    /// Let OS handle syncing (fastest, least safe)
    None,
}

impl WriteAheadLog {
    pub fn open(wal_dir: impl AsRef<Path>, sync_mode: SyncMode) -> io::Result<Self> {
        let wal_dir = wal_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&wal_dir)?;

        // Find the latest WAL file or create new one
        let (file_index, sequence) = Self::find_latest_wal(&wal_dir)?;

        let wal_path = wal_dir.join(format!("wal_{:08}.log", file_index));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;

        Ok(Self {
            file: BufWriter::new(file),
            sequence,
            wal_dir,
            file_index,
            max_file_size: 100 * 1024 * 1024, // 100MB
            current_size: std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0),
            sync_mode,
        })
    }

    /// Append an event to the WAL
    pub fn append(&mut self, event: WalEvent) -> io::Result<u64> {
        self.sequence += 1;
        let seq = self.sequence;

        // Serialize event with length prefix
        let encoded = bincode::serialize(&event)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes) + data
        let len = encoded.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&encoded)?;

        self.current_size += 4 + encoded.len() as u64;

        // Handle sync mode
        match self.sync_mode {
            SyncMode::EveryWrite => {
                self.file.flush()?;
                self.file.get_ref().sync_data()?;
            }
            SyncMode::Batched(n) if seq % n as u64 == 0 => {
                self.file.flush()?;
                self.file.get_ref().sync_data()?;
            }
            _ => {}
        }

        // Rotate if needed
        if self.current_size >= self.max_file_size {
            self.rotate()?;
        }

        Ok(seq)
    }

    /// Rotate to a new WAL file
    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()?;

        self.file_index += 1;
        let new_path = self.wal_dir.join(format!("wal_{:08}.log", self.file_index));

        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&new_path)?;

        self.file = BufWriter::new(new_file);
        self.current_size = 0;

        Ok(())
    }

    /// Replay all events from WAL files
    pub fn replay<F>(&self, mut handler: F) -> io::Result<u64>
    where
        F: FnMut(WalEvent) -> io::Result<()>,
    {
        let mut count = 0;

        // Get all WAL files sorted by index
        let mut wal_files: Vec<_> = std::fs::read_dir(&self.wal_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "log").unwrap_or(false))
            .map(|e| e.path())
            .collect();

        wal_files.sort();

        for wal_path in wal_files {
            let file = File::open(&wal_path)?;
            let mut reader = BufReader::new(file);

            loop {
                // Read length prefix
                let mut len_buf = [0u8; 4];
                match reader.read_exact(&mut len_buf) {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                }

                let len = u32::from_le_bytes(len_buf) as usize;

                // Read event data
                let mut data = vec![0u8; len];
                reader.read_exact(&mut data)?;

                // Deserialize
                let event: WalEvent = bincode::deserialize(&data)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                handler(event)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Force sync to disk
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()
    }

    fn find_latest_wal(wal_dir: &Path) -> io::Result<(u64, u64)> {
        // Find highest file index and replay to get sequence
        // ... implementation details ...
        Ok((0, 0))
    }
}

/// Checkpoint manager for faster recovery
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    checkpoint_interval: u64,
    last_checkpoint_seq: u64,
}

impl CheckpointManager {
    /// Create a checkpoint of current state
    pub fn create_checkpoint(
        &mut self,
        sequence: u64,
        order_book: &OrderBook,
    ) -> io::Result<String> {
        let checkpoint_path = self.checkpoint_dir
            .join(format!("checkpoint_{:012}.bin", sequence));

        let file = File::create(&checkpoint_path)?;
        let mut writer = BufWriter::new(file);

        // Serialize full order book state
        bincode::serialize_into(&mut writer, order_book)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        writer.flush()?;
        writer.get_ref().sync_all()?;

        self.last_checkpoint_seq = sequence;

        Ok(checkpoint_path.to_string_lossy().to_string())
    }

    /// Load latest checkpoint
    pub fn load_latest_checkpoint(&self) -> io::Result<Option<(u64, OrderBook)>> {
        // Find and load most recent checkpoint
        // ... implementation details ...
        Ok(None)
    }
}
```

### Recovery Process

```rust
/// Full recovery process
pub async fn recover_order_book(
    wal: &WriteAheadLog,
    checkpoint_mgr: &CheckpointManager,
) -> io::Result<OrderBook> {
    // 1. Try to load latest checkpoint
    let (mut book, start_seq) = match checkpoint_mgr.load_latest_checkpoint()? {
        Some((seq, book)) => {
            println!("Loaded checkpoint at sequence {}", seq);
            (book, seq)
        }
        None => {
            println!("No checkpoint found, starting fresh");
            (OrderBook::new(), 0)
        }
    };

    // 2. Replay WAL events after checkpoint
    let mut replayed = 0;
    wal.replay(|event| {
        match &event {
            WalEvent::OrderSubmitted { sequence, order, .. } if *sequence > start_seq => {
                book.add_order(order.clone());
                replayed += 1;
            }
            WalEvent::OrderCancelled { sequence, order_id, symbol, .. } if *sequence > start_seq => {
                book.cancel_order(symbol, *order_id);
                replayed += 1;
            }
            WalEvent::TradeExecuted { sequence, trade, .. } if *sequence > start_seq => {
                book.apply_trade(trade);
                replayed += 1;
            }
            _ => {}
        }
        Ok(())
    })?;

    println!("Replayed {} events from WAL", replayed);

    Ok(book)
}
```

### Alternatives Comparison

| Approach | Durability | Performance | Complexity |
|----------|------------|-------------|------------|
| **Custom WAL** | High | High | Medium |
| **SQLite WAL mode** | High | Medium | Low |
| **RocksDB** | Very High | High | Medium |
| **LMDB** | High | Very High | Low |
| **Event Store (DB)** | Very High | Medium | High |
| **Kafka** | Very High | Medium | High |

**Recommendation:** Start with custom WAL for learning. For production, consider RocksDB or LMDB for their proven reliability.

---

## 8. Circuit Breakers & Risk Controls

### Why This Matters

Circuit breakers prevent catastrophic losses from:
- Flash crashes
- Fat-finger errors
- Algorithm malfunctions
- Market manipulation

### Implementation Code

**File: `src/risk/circuit_breaker.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::VecDeque;
use chrono::{DateTime, Duration, Utc};

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum price change (%) before halt
    pub max_price_change_pct: Decimal,

    /// Time window for price change calculation
    pub price_window: Duration,

    /// Minimum trades before circuit breaker activates
    pub min_trades_for_activation: u32,

    /// How long to halt trading
    pub halt_duration: Duration,

    /// Maximum order size (quantity)
    pub max_order_size: Decimal,

    /// Maximum order value (price × quantity)
    pub max_order_value: Decimal,

    /// Maximum orders per second per user
    pub max_orders_per_second: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_price_change_pct: dec!(10),     // 10% price move
            price_window: Duration::minutes(5),
            min_trades_for_activation: 10,
            halt_duration: Duration::minutes(5),
            max_order_size: dec!(1_000_000),
            max_order_value: dec!(10_000_000),
            max_orders_per_second: 100,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal trading
    Normal,
    /// Trading halted
    Halted { until: DateTime<Utc>, reason: HaltReason },
    /// Cooling off (limited trading)
    CoolingOff { until: DateTime<Utc> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltReason {
    PriceVolatility,
    VolumeSpike,
    TechnicalIssue,
    Manual,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,

    /// Recent prices for volatility calculation
    price_history: VecDeque<(DateTime<Utc>, Decimal)>,

    /// Reference price (usually opening or last clear price)
    reference_price: Option<Decimal>,

    /// Trade count in current window
    trade_count: u32,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Normal,
            price_history: VecDeque::new(),
            reference_price: None,
            trade_count: 0,
        }
    }

    /// Check if trading is allowed
    pub fn is_trading_allowed(&self) -> bool {
        match self.state {
            CircuitState::Normal => true,
            CircuitState::Halted { until, .. } => Utc::now() >= until,
            CircuitState::CoolingOff { .. } => true, // Limited trading allowed
        }
    }

    /// Validate an order against risk limits
    pub fn validate_order(&self, order: &Order) -> Result<(), RiskError> {
        // Check circuit state
        if let CircuitState::Halted { until, reason } = self.state {
            if Utc::now() < until {
                return Err(RiskError::TradingHalted { reason, resume_at: until });
            }
        }

        // Check order size
        if order.quantity > self.config.max_order_size {
            return Err(RiskError::OrderTooLarge {
                quantity: order.quantity,
                max: self.config.max_order_size,
            });
        }

        // Check order value
        if let Some(price) = order.price {
            let value = price * order.quantity;
            if value > self.config.max_order_value {
                return Err(RiskError::OrderValueTooHigh {
                    value,
                    max: self.config.max_order_value,
                });
            }
        }

        Ok(())
    }

    /// Process a trade and check for circuit breaker triggers
    pub fn on_trade(&mut self, trade_price: Decimal, timestamp: DateTime<Utc>) -> Option<HaltReason> {
        // Update price history
        self.price_history.push_back((timestamp, trade_price));

        // Remove old prices outside window
        let cutoff = timestamp - self.config.price_window;
        while let Some((ts, _)) = self.price_history.front() {
            if *ts < cutoff {
                self.price_history.pop_front();
            } else {
                break;
            }
        }

        self.trade_count += 1;

        // Set reference price if not set
        if self.reference_price.is_none() {
            self.reference_price = Some(trade_price);
        }

        // Check for trigger conditions
        if self.trade_count >= self.config.min_trades_for_activation {
            if let Some(reason) = self.check_triggers(trade_price) {
                self.trigger_halt(reason);
                return Some(reason);
            }
        }

        None
    }

    fn check_triggers(&self, current_price: Decimal) -> Option<HaltReason> {
        let ref_price = self.reference_price?;

        // Calculate price change percentage
        let change_pct = ((current_price - ref_price) / ref_price).abs() * dec!(100);

        if change_pct >= self.config.max_price_change_pct {
            return Some(HaltReason::PriceVolatility);
        }

        None
    }

    fn trigger_halt(&mut self, reason: HaltReason) {
        let until = Utc::now() + self.config.halt_duration;
        self.state = CircuitState::Halted { until, reason };

        // Reset trade count
        self.trade_count = 0;

        // Update reference price to current for next period
        if let Some((_, price)) = self.price_history.back() {
            self.reference_price = Some(*price);
        }
    }

    /// Manually halt trading
    pub fn manual_halt(&mut self, duration: Duration) {
        let until = Utc::now() + duration;
        self.state = CircuitState::Halted {
            until,
            reason: HaltReason::Manual
        };
    }

    /// Resume trading
    pub fn resume(&mut self) {
        self.state = CircuitState::Normal;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RiskError {
    #[error("Trading halted due to {reason:?}, resumes at {resume_at}")]
    TradingHalted {
        reason: HaltReason,
        resume_at: DateTime<Utc>,
    },

    #[error("Order quantity {quantity} exceeds maximum {max}")]
    OrderTooLarge {
        quantity: Decimal,
        max: Decimal,
    },

    #[error("Order value {value} exceeds maximum {max}")]
    OrderValueTooHigh {
        value: Decimal,
        max: Decimal,
    },

    #[error("Rate limit exceeded: {orders_per_second} orders/second")]
    RateLimitExceeded {
        orders_per_second: u32,
    },
}
```

---

## 9. Memory-Mapped Ring Buffer (Disruptor Pattern)

### Why This Matters

The LMAX Disruptor achieved **6 million orders/second** using this pattern. Key insights:
- Pre-allocate all memory (no allocations in hot path)
- Single-writer principle (no locks)
- Batch processing for throughput
- Cache-line awareness

### Implementation Code

**File: `src/disruptor/ring_buffer.rs`**

```rust
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::mem::MaybeUninit;

/// Cache line size (64 bytes on most modern CPUs)
const CACHE_LINE_SIZE: usize = 64;

/// Padding to prevent false sharing between atomic variables
#[repr(align(64))]
struct CacheLinePadded<T>(T);

/// Lock-free ring buffer using LMAX Disruptor pattern
pub struct RingBuffer<T: Copy> {
    /// Pre-allocated buffer of fixed size (must be power of 2)
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,

    /// Buffer capacity (power of 2 for fast modulo via bitwise AND)
    capacity: usize,

    /// Mask for fast modulo: index & mask == index % capacity
    index_mask: usize,

    /// Next sequence to write (only modified by producer)
    write_cursor: CacheLinePadded<AtomicU64>,

    /// Sequences that consumers have processed
    /// Multiple consumers can have different positions
    read_cursors: Vec<CacheLinePadded<AtomicU64>>,

    /// Minimum read cursor (cached for producer)
    min_read_cursor: CacheLinePadded<AtomicU64>,
}

// Safety: We ensure single-writer and proper synchronization
unsafe impl<T: Copy + Send> Send for RingBuffer<T> {}
unsafe impl<T: Copy + Send> Sync for RingBuffer<T> {}

impl<T: Copy> RingBuffer<T> {
    /// Create a new ring buffer with given capacity (must be power of 2)
    pub fn new(capacity: usize, num_consumers: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");

        let buffer: Vec<UnsafeCell<MaybeUninit<T>>> = (0..capacity)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect();

        let read_cursors = (0..num_consumers)
            .map(|_| CacheLinePadded(AtomicU64::new(0)))
            .collect();

        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            index_mask: capacity - 1,
            write_cursor: CacheLinePadded(AtomicU64::new(0)),
            read_cursors,
            min_read_cursor: CacheLinePadded(AtomicU64::new(0)),
        }
    }

    /// Publish a single item (producer only)
    /// Returns the sequence number
    #[inline]
    pub fn publish(&self, item: T) -> u64 {
        let sequence = self.claim_next();
        self.write(sequence, item);
        self.commit(sequence);
        sequence
    }

    /// Claim the next sequence for writing
    #[inline]
    fn claim_next(&self) -> u64 {
        let next = self.write_cursor.0.fetch_add(1, Ordering::Relaxed);

        // Wait if we would overwrite unread data
        // (when write catches up to slowest reader)
        loop {
            let min_read = self.min_read_cursor.0.load(Ordering::Acquire);

            // Check if we have room (at least capacity slots between write and min read)
            if next < min_read + self.capacity as u64 {
                break;
            }

            // Update min_read_cursor from actual reader positions
            self.update_min_read_cursor();

            // Spin wait (in production, might want to park or yield)
            std::hint::spin_loop();
        }

        next
    }

    /// Write data to slot (no synchronization needed - single writer)
    #[inline]
    fn write(&self, sequence: u64, item: T) {
        let index = (sequence as usize) & self.index_mask;
        unsafe {
            (*self.buffer[index].get()).write(item);
        }
    }

    /// Make the write visible to consumers
    #[inline]
    fn commit(&self, _sequence: u64) {
        // The fetch_add in claim_next already published the sequence
        // A store fence ensures writes are visible
        std::sync::atomic::fence(Ordering::Release);
    }

    /// Read an item (consumer)
    #[inline]
    pub fn read(&self, consumer_id: usize, sequence: u64) -> Option<T> {
        let write_seq = self.write_cursor.0.load(Ordering::Acquire);

        if sequence >= write_seq {
            return None; // No data available yet
        }

        let index = (sequence as usize) & self.index_mask;
        let item = unsafe {
            (*self.buffer[index].get()).assume_init()
        };

        // Update consumer's read position
        self.read_cursors[consumer_id].0.store(sequence + 1, Ordering::Release);

        Some(item)
    }

    /// Batch read for higher throughput
    pub fn read_batch(&self, consumer_id: usize, max_items: usize) -> Vec<T> {
        let mut items = Vec::with_capacity(max_items);
        let start_seq = self.read_cursors[consumer_id].0.load(Ordering::Relaxed);
        let available = self.write_cursor.0.load(Ordering::Acquire);

        let end_seq = (start_seq + max_items as u64).min(available);

        for seq in start_seq..end_seq {
            let index = (seq as usize) & self.index_mask;
            let item = unsafe {
                (*self.buffer[index].get()).assume_init()
            };
            items.push(item);
        }

        if !items.is_empty() {
            self.read_cursors[consumer_id].0.store(end_seq, Ordering::Release);
        }

        items
    }

    fn update_min_read_cursor(&self) {
        let min = self.read_cursors
            .iter()
            .map(|c| c.0.load(Ordering::Relaxed))
            .min()
            .unwrap_or(0);

        self.min_read_cursor.0.store(min, Ordering::Release);
    }
}
```

### Usage Example

```rust
// Create ring buffer for order events
let buffer: RingBuffer<OrderEvent> = RingBuffer::new(
    1024 * 1024,  // 1M slots
    3,            // 3 consumers: matching engine, WAL writer, market data
);

// Producer (order gateway)
let sequence = buffer.publish(OrderEvent::NewOrder(order));

// Consumer 1: Matching engine (latency-critical)
while let Some(event) = buffer.read(0, next_seq) {
    match event {
        OrderEvent::NewOrder(order) => engine.process(order),
        // ...
    }
    next_seq += 1;
}

// Consumer 2: WAL writer (can batch)
let events = buffer.read_batch(1, 1000);
wal.append_batch(&events);

// Consumer 3: Market data publisher
let events = buffer.read_batch(2, 100);
for event in events {
    broadcaster.publish(event);
}
```

---

## Summary: Feature Priority Matrix

| Feature | Learning Value | Production Value | Complexity | Suggested Order |
|---------|---------------|------------------|------------|-----------------|
| Latency Measurement | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Low | 1st |
| Stop Orders | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Medium | 2nd |
| Book Imbalance/Microprice | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Low | 3rd |
| Circuit Breakers | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Medium | 4th |
| Iceberg Orders | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Medium | 5th |
| Binary Protocol | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Medium | 6th |
| WAL Persistence | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | High | 7th |
| TWAP/VWAP | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | High | 8th |
| Ring Buffer | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Very High | 9th |

---

## Next Steps

1. **See [LOCK_FREE_ARCHITECTURE.md](./LOCK_FREE_ARCHITECTURE.md)** for deep dive into lock-free programming
2. **See [DATABASE_DATA_ENGINEERING.md](./DATABASE_DATA_ENGINEERING.md)** for persistence and analytics infrastructure
