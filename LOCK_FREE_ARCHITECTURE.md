# Lock-Free Order Book Architecture

> A comprehensive deep dive into lock-free programming for high-frequency trading systems in Rust.

---

## Table of Contents

1. [Why Lock-Free Matters](#1-why-lock-free-matters)
2. [Memory Ordering Fundamentals](#2-memory-ordering-fundamentals)
3. [Atomic Operations Deep Dive](#3-atomic-operations-deep-dive)
4. [Lock-Free Data Structures](#4-lock-free-data-structures)
5. [Lock-Free Order Book Implementation](#5-lock-free-order-book-implementation)
6. [Epoch-Based Memory Reclamation](#6-epoch-based-memory-reclamation)
7. [Hazard Pointers Alternative](#7-hazard-pointers-alternative)
8. [Testing Lock-Free Code](#8-testing-lock-free-code)
9. [Performance Benchmarking](#9-performance-benchmarking)
10. [Production Considerations](#10-production-considerations)
11. [Alternative Approaches](#11-alternative-approaches)

---

## 1. Why Lock-Free Matters

### The Problem with Locks

Your current implementation uses `Arc<RwLock<HashMap<String, OrderBook>>>`:

```rust
// Current approach
pub struct OrderBookEngine {
    books: Arc<RwLock<HashMap<String, OrderBook>>>,
}

// What happens on each order:
async fn submit_order(&self, order: Order) -> Result<MatchResult> {
    let mut books = self.books.write().await;  // ← BLOCKS ALL OTHER THREADS
    let book = books.get_mut(&order.symbol).ok_or(...)?;
    book.match_order(order)
}
```

**Problems:**

| Issue | Impact | Lock-Free Solution |
|-------|--------|-------------------|
| Lock contention | Threads wait in queue | No waiting, retry on conflict |
| Priority inversion | Low-priority thread holds lock | No locks to hold |
| Deadlocks | System hangs | Impossible by design |
| Context switches | ~1-10μs per switch | No switches |
| Latency variance | p99 >> p50 | Consistent latency |

### Performance Comparison

```
┌────────────────────────────────────────────────────────────────┐
│ Latency Distribution: RwLock vs Lock-Free                      │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│ RwLock (your current implementation):                          │
│   p50:  5μs   ████                                             │
│   p99:  150μs ██████████████████████████████████████████████  │
│   p999: 2ms   (off chart - rare but devastating)              │
│                                                                │
│ Lock-Free:                                                     │
│   p50:  2μs   ██                                               │
│   p99:  8μs   ████████                                         │
│   p999: 15μs  ███████████████                                  │
│                                                                │
│ Key insight: Lock-free has MUCH tighter tail latency           │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### When to Use Lock-Free

✅ **Use lock-free when:**
- Latency variance matters (HFT, gaming, real-time)
- High contention (many threads competing)
- Critical sections are short
- You need predictable performance

❌ **Avoid lock-free when:**
- Complexity isn't justified
- Single-threaded is sufficient
- Operations are long (I/O, network)
- Correctness is hard to verify

---

## 2. Memory Ordering Fundamentals

### The Memory Model Problem

Modern CPUs reorder instructions for performance. Without proper synchronization:

```rust
// Thread 1                     // Thread 2
data = 42;                      if ready {
ready = true;                       println!("{}", data);  // Might print 0!
                                }
```

The CPU might reorder Thread 1 to set `ready` before `data`. This is called a **data race**.

### Rust's Memory Orderings

```rust
use std::sync::atomic::Ordering;

// From weakest to strongest:

Ordering::Relaxed
// - No synchronization guarantees
// - Only guarantees atomicity of the operation itself
// - Use for: counters, statistics where order doesn't matter

Ordering::Acquire
// - All reads/writes AFTER this cannot be reordered BEFORE it
// - "Acquire" the data that another thread released
// - Use for: load operations when reading shared data

Ordering::Release
// - All reads/writes BEFORE this cannot be reordered AFTER it
// - "Release" data for another thread to acquire
// - Use for: store operations when publishing shared data

Ordering::AcqRel
// - Combines Acquire + Release
// - Use for: read-modify-write operations (compare_and_swap)

Ordering::SeqCst
// - Sequential consistency - total global order
// - All threads see the same order of operations
// - Use for: when you need a total order (rare)
// - Performance cost: highest
```

### Visualizing Memory Ordering

```
┌─────────────────────────────────────────────────────────────────┐
│ Memory Ordering: Acquire/Release Pattern                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Thread A (Producer)              Thread B (Consumer)            │
│  ──────────────────              ──────────────────             │
│                                                                  │
│  ┌─────────────────┐                                            │
│  │ Write data      │ ◄── These writes...                        │
│  │ more_data = 42  │                                            │
│  └────────┬────────┘                                            │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐             ┌─────────────────┐            │
│  │ flag.store(     │             │ if flag.load(   │            │
│  │   true,         │ ──────────► │   Acquire) {    │            │
│  │   Release)      │             │                 │            │
│  └─────────────────┘             └────────┬────────┘            │
│                                           │                      │
│                                           ▼                      │
│                                  ┌─────────────────┐            │
│                                  │ Read data       │ ◄── ...are │
│                                  │ // sees 42!     │   visible   │
│                                  └─────────────────┘   here      │
│                                                                  │
│  Release "publishes" all prior writes                           │
│  Acquire "subscribes" to see those writes                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Common Patterns

```rust
// Pattern 1: Simple flag (producer-consumer)
static FLAG: AtomicBool = AtomicBool::new(false);
static mut DATA: i32 = 0;

// Producer
unsafe { DATA = 42; }
FLAG.store(true, Ordering::Release);

// Consumer
if FLAG.load(Ordering::Acquire) {
    unsafe { println!("{}", DATA); }  // Guaranteed to see 42
}

// Pattern 2: Sequence lock (for read-mostly data)
struct SeqLock<T> {
    sequence: AtomicU64,
    data: UnsafeCell<T>,
}

impl<T: Copy> SeqLock<T> {
    fn read(&self) -> T {
        loop {
            let seq1 = self.sequence.load(Ordering::Acquire);
            if seq1 & 1 != 0 { continue; }  // Write in progress

            let data = unsafe { *self.data.get() };

            let seq2 = self.sequence.load(Ordering::Acquire);
            if seq1 == seq2 { return data; }  // No write during our read
        }
    }

    fn write(&self, value: T) {
        let seq = self.sequence.load(Ordering::Relaxed);
        self.sequence.store(seq + 1, Ordering::Release);  // Odd = writing
        unsafe { *self.data.get() = value; }
        self.sequence.store(seq + 2, Ordering::Release);  // Even = done
    }
}
```

---

## 3. Atomic Operations Deep Dive

### Compare-And-Swap (CAS)

CAS is the fundamental building block of lock-free programming:

```rust
// Atomically: if current == expected, set to new and return Ok(expected)
//             else return Err(current)

let atomic = AtomicU64::new(100);

// Try to change 100 → 200
match atomic.compare_exchange(
    100,                    // expected
    200,                    // new value
    Ordering::AcqRel,       // success ordering
    Ordering::Acquire,      // failure ordering
) {
    Ok(100) => println!("Changed 100 to 200"),
    Err(current) => println!("Failed, current value is {}", current),
}
```

### CAS Loop Pattern

```rust
/// Atomically increment a counter, returning the old value
fn atomic_increment(counter: &AtomicU64) -> u64 {
    loop {
        let current = counter.load(Ordering::Relaxed);
        let new = current + 1;

        match counter.compare_exchange_weak(
            current,
            new,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(old) => return old,
            Err(_) => continue,  // Another thread changed it, retry
        }
    }
}

// But Rust provides this directly:
let old = counter.fetch_add(1, Ordering::AcqRel);
```

### The ABA Problem

```
┌─────────────────────────────────────────────────────────────────┐
│ The ABA Problem                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Thread 1:                    Thread 2:                          │
│ 1. Read ptr = A             2. Change ptr: A → B                │
│ 3. (preempted)              4. Change ptr: B → A (recycled!)    │
│ 5. CAS(A, C) succeeds!      5. (done)                           │
│                                                                  │
│ Problem: Thread 1 thinks nothing changed because ptr == A        │
│ But the data A points to might be completely different now!      │
│                                                                  │
│ Solutions:                                                       │
│ 1. Tagged pointers (version counter + pointer)                  │
│ 2. Epoch-based reclamation (defer freeing)                      │
│ 3. Hazard pointers (announce what you're using)                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Tagged Pointer Solution

```rust
/// A pointer with a version tag to solve ABA
#[derive(Clone, Copy)]
struct TaggedPtr {
    /// Lower 48 bits: pointer, Upper 16 bits: version tag
    bits: u64,
}

impl TaggedPtr {
    const PTR_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
    const TAG_SHIFT: u32 = 48;

    fn new(ptr: *mut u8, tag: u16) -> Self {
        let ptr_bits = ptr as u64 & Self::PTR_MASK;
        let tag_bits = (tag as u64) << Self::TAG_SHIFT;
        Self { bits: ptr_bits | tag_bits }
    }

    fn ptr(&self) -> *mut u8 {
        (self.bits & Self::PTR_MASK) as *mut u8
    }

    fn tag(&self) -> u16 {
        (self.bits >> Self::TAG_SHIFT) as u16
    }

    fn with_incremented_tag(&self, new_ptr: *mut u8) -> Self {
        Self::new(new_ptr, self.tag().wrapping_add(1))
    }
}

// Now CAS will fail if the tag changed, even if the pointer is the same
```

---

## 4. Lock-Free Data Structures

### Lock-Free Stack (Treiber Stack)

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

struct Node<T> {
    data: T,
    next: *mut Node<T>,
}

pub struct LockFreeStack<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> LockFreeStack<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn push(&self, data: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data,
            next: ptr::null_mut(),
        }));

        loop {
            let old_head = self.head.load(Ordering::Acquire);
            unsafe { (*new_node).next = old_head; }

            match self.head.compare_exchange_weak(
                old_head,
                new_node,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            let old_head = self.head.load(Ordering::Acquire);

            if old_head.is_null() {
                return None;
            }

            let new_head = unsafe { (*old_head).next };

            match self.head.compare_exchange_weak(
                old_head,
                new_head,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let node = unsafe { Box::from_raw(old_head) };
                    return Some(node.data);
                }
                Err(_) => continue,
            }
        }
    }
}

// WARNING: This has memory safety issues! See Section 6 for proper solution.
```

### Lock-Free Queue (Michael-Scott)

```rust
/// Lock-free MPMC queue
/// Uses sentinel node to simplify empty queue handling
pub struct LockFreeQueue<T> {
    head: AtomicPtr<QueueNode<T>>,
    tail: AtomicPtr<QueueNode<T>>,
}

struct QueueNode<T> {
    data: Option<T>,
    next: AtomicPtr<QueueNode<T>>,
}

impl<T> LockFreeQueue<T> {
    pub fn new() -> Self {
        // Create sentinel node
        let sentinel = Box::into_raw(Box::new(QueueNode {
            data: None,
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        Self {
            head: AtomicPtr::new(sentinel),
            tail: AtomicPtr::new(sentinel),
        }
    }

    pub fn enqueue(&self, data: T) {
        let new_node = Box::into_raw(Box::new(QueueNode {
            data: Some(data),
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_next = unsafe { (*tail).next.load(Ordering::Acquire) };

            // Check if tail is still valid
            if tail != self.tail.load(Ordering::Acquire) {
                continue;
            }

            if tail_next.is_null() {
                // Try to link new node
                if unsafe { (*tail).next.compare_exchange(
                    ptr::null_mut(),
                    new_node,
                    Ordering::Release,
                    Ordering::Relaxed,
                ).is_ok() } {
                    // Try to swing tail (ok if fails, another thread will do it)
                    let _ = self.tail.compare_exchange(
                        tail,
                        new_node,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                    return;
                }
            } else {
                // Tail is behind, try to advance it
                let _ = self.tail.compare_exchange(
                    tail,
                    tail_next,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            }
        }
    }

    pub fn dequeue(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let head_next = unsafe { (*head).next.load(Ordering::Acquire) };

            // Check if head is still valid
            if head != self.head.load(Ordering::Acquire) {
                continue;
            }

            if head == tail {
                if head_next.is_null() {
                    return None;  // Queue is empty
                }
                // Tail is behind, advance it
                let _ = self.tail.compare_exchange(
                    tail,
                    head_next,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            } else {
                // Read data before CAS
                let data = unsafe { (*head_next).data.take() };

                if self.head.compare_exchange(
                    head,
                    head_next,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ).is_ok() {
                    // Successfully dequeued, free old head
                    // (needs epoch-based reclamation!)
                    return data;
                }
            }
        }
    }
}
```

---

## 5. Lock-Free Order Book Implementation

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Lock-Free Order Book Architecture                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Order Book                            │    │
│  │  ┌─────────────────────┐  ┌─────────────────────────┐   │    │
│  │  │ Bids (SkipList)     │  │ Asks (SkipList)         │   │    │
│  │  │                     │  │                         │   │    │
│  │  │ $100.05 ─┬─ Level   │  │ $100.10 ─┬─ Level      │   │    │
│  │  │          │  Orders  │  │          │  Orders     │   │    │
│  │  │          ▼          │  │          ▼             │   │    │
│  │  │ $100.00 ─┬─ Level   │  │ $100.15 ─┬─ Level      │   │    │
│  │  │          │  Orders  │  │          │  Orders     │   │    │
│  │  │          ▼          │  │          ▼             │   │    │
│  │  │ $99.95  ─── Level   │  │ $100.20 ─── Level      │   │    │
│  │  │                     │  │                         │   │    │
│  │  └─────────────────────┘  └─────────────────────────┘   │    │
│  │                                                          │    │
│  │  Each PriceLevel:                                        │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │ price: Decimal (immutable key)                   │   │    │
│  │  │ total_quantity: AtomicI64 (fixed-point)          │   │    │
│  │  │ orders: LockFreeQueue<OrderId>                   │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  Order Storage:                                          │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │ DashMap<OrderId, Order>  (concurrent hashmap)    │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  Key Design Choices:                                             │
│  • SkipList for price levels: O(log n), lock-free               │
│  • Atomic quantity updates: no lock for fills                   │
│  • Order queue per level: maintains time priority               │
│  • Separate order storage: fast lookup by ID                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Using Crossbeam

**File: `src/lockfree/orderbook.rs`**

```rust
use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use rust_decimal::Decimal;
use uuid::Uuid;

/// Price multiplier for fixed-point atomic operations
/// Converts Decimal to i64: price * PRICE_SCALE
const PRICE_SCALE: i64 = 100_000_000;  // 10^8 for 8 decimal places

/// Wrapper for Decimal that implements Ord for SkipMap
/// For bids: Reverse order (highest first)
/// For asks: Normal order (lowest first)
#[derive(Clone, Copy, PartialEq, Eq)]
struct PriceKey {
    /// Price as fixed-point integer
    price_fixed: i64,
    /// True for bids (reverse order)
    is_bid: bool,
}

impl Ord for PriceKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_bid {
            // Bids: higher price first (reverse)
            other.price_fixed.cmp(&self.price_fixed)
        } else {
            // Asks: lower price first (normal)
            self.price_fixed.cmp(&other.price_fixed)
        }
    }
}

impl PartialOrd for PriceKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A single price level in the order book
pub struct PriceLevel {
    /// Price (immutable)
    pub price: Decimal,

    /// Total quantity at this level (atomic for lock-free updates)
    /// Stored as fixed-point: quantity * PRICE_SCALE
    quantity: AtomicI64,

    /// Orders at this level in FIFO order
    /// Using crossbeam's lock-free queue
    orders: crossbeam_queue::SegQueue<Uuid>,

    /// Count of orders (for fast size check)
    order_count: AtomicU64,
}

impl PriceLevel {
    pub fn new(price: Decimal) -> Self {
        Self {
            price,
            quantity: AtomicI64::new(0),
            orders: crossbeam_queue::SegQueue::new(),
            order_count: AtomicU64::new(0),
        }
    }

    /// Add an order to this level
    pub fn add_order(&self, order_id: Uuid, quantity: Decimal) {
        let qty_fixed = decimal_to_fixed(quantity);
        self.quantity.fetch_add(qty_fixed, Ordering::AcqRel);
        self.orders.push(order_id);
        self.order_count.fetch_add(1, Ordering::Release);
    }

    /// Remove quantity (for fills)
    /// Returns true if level is now empty
    pub fn remove_quantity(&self, quantity: Decimal) -> bool {
        let qty_fixed = decimal_to_fixed(quantity);
        let new_qty = self.quantity.fetch_sub(qty_fixed, Ordering::AcqRel) - qty_fixed;
        new_qty <= 0
    }

    /// Get the next order in queue (peek)
    pub fn peek_order(&self) -> Option<Uuid> {
        // Note: SegQueue doesn't have peek, this is a limitation
        // In production, use a different structure or keep a cached head
        None // Simplified for example
    }

    /// Pop the front order
    pub fn pop_order(&self) -> Option<Uuid> {
        match self.orders.pop() {
            Some(id) => {
                self.order_count.fetch_sub(1, Ordering::Release);
                Some(id)
            }
            None => None,
        }
    }

    pub fn total_quantity(&self) -> Decimal {
        fixed_to_decimal(self.quantity.load(Ordering::Acquire))
    }

    pub fn is_empty(&self) -> bool {
        self.order_count.load(Ordering::Acquire) == 0
    }
}

/// Lock-free order book implementation
pub struct LockFreeOrderBook {
    /// Symbol (immutable)
    pub symbol: String,

    /// Bid price levels (sorted high to low)
    bids: SkipMap<PriceKey, Arc<PriceLevel>>,

    /// Ask price levels (sorted low to high)
    asks: SkipMap<PriceKey, Arc<PriceLevel>>,

    /// Order storage for O(1) lookup
    orders: DashMap<Uuid, Order>,

    /// Sequence number for ordering events
    sequence: AtomicU64,
}

impl LockFreeOrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: SkipMap::new(),
            asks: SkipMap::new(),
            orders: DashMap::new(),
            sequence: AtomicU64::new(0),
        }
    }

    /// Get or create a price level
    fn get_or_create_level(&self, side: OrderSide, price: Decimal) -> Arc<PriceLevel> {
        let key = PriceKey {
            price_fixed: decimal_to_fixed(price),
            is_bid: side == OrderSide::Buy,
        };

        let map = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };

        // Try to get existing
        if let Some(entry) = map.get(&key) {
            return entry.value().clone();
        }

        // Create new level
        let level = Arc::new(PriceLevel::new(price));

        // Insert (race condition handled by SkipMap)
        match map.get_or_insert(key, level.clone()) {
            entry => entry.value().clone(),
        }
    }

    /// Add a new order to the book
    pub fn add_order(&self, order: Order) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);

        if let Some(price) = order.price {
            let level = self.get_or_create_level(order.side, price);
            level.add_order(order.id, order.quantity);
        }

        self.orders.insert(order.id, order);
        seq
    }

    /// Cancel an order
    pub fn cancel_order(&self, order_id: Uuid) -> Option<Order> {
        // Remove from order storage
        let (_, order) = self.orders.remove(&order_id)?;

        // Update price level quantity
        if let Some(price) = order.price {
            let remaining = order.quantity - order.filled_quantity;
            if remaining > Decimal::ZERO {
                let key = PriceKey {
                    price_fixed: decimal_to_fixed(price),
                    is_bid: order.side == OrderSide::Buy,
                };

                let map = match order.side {
                    OrderSide::Buy => &self.bids,
                    OrderSide::Sell => &self.asks,
                };

                if let Some(entry) = map.get(&key) {
                    let is_empty = entry.value().remove_quantity(remaining);

                    // Clean up empty levels
                    if is_empty {
                        map.remove(&key);
                    }
                }
            }
        }

        Some(order)
    }

    /// Get best bid price and quantity
    pub fn best_bid(&self) -> Option<(Decimal, Decimal)> {
        self.bids.front().map(|entry| {
            let level = entry.value();
            (level.price, level.total_quantity())
        })
    }

    /// Get best ask price and quantity
    pub fn best_ask(&self) -> Option<(Decimal, Decimal)> {
        self.asks.front().map(|entry| {
            let level = entry.value();
            (level.price, level.total_quantity())
        })
    }

    /// Get order book depth
    pub fn depth(&self, levels: usize) -> OrderBookDepth {
        let bids: Vec<_> = self.bids
            .iter()
            .take(levels)
            .map(|e| (e.value().price, e.value().total_quantity()))
            .collect();

        let asks: Vec<_> = self.asks
            .iter()
            .take(levels)
            .map(|e| (e.value().price, e.value().total_quantity()))
            .collect();

        OrderBookDepth { bids, asks }
    }
}

// Helper functions
fn decimal_to_fixed(d: Decimal) -> i64 {
    (d * Decimal::new(PRICE_SCALE, 0))
        .to_i64()
        .unwrap_or(0)
}

fn fixed_to_decimal(fixed: i64) -> Decimal {
    Decimal::new(fixed, 8)
}

#[derive(Debug, Clone)]
pub struct OrderBookDepth {
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
}
```

### Lock-Free Matching Engine

```rust
/// Lock-free matching engine
pub struct LockFreeMatchingEngine {
    books: DashMap<String, Arc<LockFreeOrderBook>>,
}

impl LockFreeMatchingEngine {
    pub fn new() -> Self {
        Self {
            books: DashMap::new(),
        }
    }

    /// Process an incoming order
    pub fn process_order(&self, mut order: Order) -> MatchResult {
        let book = self.get_or_create_book(&order.symbol);
        let mut trades = Vec::new();

        // Match against opposite side
        match order.order_type {
            OrderType::Market => {
                self.match_market_order(&book, &mut order, &mut trades);
            }
            OrderType::Limit => {
                self.match_limit_order(&book, &mut order, &mut trades);

                // Add remaining to book if not fully filled
                if order.remaining_quantity() > Decimal::ZERO {
                    book.add_order(order.clone());
                }
            }
        }

        MatchResult {
            order_id: order.id,
            status: if order.is_filled() {
                OrderStatus::Filled
            } else if order.filled_quantity > Decimal::ZERO {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::New
            },
            trades,
        }
    }

    fn match_limit_order(
        &self,
        book: &LockFreeOrderBook,
        incoming: &mut Order,
        trades: &mut Vec<Trade>,
    ) {
        let opposite_levels = match incoming.side {
            OrderSide::Buy => &book.asks,
            OrderSide::Sell => &book.bids,
        };

        // Iterate through price levels
        for entry in opposite_levels.iter() {
            let level = entry.value();

            // Check price crosses
            let crosses = match incoming.side {
                OrderSide::Buy => incoming.price.unwrap() >= level.price,
                OrderSide::Sell => incoming.price.unwrap() <= level.price,
            };

            if !crosses {
                break;  // No more matching levels
            }

            // Match at this level
            while incoming.remaining_quantity() > Decimal::ZERO {
                // Get next order at this level
                let resting_id = match level.pop_order() {
                    Some(id) => id,
                    None => break,  // Level exhausted
                };

                // Get resting order details
                let mut resting = match book.orders.get_mut(&resting_id) {
                    Some(o) => o,
                    None => continue,  // Order was cancelled
                };

                // Calculate match quantity
                let match_qty = incoming.remaining_quantity()
                    .min(resting.remaining_quantity());

                // Execute trade
                let trade = Trade {
                    id: Uuid::new_v4(),
                    symbol: incoming.symbol.clone(),
                    price: level.price,  // Trade at resting order's price
                    quantity: match_qty,
                    buyer_order_id: if incoming.side == OrderSide::Buy {
                        incoming.id
                    } else {
                        resting.id
                    },
                    seller_order_id: if incoming.side == OrderSide::Sell {
                        incoming.id
                    } else {
                        resting.id
                    },
                    timestamp: Utc::now(),
                    // ... fees, etc.
                };

                // Update quantities atomically
                incoming.filled_quantity += match_qty;
                resting.filled_quantity += match_qty;

                // Update level quantity
                level.remove_quantity(match_qty);

                // Remove filled order from storage
                if resting.is_filled() {
                    drop(resting);  // Release lock before remove
                    book.orders.remove(&resting_id);
                }

                trades.push(trade);
            }

            // Clean up empty level
            if level.is_empty() {
                let key = entry.key();
                opposite_levels.remove(key);
            }

            if incoming.remaining_quantity() == Decimal::ZERO {
                break;
            }
        }
    }

    fn match_market_order(
        &self,
        book: &LockFreeOrderBook,
        incoming: &mut Order,
        trades: &mut Vec<Trade>,
    ) {
        // Similar to limit order but no price check
        // ... implementation similar to above
    }

    fn get_or_create_book(&self, symbol: &str) -> Arc<LockFreeOrderBook> {
        self.books
            .entry(symbol.to_string())
            .or_insert_with(|| Arc::new(LockFreeOrderBook::new(symbol.to_string())))
            .clone()
    }
}
```

---

## 6. Epoch-Based Memory Reclamation

### The Problem

In lock-free code, you can't free memory while another thread might be reading it:

```rust
// Thread 1                     // Thread 2
let node = head.load();
                                head.swap(new_node);  // Remove old node
                                drop(old_node);       // FREE MEMORY
println!("{}", node.data);      // USE AFTER FREE!
```

### Epoch-Based Reclamation (crossbeam-epoch)

```rust
use crossbeam_epoch::{self as epoch, Atomic, Guard, Owned, Shared};
use std::sync::atomic::Ordering;

/// Node in our lock-free structure
struct Node<T> {
    data: T,
    next: Atomic<Node<T>>,
}

/// Lock-free stack with proper memory reclamation
pub struct EpochStack<T> {
    head: Atomic<Node<T>>,
}

impl<T> EpochStack<T> {
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    pub fn push(&self, data: T) {
        let new_node = Owned::new(Node {
            data,
            next: Atomic::null(),
        });

        // Pin the current thread to an epoch
        let guard = epoch::pin();

        loop {
            // Load current head with the guard
            let head = self.head.load(Ordering::Acquire, &guard);

            // Set new node's next to current head
            new_node.next.store(head, Ordering::Relaxed);

            // Try to CAS head to new node
            match self.head.compare_exchange(
                head,
                new_node,
                Ordering::Release,
                Ordering::Relaxed,
                &guard,
            ) {
                Ok(_) => break,
                Err(e) => {
                    // CAS failed, e.new is our node, retry
                    new_node = e.new;
                }
            }
        }
        // guard dropped here, epoch advances
    }

    pub fn pop(&self) -> Option<T> {
        let guard = epoch::pin();

        loop {
            let head = self.head.load(Ordering::Acquire, &guard);

            // Get reference to head node, if any
            let head_ref = unsafe { head.as_ref() }?;

            let next = head_ref.next.load(Ordering::Acquire, &guard);

            // Try to swing head to next
            match self.head.compare_exchange(
                head,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
                &guard,
            ) {
                Ok(_) => {
                    // Success! Defer destruction of old head
                    unsafe {
                        // This schedules the node for deletion after all
                        // threads in the current epoch have finished
                        guard.defer_destroy(head);
                    }

                    // Read data before guard is dropped
                    let data = unsafe { std::ptr::read(&head_ref.data) };
                    return Some(data);
                }
                Err(_) => continue,
            }
        }
    }
}

// Crossbeam automatically handles epoch advancement and garbage collection
```

### How Epochs Work

```
┌─────────────────────────────────────────────────────────────────┐
│ Epoch-Based Memory Reclamation                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Global Epoch: 5                                                │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                      │
│  │ Thread 1 │  │ Thread 2 │  │ Thread 3 │                      │
│  │ Epoch: 5 │  │ Epoch: 5 │  │ Epoch: 4 │ ◄── Still in old    │
│  │ (active) │  │ (active) │  │ (active) │     epoch            │
│  └──────────┘  └──────────┘  └──────────┘                      │
│                                                                  │
│  Garbage Queues:                                                │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Epoch 3: [Node A, Node B]  ← Can be freed! All threads    │ │
│  │                              have moved past epoch 3       │ │
│  │ Epoch 4: [Node C, Node D]  ← Cannot free yet, Thread 3    │ │
│  │                              might still access these      │ │
│  │ Epoch 5: [Node E]          ← Cannot free, current epoch   │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                  │
│  Rules:                                                          │
│  1. Threads "pin" themselves to the current epoch               │
│  2. Deleted nodes go to current epoch's garbage queue           │
│  3. Garbage is freed only when ALL threads have advanced        │
│     past that epoch                                              │
│  4. Epochs advance periodically (every N operations)            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Hazard Pointers Alternative

Hazard pointers are another approach to safe memory reclamation:

```rust
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;

const MAX_THREADS: usize = 64;
const HAZARDS_PER_THREAD: usize = 2;

/// Global hazard pointer registry
static HAZARD_POINTERS: HazardRegistry = HazardRegistry::new();

struct HazardRegistry {
    /// Each thread can protect up to HAZARDS_PER_THREAD pointers
    hazards: [[AtomicPtr<()>; HAZARDS_PER_THREAD]; MAX_THREADS],
    /// Retired nodes waiting to be freed
    retired: [AtomicPtr<RetiredList>; MAX_THREADS],
}

impl HazardRegistry {
    const fn new() -> Self {
        // Initialize with null pointers
        // ... const initialization
        todo!()
    }

    /// Protect a pointer from being freed
    fn protect<T>(&self, thread_id: usize, slot: usize, ptr: *mut T) {
        self.hazards[thread_id][slot].store(ptr as *mut (), Ordering::SeqCst);
    }

    /// Release protection
    fn release(&self, thread_id: usize, slot: usize) {
        self.hazards[thread_id][slot].store(ptr::null_mut(), Ordering::Release);
    }

    /// Check if a pointer is protected by any thread
    fn is_protected<T>(&self, ptr: *mut T) -> bool {
        let ptr_void = ptr as *mut ();

        for thread_hazards in &self.hazards {
            for hazard in thread_hazards {
                if hazard.load(Ordering::Acquire) == ptr_void {
                    return true;
                }
            }
        }
        false
    }

    /// Retire a pointer (schedule for deletion)
    fn retire<T>(&self, thread_id: usize, ptr: *mut T) {
        // Add to retired list
        // Periodically scan and free unprotected pointers
    }
}

/// RAII guard for hazard pointer protection
pub struct HazardGuard<'a, T> {
    registry: &'a HazardRegistry,
    thread_id: usize,
    slot: usize,
    ptr: *mut T,
}

impl<'a, T> HazardGuard<'a, T> {
    pub fn protect(ptr: &AtomicPtr<T>, thread_id: usize, slot: usize) -> Option<Self> {
        let registry = &HAZARD_POINTERS;

        loop {
            let p = ptr.load(Ordering::Acquire);
            if p.is_null() {
                return None;
            }

            // Announce we're using this pointer
            registry.protect(thread_id, slot, p);

            // Verify pointer hasn't changed
            if ptr.load(Ordering::Acquire) == p {
                return Some(Self {
                    registry,
                    thread_id,
                    slot,
                    ptr: p,
                });
            }
            // Pointer changed, retry
        }
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<'a, T> Drop for HazardGuard<'a, T> {
    fn drop(&mut self) {
        self.registry.release(self.thread_id, self.slot);
    }
}
```

### Epoch vs Hazard Pointers

| Aspect | Epoch-Based | Hazard Pointers |
|--------|-------------|-----------------|
| Memory overhead | Higher (deferred freeing) | Lower (immediate check) |
| CPU overhead | Lower (batched) | Higher (per-access check) |
| Latency | More variance | More predictable |
| Implementation | Simpler | More complex |
| Best for | General use | Real-time systems |

**Recommendation:** Use crossbeam-epoch for most cases. Consider hazard pointers only for strict real-time requirements.

---

## 8. Testing Lock-Free Code

### The Challenge

Lock-free bugs are **non-deterministic**:
- May only appear under specific timing
- Impossible to reproduce reliably
- Standard testing is insufficient

### Tools and Techniques

#### 1. Loom (Exhaustive Testing)

```rust
#[cfg(test)]
mod tests {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;

    #[test]
    fn test_concurrent_increment() {
        loom::model(|| {
            let counter = loom::sync::Arc::new(AtomicUsize::new(0));

            let threads: Vec<_> = (0..2)
                .map(|_| {
                    let counter = counter.clone();
                    thread::spawn(move || {
                        counter.fetch_add(1, Ordering::SeqCst);
                    })
                })
                .collect();

            for t in threads {
                t.join().unwrap();
            }

            assert_eq!(counter.load(Ordering::SeqCst), 2);
        });
    }
}
```

Loom explores **all possible interleavings** of concurrent operations, finding bugs that random testing would miss.

#### 2. Stress Testing

```rust
#[test]
fn stress_test_lock_free_queue() {
    use std::sync::Arc;
    use std::thread;

    let queue = Arc::new(LockFreeQueue::new());
    let num_threads = 8;
    let ops_per_thread = 100_000;

    let producers: Vec<_> = (0..num_threads)
        .map(|i| {
            let q = queue.clone();
            thread::spawn(move || {
                for j in 0..ops_per_thread {
                    q.enqueue(i * ops_per_thread + j);
                }
            })
        })
        .collect();

    let consumers: Vec<_> = (0..num_threads)
        .map(|_| {
            let q = queue.clone();
            thread::spawn(move || {
                let mut count = 0;
                loop {
                    if q.dequeue().is_some() {
                        count += 1;
                    }
                    if count >= ops_per_thread {
                        break;
                    }
                }
                count
            })
        })
        .collect();

    for p in producers {
        p.join().unwrap();
    }

    let total: usize = consumers
        .into_iter()
        .map(|c| c.join().unwrap())
        .sum();

    assert_eq!(total, num_threads * ops_per_thread);
}
```

#### 3. ThreadSanitizer (TSan)

```bash
# Build with thread sanitizer
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test

# Will detect:
# - Data races
# - Lock order inversions
# - Use-after-free in concurrent code
```

#### 4. Miri (Undefined Behavior Detection)

```bash
# Run tests under Miri
cargo +nightly miri test

# Detects:
# - Invalid memory access
# - Use of uninitialized memory
# - Memory leaks
# - Incorrect atomic orderings (experimental)
```

---

## 9. Performance Benchmarking

### Micro-Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_order_book_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("OrderBook");

    // Compare RwLock vs Lock-Free
    for num_threads in [1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("RwLock", num_threads),
            &num_threads,
            |b, &n| {
                let book = Arc::new(RwLockOrderBook::new("BTC-USD".into()));
                b.iter(|| {
                    // Spawn n threads, each doing 1000 operations
                    let handles: Vec<_> = (0..n)
                        .map(|_| {
                            let book = book.clone();
                            std::thread::spawn(move || {
                                for _ in 0..1000 {
                                    book.add_order(create_random_order());
                                }
                            })
                        })
                        .collect();

                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("LockFree", num_threads),
            &num_threads,
            |b, &n| {
                let book = Arc::new(LockFreeOrderBook::new("BTC-USD".into()));
                b.iter(|| {
                    let handles: Vec<_> = (0..n)
                        .map(|_| {
                            let book = book.clone();
                            std::thread::spawn(move || {
                                for _ in 0..1000 {
                                    book.add_order(create_random_order());
                                }
                            })
                        })
                        .collect();

                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_order_book_operations);
criterion_main!(benches);
```

### Expected Results

```
OrderBook/RwLock/1       time: [15.2 μs]
OrderBook/RwLock/2       time: [45.3 μs]  ← Contention starts
OrderBook/RwLock/4       time: [125.7 μs] ← Severe contention
OrderBook/RwLock/8       time: [312.4 μs]
OrderBook/RwLock/16      time: [847.2 μs]

OrderBook/LockFree/1     time: [18.1 μs]  ← Slightly slower (overhead)
OrderBook/LockFree/2     time: [22.4 μs]  ← Scales well!
OrderBook/LockFree/4     time: [31.2 μs]
OrderBook/LockFree/8     time: [48.7 μs]
OrderBook/LockFree/16    time: [72.3 μs]  ← 10x faster than RwLock!
```

---

## 10. Production Considerations

### When Lock-Free is Worth It

```
┌─────────────────────────────────────────────────────────────────┐
│ Decision Matrix: Lock-Free vs Locks                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                      Contention Level                            │
│                   Low         Medium        High                 │
│              ┌──────────┬──────────────┬──────────────┐         │
│   Critical   │  Locks   │  Lock-Free   │  Lock-Free   │         │
│   Section    │  OK      │  Better      │  Required    │         │
│   Length:    ├──────────┼──────────────┼──────────────┤         │
│   Short      │  Either  │  Lock-Free   │  Lock-Free   │         │
│              ├──────────┼──────────────┼──────────────┤         │
│   Long       │  Locks   │  Locks       │  Redesign!   │         │
│              └──────────┴──────────────┴──────────────┘         │
│                                                                  │
│  Additional factors:                                             │
│  • Latency requirements (lock-free = predictable)               │
│  • Team expertise (locks = simpler to reason about)             │
│  • Debugging difficulty (lock-free = much harder)               │
│  • Memory usage (epoch-based = higher)                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Hybrid Approach

For most trading systems, a **hybrid approach** works best:

```rust
/// Hybrid order book: lock-free reads, locked writes
pub struct HybridOrderBook {
    /// Current version (immutable snapshot)
    current: AtomicPtr<OrderBookSnapshot>,

    /// Write lock (only one writer at a time)
    write_lock: Mutex<()>,

    /// Epoch for safe memory reclamation
    epoch: epoch::Collector,
}

impl HybridOrderBook {
    /// Lock-free read (hot path)
    pub fn get_depth(&self, levels: usize) -> OrderBookDepth {
        let guard = epoch::pin();
        let snapshot = unsafe {
            self.current.load(Ordering::Acquire, &guard).as_ref()
        };
        snapshot.map(|s| s.get_depth(levels)).unwrap_or_default()
    }

    /// Locked write (less frequent)
    pub fn apply_update(&self, update: OrderBookUpdate) {
        let _lock = self.write_lock.lock().unwrap();

        let guard = epoch::pin();
        let old = self.current.load(Ordering::Acquire, &guard);

        // Create new snapshot with update applied
        let new_snapshot = unsafe {
            old.as_ref()
                .map(|s| s.with_update(update))
                .unwrap_or_else(|| OrderBookSnapshot::from_update(update))
        };

        let new_ptr = Owned::new(new_snapshot);

        // Publish new snapshot
        let old_ptr = self.current.swap(new_ptr, Ordering::AcqRel, &guard);

        // Defer destruction of old snapshot
        unsafe {
            guard.defer_destroy(old_ptr);
        }
    }
}
```

---

## 11. Alternative Approaches

### 1. Sharded Order Books

Instead of lock-free, shard by symbol:

```rust
pub struct ShardedOrderBookEngine {
    /// One shard per CPU core, each with its own lock
    shards: Vec<RwLock<HashMap<String, OrderBook>>>,
}

impl ShardedOrderBookEngine {
    pub fn get_shard(&self, symbol: &str) -> &RwLock<HashMap<String, OrderBook>> {
        let hash = fxhash::hash64(symbol);
        let shard_id = (hash as usize) % self.shards.len();
        &self.shards[shard_id]
    }
}
```

**Pros:** Simple, good locality, works well when symbols are independent
**Cons:** Hot symbols still contend, cross-symbol operations are complex

### 2. Actor Model (Tokio/Actix)

Each order book is an actor with a message queue:

```rust
use tokio::sync::mpsc;

pub struct OrderBookActor {
    book: OrderBook,
    rx: mpsc::Receiver<OrderBookMessage>,
}

impl OrderBookActor {
    pub async fn run(mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                OrderBookMessage::AddOrder { order, reply } => {
                    let result = self.book.add_order(order);
                    let _ = reply.send(result);
                }
                OrderBookMessage::Cancel { order_id, reply } => {
                    let result = self.book.cancel(order_id);
                    let _ = reply.send(result);
                }
                // ...
            }
        }
    }
}
```

**Pros:** No shared mutable state, easy to reason about, natural backpressure
**Cons:** Message passing overhead (~1μs), not truly parallel for single symbol

### 3. Read-Copy-Update (RCU)

Used in Linux kernel, similar to hybrid approach:

```rust
/// RCU-style order book
pub struct RcuOrderBook {
    /// Current version
    current: AtomicPtr<OrderBook>,
    /// Update lock
    update_lock: Mutex<()>,
}

impl RcuOrderBook {
    /// Readers get a reference, no locks
    pub fn read(&self) -> &OrderBook {
        unsafe { &*self.current.load(Ordering::Acquire) }
    }

    /// Writers copy-on-write
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&OrderBook) -> OrderBook,
    {
        let _lock = self.update_lock.lock().unwrap();

        let old = self.current.load(Ordering::Acquire);
        let new = Box::into_raw(Box::new(f(unsafe { &*old })));

        self.current.store(new, Ordering::Release);

        // Wait for readers to finish with old version
        // (grace period)
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Free old version
        unsafe { drop(Box::from_raw(old)); }
    }
}
```

**Pros:** Very fast reads, simple mental model
**Cons:** Writes are slow (full copy), grace period management is tricky

### Comparison Summary

| Approach | Read Latency | Write Latency | Complexity | Best For |
|----------|--------------|---------------|------------|----------|
| RwLock | ~5μs | ~10μs | Low | Low contention |
| Lock-Free | ~1μs | ~3μs | Very High | High contention HFT |
| Sharded | ~5μs | ~10μs | Low | Many independent symbols |
| Actor | ~2μs | ~5μs | Medium | Complex workflows |
| RCU | ~100ns | ~100μs | Medium | Read-heavy workloads |
| Hybrid | ~500ns | ~5μs | High | Production trading |

---

## Dependencies for Lock-Free Implementation

```toml
[dependencies]
crossbeam-epoch = "0.9"       # Epoch-based memory reclamation
crossbeam-skiplist = "0.1"    # Lock-free skip list
crossbeam-queue = "0.3"       # Lock-free queues
dashmap = "6.0"               # Concurrent HashMap
parking_lot = "0.12"          # Faster mutexes when needed

[dev-dependencies]
loom = "0.7"                  # Exhaustive concurrency testing
criterion = "0.5"             # Benchmarking
```

---

## Learning Path

1. **Week 1:** Study memory orderings, write simple atomic counters
2. **Week 2:** Implement Treiber stack, understand ABA problem
3. **Week 3:** Learn crossbeam-epoch, implement safe lock-free stack
4. **Week 4:** Build lock-free queue (Michael-Scott)
5. **Week 5:** Implement lock-free price level using SkipMap
6. **Week 6:** Build full lock-free order book
7. **Week 7:** Test with loom, stress test, benchmark
8. **Week 8:** Optimize based on profiling

---

## Recommended Reading

1. **"The Art of Multiprocessor Programming"** - Herlihy & Shavit
2. **"C++ Concurrency in Action"** - Anthony Williams (concepts apply to Rust)
3. **Crossbeam documentation** - https://docs.rs/crossbeam
4. **Rust Atomics and Locks** - Mara Bos (excellent Rust-specific resource)
5. **LMAX Disruptor paper** - https://lmax-exchange.github.io/disruptor/

---

## Next Steps

1. Start with the hybrid approach (Section 10) for production
2. Use crossbeam crates rather than implementing from scratch
3. Benchmark your specific workload before committing to lock-free
4. See [DATABASE_DATA_ENGINEERING.md](./DATABASE_DATA_ENGINEERING.md) for persistence layer
