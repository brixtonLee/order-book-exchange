# Rust Atomics: Deep Dive and ACID Comparison

## Table of Contents
1. [Introduction: The Concurrency Problem](#introduction-the-concurrency-problem)
2. [What Are Atomics?](#what-are-atomics)
3. [Hardware Foundation: How CPUs Implement Atomicity](#hardware-foundation-how-cpus-implement-atomicity)
4. [Memory Ordering Models](#memory-ordering-models)
5. [Rust Atomic Types](#rust-atomic-types)
6. [Atomic Operations in Detail](#atomic-operations-in-detail)
7. [Atomics vs ACID Transactions](#atomics-vs-acid-transactions)
8. [Practical Patterns](#practical-patterns)
9. [Performance Implications](#performance-implications)
10. [Common Pitfalls](#common-pitfalls)

---

## Introduction: The Concurrency Problem

### The Race Condition

```rust
use std::thread;
use std::sync::Arc;

fn demonstrate_race_condition() {
    let counter = Arc::new(0u64);
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                // THIS IS WRONG - Not atomic!
                // let value = *counter;
                // *counter = value + 1;
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Expected: 10,000
    // Actual: Random number < 10,000 (race condition)
    // println!("Final count: {}", *counter);
}
```

**Why this fails:**
1. Thread A reads counter value: 100
2. Thread B reads counter value: 100 (before A writes)
3. Thread A writes: 101
4. Thread B writes: 101 (should be 102!)
5. Lost update!

This is called a **race condition** - the outcome depends on the unpredictable timing of thread execution.

---

## What Are Atomics?

### Definition

**Atomics** are types that guarantee operations complete as a single, indivisible unit - they either fully succeed or fully fail, with no intermediate state visible to other threads.

### Key Properties

1. **Indivisibility**: Operations cannot be interrupted midway
2. **Visibility**: Changes are immediately visible to other threads (with proper ordering)
3. **Ordering**: Control over when and how changes become visible

### The Three Levels of Synchronization

```
Level 1: No Synchronization (Race Conditions)
    ↓
Level 2: Atomics (Lock-Free Synchronization)
    ↓
Level 3: Locks (Mutual Exclusion)
```

---

## Hardware Foundation: How CPUs Implement Atomicity

### CPU Architecture Fundamentals

Modern CPUs don't directly read/write main memory. They use a hierarchy:

```
[CPU Core]
    ↓
[L1 Cache] - Private to core (32KB typical)
    ↓
[L2 Cache] - Private to core (256KB typical)
    ↓
[L3 Cache] - Shared across cores (8MB+ typical)
    ↓
[Main Memory (RAM)]
```

### The MESI Protocol (Cache Coherency)

CPUs use the **MESI protocol** to keep caches synchronized:

- **M**odified: This cache has the only valid copy, modified
- **E**xclusive: This cache has the only valid copy, unmodified
- **S**hared: Multiple caches have valid copies
- **I**nvalid: This cache line is invalid

**Example scenario:**
```
Initial state:
Core 0: [x=5, State=S]
Core 1: [x=5, State=S]

Core 0 writes x=6:
1. Core 0 sends "invalidate" message
2. Core 1 marks x as Invalid
3. Core 0 sets state to Modified
4. Core 0's cache: [x=6, State=M]
5. Core 1's cache: [x=?, State=I]

Core 1 reads x:
1. Core 1 sends read request
2. Core 0 writes back to memory
3. Both cores load x=6
4. Both caches: [x=6, State=S]
```

### Hardware Atomic Instructions

CPUs provide special instructions for atomic operations:

#### 1. **Compare-And-Swap (CAS)**
```assembly
; x86-64 instruction: CMPXCHG
; Pseudocode:
atomic_cas(addr, expected, new):
    current = *addr
    if current == expected:
        *addr = new
        return true
    else:
        return false
```

**Hardware guarantees:**
- Reads current value
- Compares with expected
- Writes new value
- All in ONE indivisible operation
- Uses cache line locking

#### 2. **Fetch-And-Add**
```assembly
; x86-64 instruction: LOCK XADD
; Pseudocode:
atomic_fetch_add(addr, delta):
    old_value = *addr
    *addr = old_value + delta
    return old_value
```

#### 3. **Load-Linked / Store-Conditional (LL/SC)**
ARM architecture alternative to CAS:
```assembly
; ARM instructions: LDREX / STREX
load_linked(addr):
    value = *addr
    mark_reservation(addr)
    return value

store_conditional(addr, value):
    if reservation_valid(addr):
        *addr = value
        return success
    else:
        return failure
```

### Memory Barriers (Fences)

CPUs can reorder instructions for performance. Memory barriers prevent this:

#### Types of Barriers:
1. **Compiler Barrier**: Prevents compiler reordering
2. **CPU Barrier**: Prevents CPU reordering

```assembly
; x86-64 memory barriers
MFENCE  ; Full memory barrier (load + store)
LFENCE  ; Load barrier
SFENCE  ; Store barrier
```

**Example of CPU reordering:**
```rust
// Source code:
let mut data = 0;
let mut ready = false;

// Thread 1:
data = 42;
ready = true;

// Thread 2:
if ready {
    println!("{}", data);
}

// Without barriers, CPU might reorder to:
// Thread 1:
ready = true;  // Reordered!
data = 42;

// Thread 2 might see ready=true but data=0!
```

---

## Memory Ordering Models

Memory ordering controls **when** atomic operations become visible to other threads.

### The Five Ordering Levels

#### 1. **Relaxed** (Weakest)

**Guarantee**: Only atomicity, NO ordering guarantees

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn relaxed_ordering() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                // Only guarantees this specific operation is atomic
                // Other threads might see updates in any order
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final: {}", counter.load(Ordering::Relaxed));
    // Will be 10,000 - atomicity guaranteed
    // But intermediate values might be seen out-of-order by other threads
}
```

**Hardware implementation:**
- Simple atomic instruction
- NO memory barriers
- Fastest, but dangerous for complex logic

**When to use:**
- Simple counters
- Monotonically increasing IDs
- When order doesn't matter

#### 2. **Acquire** (Load operation)

**Guarantee**: All operations AFTER this load cannot be reordered BEFORE it

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static DATA: AtomicU64 = AtomicU64::new(0);
static READY: AtomicBool = AtomicBool::new(false);

fn acquire_ordering() {
    // Writer thread
    let writer = thread::spawn(|| {
        DATA.store(42, Ordering::Relaxed);
        // Release: Ensures DATA write happens before READY write
        READY.store(true, Ordering::Release);
    });

    // Reader thread
    let reader = thread::spawn(|| {
        // Acquire: Ensures READY read happens before DATA read
        while !READY.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }

        // This is guaranteed to see DATA=42
        let value = DATA.load(Ordering::Relaxed);
        println!("Data: {}", value);  // Always prints 42
    });

    writer.join().unwrap();
    reader.join().unwrap();
}
```

**Hardware implementation:**
- Load with **load barrier** after
- Prevents younger loads/stores from moving before this load

**Visual representation:**
```
[Acquire Load]
─────────────── Barrier (nothing can cross up)
[All subsequent operations]
```

#### 3. **Release** (Store operation)

**Guarantee**: All operations BEFORE this store cannot be reordered AFTER it

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn release_ordering() {
    let data = Arc::new(AtomicU64::new(0));
    let flag = Arc::new(AtomicBool::new(false));

    let data_clone = Arc::clone(&data);
    let flag_clone = Arc::clone(&flag);

    // Publisher thread
    thread::spawn(move || {
        // All these writes are guaranteed to complete
        // before the Release store below
        data_clone.store(100, Ordering::Relaxed);
        data_clone.fetch_add(50, Ordering::Relaxed);

        // Release: publishes all previous writes
        flag_clone.store(true, Ordering::Release);
    });

    // Subscriber thread
    thread::spawn(move || {
        // Acquire: synchronizes with Release above
        while !flag.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        // Guaranteed to see all writes from publisher
        println!("Data: {}", data.load(Ordering::Relaxed));  // 150
    });
}
```

**Hardware implementation:**
- Store with **store barrier** before
- Prevents older loads/stores from moving after this store

**Visual representation:**
```
[All prior operations]
─────────────── Barrier (nothing can cross down)
[Release Store]
```

#### 4. **AcqRel** (Acquire-Release)

**Guarantee**: Combination of Acquire + Release

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn acqrel_ordering() {
    let counter = Arc::new(AtomicU64::new(0));

    // Read-Modify-Write operations need AcqRel
    let result = counter.fetch_add(5, Ordering::AcqRel);

    // Equivalent to:
    // 1. Acquire load (synchronizes with previous Release)
    // 2. Modify
    // 3. Release store (publishes to next Acquire)
}
```

**When to use:**
- Read-Modify-Write operations
- Fetch-and-add, Compare-and-swap
- Building synchronization primitives

#### 5. **SeqCst** (Sequentially Consistent) - Strongest

**Guarantee**: Total global ordering of all SeqCst operations

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

static X: AtomicU64 = AtomicU64::new(0);
static Y: AtomicU64 = AtomicU64::new(0);

fn seqcst_ordering() {
    // Thread 1
    let t1 = thread::spawn(|| {
        X.store(1, Ordering::SeqCst);
        let y = Y.load(Ordering::SeqCst);
        y
    });

    // Thread 2
    let t2 = thread::spawn(|| {
        Y.store(1, Ordering::SeqCst);
        let x = X.load(Ordering::SeqCst);
        x
    });

    let y = t1.join().unwrap();
    let x = t2.join().unwrap();

    // With SeqCst: At least one will see the other's write
    // IMPOSSIBLE: x=0 && y=0
    // Possible: x=1, y=0 or x=0, y=1 or x=1, y=1

    println!("x={}, y={}", x, y);
}
```

**Hardware implementation:**
- Full memory barriers (both load and store)
- Ensures total ordering across ALL threads

**Performance cost:**
- Slowest (heavy barriers)
- But simplest mental model

### Memory Ordering Summary Table

| Ordering | Load | Store | RMW | Barrier | Use Case |
|----------|------|-------|-----|---------|----------|
| Relaxed | ✓ | ✓ | ✓ | None | Counters, stats |
| Acquire | ✓ | ✗ | ✗ | Load barrier after | Lock acquire |
| Release | ✗ | ✓ | ✗ | Store barrier before | Lock release |
| AcqRel | ✗ | ✗ | ✓ | Both | RMW operations |
| SeqCst | ✓ | ✓ | ✓ | Full | Simple correctness |

### The Synchronizes-With Relationship

**Key concept:** Release-Acquire creates a "synchronizes-with" relationship:

```
Thread A                    Thread B
--------                    --------
[Operations A1]
[Operations A2]
[Release Store] ─────────→ [Acquire Load]
                            [Operations B1]
                            [Operations B2]
```

**Guarantee:** Thread B's Acquire load will see all operations before Thread A's Release store.

---

## Rust Atomic Types

### Available Atomic Types

```rust
use std::sync::atomic::*;

// Integer types
AtomicBool      // atomic bool
AtomicI8        // atomic i8
AtomicI16       // atomic i16
AtomicI32       // atomic i32
AtomicI64       // atomic i64
AtomicIsize     // atomic isize
AtomicU8        // atomic u8
AtomicU16       // atomic u16
AtomicU32       // atomic u32
AtomicU64       // atomic u64
AtomicUsize     // atomic usize

// Pointer type
AtomicPtr<T>    // atomic *mut T
```

### Basic Operations

```rust
use std::sync::atomic::{AtomicU64, Ordering};

fn atomic_operations() {
    let x = AtomicU64::new(0);

    // Load
    let value = x.load(Ordering::SeqCst);

    // Store
    x.store(42, Ordering::SeqCst);

    // Swap
    let old = x.swap(100, Ordering::SeqCst);

    // Compare and swap
    let result = x.compare_exchange(
        100,                    // expected
        200,                    // new
        Ordering::SeqCst,       // success ordering
        Ordering::SeqCst        // failure ordering
    );

    match result {
        Ok(previous) => println!("Swapped! Previous: {}", previous),
        Err(current) => println!("Failed! Current: {}", current),
    }

    // Fetch-and-add
    let previous = x.fetch_add(10, Ordering::SeqCst);

    // Fetch-and-sub
    let previous = x.fetch_sub(5, Ordering::SeqCst);

    // Fetch-and-or (bitwise)
    let previous = x.fetch_or(0b1111, Ordering::SeqCst);

    // Fetch-and-and (bitwise)
    let previous = x.fetch_and(0b1010, Ordering::SeqCst);
}
```

---

## Atomic Operations in Detail

### Compare-And-Swap (CAS) Deep Dive

CAS is the fundamental building block of lock-free algorithms.

#### Strong CAS vs Weak CAS

```rust
use std::sync::atomic::{AtomicU64, Ordering};

fn cas_comparison() {
    let x = AtomicU64::new(100);

    // Strong CAS - Guaranteed to retry on spurious failures
    let result = x.compare_exchange(
        100,                // expected
        200,                // new
        Ordering::SeqCst,   // success
        Ordering::SeqCst    // failure
    );

    // Weak CAS - May spuriously fail even if value matches
    // Cheaper on some platforms (ARM LL/SC)
    let result = x.compare_exchange_weak(
        200,                // expected
        300,                // new
        Ordering::SeqCst,   // success
        Ordering::SeqCst    // failure
    );

    // Weak CAS usage pattern (loop until success)
    loop {
        let current = x.load(Ordering::SeqCst);
        let new = current + 1;

        match x.compare_exchange_weak(
            current,
            new,
            Ordering::SeqCst,
            Ordering::SeqCst
        ) {
            Ok(_) => break,
            Err(_) => continue,  // Retry on failure
        }
    }
}
```

#### CAS Loop Pattern (Lock-Free Increment)

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn lockfree_increment() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                // Lock-free increment using CAS loop
                loop {
                    let current = counter.load(Ordering::Acquire);
                    let new = current + 1;

                    match counter.compare_exchange_weak(
                        current,
                        new,
                        Ordering::Release,
                        Ordering::Acquire
                    ) {
                        Ok(_) => break,     // Success!
                        Err(_) => continue, // Retry
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final: {}", counter.load(Ordering::SeqCst));  // 10,000
}
```

### Fetch Operations

```rust
use std::sync::atomic::{AtomicU64, Ordering};

fn fetch_operations() {
    let x = AtomicU64::new(100);

    // All fetch operations return the PREVIOUS value

    // Fetch-add: returns old value, stores old + delta
    let old = x.fetch_add(50, Ordering::SeqCst);
    println!("Old: {}, New: {}", old, x.load(Ordering::SeqCst));
    // Output: Old: 100, New: 150

    // Fetch-sub
    let old = x.fetch_sub(25, Ordering::SeqCst);
    println!("Old: {}, New: {}", old, x.load(Ordering::SeqCst));
    // Output: Old: 150, New: 125

    // Fetch-update: Conditional update with function
    let result = x.fetch_update(
        Ordering::SeqCst,
        Ordering::SeqCst,
        |current| {
            if current > 100 {
                Some(current * 2)  // Update if > 100
            } else {
                None               // Don't update
            }
        }
    );

    match result {
        Ok(previous) => println!("Updated! Previous: {}", previous),
        Err(current) => println!("Not updated. Current: {}", current),
    }
}
```

---

## Atomics vs ACID Transactions

### Database ACID Properties

**ACID** is a set of properties for database transactions:

#### 1. **Atomicity** (All or Nothing)
```sql
BEGIN TRANSACTION;
    UPDATE accounts SET balance = balance - 100 WHERE id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- If either UPDATE fails, BOTH are rolled back
-- Transaction is atomic: all changes or no changes
```

#### 2. **Consistency** (Invariants Maintained)
```sql
-- Invariant: total_balance = sum(all accounts)
-- Database ensures this is ALWAYS true
-- Even if transaction is in progress

BEGIN TRANSACTION;
    -- At this point, invariant might be temporarily violated
    UPDATE accounts SET balance = balance - 100 WHERE id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE id = 2;
    -- Invariant restored
COMMIT;
```

#### 3. **Isolation** (Concurrent Transactions Don't Interfere)
```sql
-- Transaction T1:
BEGIN TRANSACTION;
    SELECT balance FROM accounts WHERE id = 1;  -- Reads 1000
    -- ... do some computation ...
    UPDATE accounts SET balance = 900 WHERE id = 1;
COMMIT;

-- Transaction T2 (running concurrently):
BEGIN TRANSACTION;
    SELECT balance FROM accounts WHERE id = 1;  -- What does this see?
COMMIT;

-- Isolation level determines visibility:
-- - READ UNCOMMITTED: Might see partial T1 changes (dirty read)
-- - READ COMMITTED: Sees committed data only
-- - REPEATABLE READ: Sees snapshot from start of T2
-- - SERIALIZABLE: As if T1 and T2 ran sequentially
```

#### 4. **Durability** (Persisted After Commit)
```sql
BEGIN TRANSACTION;
    UPDATE accounts SET balance = 1000 WHERE id = 1;
COMMIT;

-- After COMMIT, even if:
-- - System crashes
-- - Power outage
-- - Disk failure
-- Database guarantees this change survives
-- (via write-ahead logging, checkpoints, etc.)
```

### Rust Atomics: A Different Model

Rust atomics provide **memory-level atomicity**, NOT **transaction-level atomicity**.

#### Comparison Table

| Property | Rust Atomics | Database ACID |
|----------|--------------|---------------|
| **Scope** | Single memory location | Multiple operations |
| **Atomicity** | Individual operation | Entire transaction |
| **Consistency** | Manual (programmer's job) | Automatic (constraints) |
| **Isolation** | Memory ordering only | Transaction isolation levels |
| **Durability** | Volatile (RAM only) | Persistent (disk) |
| **Rollback** | ❌ No automatic rollback | ✅ Automatic rollback |
| **Multi-location** | ❌ Can't atomic across locations | ✅ Multi-row updates |
| **Performance** | Nanoseconds | Milliseconds |

### Example: Money Transfer

#### Database ACID Transaction
```sql
BEGIN TRANSACTION;
    -- Atomicity: Both succeed or both fail
    UPDATE accounts SET balance = balance - 100 WHERE id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE id = 2;

    -- Consistency: Check constraint
    ASSERT (SELECT balance FROM accounts WHERE id = 1) >= 0;

    -- If any fails, automatic rollback
COMMIT;
```

#### Rust Atomics (Can't Do This!)
```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct Account {
    balance: AtomicU64,
}

fn transfer(from: &Account, to: &Account, amount: u64) {
    // ❌ THIS IS WRONG - Not atomic across two locations!

    // Problem: If thread crashes between these two lines,
    // money disappears!
    from.balance.fetch_sub(amount, Ordering::SeqCst);
    to.balance.fetch_add(amount, Ordering::SeqCst);

    // No automatic rollback
    // No consistency checks
    // No isolation from other transfers
}
```

#### Solution: Use Mutex (Lock-Based)
```rust
use std::sync::{Arc, Mutex};

struct Account {
    balance: Mutex<u64>,
}

fn transfer(from: &Account, to: &Account, amount: u64) -> Result<(), String> {
    // Lock both accounts (in consistent order to avoid deadlock)
    let mut from_balance = from.balance.lock().unwrap();
    let mut to_balance = to.balance.lock().unwrap();

    // Check constraint
    if *from_balance < amount {
        return Err("Insufficient funds".to_string());
    }

    // Perform transfer atomically
    *from_balance -= amount;
    *to_balance += amount;

    Ok(())
    // Locks released automatically when guards drop
}
```

### When Atomics Excel vs Databases

#### Use Atomics When:
1. **Single location updates**
   - Counters, flags, monotonic IDs
2. **Ultra-low latency required**
   - HFT systems (nanoseconds matter)
3. **In-memory only**
   - No persistence needed
4. **Lock-free algorithms**
   - Maximum concurrency

#### Use Database Transactions When:
1. **Multi-entity operations**
   - Transfer between accounts
2. **Complex consistency requirements**
   - Foreign keys, constraints
3. **Durability required**
   - Must survive crashes
4. **Isolation levels needed**
   - Prevent concurrent interference

### Hybrid Approach: Atomics + Persistence

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::fs::OpenOptions;
use std::io::Write;

struct DurableCounter {
    value: AtomicU64,
    wal_file: Mutex<std::fs::File>,
}

impl DurableCounter {
    fn increment(&self) -> u64 {
        // Step 1: Write to WAL (Write-Ahead Log)
        let new_value = self.value.fetch_add(1, Ordering::SeqCst) + 1;

        let mut file = self.wal_file.lock().unwrap();
        writeln!(file, "INC {}", new_value).unwrap();
        file.sync_all().unwrap();  // Force to disk

        // Step 2: Now it's durable!
        new_value
    }
}
```

---

## Practical Patterns

### Pattern 1: Atomic Counter (Statistics)

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

struct TradingStats {
    total_trades: AtomicU64,
    total_volume: AtomicU64,
    rejected_orders: AtomicU64,
}

impl TradingStats {
    fn new() -> Self {
        Self {
            total_trades: AtomicU64::new(0),
            total_volume: AtomicU64::new(0),
            rejected_orders: AtomicU64::new(0),
        }
    }

    fn record_trade(&self, volume: u64) {
        self.total_trades.fetch_add(1, Ordering::Relaxed);
        self.total_volume.fetch_add(volume, Ordering::Relaxed);
    }

    fn record_rejection(&self) {
        self.rejected_orders.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.total_trades.load(Ordering::Relaxed),
            self.total_volume.load(Ordering::Relaxed),
            self.rejected_orders.load(Ordering::Relaxed),
        )
    }
}
```

**Why Relaxed?** Order of increments doesn't matter for independent counters.

### Pattern 2: Lock-Free Stack (Treiber Stack)

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

struct Node<T> {
    data: T,
    next: *mut Node<T>,
}

struct LockFreeStack<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> LockFreeStack<T> {
    fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn push(&self, data: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data,
            next: ptr::null_mut(),
        }));

        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe {
                (*new_node).next = head;
            }

            match self.head.compare_exchange_weak(
                head,
                new_node,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(_) => continue,  // Retry
            }
        }
    }

    fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            if head.is_null() {
                return None;
            }

            let next = unsafe { (*head).next };

            match self.head.compare_exchange_weak(
                head,
                next,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let data = unsafe { Box::from_raw(head).data };
                    return Some(data);
                }
                Err(_) => continue,  // Retry
            }
        }
    }
}
```

### Pattern 3: Spinlock (Custom Lock)

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::UnsafeCell;

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        // Spin until we acquire the lock
        while self.locked.swap(true, Ordering::Acquire) {
            // CPU hint: we're spinning
            std::hint::spin_loop();
        }

        SpinLockGuard { lock: self }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<T> std::ops::Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> std::ops::DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}
```

### Pattern 4: Lazy Initialization (Once)

```rust
use std::sync::atomic::{AtomicU8, Ordering};
use std::cell::UnsafeCell;

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

pub struct LazyInit<T> {
    state: AtomicU8,
    data: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send> Sync for LazyInit<T> {}

impl<T> LazyInit<T> {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(UNINITIALIZED),
            data: UnsafeCell::new(None),
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        // Fast path: already initialized
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            return unsafe { (*self.data.get()).as_ref().unwrap() };
        }

        // Slow path: need to initialize
        self.initialize(f);

        unsafe { (*self.data.get()).as_ref().unwrap() }
    }

    fn initialize<F>(&self, f: F)
    where
        F: FnOnce() -> T,
    {
        loop {
            match self.state.compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::Acquire,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // We won the race, initialize
                    let value = f();
                    unsafe {
                        *self.data.get() = Some(value);
                    }
                    self.state.store(INITIALIZED, Ordering::Release);
                    return;
                }
                Err(INITIALIZED) => {
                    // Someone else finished
                    return;
                }
                Err(INITIALIZING) => {
                    // Someone else is initializing, spin
                    std::hint::spin_loop();
                }
                Err(_) => unreachable!(),
            }
        }
    }
}
```

### Pattern 5: Sequence Lock (Reader-Writer)

```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct SeqLock<T> {
    seq: AtomicU64,
    data: std::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    fn new(data: T) -> Self {
        Self {
            seq: AtomicU64::new(0),
            data: std::cell::UnsafeCell::new(data),
        }
    }

    // Writers
    fn write(&self, value: T) {
        // Increment seq to odd (indicates write in progress)
        let seq = self.seq.fetch_add(1, Ordering::Acquire);

        // Write data
        unsafe {
            *self.data.get() = value;
        }

        // Increment seq to even (indicates write complete)
        self.seq.store(seq + 2, Ordering::Release);
    }

    // Readers (lock-free!)
    fn read(&self) -> T {
        loop {
            let seq1 = self.seq.load(Ordering::Acquire);

            // If odd, writer is active, retry
            if seq1 % 2 != 0 {
                std::hint::spin_loop();
                continue;
            }

            // Read data
            let data = unsafe { *self.data.get() };

            // Check if writer interfered
            let seq2 = self.seq.load(Ordering::Acquire);
            if seq1 == seq2 {
                return data;
            }

            // Writer interfered, retry
        }
    }
}
```

---

## Performance Implications

### Hardware Cost Comparison

```
Operation                   | Latency (approx)
----------------------------|------------------
Register access             | 0.3 ns
L1 cache hit                | 1 ns
L2 cache hit                | 3 ns
L3 cache hit                | 12 ns
Atomic (no contention)      | 5-20 ns
Atomic (with contention)    | 50-500 ns
Mutex lock/unlock           | 25-100 ns
System call                 | 1000 ns (1 µs)
```

### Ordering Performance

```
Ordering    | x86-64 Cost           | ARM Cost
------------|----------------------|-------------------
Relaxed     | ~5 ns (no barrier)   | ~5 ns
Acquire     | ~10 ns (LFENCE)      | ~20 ns (DMB)
Release     | ~10 ns (SFENCE)      | ~20 ns (DMB)
AcqRel      | ~15 ns (both)        | ~30 ns (both)
SeqCst      | ~25 ns (MFENCE)      | ~50 ns (full DMB)
```

### Benchmark: Counter Implementation

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

fn bench_counter() {
    const ITERATIONS: u64 = 1_000_000;
    const THREADS: usize = 4;

    // Atomic Relaxed
    let atomic_counter = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let counter = Arc::clone(&atomic_counter);
            thread::spawn(move || {
                for _ in 0..ITERATIONS {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    println!("Atomic Relaxed: {:?}", start.elapsed());

    // Atomic SeqCst
    let atomic_counter = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let counter = Arc::clone(&atomic_counter);
            thread::spawn(move || {
                for _ in 0..ITERATIONS {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    println!("Atomic SeqCst: {:?}", start.elapsed());

    // Mutex
    let mutex_counter = Arc::new(Mutex::new(0u64));
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let counter = Arc::clone(&mutex_counter);
            thread::spawn(move || {
                for _ in 0..ITERATIONS {
                    *counter.lock().unwrap() += 1;
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    println!("Mutex: {:?}", start.elapsed());
}

// Typical results:
// Atomic Relaxed: 50ms
// Atomic SeqCst: 80ms
// Mutex: 200ms
```

### Cache Line Contention (False Sharing)

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

// ❌ BAD: False sharing
struct BadCounters {
    counter1: AtomicU64,  // Same cache line
    counter2: AtomicU64,  // Same cache line
}

// ✅ GOOD: Padding to separate cache lines
#[repr(align(64))]  // Cache line size
struct GoodCounters {
    counter1: AtomicU64,
    _padding: [u8; 56],   // Fill rest of cache line
    counter2: AtomicU64,
}

// False sharing causes constant cache invalidation between cores
// Performance can drop by 10x!
```

---

## Common Pitfalls

### Pitfall 1: Using Relaxed for Non-Counter Logic

```rust
// ❌ WRONG: Relaxed doesn't order with other operations
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static DATA: AtomicU64 = AtomicU64::new(0);
static READY: AtomicBool = AtomicBool::new(false);

// Writer thread
DATA.store(42, Ordering::Relaxed);
READY.store(true, Ordering::Relaxed);  // ❌ No ordering guarantee!

// Reader thread
if READY.load(Ordering::Relaxed) {
    let value = DATA.load(Ordering::Relaxed);
    // Might see DATA=0! (old value)
}

// ✅ CORRECT: Use Release-Acquire
DATA.store(42, Ordering::Relaxed);
READY.store(true, Ordering::Release);  // ✅ Publishes DATA write

if READY.load(Ordering::Acquire) {  // ✅ Synchronizes with Release
    let value = DATA.load(Ordering::Relaxed);
    // Guaranteed to see DATA=42
}
```

### Pitfall 2: ABA Problem

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

struct Node {
    value: i32,
    next: *mut Node,
}

// ❌ Problem: ABA issue
fn aba_problem(head: &AtomicPtr<Node>) {
    // Thread 1: Read head (A)
    let head_ptr = head.load(Ordering::Acquire);
    let next = unsafe { (*head_ptr).next };

    // Thread 2: Pop A, Pop B, Push A back
    // Now head is A again, but it's a DIFFERENT A!

    // Thread 1: CAS succeeds (sees A == A)
    // But next might point to freed memory!
    head.compare_exchange(
        head_ptr,
        next,
        Ordering::Release,
        Ordering::Acquire,
    ); // ❌ Dangerous!
}

// ✅ Solution: Tagged pointers or hazard pointers
```

### Pitfall 3: Forgetting Fences

```rust
use std::sync::atomic::{fence, AtomicBool, Ordering};

static mut DATA: Vec<u32> = Vec::new();
static READY: AtomicBool = AtomicBool::new(false);

// ❌ WRONG: Non-atomic write without fence
unsafe {
    DATA = vec![1, 2, 3];
}
READY.store(true, Ordering::Release);  // Not enough!

// ✅ CORRECT: Use fence
unsafe {
    DATA = vec![1, 2, 3];
}
fence(Ordering::Release);  // Fence for non-atomic
READY.store(true, Ordering::Release);
```

### Pitfall 4: Deadlock with CAS Loops

```rust
// ❌ WRONG: Can livelock or deadlock
use std::sync::atomic::{AtomicU64, Ordering};

fn problematic_update(x: &AtomicU64, y: &AtomicU64) {
    loop {
        let x_val = x.load(Ordering::SeqCst);
        let y_val = y.load(Ordering::SeqCst);

        // Try to update both
        if x.compare_exchange(x_val, x_val + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            continue;
        }
        if y.compare_exchange(y_val, y_val + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            // Roll back x? Can't easily!
            // This is why atomics can't do transactions
        }
    }
}

// ✅ CORRECT: Use mutex for multi-variable updates
```

---

## Summary

### Key Takeaways

1. **Atomics are NOT transactions**
   - Single-location atomicity only
   - No rollback, no multi-variable consistency
   - Programmer must maintain invariants

2. **Memory ordering matters**
   - Relaxed: Counters/stats (fastest)
   - Acquire-Release: Synchronization
   - SeqCst: When in doubt (slowest but simplest)

3. **Hardware matters**
   - x86 has strong memory model (most operations are already ordered)
   - ARM has weak memory model (barriers are expensive)
   - Cache line contention kills performance

4. **Use the right tool**
   - Simple counters → Atomics
   - Complex state → Mutex
   - Multi-variable consistency → Transactions (database or STM)

5. **Atomics vs ACID**
   - Atomics: Memory-level, nanoseconds, single location
   - ACID: Transaction-level, milliseconds, multi-entity

### When to Use What

```
Simple counter?
    → Atomic with Relaxed

Synchronization flag?
    → Atomic with Release-Acquire

Multi-variable update?
    → Mutex (or database transaction)

Lock-free queue?
    → Atomic with CAS loops (advanced!)

Need persistence?
    → Database with ACID

Ultra-low latency?
    → Atomics + careful design
```

### The Memory Ordering Decision Tree

```
Do operations need to be ordered?
    No → Relaxed
    Yes ↓

Publishing data to other threads?
    Yes → Release (writer) + Acquire (reader)
    No ↓

Read-Modify-Write operation?
    Yes → AcqRel
    No ↓

Not sure / Maximum safety?
    → SeqCst
```

This guide covered the deep fundamentals of Rust atomics and how they compare to database ACID transactions. Atomics are low-level primitives for building concurrent systems, while ACID transactions are high-level guarantees for data integrity. Both are "atomic" but at very different levels of abstraction!
