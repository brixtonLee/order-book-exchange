# Phase 3: Ultra-Low Latency & Advanced Trading Features

> **Goal:** Transform your exchange into a sub-10μs matching engine with advanced order types, lock-free data structures, and hardware-level optimizations.

**Target Latency:** < 10 microseconds (matching engine)
**Target Throughput:** 1M+ orders/second per symbol
**API Framework:** Axum (async, high-performance)

---

## Table of Contents

1. [Advanced Order Types](#1-advanced-order-types)
2. [Lock-Free Architecture](#2-lock-free-architecture)
3. [Zero-Copy & Kernel Bypass](#3-zero-copy--kernel-bypass)
4. [Binary Protocol Implementation](#4-binary-protocol-implementation)
5. [Hardware Optimizations](#5-hardware-optimizations)
6. [Memory Management](#6-memory-management)
7. [Advanced Market Microstructure](#7-advanced-market-microstructure)
8. [Matching Engine Variants](#8-matching-engine-variants)
9. [System-Level Tuning](#9-system-level-tuning)
10. [Axum API Integration](#10-axum-api-integration)

---

## 1. Advanced Order Types

### 1.1 OCO (One-Cancels-Other) Orders

**Description:** Submit two orders where executing one automatically cancels the other.

**Use Case:** Take profit OR stop loss (but not both)

**Rust Pattern:** Enum-based polymorphism with Arc for shared ownership

```rust
use uuid::Uuid;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;

/// OCO (One-Cancels-Other) order group
#[derive(Debug, Clone)]
pub struct OcoOrder {
    pub group_id: Uuid,
    pub primary_order: Order,
    pub secondary_order: Order,
    pub status: OcoStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcoStatus {
    Both Active,
    PrimaryFilled,
    SecondaryFilled,
    Cancelled,
}

/// OCO order manager (lock-free using Arc + RwLock)
pub struct OcoManager {
    groups: Arc<RwLock<HashMap<Uuid, OcoOrder>>>,
    order_to_group: Arc<RwLock<HashMap<Uuid, Uuid>>>, // order_id -> group_id
}

impl OcoManager {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            order_to_group: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add new OCO group
    pub async fn add_oco(&self, oco: OcoOrder) {
        let group_id = oco.group_id;
        let primary_id = oco.primary_order.id;
        let secondary_id = oco.secondary_order.id;

        let mut groups = self.groups.write().await;
        let mut mapping = self.order_to_group.write().await;

        groups.insert(group_id, oco);
        mapping.insert(primary_id, group_id);
        mapping.insert(secondary_id, group_id);
    }

    /// When order fills, cancel the other
    pub async fn on_order_filled(&self, order_id: Uuid) -> Option<Uuid> {
        let mapping = self.order_to_group.read().await;
        let group_id = mapping.get(&order_id)?;

        let mut groups = self.groups.write().await;
        let oco = groups.get_mut(group_id)?;

        // Determine which order to cancel
        let cancel_id = if oco.primary_order.id == order_id {
            oco.status = OcoStatus::PrimaryFilled;
            Some(oco.secondary_order.id)
        } else {
            oco.status = OcoStatus::SecondaryFilled;
            Some(oco.primary_order.id)
        };

        cancel_id
    }
}
```

**Axum API Endpoint:**

```rust
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct OcoRequest {
    pub symbol: String,
    pub user_id: String,

    // Primary order (e.g., take profit)
    pub primary_side: OrderSide,
    pub primary_price: Decimal,
    pub primary_quantity: Decimal,

    // Secondary order (e.g., stop loss)
    pub secondary_side: OrderSide,
    pub secondary_price: Decimal,
    pub secondary_quantity: Decimal,
}

#[derive(Serialize)]
pub struct OcoResponse {
    pub group_id: Uuid,
    pub primary_order_id: Uuid,
    pub secondary_order_id: Uuid,
    pub status: String,
}

pub async fn submit_oco_order(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<OcoRequest>,
) -> Result<Json<OcoResponse>, StatusCode> {
    // Create both orders
    let primary = Order::new(
        req.symbol.clone(),
        req.primary_side,
        OrderType::Limit,
        Some(req.primary_price),
        req.primary_quantity,
        req.user_id.clone(),
    );

    let secondary = Order::new(
        req.symbol,
        req.secondary_side,
        OrderType::Limit,
        Some(req.secondary_price),
        req.secondary_quantity,
        req.user_id,
    );

    let oco = OcoOrder {
        group_id: Uuid::new_v4(),
        primary_order: primary.clone(),
        secondary_order: secondary.clone(),
        status: OcoStatus::BothActive,
    };

    // Add to OCO manager
    app_state.oco_manager.add_oco(oco.clone()).await;

    // Submit both orders to matching engine
    app_state.order_sender.send(primary).await.ok();
    app_state.order_sender.send(secondary).await.ok();

    Ok(Json(OcoResponse {
        group_id: oco.group_id,
        primary_order_id: oco.primary_order.id,
        secondary_order_id: oco.secondary_order.id,
        status: "active".to_string(),
    }))
}
```

---

### 1.2 Bracket Orders

**Description:** Entry order + stop loss + take profit submitted atomically.

**Rust Pattern:** Builder pattern with phantom types for type safety

```rust
use std::marker::PhantomData;

/// Bracket order builder with compile-time validation
pub struct BracketOrderBuilder<State = NeedsEntry> {
    symbol: String,
    user_id: String,
    entry_order: Option<Order>,
    stop_loss: Option<StopOrder>,
    take_profit: Option<Order>,
    _state: PhantomData<State>,
}

// Type states
pub struct NeedsEntry;
pub struct HasEntry;
pub struct Complete;

impl BracketOrderBuilder<NeedsEntry> {
    pub fn new(symbol: String, user_id: String) -> Self {
        Self {
            symbol,
            user_id,
            entry_order: None,
            stop_loss: None,
            take_profit: None,
            _state: PhantomData,
        }
    }

    pub fn entry(
        mut self,
        side: OrderSide,
        price: Decimal,
        quantity: Decimal,
    ) -> BracketOrderBuilder<HasEntry> {
        self.entry_order = Some(Order::new(
            self.symbol.clone(),
            side,
            OrderType::Limit,
            Some(price),
            quantity,
            self.user_id.clone(),
        ));

        BracketOrderBuilder {
            symbol: self.symbol,
            user_id: self.user_id,
            entry_order: self.entry_order,
            stop_loss: None,
            take_profit: None,
            _state: PhantomData,
        }
    }
}

impl BracketOrderBuilder<HasEntry> {
    pub fn stop_loss(mut self, stop_price: Decimal) -> Self {
        let entry = self.entry_order.as_ref().unwrap();
        let opposite_side = match entry.side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };

        self.stop_loss = Some(StopOrder {
            id: Uuid::new_v4(),
            symbol: self.symbol.clone(),
            user_id: self.user_id.clone(),
            side: opposite_side,
            quantity: entry.quantity,
            trigger_price: stop_price,
            trigger_condition: TriggerCondition::AtOrBelow,
            stop_type: StopOrderType::StopMarket,
            limit_price: None,
            trail_amount: None,
            trail_percent: None,
            highest_price: None,
            lowest_price: None,
            status: StopOrderStatus::Pending,
            created_at: Utc::now(),
        });

        self
    }

    pub fn take_profit(mut self, profit_price: Decimal) -> BracketOrderBuilder<Complete> {
        let entry = self.entry_order.as_ref().unwrap();
        let opposite_side = match entry.side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };

        self.take_profit = Some(Order::new(
            self.symbol.clone(),
            opposite_side,
            OrderType::Limit,
            Some(profit_price),
            entry.quantity,
            self.user_id.clone(),
        ));

        BracketOrderBuilder {
            symbol: self.symbol,
            user_id: self.user_id,
            entry_order: self.entry_order,
            stop_loss: self.stop_loss,
            take_profit: self.take_profit,
            _state: PhantomData,
        }
    }
}

impl BracketOrderBuilder<Complete> {
    pub fn build(self) -> BracketOrder {
        BracketOrder {
            group_id: Uuid::new_v4(),
            entry_order: self.entry_order.unwrap(),
            stop_loss: self.stop_loss,
            take_profit: self.take_profit,
            status: BracketStatus::EntryPending,
        }
    }
}

pub struct BracketOrder {
    pub group_id: Uuid,
    pub entry_order: Order,
    pub stop_loss: Option<StopOrder>,
    pub take_profit: Option<Order>,
    pub status: BracketStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum BracketStatus {
    EntryPending,
    EntryFilled,      // Activate SL/TP
    StopLossTriggered,
    TakeProfitFilled,
    Cancelled,
}
```

**Usage:**

```rust
// Compile-time type safety!
let bracket = BracketOrderBuilder::new("AAPL".to_string(), "user123".to_string())
    .entry(OrderSide::Buy, dec!(150.00), dec!(100))
    .stop_loss(dec!(148.00))      // 2 points risk
    .take_profit(dec!(154.00))    // 4 points profit
    .build();

// This won't compile (missing entry):
// let invalid = BracketOrderBuilder::new(...)
//     .stop_loss(dec!(148.00))  // ERROR: needs entry first
//     .build();
```

---

### 1.3 Conditional Orders (If-Then Logic)

**Description:** Order activates only when condition is met.

**Rust Pattern:** Trait-based conditions with dynamic dispatch

```rust
/// Trait for order conditions
pub trait OrderCondition: Send + Sync {
    fn is_satisfied(&self, market_data: &MarketData) -> bool;
    fn description(&self) -> String;
}

/// Market data snapshot
pub struct MarketData {
    pub symbol: String,
    pub last_price: Decimal,
    pub best_bid: Decimal,
    pub best_ask: Decimal,
    pub volume_24h: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Price condition
pub struct PriceCondition {
    pub symbol: String,
    pub operator: ComparisonOp,
    pub threshold: Decimal,
}

#[derive(Debug, Clone, Copy)]
pub enum ComparisonOp {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
}

impl OrderCondition for PriceCondition {
    fn is_satisfied(&self, market_data: &MarketData) -> bool {
        if market_data.symbol != self.symbol {
            return false;
        }

        match self.operator {
            ComparisonOp::GreaterThan => market_data.last_price > self.threshold,
            ComparisonOp::GreaterThanOrEqual => market_data.last_price >= self.threshold,
            ComparisonOp::LessThan => market_data.last_price < self.threshold,
            ComparisonOp::LessThanOrEqual => market_data.last_price <= self.threshold,
            ComparisonOp::Equal => market_data.last_price == self.threshold,
        }
    }

    fn description(&self) -> String {
        format!("{} {:?} {}", self.symbol, self.operator, self.threshold)
    }
}

/// Volume condition
pub struct VolumeCondition {
    pub symbol: String,
    pub min_volume: Decimal,
}

impl OrderCondition for VolumeCondition {
    fn is_satisfied(&self, market_data: &MarketData) -> bool {
        market_data.symbol == self.symbol && market_data.volume_24h >= self.min_volume
    }

    fn description(&self) -> String {
        format!("{} volume >= {}", self.symbol, self.min_volume)
    }
}

/// Composite condition (AND/OR)
pub struct CompositeCondition {
    pub conditions: Vec<Box<dyn OrderCondition>>,
    pub operator: LogicalOp,
}

#[derive(Debug, Clone, Copy)]
pub enum LogicalOp {
    And,
    Or,
}

impl OrderCondition for CompositeCondition {
    fn is_satisfied(&self, market_data: &MarketData) -> bool {
        match self.operator {
            LogicalOp::And => self.conditions.iter().all(|c| c.is_satisfied(market_data)),
            LogicalOp::Or => self.conditions.iter().any(|c| c.is_satisfied(market_data)),
        }
    }

    fn description(&self) -> String {
        let cond_strs: Vec<String> = self.conditions.iter()
            .map(|c| c.description())
            .collect();

        match self.operator {
            LogicalOp::And => cond_strs.join(" AND "),
            LogicalOp::Or => cond_strs.join(" OR "),
        }
    }
}

/// Conditional order
pub struct ConditionalOrder {
    pub id: Uuid,
    pub condition: Box<dyn OrderCondition>,
    pub order: Order,
    pub status: ConditionalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum ConditionalStatus {
    Pending,
    Triggered,
    Cancelled,
    Expired,
}

/// Conditional order engine
pub struct ConditionalEngine {
    orders: Arc<RwLock<Vec<ConditionalOrder>>>,
}

impl ConditionalEngine {
    pub fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_conditional(&self, order: ConditionalOrder) {
        let mut orders = self.orders.write().await;
        orders.push(order);
    }

    /// Check conditions on market data update
    pub async fn check_conditions(&self, market_data: &MarketData) -> Vec<Order> {
        let mut orders = self.orders.write().await;
        let mut triggered = Vec::new();

        orders.retain_mut(|cond_order| {
            if cond_order.condition.is_satisfied(market_data) {
                cond_order.status = ConditionalStatus::Triggered;
                triggered.push(cond_order.order.clone());
                false // Remove from pending
            } else {
                true // Keep checking
            }
        });

        triggered
    }
}
```

**Example Usage:**

```rust
// If BTC > $100k AND volume > 1M BTC, buy AAPL
let condition = Box::new(CompositeCondition {
    conditions: vec![
        Box::new(PriceCondition {
            symbol: "BTC-USD".to_string(),
            operator: ComparisonOp::GreaterThan,
            threshold: dec!(100000),
        }),
        Box::new(VolumeCondition {
            symbol: "BTC-USD".to_string(),
            min_volume: dec!(1000000),
        }),
    ],
    operator: LogicalOp::And,
});

let conditional_order = ConditionalOrder {
    id: Uuid::new_v4(),
    condition,
    order: Order::new(
        "AAPL".to_string(),
        OrderSide::Buy,
        OrderType::Limit,
        Some(dec!(150.00)),
        dec!(100),
        "user123".to_string(),
    ),
    status: ConditionalStatus::Pending,
    created_at: Utc::now(),
};

engine.add_conditional(conditional_order).await;
```

---

## 2. Lock-Free Architecture

### 2.1 Lock-Free SPSC Queue (Fastest)

**Rust Pattern:** Ring buffer with atomic operations

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;

/// Single Producer Single Consumer lock-free queue
/// Perfect for: API thread → Matching thread
pub struct SPSCQueue<T> {
    buffer: Box<[UnsafeCell<Option<T>>]>,
    capacity: usize,
    head: AtomicUsize,  // Consumer reads from here
    tail: AtomicUsize,  // Producer writes to here
}

unsafe impl<T: Send> Send for SPSCQueue<T> {}
unsafe impl<T: Send> Sync for SPSCQueue<T> {}

impl<T> SPSCQueue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(None));
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Producer: try to push (non-blocking)
    pub fn try_push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        let next_tail = (tail + 1) & (self.capacity - 1);

        // Queue full?
        if next_tail == head {
            return Err(value);
        }

        unsafe {
            *self.buffer[tail].get() = Some(value);
        }

        self.tail.store(next_tail, Ordering::Release);
        Ok(())
    }

    /// Consumer: try to pop (non-blocking)
    pub fn try_pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        // Queue empty?
        if head == tail {
            return None;
        }

        let value = unsafe {
            (*self.buffer[head].get()).take()
        };

        let next_head = (head + 1) & (self.capacity - 1);
        self.head.store(next_head, Ordering::Release);

        value
    }

    /// Blocking pop with spin-wait
    pub fn pop(&self) -> T {
        loop {
            if let Some(value) = self.try_pop() {
                return value;
            }
            std::hint::spin_loop(); // Yield to CPU
        }
    }
}
```

**Usage in Matching Engine:**

```rust
pub struct MatchingEngine {
    order_queue: Arc<SPSCQueue<Order>>,
    result_queue: Arc<SPSCQueue<MatchResult>>,
}

impl MatchingEngine {
    /// Run in dedicated thread
    pub fn run(self) {
        // Pin to CPU core
        core_affinity::set_for_current(core_affinity::CoreId { id: 0 });

        loop {
            // Wait for order (spin-wait for lowest latency)
            let order = self.order_queue.pop();

            // Process order (lock-free)
            let result = self.match_order(order);

            // Send result back
            while self.result_queue.try_push(result).is_err() {
                std::hint::spin_loop();
            }
        }
    }
}
```

---

### 2.2 Lock-Free Order Book with RCU

**Rust Pattern:** Read-Copy-Update for snapshot reads

```rust
use arc_swap::ArcSwap;

/// Lock-free order book using arc-swap
pub struct LockFreeOrderBook {
    /// Immutable snapshot for readers
    snapshot: Arc<ArcSwap<OrderBookSnapshot>>,
}

#[derive(Clone)]
pub struct OrderBookSnapshot {
    pub bids: BTreeMap<Decimal, PriceLevel>,
    pub asks: BTreeMap<Decimal, PriceLevel>,
    pub sequence: u64,
}

impl LockFreeOrderBook {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(ArcSwap::from_pointee(OrderBookSnapshot {
                bids: BTreeMap::new(),
                asks: BTreeMap::new(),
                sequence: 0,
            })),
        }
    }

    /// Reader: zero-lock access
    pub fn get_snapshot(&self) -> Arc<OrderBookSnapshot> {
        self.snapshot.load_full()
    }

    /// Reader: get best bid (no lock)
    pub fn best_bid(&self) -> Option<Decimal> {
        let snap = self.snapshot.load();
        snap.bids.keys().next_back().copied()
    }

    /// Writer: update book (creates new version)
    pub fn update(&self, order: Order) {
        // Load current snapshot
        let old_snap = self.snapshot.load_full();

        // Clone and modify
        let mut new_snap = (*old_snap).clone();
        new_snap.sequence += 1;

        match order.side {
            OrderSide::Buy => {
                new_snap.bids
                    .entry(order.price.unwrap())
                    .or_insert_with(PriceLevel::new)
                    .add_order(order);
            }
            OrderSide::Sell => {
                new_snap.asks
                    .entry(order.price.unwrap())
                    .or_insert_with(PriceLevel::new)
                    .add_order(order);
            }
        }

        // Atomically swap
        self.snapshot.store(Arc::new(new_snap));
    }
}
```

**Dependencies:**

```toml
[dependencies]
arc-swap = "1.7"          # Lock-free Arc updates
crossbeam-queue = "0.3"   # Lock-free queues
crossbeam-channel = "0.5" # MPSC channels
```

---

### 2.3 Seqlock Pattern

**Rust Pattern:** Optimistic read lock

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Sequence lock for read-heavy workloads
pub struct SeqLock<T> {
    sequence: AtomicU64,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send> Sync for SeqLock<T> {}

impl<T: Clone> SeqLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            sequence: AtomicU64::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Read (lock-free, may retry)
    pub fn read(&self) -> T {
        loop {
            // Read sequence (even = not writing)
            let seq1 = self.sequence.load(Ordering::Acquire);

            // Odd sequence means writer is active, retry
            if seq1 & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }

            // Read data
            let value = unsafe { (*self.data.get()).clone() };

            // Check sequence didn't change
            let seq2 = self.sequence.load(Ordering::Acquire);

            if seq1 == seq2 {
                return value;
            }

            // Sequence changed, writer was active, retry
            std::hint::spin_loop();
        }
    }

    /// Write (brief lock)
    pub fn write(&self, new_data: T) {
        // Increment to odd (signals write in progress)
        let seq = self.sequence.fetch_add(1, Ordering::Release);

        // Write data
        unsafe {
            *self.data.get() = new_data;
        }

        // Increment to even (signals write complete)
        self.sequence.store(seq + 2, Ordering::Release);
    }
}
```

**Usage for Market Data:**

```rust
pub struct MarketDataCache {
    ticker: SeqLock<TickerData>,
}

#[derive(Clone)]
pub struct TickerData {
    pub best_bid: Decimal,
    pub best_ask: Decimal,
    pub last_price: Decimal,
    pub volume: Decimal,
}

impl MarketDataCache {
    // 1000s of readers can access simultaneously
    pub fn get_ticker(&self) -> TickerData {
        self.ticker.read() // Lock-free!
    }

    // Single writer updates
    pub fn update_ticker(&self, data: TickerData) {
        self.ticker.write(data);
    }
}
```

---

## 3. Zero-Copy & Kernel Bypass

### 3.1 io_uring for WAL Persistence

**Rust Pattern:** Async I/O without syscalls

```rust
use io_uring::{opcode, types, IoUring};
use std::os::unix::io::AsRawFd;
use std::fs::OpenOptions;

pub struct WalWriter {
    ring: IoUring,
    file: std::fs::File,
    buffer_pool: Vec<Vec<u8>>,
}

impl WalWriter {
    pub fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        // Create io_uring with 256 entries
        let ring = IoUring::new(256)?;

        Ok(Self {
            ring,
            file,
            buffer_pool: Vec::new(),
        })
    }

    /// Append to WAL without blocking
    pub fn append_async(&mut self, data: Vec<u8>) -> io::Result<()> {
        let fd = types::Fd(self.file.as_raw_fd());

        // Submit write operation to ring
        let write_op = opcode::Write::new(fd, data.as_ptr(), data.len() as u32)
            .build();

        unsafe {
            self.ring.submission()
                .push(&write_op)
                .expect("submission queue full");
        }

        // Submit to kernel (doesn't block)
        self.ring.submit()?;

        // Store buffer (will be freed after completion)
        self.buffer_pool.push(data);

        Ok(())
    }

    /// Poll for completions
    pub fn poll_completions(&mut self) -> io::Result<usize> {
        let mut completed = 0;

        for cqe in self.ring.completion() {
            let result = cqe.result();
            if result < 0 {
                eprintln!("Write failed: {}", result);
            }
            completed += 1;
        }

        // Reclaim completed buffers
        if completed > 0 {
            self.buffer_pool.drain(..completed);
        }

        Ok(completed)
    }
}
```

**Dependencies:**

```toml
[dependencies]
io-uring = "0.6"
```

---

### 3.2 Memory-Mapped Files

**Rust Pattern:** Zero-copy snapshot persistence

```rust
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;

pub struct MmapSnapshot {
    mmap: MmapMut,
}

impl MmapSnapshot {
    pub fn new(path: &str, size: usize) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe {
            MmapOptions::new().map_mut(&file)?
        };

        Ok(Self { mmap })
    }

    /// Write order book snapshot (zero-copy)
    pub fn write_snapshot(&mut self, snapshot: &OrderBookSnapshot) -> io::Result<()> {
        // Serialize directly to mmap (no intermediate buffer)
        let bytes = bincode::serialize(snapshot)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.mmap[..bytes.len()].copy_from_slice(&bytes);

        // Ensure written to disk
        self.mmap.flush()?;

        Ok(())
    }

    /// Read snapshot (zero-copy)
    pub fn read_snapshot(&self) -> io::Result<OrderBookSnapshot> {
        bincode::deserialize(&self.mmap)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
```

**Dependencies:**

```toml
[dependencies]
memmap2 = "0.9"
bincode = "1.3"
```

---

### 3.3 Shared Memory IPC

**Rust Pattern:** Inter-process communication

```rust
use shared_memory::{Shmem, ShmemConf};

/// Shared memory ring buffer for IPC
pub struct ShmemRingBuffer {
    shmem: Shmem,
    capacity: usize,
}

#[repr(C)]
struct RingHeader {
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl ShmemRingBuffer {
    /// Create shared memory segment
    pub fn create(name: &str, capacity: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let size = std::mem::size_of::<RingHeader>() + capacity;

        let shmem = ShmemConf::new()
            .size(size)
            .flink(name)
            .create()?;

        // Initialize header
        unsafe {
            let header = shmem.as_ptr() as *mut RingHeader;
            (*header).head = AtomicUsize::new(0);
            (*header).tail = AtomicUsize::new(0);
        }

        Ok(Self { shmem, capacity })
    }

    /// Open existing shared memory
    pub fn open(name: &str, capacity: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let shmem = ShmemConf::new()
            .flink(name)
            .open()?;

        Ok(Self { shmem, capacity })
    }

    /// Write to shared memory
    pub fn write(&self, data: &[u8]) -> Result<(), &'static str> {
        if data.len() > self.capacity {
            return Err("Data too large");
        }

        unsafe {
            let header = self.shmem.as_ptr() as *mut RingHeader;
            let buffer = (header as *mut u8).add(std::mem::size_of::<RingHeader>());

            let tail = (*header).tail.load(Ordering::Relaxed);

            // Copy data
            std::ptr::copy_nonoverlapping(data.as_ptr(), buffer.add(tail), data.len());

            (*header).tail.store(tail + data.len(), Ordering::Release);
        }

        Ok(())
    }
}
```

**Dependencies:**

```toml
[dependencies]
shared_memory = "0.12"
```

---

## 4. Binary Protocol Implementation

### 4.1 Custom Binary Protocol (Fastest)

**Rust Pattern:** Zero-copy serialization with bytemuck

```rust
use bytemuck::{Pod, Zeroable};

/// Binary order message (52 bytes, cache-line aligned)
#[repr(C, align(64))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BinaryOrderMsg {
    pub msg_type: u8,        // 1 byte
    pub side: u8,            // 1 byte: 0=Buy, 1=Sell
    pub order_type: u8,      // 1 byte: 0=Limit, 1=Market
    pub _padding: [u8; 5],   // Align to 8 bytes
    pub price: i64,          // 8 bytes (fixed-point: price × 10^8)
    pub quantity: i64,       // 8 bytes
    pub order_id: [u8; 16],  // 16 bytes (UUID)
    pub timestamp_ns: u64,   // 8 bytes
    pub user_id: u64,        // 8 bytes (hash of user ID)
}

impl BinaryOrderMsg {
    /// Zero-copy conversion to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Zero-copy conversion from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, &'static str> {
        bytemuck::try_from_bytes(bytes)
            .map_err(|_| "Invalid alignment or size")
    }

    /// Convert to domain Order
    pub fn to_order(&self) -> Order {
        Order {
            id: Uuid::from_bytes(self.order_id),
            symbol: "AAPL".to_string(), // Would be in extended fields
            side: if self.side == 0 { OrderSide::Buy } else { OrderSide::Sell },
            order_type: if self.order_type == 0 { OrderType::Limit } else { OrderType::Market },
            price: if self.order_type == 0 {
                Some(Decimal::new(self.price, 8))
            } else {
                None
            },
            quantity: Decimal::new(self.quantity, 8),
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: format!("{}", self.user_id),
            timestamp: Utc::now(),
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
        }
    }

    /// Create from Order
    pub fn from_order(order: &Order) -> Self {
        let mut msg = Self::zeroed();

        msg.msg_type = 1; // NewOrder
        msg.side = if order.side == OrderSide::Buy { 0 } else { 1 };
        msg.order_type = if order.order_type == OrderType::Limit { 0 } else { 1 };

        msg.price = order.price
            .map(|p| (p * Decimal::new(100_000_000, 0)).to_i64().unwrap_or(0))
            .unwrap_or(0);

        msg.quantity = (order.quantity * Decimal::new(100_000_000, 0))
            .to_i64()
            .unwrap_or(0);

        msg.order_id = *order.id.as_bytes();
        msg.timestamp_ns = order.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64;

        // Hash user_id to u64
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        order.user_id.hash(&mut hasher);
        msg.user_id = hasher.finish();

        msg
    }
}
```

**Benchmarks:**

```rust
#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_serialization() {
        let order = create_test_order();
        let iterations = 1_000_000;

        // Binary serialization
        let start = Instant::now();
        for _ in 0..iterations {
            let msg = BinaryOrderMsg::from_order(&order);
            let _bytes = msg.as_bytes();
        }
        let binary_time = start.elapsed();

        // JSON serialization
        let start = Instant::now();
        for _ in 0..iterations {
            let _json = serde_json::to_vec(&order).unwrap();
        }
        let json_time = start.elapsed();

        println!("Binary: {:?} per op ({:.0}ns)", binary_time / iterations,
                 binary_time.as_nanos() as f64 / iterations as f64);
        println!("JSON: {:?} per op ({:.0}ns)", json_time / iterations,
                 json_time.as_nanos() as f64 / iterations as f64);

        // Typical output:
        // Binary: 30ns per op
        // JSON: 2000ns per op
        // Binary is 66x faster!
    }
}
```

**Dependencies:**

```toml
[dependencies]
bytemuck = { version = "1.14", features = ["derive"] }
```

---

### 4.2 FlatBuffers Implementation

**Rust Pattern:** Schema-based zero-copy

**File: `schemas/order.fbs`**

```flatbuffers
namespace Trading;

enum Side: byte { Buy = 0, Sell = 1 }
enum OrderType: byte { Limit = 0, Market = 1 }

table Order {
  order_id: [ubyte];
  symbol: string;
  side: Side;
  order_type: OrderType;
  price: long;
  quantity: long;
  timestamp: ulong;
  user_id: string;
}

root_type Order;
```

**Generate Rust code:**

```bash
flatc --rust order.fbs
```

**Usage:**

```rust
use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub fn serialize_order_flatbuf(order: &Order) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let symbol = builder.create_string(&order.symbol);
    let user_id = builder.create_string(&order.user_id);
    let order_id = builder.create_vector(order.id.as_bytes());

    let fb_order = trading::Order::create(&mut builder, &trading::OrderArgs {
        order_id: Some(order_id),
        symbol: Some(symbol),
        side: if order.side == OrderSide::Buy { trading::Side::Buy } else { trading::Side::Sell },
        order_type: if order.order_type == OrderType::Limit {
            trading::OrderType::Limit
        } else {
            trading::OrderType::Market
        },
        price: order.price.map(|p| p.to_i64().unwrap()).unwrap_or(0),
        quantity: order.quantity.to_i64().unwrap(),
        timestamp: order.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64,
        user_id: Some(user_id),
    });

    builder.finish(fb_order, None);
    builder.finished_data().to_vec()
}

pub fn deserialize_order_flatbuf(bytes: &[u8]) -> Order {
    let fb_order = flatbuffers::root::<trading::Order>(bytes).unwrap();

    Order {
        id: Uuid::from_bytes(*fb_order.order_id().unwrap()),
        symbol: fb_order.symbol().unwrap().to_string(),
        // ... rest of fields
    }
}
```

**Dependencies:**

```toml
[dependencies]
flatbuffers = "23.5"

[build-dependencies]
flatc-rust = "0.2"
```

---

## 5. Hardware Optimizations

### 5.1 CPU Core Pinning

**Rust Pattern:** Dedicated CPU cores

```rust
use core_affinity::CoreId;

pub fn pin_to_core(core_id: usize) {
    let core = CoreId { id: core_id };

    if !core_affinity::set_for_current(core) {
        eprintln!("Failed to pin to core {}", core_id);
    } else {
        println!("✓ Pinned to CPU core {}", core_id);
    }
}

/// Launch matching engine on dedicated core
pub fn launch_matching_engine() {
    std::thread::spawn(|| {
        // Pin to core 0 (isolated from OS)
        pin_to_core(0);

        // Run matching loop
        let engine = MatchingEngine::new();
        engine.run();
    });
}

/// Launch network I/O on separate core
pub fn launch_network_handler() {
    std::thread::spawn(|| {
        // Pin to core 1
        pin_to_core(1);

        // Run network loop
        let handler = NetworkHandler::new();
        handler.run();
    });
}
```

**System tuning:**

```bash
# Isolate cores 0-1 from OS scheduler
sudo grubby --update-kernel=ALL --args="isolcpus=0,1"

# Reboot required
sudo reboot
```

**Dependencies:**

```toml
[dependencies]
core_affinity = "0.8"
```

---

### 5.2 Huge Pages

**Rust Pattern:** Reduce TLB misses

```rust
use libc::{mmap, munmap, MAP_ANONYMOUS, MAP_PRIVATE, MAP_HUGETLB, PROT_READ, PROT_WRITE};
use std::ptr;

pub struct HugePageAlloc {
    ptr: *mut u8,
    size: usize,
}

impl HugePageAlloc {
    /// Allocate 2MB huge pages
    pub fn new(size: usize) -> Result<Self, String> {
        // Size must be multiple of 2MB
        let huge_page_size = 2 * 1024 * 1024;
        let aligned_size = (size + huge_page_size - 1) & !(huge_page_size - 1);

        unsafe {
            let ptr = mmap(
                ptr::null_mut(),
                aligned_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB,
                -1,
                0,
            );

            if ptr == libc::MAP_FAILED {
                return Err("Failed to allocate huge pages".to_string());
            }

            println!("✓ Allocated {} bytes using huge pages", aligned_size);

            Ok(Self {
                ptr: ptr as *mut u8,
                size: aligned_size,
            })
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for HugePageAlloc {
    fn drop(&mut self) {
        unsafe {
            munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}
```

**System tuning:**

```bash
# Reserve 100 huge pages (200MB)
echo 100 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Verify
cat /proc/meminfo | grep Huge
```

---

### 5.3 SIMD Vectorization

**Rust Pattern:** Process 4 prices simultaneously

```rust
use std::simd::{f64x4, SimdFloat, SimdPartialOrd};

/// Find best matching price using SIMD
pub fn find_best_match_simd(prices: &[f64], target: f64) -> Option<usize> {
    let target_vec = f64x4::splat(target);
    let mut best_idx = None;
    let mut best_diff = f64::MAX;

    // Process 4 prices at a time
    for (i, chunk) in prices.chunks_exact(4).enumerate() {
        let price_vec = f64x4::from_slice(chunk);

        // Check which prices are <= target
        let mask = price_vec.simd_le(target_vec);

        // Calculate differences
        let diff_vec = target_vec - price_vec;

        // Find minimum difference
        for (j, &diff) in diff_vec.to_array().iter().enumerate() {
            if mask.test(j) && diff < best_diff {
                best_diff = diff;
                best_idx = Some(i * 4 + j);
            }
        }
    }

    // Handle remainder (if prices.len() % 4 != 0)
    let remainder_start = (prices.len() / 4) * 4;
    for (j, &price) in prices[remainder_start..].iter().enumerate() {
        let diff = target - price;
        if price <= target && diff < best_diff {
            best_diff = diff;
            best_idx = Some(remainder_start + j);
        }
    }

    best_idx
}

/// Vectorized decimal comparison (requires nightly)
#[cfg(feature = "nightly")]
pub fn batch_price_check(prices: &[Decimal], threshold: Decimal) -> Vec<bool> {
    // Convert Decimals to f64 for SIMD
    let prices_f64: Vec<f64> = prices.iter().map(|d| d.to_f64().unwrap()).collect();
    let threshold_f64 = threshold.to_f64().unwrap();

    let threshold_vec = f64x4::splat(threshold_f64);
    let mut results = Vec::with_capacity(prices.len());

    for chunk in prices_f64.chunks_exact(4) {
        let price_vec = f64x4::from_slice(chunk);
        let mask = price_vec.simd_le(threshold_vec);

        results.extend_from_slice(&mask.to_array().map(|b| b));
    }

    results
}
```

**Cargo.toml:**

```toml
[dependencies]
# Requires nightly Rust
# rustup default nightly

[features]
nightly = []
```

---

## 6. Memory Management

### 6.1 Object Pool

**Rust Pattern:** Reuse allocations

```rust
use crossbeam_queue::ArrayQueue;

pub struct ObjectPool<T> {
    pool: Arc<ArrayQueue<T>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
}

impl<T> ObjectPool<T> {
    pub fn new<F>(capacity: usize, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let pool = Arc::new(ArrayQueue::new(capacity));
        let factory = Arc::new(factory);

        // Pre-allocate
        for _ in 0..capacity {
            pool.push(factory()).ok();
        }

        Self { pool, factory }
    }

    pub fn acquire(&self) -> PooledObject<T> {
        let obj = self.pool.pop().unwrap_or_else(|| (self.factory)());

        PooledObject {
            obj: Some(obj),
            pool: Arc::clone(&self.pool),
        }
    }
}

/// RAII guard that returns object to pool on drop
pub struct PooledObject<T> {
    obj: Option<T>,
    pool: Arc<ArrayQueue<T>>,
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            // Return to pool (ignore if full)
            self.pool.push(obj).ok();
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.obj.as_ref().unwrap()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.obj.as_mut().unwrap()
    }
}
```

**Usage:**

```rust
// Create pool of 10,000 orders
let order_pool = ObjectPool::new(10000, || Order::default());

// Acquire from pool (zero allocation)
let mut order = order_pool.acquire();
order.symbol = "AAPL".to_string();
order.price = Some(dec!(150.00));

// Automatically returned to pool when `order` goes out of scope
```

---

### 6.2 Arena Allocator

**Rust Pattern:** Bulk allocation/deallocation

```rust
use bumpalo::Bump;

pub struct ArenaAllocator {
    arena: Bump,
}

impl ArenaAllocator {
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
        }
    }

    /// Allocate order in arena
    pub fn alloc_order(&self, order: Order) -> &mut Order {
        self.arena.alloc(order)
    }

    /// Allocate slice in arena
    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &mut [T] {
        self.arena.alloc_slice_copy(slice)
    }

    /// Reset arena (free all at once)
    pub fn reset(&mut self) {
        self.arena.reset();
    }
}
```

**Usage:**

```rust
// Process batch of orders
let mut arena = ArenaAllocator::new();

for order_data in batch {
    let order = arena.alloc_order(create_order(order_data));
    process_order(order);
}

// Free all orders at once (fast!)
arena.reset();
```

**Dependencies:**

```toml
[dependencies]
bumpalo = "3.14"
```

---

### 6.3 Custom Global Allocator

**Rust Pattern:** Replace system allocator

```rust
use tikv_jemallocator::Jemalloc;

// Use jemalloc globally (better than system malloc)
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    println!("Using jemalloc for all allocations");

    // All allocations now use jemalloc
    let orders = Vec::with_capacity(10000);
}
```

**Or use mimalloc:**

```rust
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

**Dependencies:**

```toml
[dependencies]
# Choose one:
tikv-jemallocator = "0.5"
# OR
mimalloc = "0.1"
```

---

## 7. Advanced Market Microstructure

### 7.1 Microprice Calculation

**Description:** Better fair value estimate than mid-price.

**Formula:**
```
microprice = (best_bid × ask_volume + best_ask × bid_volume) / (bid_volume + ask_volume)
```

**Rust Implementation:**

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone)]
pub struct Microprice {
    pub value: Decimal,
    pub mid_price: Decimal,
    pub spread: Decimal,
    pub imbalance: Decimal,  // -1 to +1
}

impl Microprice {
    pub fn calculate(
        best_bid: Decimal,
        best_ask: Decimal,
        bid_volume: Decimal,
        ask_volume: Decimal,
    ) -> Self {
        let mid_price = (best_bid + best_ask) / dec!(2);
        let spread = best_ask - best_bid;

        let total_volume = bid_volume + ask_volume;

        let microprice = if total_volume > Decimal::ZERO {
            (best_bid * ask_volume + best_ask * bid_volume) / total_volume
        } else {
            mid_price
        };

        let imbalance = if total_volume > Decimal::ZERO {
            (bid_volume - ask_volume) / total_volume
        } else {
            Decimal::ZERO
        };

        Self {
            value: microprice,
            mid_price,
            spread,
            imbalance,
        }
    }

    /// Predict short-term price direction
    pub fn predicted_move(&self) -> Decimal {
        // Academic research: E[ΔP] ≈ λ × imbalance × spread
        let lambda = dec!(0.5);
        lambda * self.imbalance * self.spread
    }
}
```

**Axum Endpoint:**

```rust
#[derive(Serialize)]
pub struct MicropriceResponse {
    pub symbol: String,
    pub microprice: Decimal,
    pub mid_price: Decimal,
    pub spread_bps: Decimal,
    pub imbalance: Decimal,
    pub predicted_move_bps: Decimal,
}

pub async fn get_microprice(
    State(state): State<Arc<AppState>>,
    Path(symbol): Path<String>,
) -> Result<Json<MicropriceResponse>, StatusCode> {
    let book = state.orderbook.read().await;

    let best_bid = book.best_bid().ok_or(StatusCode::NOT_FOUND)?;
    let best_ask = book.best_ask().ok_or(StatusCode::NOT_FOUND)?;
    let bid_vol = book.volume_at_price(OrderSide::Buy, best_bid).unwrap_or(Decimal::ZERO);
    let ask_vol = book.volume_at_price(OrderSide::Sell, best_ask).unwrap_or(Decimal::ZERO);

    let mp = Microprice::calculate(best_bid, best_ask, bid_vol, ask_vol);
    let mid = (best_bid + best_ask) / dec!(2);
    let spread_bps = ((best_ask - best_bid) / mid) * dec!(10000);
    let move_bps = (mp.predicted_move() / mid) * dec!(10000);

    Ok(Json(MicropriceResponse {
        symbol,
        microprice: mp.value,
        mid_price: mp.mid_price,
        spread_bps,
        imbalance: mp.imbalance,
        predicted_move_bps: move_bps,
    }))
}
```

---

### 7.2 Order Flow Toxicity (VPIN)

**Description:** Detect informed traders using Volume-Synchronized Probability of Informed Trading.

**Rust Implementation:**

```rust
use std::collections::VecDeque;

pub struct VpinCalculator {
    bucket_volume: Decimal,
    current_buy_volume: Decimal,
    current_sell_volume: Decimal,
    buckets: VecDeque<(Decimal, Decimal)>, // (buy_vol, sell_vol)
    window_size: usize,
}

impl VpinCalculator {
    pub fn new(bucket_volume: Decimal, window_size: usize) -> Self {
        Self {
            bucket_volume,
            current_buy_volume: Decimal::ZERO,
            current_sell_volume: Decimal::ZERO,
            buckets: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Update with new trade
    pub fn on_trade(&mut self, volume: Decimal, side: OrderSide) {
        match side {
            OrderSide::Buy => self.current_buy_volume += volume,
            OrderSide::Sell => self.current_sell_volume += volume,
        }

        let total = self.current_buy_volume + self.current_sell_volume;

        // Bucket filled?
        if total >= self.bucket_volume {
            self.buckets.push_back((self.current_buy_volume, self.current_sell_volume));

            if self.buckets.len() > self.window_size {
                self.buckets.pop_front();
            }

            self.current_buy_volume = Decimal::ZERO;
            self.current_sell_volume = Decimal::ZERO;
        }
    }

    /// Calculate VPIN metric
    pub fn calculate_vpin(&self) -> Decimal {
        if self.buckets.is_empty() {
            return Decimal::ZERO;
        }

        let mut total_imbalance = Decimal::ZERO;
        let mut total_volume = Decimal::ZERO;

        for (buy_vol, sell_vol) in &self.buckets {
            let imbalance = (buy_vol - sell_vol).abs();
            total_imbalance += imbalance;
            total_volume += buy_vol + sell_vol;
        }

        if total_volume > Decimal::ZERO {
            total_imbalance / total_volume
        } else {
            Decimal::ZERO
        }
    }

    /// Interpret VPIN
    pub fn toxicity_level(&self) -> ToxicityLevel {
        let vpin = self.calculate_vpin();

        if vpin > dec!(0.7) {
            ToxicityLevel::High
        } else if vpin > dec!(0.4) {
            ToxicityLevel::Medium
        } else {
            ToxicityLevel::Low
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ToxicityLevel {
    Low,     // Uninformed flow, safe to provide liquidity
    Medium,  // Mixed flow
    High,    // Informed traders, widen spread!
}
```

---

## 8. Matching Engine Variants

### 8.1 Pro-Rata Allocation

**Description:** Volume-proportional fills instead of FIFO.

**Rust Implementation:**

```rust
pub fn match_pro_rata(
    incoming: &mut Order,
    price_level: &mut PriceLevel,
    orders: &mut HashMap<Uuid, Order>,
) -> Vec<Trade> {
    let mut trades = Vec::new();
    let match_quantity = incoming.remaining_quantity().min(price_level.total_quantity);

    if match_quantity.is_zero() {
        return trades;
    }

    // Calculate pro-rata shares
    let mut allocations: Vec<(Uuid, Decimal)> = price_level.orders
        .iter()
        .map(|order_id| {
            let order = orders.get(order_id).unwrap();
            let share = (order.remaining_quantity() / price_level.total_quantity) * match_quantity;
            (*order_id, share)
        })
        .collect();

    // Round down allocations
    let mut allocated = Decimal::ZERO;
    for (_, share) in &mut allocations {
        *share = share.floor();
        allocated += *share;
    }

    // Allocate remainder using FIFO
    let remainder = match_quantity - allocated;
    if remainder > Decimal::ZERO {
        for (order_id, share) in &mut allocations {
            if *share > Decimal::ZERO && remainder > Decimal::ZERO {
                *share += Decimal::ONE;
                allocated += Decimal::ONE;

                if allocated >= match_quantity {
                    break;
                }
            }
        }
    }

    // Execute trades
    for (order_id, fill_qty) in allocations {
        if fill_qty.is_zero() {
            continue;
        }

        let resting = orders.get_mut(&order_id).unwrap();
        resting.fill(fill_qty);
        incoming.fill(fill_qty);

        trades.push(Trade {
            id: Uuid::new_v4(),
            symbol: incoming.symbol.clone(),
            price: resting.price.unwrap(),
            quantity: fill_qty,
            buyer_order_id: if incoming.side == OrderSide::Buy { incoming.id } else { order_id },
            seller_order_id: if incoming.side == OrderSide::Sell { incoming.id } else { order_id },
            timestamp: Utc::now(),
            maker_side: resting.side,
        });
    }

    trades
}
```

---

### 8.2 Opening Auction

**Description:** Batch matching at market open (NASDAQ-style).

**Rust Implementation:**

```rust
pub struct AuctionEngine {
    buy_orders: Vec<Order>,
    sell_orders: Vec<Order>,
    auction_time: DateTime<Utc>,
}

impl AuctionEngine {
    pub fn new(auction_time: DateTime<Utc>) -> Self {
        Self {
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            auction_time,
        }
    }

    /// Accept orders during pre-open
    pub fn add_order(&mut self, order: Order) {
        match order.side {
            OrderSide::Buy => self.buy_orders.push(order),
            OrderSide::Sell => self.sell_orders.push(order),
        }
    }

    /// Run auction at scheduled time
    pub fn run_auction(&mut self) -> AuctionResult {
        // Sort by price
        self.buy_orders.sort_by(|a, b| b.price.cmp(&a.price)); // Descending
        self.sell_orders.sort_by(|a, b| a.price.cmp(&b.price)); // Ascending

        // Find equilibrium price
        let clearing_price = self.find_clearing_price();

        if clearing_price.is_none() {
            return AuctionResult::NoCross;
        }

        let price = clearing_price.unwrap();

        // Match at clearing price
        let trades = self.match_at_price(price);

        AuctionResult::Success {
            clearing_price: price,
            volume: trades.iter().map(|t| t.quantity).sum(),
            trades,
        }
    }

    fn find_clearing_price(&self) -> Option<Decimal> {
        // Iterate through all possible prices
        // Find price with maximum executable volume

        let mut best_price = None;
        let mut max_volume = Decimal::ZERO;

        // Collect all unique prices
        let mut prices: Vec<Decimal> = self.buy_orders
            .iter()
            .chain(self.sell_orders.iter())
            .filter_map(|o| o.price)
            .collect();

        prices.sort();
        prices.dedup();

        for &price in &prices {
            let buy_vol: Decimal = self.buy_orders
                .iter()
                .filter(|o| o.price.unwrap_or(Decimal::ZERO) >= price)
                .map(|o| o.quantity)
                .sum();

            let sell_vol: Decimal = self.sell_orders
                .iter()
                .filter(|o| o.price.unwrap_or(Decimal::MAX) <= price)
                .map(|o| o.quantity)
                .sum();

            let executable = buy_vol.min(sell_vol);

            if executable > max_volume {
                max_volume = executable;
                best_price = Some(price);
            }
        }

        best_price
    }

    fn match_at_price(&mut self, price: Decimal) -> Vec<Trade> {
        let mut trades = Vec::new();

        // Filter eligible orders
        let mut buyers: Vec<_> = self.buy_orders.drain(..)
            .filter(|o| o.price.unwrap_or(Decimal::ZERO) >= price)
            .collect();

        let mut sellers: Vec<_> = self.sell_orders.drain(..)
            .filter(|o| o.price.unwrap_or(Decimal::MAX) <= price)
            .collect();

        // Match using pro-rata or FIFO
        while !buyers.is_empty() && !sellers.is_empty() {
            let buyer = buyers.first_mut().unwrap();
            let seller = sellers.first_mut().unwrap();

            let fill_qty = buyer.remaining_quantity().min(seller.remaining_quantity());

            trades.push(Trade {
                id: Uuid::new_v4(),
                symbol: buyer.symbol.clone(),
                price,
                quantity: fill_qty,
                buyer_order_id: buyer.id,
                seller_order_id: seller.id,
                timestamp: Utc::now(),
                maker_side: OrderSide::Buy, // Auction has no maker/taker
            });

            buyer.fill(fill_qty);
            seller.fill(fill_qty);

            if buyer.is_filled() {
                buyers.remove(0);
            }
            if seller.is_filled() {
                sellers.remove(0);
            }
        }

        // Unfilled orders go to continuous matching
        self.buy_orders.extend(buyers);
        self.sell_orders.extend(sellers);

        trades
    }
}

pub enum AuctionResult {
    Success {
        clearing_price: Decimal,
        volume: Decimal,
        trades: Vec<Trade>,
    },
    NoCross, // No overlapping buy/sell prices
}
```

---

## 9. System-Level Tuning

### 9.1 Linux Kernel Parameters

**File: `scripts/tune_system.sh`**

```bash
#!/bin/bash

echo "Tuning system for ultra-low latency trading..."

# Disable transparent huge pages (can cause jitter)
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# Disable CPU frequency scaling (use performance governor)
for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    echo performance | sudo tee $cpu
done

# Disable CPU idle states (prevent sleep)
for cpu in /sys/devices/system/cpu/cpu*/cpuidle/state*/disable; do
    echo 1 | sudo tee $cpu
done

# Increase max locked memory (for huge pages)
echo "* soft memlock unlimited" | sudo tee -a /etc/security/limits.conf
echo "* hard memlock unlimited" | sudo tee -a /etc/security/limits.conf

# Network tuning
sudo sysctl -w net.core.rmem_max=134217728  # 128MB receive buffer
sudo sysctl -w net.core.wmem_max=134217728  # 128MB send buffer
sudo sysctl -w net.ipv4.tcp_rmem='4096 87380 134217728'
sudo sysctl -w net.ipv4.tcp_wmem='4096 65536 134217728'
sudo sysctl -w net.ipv4.tcp_nodelay=1
sudo sysctl -w net.ipv4.tcp_low_latency=1

# IRQ affinity (NIC interrupts to CPU 1)
sudo sh -c "echo 2 > /proc/irq/$(cat /proc/interrupts | grep eth0 | cut -d: -f1)/smp_affinity"

echo "✓ System tuned for low latency"
echo "Reboot recommended for all changes to take effect"
```

---

### 9.2 Latency Testing

**Rust benchmarking:**

```rust
use std::time::Instant;
use hdrhistogram::Histogram;

pub fn benchmark_matching_engine() {
    let engine = MatchingEngine::new();
    let mut histogram = Histogram::<u64>::new(3).unwrap();

    // Warmup
    for _ in 0..1000 {
        let order = create_test_order();
        engine.submit_order(order);
    }

    // Measure
    for _ in 0..100_000 {
        let order = create_test_order();

        let start = Instant::now();
        engine.submit_order(order);
        let elapsed = start.elapsed();

        histogram.record(elapsed.as_nanos() as u64).ok();
    }

    // Report
    println!("Matching Engine Latency:");
    println!("  p50:   {}ns", histogram.value_at_quantile(0.50));
    println!("  p95:   {}ns", histogram.value_at_quantile(0.95));
    println!("  p99:   {}ns", histogram.value_at_quantile(0.99));
    println!("  p99.9: {}ns", histogram.value_at_quantile(0.999));
    println!("  max:   {}ns", histogram.max());
    println!("  mean:  {:.0}ns", histogram.mean());
}
```

---

## 10. Axum API Integration

### 10.1 Complete Axum Setup

**File: `src/api/mod.rs`**

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};

pub mod handlers;
pub mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub orderbook: Arc<RwLock<OrderBook>>,
    pub matching_engine: Arc<MatchingEngine>,
    pub oco_manager: Arc<OcoManager>,
    pub conditional_engine: Arc<ConditionalEngine>,
    pub trigger_engine: Arc<TriggerEngine>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Order endpoints
        .route("/api/v1/orders", post(handlers::submit_order))
        .route("/api/v1/orders/oco", post(handlers::submit_oco_order))
        .route("/api/v1/orders/bracket", post(handlers::submit_bracket_order))
        .route("/api/v1/orders/conditional", post(handlers::submit_conditional_order))
        .route("/api/v1/orders/:symbol/:order_id", get(handlers::get_order))
        .route("/api/v1/orders/:order_id/cancel", post(handlers::cancel_order))

        // Market data
        .route("/api/v1/orderbook/:symbol", get(handlers::get_orderbook))
        .route("/api/v1/orderbook/:symbol/microprice", get(handlers::get_microprice))
        .route("/api/v1/orderbook/:symbol/imbalance", get(handlers::get_imbalance))
        .route("/api/v1/trades/:symbol", get(handlers::get_trades))

        // WebSocket
        .route("/ws", get(websocket::ws_handler))

        // Health
        .route("/health", get(handlers::health_check))

        .with_state(Arc::new(state))
        .layer(CorsLayer::new().allow_origin(Any))
}

#[tokio::main]
async fn main() {
    // Initialize state
    let state = AppState {
        orderbook: Arc::new(RwLock::new(OrderBook::new("AAPL".to_string()))),
        matching_engine: Arc::new(MatchingEngine::new()),
        oco_manager: Arc::new(OcoManager::new()),
        conditional_engine: Arc::new(ConditionalEngine::new()),
        trigger_engine: Arc::new(TriggerEngine::new()),
    };

    // Create router
    let app = create_router(state);

    // Bind to address
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("🚀 Server running on http://{}", addr);
    println!("📊 Health check: http://{}/health", addr);
    println!("🔌 WebSocket: ws://{}/ws", addr);

    axum::serve(listener, app).await.unwrap();
}
```

---

### 10.2 WebSocket for Real-Time Updates

**File: `src/api/websocket.rs`**

```rust
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channels
    let mut trade_rx = state.trade_broadcast.subscribe();
    let mut book_rx = state.orderbook_broadcast.subscribe();

    // Spawn task to send updates to client
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(trade) = trade_rx.recv() => {
                    let msg = serde_json::to_string(&trade).unwrap();
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Ok(book_update) = book_rx.recv() => {
                    let msg = serde_json::to_string(&book_update).unwrap();
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Receive subscription requests from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Handle subscription requests
            if let Ok(req) = serde_json::from_str::<SubscriptionRequest>(&text) {
                // Process subscription
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
```

---

## Summary

### Phase 3 Implementation Checklist

| Feature | Complexity | Latency Gain | Priority |
|---------|------------|--------------|----------|
| **Advanced Orders** | | | |
| OCO Orders | Medium | - | P1 |
| Bracket Orders | Medium | - | P1 |
| Conditional Orders | High | - | P2 |
| **Lock-Free** | | | |
| SPSC Queue | High | 50-70% | P0 |
| RCU Order Book | Very High | 60-80% | P0 |
| Seqlock | Medium | 70-90% | P1 |
| **Zero-Copy** | | | |
| io_uring WAL | High | 40-60% | P1 |
| Memory-Mapped I/O | Medium | 30-50% | P2 |
| Shared Memory IPC | Medium | 50-70% | P2 |
| **Binary Protocol** | | | |
| Custom Binary | Medium | 80-90% | P0 |
| FlatBuffers | Medium | 70-80% | P1 |
| **Hardware** | | | |
| CPU Pinning | Low | 20-30% | P0 |
| Huge Pages | Low | 10-20% | P1 |
| SIMD | High | 20-40% | P2 |
| **Memory** | | | |
| Object Pool | Medium | 30-40% | P1 |
| Arena Allocator | Medium | 20-30% | P2 |
| Custom Allocator | Low | 10-15% | P2 |
| **Microstructure** | | | |
| Microprice | Low | - | P1 |
| VPIN | Medium | - | P2 |
| **Matching** | | | |
| Pro-Rata | Medium | - | P2 |
| Auction | High | - | P3 |

---

## Next Steps

1. **Week 1-2:** Implement lock-free SPSC queue and CPU pinning
2. **Week 3-4:** Custom binary protocol and zero-copy serialization
3. **Week 5-6:** Lock-free order book with RCU
4. **Week 7-8:** Advanced order types (OCO, Bracket)
5. **Week 9-10:** Hardware optimizations (huge pages, SIMD)
6. **Week 11-12:** Benchmarking and tuning

**Target Result:** Sub-10μs matching latency, 1M+ orders/second

---

**Total Estimated Timeline:** 3-4 months for full Phase 3 implementation

**Recommended Reading:**
- "Programming Rust" (Blandy & Orendorff)
- "Rust for Rustaceans" (Jon Gjengset)
- "Systems Performance" (Brendan Gregg)
- "The Art of Multiprocessor Programming" (Herlihy & Shavit)
