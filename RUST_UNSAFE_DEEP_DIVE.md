# Rust Unsafe: The Complete Guide

**Understanding `unsafe` in Rust vs C#, with practical examples for systems programming**

---

## Table of Contents

1. [What is Unsafe?](#1-what-is-unsafe)
2. [Rust unsafe vs C# unsafe](#2-rust-unsafe-vs-c-unsafe)
3. [The Five Unsafe Superpowers](#3-the-five-unsafe-superpowers)
4. [When to Use Unsafe](#4-when-to-use-unsafe)
5. [Safety Contracts and Invariants](#5-safety-contracts-and-invariants)
6. [Common Unsafe Patterns](#6-common-unsafe-patterns)
7. [Unsafe in Trading Systems](#7-unsafe-in-trading-systems)
8. [Writing Safe Abstractions](#8-writing-safe-abstractions)
9. [Auditing Unsafe Code](#9-auditing-unsafe-code)
10. [Common Pitfalls and How to Avoid Them](#10-common-pitfalls-and-how-to-avoid-them)
11. [Tools and Testing](#11-tools-and-testing)

---

## 1. What is Unsafe?

### The Rust Promise

Rust's core promise is **memory safety without garbage collection**. The compiler enforces:

- No null pointer dereferences
- No dangling pointers
- No data races
- No buffer overflows
- No use-after-free

**But sometimes you need to break these rules.**

### What unsafe Means

```rust
// ✅ Safe Rust: Compiler checks everything
fn safe_function() {
    let v = vec![1, 2, 3];
    let first = v.get(0);  // Returns Option<&i32>, can't crash
}

// ⚠️ Unsafe Rust: Programmer must uphold invariants
unsafe fn unsafe_function() {
    let v = vec![1, 2, 3];
    let first = v.get_unchecked(0);  // Panics if out of bounds!
}
```

**Key insight**: `unsafe` is an **escape hatch**, not a permission slip to ignore safety. You're telling the compiler "I'll manually ensure safety here."

### The Safety Boundary

```rust
// Public safe API
pub fn process_orders(orders: &[Order]) -> Vec<Trade> {
    // Internally uses unsafe for performance
    unsafe {
        // But maintains safety guarantees!
        fast_process_unchecked(orders)
    }
}

// Unsafe is encapsulated
unsafe fn fast_process_unchecked(orders: &[Order]) -> Vec<Trade> {
    // Performance-critical code with manual safety checks
}
```

**Philosophy**:
- Keep `unsafe` blocks **small** and **isolated**
- Wrap unsafe code in **safe abstractions**
- Document **safety invariants**

---

## 2. Rust unsafe vs C# unsafe

### Comparison Table

| Feature | Rust `unsafe` | C# `unsafe` |
|---------|---------------|-------------|
| **Purpose** | Bypass borrow checker, direct memory access | Pointer operations, unmanaged memory |
| **Scope** | Functions, blocks, traits, implementations | Methods, blocks, types |
| **Garbage Collection** | No GC (manual memory management) | GC still runs (managed memory) |
| **Dereferencing pointers** | Must be in unsafe block | Allowed in unsafe context |
| **Memory model** | Ownership + borrowing rules | GC + managed heap |
| **FFI** | Required for C interop | P/Invoke, COM interop |
| **What it enables** | 5 specific operations | Pointer arithmetic, pinning, marshalling |
| **Default assumption** | Unsafe is rare | Unsafe is common in low-level code |

### C# unsafe Example

```csharp
// C# unsafe: Mostly about pointers
public unsafe class Example {
    public unsafe int* CreateArray(int size) {
        int* arr = stackalloc int[size];
        for (int i = 0; i < size; i++) {
            arr[i] = i;
        }
        return arr;  // ⚠️ Stack memory, will be invalid!
    }

    public unsafe void ProcessBuffer(byte* buffer, int length) {
        for (int i = 0; i < length; i++) {
            buffer[i] = 0;
        }
    }
}
```

**C# unsafe is about:**
- Working with pointers (`*` operator)
- Stack allocation (`stackalloc`)
- Pinning managed objects (`fixed`)
- **GC still manages memory** - you're just using pointers

### Rust unsafe Example

```rust
// Rust unsafe: About breaking borrow checker rules
pub struct Example;

impl Example {
    // Rust unsafe: Manual memory management + breaking borrow rules
    pub unsafe fn create_array(size: usize) -> *mut i32 {
        let layout = std::alloc::Layout::array::<i32>(size).unwrap();
        let ptr = std::alloc::alloc(layout) as *mut i32;

        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        for i in 0..size {
            ptr.add(i).write(i as i32);
        }

        ptr  // Caller must deallocate!
    }

    pub unsafe fn process_buffer(buffer: *mut u8, length: usize) {
        for i in 0..length {
            *buffer.add(i) = 0;
        }
    }
}
```

**Rust unsafe is about:**
- Raw pointer dereferencing
- Manual memory allocation/deallocation
- Breaking borrow checker rules
- **No GC** - you're responsible for everything

### Key Differences

#### Memory Management

```csharp
// C#: GC handles deallocation
unsafe void CSharpExample() {
    byte* ptr = stackalloc byte[1024];
    // Use ptr...
    // No cleanup needed - stack memory auto-freed
    // Or GC handles heap allocations
}
```

```rust
// Rust: Manual deallocation required
unsafe fn rust_example() {
    let layout = Layout::array::<u8>(1024).unwrap();
    let ptr = alloc(layout);
    // Use ptr...
    dealloc(ptr, layout);  // MUST manually free!
}
```

#### Borrow Checker

```csharp
// C#: No borrow checker at all
class Account {
    public int balance;
}

void Transfer(Account from, Account to) {
    from.balance -= 100;  // ✅ OK in C#
    to.balance += 100;    // Can mutate both
}
```

```rust
// Rust: Borrow checker enforced
struct Account {
    balance: i32,
}

// ❌ Won't compile - can't have two mutable references
fn transfer(from: &mut Account, to: &mut Account) {
    from.balance -= 100;
    to.balance += 100;
}

// Need unsafe to bypass borrow checker (if you know it's safe)
unsafe fn transfer_unchecked(from: *mut Account, to: *mut Account) {
    (*from).balance -= 100;
    (*to).balance += 100;
}
```

---

## 3. The Five Unsafe Superpowers

Rust's `unsafe` allows exactly **five operations** that safe Rust prohibits:

### 1. Dereference Raw Pointers

```rust
// Safe: References enforce borrowing rules
fn safe_dereference(x: &i32) -> i32 {
    *x  // ✅ Compiler verifies this is safe
}

// Unsafe: Raw pointers have no guarantees
unsafe fn unsafe_dereference(ptr: *const i32) -> i32 {
    *ptr  // ⚠️ Might be null, dangling, or invalid!
}

// Example
fn example() {
    let x = 42;
    let ptr: *const i32 = &x;

    // ❌ Can't dereference outside unsafe
    // let value = *ptr;  // Compile error!

    // ✅ Must use unsafe block
    let value = unsafe { *ptr };  // OK
}
```

**Why it's unsafe:**
- Pointer might be **null**
- Pointer might be **dangling** (pointing to freed memory)
- Pointer might be **unaligned**
- Pointer might **alias** mutable references (data races)

### 2. Call Unsafe Functions

```rust
// Declare unsafe function
unsafe fn dangerous_operation() {
    // Implementation
}

fn caller() {
    // ❌ Can't call without unsafe
    // dangerous_operation();  // Compile error!

    // ✅ Must acknowledge the danger
    unsafe {
        dangerous_operation();
    }
}
```

**Example: FFI (Foreign Function Interface)**

```rust
// C function declaration
extern "C" {
    fn abs(input: i32) -> i32;
}

fn call_c_function() {
    let x = -42;

    // Must use unsafe to call C code
    let result = unsafe {
        abs(x)  // C doesn't enforce Rust's safety guarantees
    };

    println!("Result: {}", result);
}
```

### 3. Access or Modify Mutable Static Variables

```rust
// Static variables are global
static mut COUNTER: u32 = 0;

fn increment() {
    // ❌ Can't access mutable static without unsafe
    // COUNTER += 1;  // Compile error!

    // ✅ Must use unsafe
    unsafe {
        COUNTER += 1;  // Data race possible if multi-threaded!
    }
}

fn read_counter() -> u32 {
    unsafe { COUNTER }
}
```

**Why it's unsafe:**
- **Data races**: Multiple threads can access simultaneously
- No synchronization guarantees
- No borrow checker protection

**Safe alternative:**

```rust
use std::sync::atomic::{AtomicU32, Ordering};

// ✅ Safe: Atomic operations
static COUNTER: AtomicU32 = AtomicU32::new(0);

fn increment() {
    COUNTER.fetch_add(1, Ordering::Relaxed);  // Safe!
}
```

### 4. Implement Unsafe Traits

```rust
// Unsafe trait: Implementer must uphold invariants
unsafe trait UnsafeTrait {
    fn method(&self);
}

struct MyType;

// Must mark implementation as unsafe
unsafe impl UnsafeTrait for MyType {
    fn method(&self) {
        // Implementation must uphold trait's safety invariants
    }
}
```

**Example: Send and Sync**

```rust
// Send: Type can be transferred between threads
// Sync: Type can be referenced from multiple threads
struct MyRawPointer(*mut i32);

// Raw pointers are not Send/Sync by default
// But if we know it's safe...
unsafe impl Send for MyRawPointer {}
unsafe impl Sync for MyRawPointer {}

// ⚠️ We're promising this is safe!
// If wrong, undefined behavior!
```

### 5. Access Fields of Union Types

```rust
// Union: Only one field is valid at a time
union MyUnion {
    i: i32,
    f: f32,
}

fn use_union() {
    let u = MyUnion { i: 42 };

    unsafe {
        // ⚠️ We must know which field is valid!
        println!("As i32: {}", u.i);
        println!("As f32: {}", u.f);  // Wrong interpretation!
    }
}
```

**Why it's unsafe:**
- Reading wrong field gives garbage data
- No type safety
- Easy to corrupt memory

---

## 4. When to Use Unsafe

### Valid Use Cases

#### ✅ 1. Performance-Critical Code

```rust
// Safe but slower
pub fn sum_safe(slice: &[i32]) -> i32 {
    slice.iter().sum()
}

// Unsafe but faster (skip bounds checks)
pub fn sum_unsafe(slice: &[i32]) -> i32 {
    let mut sum = 0;
    for i in 0..slice.len() {
        // get_unchecked skips bounds checking
        sum += unsafe { *slice.get_unchecked(i) };
    }
    sum
}

// But only worth it in hot loops with benchmarks proving it!
```

**Guideline**: Only after benchmarking proves it's a bottleneck.

#### ✅ 2. FFI (Calling C/C++ Libraries)

```rust
// Calling external C library
#[repr(C)]
struct COrder {
    symbol: [u8; 16],
    price: f64,
    quantity: f64,
}

extern "C" {
    fn process_order_c(order: *const COrder) -> i32;
}

pub fn submit_order(order: &Order) -> Result<(), Error> {
    let c_order = convert_to_c_order(order);

    let result = unsafe {
        process_order_c(&c_order)
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::SubmissionFailed)
    }
}
```

#### ✅ 3. Low-Level Abstractions

```rust
// Implementing efficient data structures
pub struct RingBuffer<T> {
    data: *mut T,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::array::<T>(capacity).unwrap();
        let data = unsafe { alloc(layout) as *mut T };

        RingBuffer {
            data,
            capacity,
            read_pos: 0,
            write_pos: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            return Err(value);
        }

        unsafe {
            self.data.add(self.write_pos).write(value);
        }

        self.write_pos = (self.write_pos + 1) % self.capacity;
        Ok(())
    }
}
```

#### ✅ 4. Zero-Copy Parsing

```rust
// Parse FIX message without allocation
#[repr(packed)]
struct FIXHeader {
    begin_string: [u8; 8],
    body_length: u32,
    msg_type: u8,
}

pub fn parse_fix_header(data: &[u8]) -> Option<&FIXHeader> {
    if data.len() < std::mem::size_of::<FIXHeader>() {
        return None;
    }

    unsafe {
        let ptr = data.as_ptr() as *const FIXHeader;
        Some(&*ptr)  // Zero-copy cast
    }
}
```

### ❌ When NOT to Use Unsafe

#### ❌ 1. Premature Optimization

```rust
// ❌ DON'T: "I think this might be faster"
pub fn bad_optimization(v: &[i32]) -> i32 {
    unsafe {
        // Unnecessary unsafe, no measurable benefit
        *v.get_unchecked(0)
    }
}

// ✅ DO: Profile first, optimize later
pub fn good_approach(v: &[i32]) -> i32 {
    v[0]  // Safe, compiler already optimizes this!
}
```

#### ❌ 2. Working Around the Borrow Checker

```rust
// ❌ DON'T: Use unsafe to bypass borrow checker
struct BadExample {
    data: Vec<i32>,
}

impl BadExample {
    fn bad_method(&self) -> &mut Vec<i32> {
        unsafe {
            // Casting away const - VERY BAD!
            &mut *(self as *const Self as *mut Self).data
        }
    }
}

// ✅ DO: Use interior mutability patterns
use std::cell::RefCell;

struct GoodExample {
    data: RefCell<Vec<i32>>,
}

impl GoodExample {
    fn good_method(&self) -> std::cell::RefMut<Vec<i32>> {
        self.data.borrow_mut()  // Safe interior mutability
    }
}
```

#### ❌ 3. Convenience

```rust
// ❌ DON'T: Use unwrap_unchecked for convenience
fn bad_parse(s: &str) -> i32 {
    unsafe {
        s.parse().unwrap_unchecked()  // Lazy!
    }
}

// ✅ DO: Handle errors properly
fn good_parse(s: &str) -> Result<i32, ParseError> {
    s.parse().map_err(|e| ParseError::from(e))
}
```

---

## 5. Safety Contracts and Invariants

### What is a Safety Contract?

A safety contract is a **documented guarantee** that unsafe code relies on.

```rust
/// Calculates sum without bounds checking.
///
/// # Safety
///
/// Caller must ensure:
/// - `slice` is not empty
/// - All indices 0..slice.len() are valid
///
/// Violating these requirements results in undefined behavior.
pub unsafe fn sum_unchecked(slice: &[i32]) -> i32 {
    let mut sum = 0;
    for i in 0..slice.len() {
        sum += *slice.get_unchecked(i);
    }
    sum
}

// Caller's responsibility to uphold contract
fn caller() {
    let data = vec![1, 2, 3, 4, 5];

    // ✅ Safe: Contract requirements met
    let sum = unsafe { sum_unchecked(&data) };

    // ❌ UNDEFINED BEHAVIOR: Empty slice violates contract
    let empty: Vec<i32> = vec![];
    let bad_sum = unsafe { sum_unchecked(&empty) };
}
```

### Documenting Safety Invariants

```rust
/// A ring buffer with unsafe unchecked access methods.
pub struct RingBuffer<T> {
    data: *mut T,
    capacity: usize,
    len: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new ring buffer with the specified capacity.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0 or allocation fails.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be positive");

        let layout = Layout::array::<T>(capacity).unwrap();
        let data = unsafe { alloc(layout) as *mut T };

        if data.is_null() {
            handle_alloc_error(layout);
        }

        RingBuffer {
            data,
            capacity,
            len: 0,
        }
    }

    /// Returns a reference to the element at the given index without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure `index < self.len()`. Calling with out-of-bounds
    /// index is undefined behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut buffer = RingBuffer::new(10);
    /// buffer.push(42);
    ///
    /// // Safe: index 0 is valid
    /// let value = unsafe { buffer.get_unchecked(0) };
    /// assert_eq!(*value, 42);
    /// ```
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        debug_assert!(index < self.len, "Index out of bounds");
        &*self.data.add(index)
    }

    /// Safe wrapper around unchecked access.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }
}
```

### Invariants to Document

Always document:

1. **Preconditions**: What must be true before calling?
2. **Postconditions**: What is guaranteed after calling?
3. **Memory validity**: Are pointers valid? Aligned?
4. **Thread safety**: Can this be called from multiple threads?
5. **Lifetime assumptions**: How long must data remain valid?

---

## 6. Common Unsafe Patterns

### Pattern 1: Unchecked Indexing

```rust
// Bounds-checked (safe but slower)
pub fn safe_sum(slice: &[i32]) -> i32 {
    let mut sum = 0;
    for i in 0..slice.len() {
        sum += slice[i];  // Bounds check on every access
    }
    sum
}

// Unchecked (unsafe but faster)
pub fn unsafe_sum(slice: &[i32]) -> i32 {
    let mut sum = 0;
    unsafe {
        for i in 0..slice.len() {
            sum += *slice.get_unchecked(i);  // No bounds check
        }
    }
    sum
}

// Benchmark results (typical):
// safe_sum:   100ns
// unsafe_sum: 85ns (15% faster)
// Only worth it in hot loops!
```

### Pattern 2: Transmute (Type Conversion)

```rust
// Convert between types with same memory layout
fn transmute_example() {
    let x: f32 = 1.0;

    // ⚠️ Dangerous: Must have same size and alignment
    let y: u32 = unsafe { std::mem::transmute(x) };

    println!("f32 bits: 0x{:08x}", y);
}

// ✅ Better: Use from_bits/to_bits
fn safe_alternative() {
    let x: f32 = 1.0;
    let y: u32 = x.to_bits();  // Safe!

    let z: f32 = f32::from_bits(y);  // Safe!
}
```

**When transmute is OK:**

```rust
#[repr(transparent)]
struct OrderId(u64);

fn safe_transmute(id: OrderId) -> u64 {
    // ✅ Safe: repr(transparent) guarantees same layout
    unsafe { std::mem::transmute(id) }
}
```

**When transmute is WRONG:**

```rust
fn dangerous_transmute() {
    let x: i32 = 42;

    // ❌ UNDEFINED BEHAVIOR: Different sizes!
    let y: i64 = unsafe { std::mem::transmute(x) };

    // ❌ UNDEFINED BEHAVIOR: Invalid enum discriminant
    let z: OrderSide = unsafe { std::mem::transmute(5u8) };
}
```

### Pattern 3: Working with Uninitialized Memory

```rust
use std::mem::MaybeUninit;

// ❌ OLD WAY: Undefined behavior
fn bad_uninit() {
    let mut buffer: [i32; 1024];  // Uninitialized!
    // Reading buffer[0] is UB!
}

// ✅ NEW WAY: Safe with MaybeUninit
fn good_uninit() {
    let mut buffer: [MaybeUninit<i32>; 1024] = MaybeUninit::uninit_array();

    // Initialize all elements
    for i in 0..buffer.len() {
        buffer[i] = MaybeUninit::new(0);
    }

    // Convert to initialized array
    let initialized: [i32; 1024] = unsafe {
        MaybeUninit::array_assume_init(buffer)
    };
}

// Real-world example: Building Vec without copying
fn build_vec_unchecked() -> Vec<i32> {
    const SIZE: usize = 1000;
    let mut vec = Vec::with_capacity(SIZE);

    unsafe {
        let ptr = vec.as_mut_ptr();

        // Initialize directly into Vec's buffer
        for i in 0..SIZE {
            ptr.add(i).write(i as i32);
        }

        // Mark as initialized
        vec.set_len(SIZE);
    }

    vec
}
```

### Pattern 4: Manual Memory Management

```rust
use std::alloc::{alloc, dealloc, Layout};

struct ManualBox<T> {
    ptr: *mut T,
}

impl<T> ManualBox<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();

        let ptr = unsafe {
            let raw = alloc(layout) as *mut T;
            if raw.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            raw.write(value);
            raw
        };

        ManualBox { ptr }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T> Drop for ManualBox<T> {
    fn drop(&mut self) {
        unsafe {
            // Run destructor
            std::ptr::drop_in_place(self.ptr);

            // Free memory
            let layout = Layout::new::<T>();
            dealloc(self.ptr as *mut u8, layout);
        }
    }
}
```

### Pattern 5: Pointer Aliasing

```rust
// ⚠️ Dangerous: Aliasing mutable pointers
fn dangerous_aliasing() {
    let mut x = 42;

    let ptr1 = &mut x as *mut i32;
    let ptr2 = &mut x as *mut i32;

    unsafe {
        *ptr1 = 1;
        *ptr2 = 2;  // Compiler might optimize this away!
        // Undefined behavior: mutable aliasing
    }
}

// ✅ Safe: Use split_at_mut for non-overlapping access
fn safe_aliasing(slice: &mut [i32]) {
    let mid = slice.len() / 2;
    let (left, right) = slice.split_at_mut(mid);

    // Now can mutate both halves safely
    left[0] = 1;
    right[0] = 2;
}
```

---

## 7. Unsafe in Trading Systems

### Example 1: Fast Order Book Updates

```rust
use std::collections::BTreeMap;

pub struct OrderBook {
    bids: BTreeMap<u64, Vec<Order>>,  // Price -> Orders
    asks: BTreeMap<u64, Vec<Order>>,
}

impl OrderBook {
    /// Fast path: Skip bounds checks in hot loop
    pub fn match_order(&mut self, incoming: &Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        let price_levels = match incoming.side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };

        // Safe wrapper ensures invariants
        for (price, orders) in price_levels.iter_mut() {
            if !incoming.can_match_price(*price) {
                break;
            }

            // Hot loop: Use unsafe for performance
            unsafe {
                self.match_against_level_unchecked(incoming, orders, &mut trades);
            }

            if incoming.is_filled() {
                break;
            }
        }

        trades
    }

    /// # Safety
    ///
    /// Caller must ensure:
    /// - orders is not empty
    /// - incoming and orders are valid references
    /// - trades has sufficient capacity or can grow
    unsafe fn match_against_level_unchecked(
        &self,
        incoming: &Order,
        orders: &mut Vec<Order>,
        trades: &mut Vec<Trade>,
    ) {
        let mut i = 0;
        while i < orders.len() && !incoming.is_filled() {
            // Skip bounds check - we know i < len
            let resting = orders.get_unchecked_mut(i);

            if let Some(trade) = self.execute_trade(incoming, resting) {
                trades.push(trade);

                if resting.is_filled() {
                    orders.swap_remove(i);  // Remove filled order
                    // Don't increment i - new order at this position
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    fn execute_trade(&self, incoming: &Order, resting: &mut Order) -> Option<Trade> {
        // Safe implementation
        Some(Trade {
            price: resting.price,
            quantity: incoming.quantity.min(resting.quantity),
        })
    }
}
```

### Example 2: Zero-Copy FIX Protocol Parser

```rust
#[repr(packed)]
struct FIXMessageHeader {
    begin_string: [u8; 8],    // "FIX.4.4\0"
    body_length: u32,
    msg_type: u8,
    sender_comp_id: [u8; 16],
    target_comp_id: [u8; 16],
    msg_seq_num: u32,
}

pub struct FIXParser {
    buffer: Vec<u8>,
}

impl FIXParser {
    pub fn parse_header(&self) -> Option<ParsedHeader> {
        if self.buffer.len() < std::mem::size_of::<FIXMessageHeader>() {
            return None;
        }

        // Zero-copy parse: Cast buffer to struct
        let header = unsafe {
            &*(self.buffer.as_ptr() as *const FIXMessageHeader)
        };

        // Validate magic number
        if &header.begin_string[..7] != b"FIX.4.4" {
            return None;
        }

        // Safe: We've validated the data
        Some(ParsedHeader {
            body_length: header.body_length,
            msg_type: header.msg_type,
            msg_seq_num: header.msg_seq_num,
        })
    }

    /// # Safety
    ///
    /// Caller must ensure buffer contains valid UTF-8 at the specified range.
    pub unsafe fn get_field_unchecked(&self, start: usize, len: usize) -> &str {
        let bytes = self.buffer.get_unchecked(start..start + len);
        std::str::from_utf8_unchecked(bytes)
    }

    /// Safe wrapper with validation
    pub fn get_field(&self, start: usize, len: usize) -> Option<&str> {
        let bytes = self.buffer.get(start..start + len)?;
        std::str::from_utf8(bytes).ok()
    }
}
```

### Example 3: Lock-Free Order ID Generator

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static ORDER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[repr(transparent)]
pub struct OrderId(u64);

impl OrderId {
    /// Generates a unique order ID (thread-safe)
    pub fn generate() -> Self {
        let id = ORDER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        OrderId(id)
    }

    /// # Safety
    ///
    /// Caller must ensure `raw_id` is a valid, previously generated ID.
    pub unsafe fn from_raw_unchecked(raw_id: u64) -> Self {
        OrderId(raw_id)
    }

    /// Safe constructor with validation
    pub fn from_raw(raw_id: u64) -> Option<Self> {
        if raw_id > 0 {
            Some(OrderId(raw_id))
        } else {
            None
        }
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}
```

### Example 4: Memory-Mapped Order Log

```rust
use std::fs::File;
use std::io;
use memmap2::MmapMut;

pub struct OrderLog {
    mmap: MmapMut,
    position: usize,
}

impl OrderLog {
    pub fn open(path: &str, size: usize) -> io::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        Ok(OrderLog {
            mmap,
            position: 0,
        })
    }

    /// Appends order to memory-mapped log
    ///
    /// # Safety
    ///
    /// This is safe because:
    /// - We maintain exclusive mutable access to mmap
    /// - position is always <= mmap.len()
    /// - We check bounds before writing
    pub fn append_order(&mut self, order: &Order) -> io::Result<()> {
        let order_size = std::mem::size_of::<Order>();

        if self.position + order_size > self.mmap.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Log full",
            ));
        }

        unsafe {
            let dst = self.mmap.as_mut_ptr().add(self.position) as *mut Order;
            dst.write(*order);
        }

        self.position += order_size;
        Ok(())
    }

    /// Read order at index without bounds check
    ///
    /// # Safety
    ///
    /// Caller must ensure `index * size_of::<Order>()` is within bounds
    /// and points to a valid Order.
    pub unsafe fn get_order_unchecked(&self, index: usize) -> &Order {
        let order_size = std::mem::size_of::<Order>();
        let offset = index * order_size;

        let ptr = self.mmap.as_ptr().add(offset) as *const Order;
        &*ptr
    }

    /// Safe wrapper with bounds checking
    pub fn get_order(&self, index: usize) -> Option<&Order> {
        let order_size = std::mem::size_of::<Order>();
        let offset = index * order_size;

        if offset + order_size <= self.position {
            Some(unsafe { self.get_order_unchecked(index) })
        } else {
            None
        }
    }
}
```

---

## 8. Writing Safe Abstractions

### Principle 1: Encapsulate Unsafe

```rust
// ❌ BAD: Exposing unsafe to users
pub struct BadVec<T> {
    pub ptr: *mut T,  // Exposed!
    pub len: usize,
    pub capacity: usize,
}

impl<T> BadVec<T> {
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        &*self.ptr.add(index)
    }
}

// ✅ GOOD: Hide unsafe internals
pub struct GoodVec<T> {
    ptr: *mut T,      // Private
    len: usize,
    capacity: usize,
}

impl<T> GoodVec<T> {
    // Public safe API
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    // Private unsafe helper
    unsafe fn get_unchecked(&self, index: usize) -> &T {
        &*self.ptr.add(index)
    }
}
```

### Principle 2: Maintain Invariants

```rust
pub struct SafeVec<T> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
}

impl<T> SafeVec<T> {
    // Invariants:
    // 1. ptr is valid for reads/writes up to capacity
    // 2. Elements 0..len are initialized
    // 3. Elements len..capacity are uninitialized
    // 4. ptr is properly aligned
    // 5. capacity is > 0 or ptr is dangling

    pub fn new() -> Self {
        SafeVec {
            ptr: std::ptr::NonNull::dangling().as_ptr(),
            len: 0,
            capacity: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.grow();
        }

        unsafe {
            // Safe: We ensured capacity above
            self.ptr.add(self.len).write(value);
        }

        self.len += 1;  // Maintain invariant 2
    }

    fn grow(&mut self) {
        let new_capacity = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };

        let new_layout = Layout::array::<T>(new_capacity).unwrap();

        let new_ptr = if self.capacity == 0 {
            unsafe { alloc(new_layout) as *mut T }
        } else {
            unsafe {
                let old_layout = Layout::array::<T>(self.capacity).unwrap();
                realloc(self.ptr as *mut u8, old_layout, new_layout.size()) as *mut T
            }
        };

        if new_ptr.is_null() {
            handle_alloc_error(new_layout);
        }

        self.ptr = new_ptr;
        self.capacity = new_capacity;
        // Invariants maintained
    }
}
```

### Principle 3: Document Safety Requirements

```rust
/// A fixed-size ring buffer with zero-copy access.
///
/// # Invariants
///
/// - `data` points to valid memory for `capacity` elements
/// - Elements `read_pos..write_pos` are initialized (wrapping)
/// - `capacity` is a power of 2 (for fast modulo with bitwise AND)
/// - `read_pos` and `write_pos` are always < capacity
pub struct RingBuffer<T> {
    data: *mut T,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a ring buffer with capacity rounded up to next power of 2.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0 or exceeds `isize::MAX`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0 && capacity <= isize::MAX as usize);

        // Round up to power of 2
        let capacity = capacity.next_power_of_two();

        let layout = Layout::array::<T>(capacity).unwrap();
        let data = unsafe { alloc(layout) as *mut T };

        if data.is_null() {
            handle_alloc_error(layout);
        }

        RingBuffer {
            data,
            capacity,
            read_pos: 0,
            write_pos: 0,
        }
    }

    /// Returns element at index without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - `index < self.len()`
    /// - The element at `index` has been initialized
    ///
    /// # Examples
    ///
    /// ```
    /// let mut buf = RingBuffer::new(4);
    /// buf.push(42);
    ///
    /// // Safe: We just pushed an element
    /// let value = unsafe { buf.get_unchecked(0) };
    /// assert_eq!(*value, 42);
    /// ```
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        debug_assert!(index < self.len());
        let actual_index = (self.read_pos + index) & (self.capacity - 1);
        &*self.data.add(actual_index)
    }

    /// Safe wrapper with bounds checking.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        (self.write_pos.wrapping_sub(self.read_pos)) & (self.capacity - 1)
    }
}
```

---

## 9. Auditing Unsafe Code

### Audit Checklist

#### Memory Safety
- [ ] No null pointer dereferences
- [ ] No use-after-free
- [ ] No double-free
- [ ] No buffer overflows/underflows
- [ ] Proper alignment for all pointer casts
- [ ] All allocations are deallocated exactly once

#### Data Race Freedom
- [ ] No unsynchronized concurrent access to shared mutable state
- [ ] Proper use of atomics or locks
- [ ] Send/Sync implementations are correct

#### Type Safety
- [ ] transmute only between same-sized types
- [ ] No invalid enum discriminants
- [ ] Union access respects active field

#### API Safety
- [ ] Safety requirements are documented
- [ ] Safe wrappers provided for all unsafe functions
- [ ] Invariants are maintained across all operations

### Example Audit

```rust
// Audit this code for safety issues
pub struct Buffer {
    data: *mut u8,
    len: usize,
    capacity: usize,
}

impl Buffer {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::array::<u8>(capacity).unwrap();
        let data = unsafe { alloc(layout) };

        Buffer {
            data,
            len: 0,
            capacity,
        }
    }

    pub unsafe fn push_unchecked(&mut self, byte: u8) {
        *self.data.add(self.len) = byte;
        self.len += 1;
    }
}

// ❌ ISSUES FOUND:
// 1. new() doesn't check for null pointer after alloc
// 2. push_unchecked can write past capacity
// 3. No Drop implementation - memory leak!
// 4. Not Send/Sync safe - concurrent access possible
```

**Fixed version:**

```rust
pub struct Buffer {
    data: *mut u8,
    len: usize,
    capacity: usize,
}

// Make non-Send/Sync explicitly (contains raw pointer)
// Only make Send/Sync if thread-safety is guaranteed
impl !Send for Buffer {}
impl !Sync for Buffer {}

impl Buffer {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::array::<u8>(capacity).unwrap();
        let data = unsafe { alloc(layout) };

        // ✅ Check for null
        if data.is_null() {
            handle_alloc_error(layout);
        }

        Buffer {
            data,
            len: 0,
            capacity,
        }
    }

    /// # Safety
    ///
    /// Caller must ensure self.len < self.capacity
    pub unsafe fn push_unchecked(&mut self, byte: u8) {
        debug_assert!(self.len < self.capacity);
        *self.data.add(self.len) = byte;
        self.len += 1;
    }

    pub fn push(&mut self, byte: u8) -> Result<(), ()> {
        if self.len >= self.capacity {
            return Err(());
        }

        unsafe {
            self.push_unchecked(byte);
        }

        Ok(())
    }
}

// ✅ Implement Drop
impl Drop for Buffer {
    fn drop(&mut self) {
        if self.capacity > 0 {
            unsafe {
                let layout = Layout::array::<u8>(self.capacity).unwrap();
                dealloc(self.data, layout);
            }
        }
    }
}
```

---

## 10. Common Pitfalls and How to Avoid Them

### Pitfall 1: Dangling Pointers

```rust
// ❌ WRONG: Returns pointer to stack variable
fn dangling_pointer() -> *const i32 {
    let x = 42;
    &x as *const i32  // x is dropped, pointer is dangling!
}

// ✅ RIGHT: Return owned value or use heap allocation
fn correct_approach() -> Box<i32> {
    Box::new(42)
}
```

### Pitfall 2: Aliasing Violations

```rust
// ❌ WRONG: Creating multiple mutable references
fn aliasing_violation() {
    let mut data = vec![1, 2, 3];
    let ptr1 = &mut data[0] as *mut i32;
    let ptr2 = &mut data[0] as *mut i32;

    unsafe {
        *ptr1 = 10;
        *ptr2 = 20;  // Undefined behavior!
    }
}

// ✅ RIGHT: Don't create overlapping mutable references
fn no_aliasing() {
    let mut data = vec![1, 2, 3];

    // Safe: Non-overlapping slices
    let (left, right) = data.split_at_mut(1);
    left[0] = 10;
    right[0] = 20;
}
```

### Pitfall 3: Uninitialized Memory

```rust
// ❌ WRONG: Reading uninitialized memory
fn read_uninit() -> i32 {
    let x: i32;  // Uninitialized!
    x  // Undefined behavior!
}

// ✅ RIGHT: Use MaybeUninit
fn safe_uninit() -> i32 {
    let mut x = MaybeUninit::<i32>::uninit();
    unsafe { x.as_mut_ptr().write(42) };
    unsafe { x.assume_init() }
}
```

### Pitfall 4: Incorrect transmute

```rust
// ❌ WRONG: transmute between different sizes
fn bad_transmute() {
    let x: i32 = 42;
    let y: i64 = unsafe { std::mem::transmute(x) };  // UB!
}

// ✅ RIGHT: Use proper conversion
fn correct_conversion() {
    let x: i32 = 42;
    let y: i64 = x as i64;  // Safe cast
}
```

### Pitfall 5: Forgetting Drop

```rust
// ❌ WRONG: Manual dealloc without running destructor
struct WrongDrop {
    data: Vec<u8>,
}

impl Drop for WrongDrop {
    fn drop(&mut self) {
        // ❌ Manually freeing without drop
        let ptr = self.data.as_ptr();
        unsafe {
            // This leaks Vec's internal allocation!
            dealloc(ptr as *mut u8, Layout::new::<Vec<u8>>());
        }
    }
}

// ✅ RIGHT: Use drop_in_place
struct CorrectDrop {
    data: *mut Vec<u8>,
}

impl Drop for CorrectDrop {
    fn drop(&mut self) {
        unsafe {
            // ✅ Run destructor first
            std::ptr::drop_in_place(self.data);

            // Then deallocate
            let layout = Layout::new::<Vec<u8>>();
            dealloc(self.data as *mut u8, layout);
        }
    }
}
```

---

## 11. Tools and Testing

### Miri - Undefined Behavior Detector

```bash
# Install Miri
rustup +nightly component add miri

# Run tests with Miri
cargo +nightly miri test
```

**What Miri catches:**
- Use-after-free
- Out-of-bounds access
- Uninitialized memory reads
- Data races
- Incorrect pointer alignment
- Invalid enum discriminants

```rust
#[test]
fn test_unsafe_code() {
    let v = vec![1, 2, 3];
    let ptr = v.as_ptr();

    drop(v);

    // Miri will catch this use-after-free!
    unsafe {
        let x = *ptr;
    }
}
```

### Address Sanitizer

```bash
# Build with AddressSanitizer
RUSTFLAGS="-Z sanitizer=address" cargo +nightly build --target x86_64-unknown-linux-gnu

# Run tests
cargo +nightly test --target x86_64-unknown-linux-gnu
```

### ThreadSanitizer

```bash
# Detect data races
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test --target x86_64-unknown-linux-gnu
```

### Fuzzing with cargo-fuzz

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Initialize fuzzing
cargo fuzz init

# Create fuzz target
cargo fuzz add parse_fix_message

# Run fuzzer
cargo fuzz run parse_fix_message
```

Example fuzz target:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz the unsafe parser
    let _ = parse_fix_message(data);
});
```

---

## Summary

### Key Differences: Rust vs C# unsafe

| Aspect | Rust | C# |
|--------|------|-----|
| **Memory Model** | No GC, manual management | GC-managed heap |
| **Safety Scope** | 5 specific operations | Pointer operations |
| **Use Frequency** | Rare, avoided | Common in low-level code |
| **Purpose** | Break borrow checker | Work with pointers |
| **Danger Level** | Very high (UB possible) | Moderate (GC still works) |

### When to Use unsafe

✅ **Use unsafe for:**
- FFI (calling C/C++ libraries)
- Performance-critical hot paths (after benchmarking!)
- Implementing low-level abstractions
- Zero-copy parsing of binary protocols

❌ **Don't use unsafe for:**
- Convenience
- Working around borrow checker without understanding
- Premature optimization
- "It's faster" without proof

### Safety Principles

1. **Keep unsafe blocks small** - minimize the blast radius
2. **Document safety contracts** - explain what must be true
3. **Provide safe wrappers** - encapsulate unsafe code
4. **Test extensively** - use Miri, sanitizers, fuzzing
5. **Audit regularly** - review all unsafe code

### Remember

> **unsafe doesn't mean "unsafe to use" - it means "I guarantee this is safe"**

You're making a promise to the compiler that you'll uphold safety invariants it can't verify. Break that promise, and you get undefined behavior.

---

## Further Reading

- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) - The Dark Arts of Unsafe Rust
- [Unsafe Code Guidelines](https://rust-lang.github.io/unsafe-code-guidelines/)
- [Miri Documentation](https://github.com/rust-lang/miri)
- [Rust API Guidelines - Unsafe Code](https://rust-lang.github.io/api-guidelines/necessities.html#unsafe-functions-have-a-safety-section-c-safety)
