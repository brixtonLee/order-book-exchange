# Memory Layout Optimization in Rust: Deep Dive

**Understanding `#[repr(...)]` attributes and memory layout for high-performance systems**

---

## Table of Contents

1. [Why Memory Layout Matters](#1-why-memory-layout-matters)
2. [Default Rust Layout](#2-default-rust-layout)
3. [repr(C) - C-Compatible Layout](#3-reprc---c-compatible-layout)
4. [repr(Rust) - Optimized Layout](#4-reprrust---optimized-layout)
5. [repr(packed) - Remove Padding](#5-reprpacked---remove-padding)
6. [repr(align) - Control Alignment](#6-repralign---control-alignment)
7. [repr(transparent) - Zero-Cost Wrapper](#7-reprtransparent---zero-cost-wrapper)
8. [Enum Memory Layout](#8-enum-memory-layout)
9. [Practical Examples for Trading Systems](#9-practical-examples-for-trading-systems)
10. [Performance Impact](#10-performance-impact)
11. [Inspecting Memory Layout](#11-inspecting-memory-layout)
12. [Decision Matrix](#12-decision-matrix)

---

## 1. Why Memory Layout Matters

### The Three Pillars of Memory Layout

#### 1.1 Cache Performance

Modern CPUs don't access RAM directly - they use multiple levels of cache:

```
CPU Register: ~0.3ns
L1 Cache:     ~1ns   (32-64KB per core)
L2 Cache:     ~3ns   (256KB-512KB per core)
L3 Cache:     ~12ns  (8-32MB shared)
RAM:          ~100ns
```

**Cache Line Size**: Typically 64 bytes. When CPU reads one byte, it loads the entire 64-byte cache line.

```rust
// Bad: Fields scattered across multiple cache lines
struct BadOrder {
    id: u64,              // Bytes 0-7
    padding: [u8; 56],    // Bytes 8-63  (wasted!)
    price: f64,           // Bytes 64-71 (new cache line!)
    quantity: f64,        // Bytes 72-79
}

// Good: Hot fields fit in one cache line
struct GoodOrder {
    id: u64,         // Bytes 0-7
    price: f64,      // Bytes 8-15
    quantity: f64,   // Bytes 16-23
    timestamp: i64,  // Bytes 24-31
    // Total: 32 bytes, fits in half a cache line!
}
```

#### 1.2 Memory Efficiency (Padding)

CPUs require data to be **aligned** to specific boundaries:

```
Type     | Size  | Alignment
---------|-------|----------
u8/i8    | 1     | 1
u16/i16  | 2     | 2
u32/i32  | 4     | 4
u64/i64  | 8     | 8
f32      | 4     | 4
f64      | 8     | 8
*const T | 8     | 8 (on 64-bit)
```

**Alignment Rule**: A type with alignment `N` must be placed at a memory address that's a multiple of `N`.

**Example of Padding:**

```rust
struct Unoptimized {
    a: u8,    // 1 byte at offset 0
    // [padding: 7 bytes]
    b: u64,   // 8 bytes at offset 8 (must be 8-aligned!)
    c: u8,    // 1 byte at offset 16
    // [padding: 7 bytes to make struct size multiple of 8]
}
// Total size: 24 bytes (only 10 bytes of actual data!)
```

Visual representation:
```
Memory:  [a][_][_][_][_][_][_][_][b b b b b b b b][c][_][_][_][_][_][_][_]
Bytes:    0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23
          ^--- padding --->         ^-- data -->     ^----- padding ----->
```

#### 1.3 Predictability (for FFI and serialization)

```rust
// Unpredictable: Rust can reorder fields
struct RustLayout {
    a: u8,
    b: u64,
    c: u16,
}
// Rust might reorder to: b, c, a (to minimize padding)

// Predictable: #[repr(C)] guarantees order
#[repr(C)]
struct CLayout {
    a: u8,    // Always at offset 0
    b: u64,   // Always at offset 8
    c: u16,   // Always at offset 16
}
```

---

## 2. Default Rust Layout

### repr(Rust) - The Default

By default, Rust uses `repr(Rust)` which gives the compiler freedom to:
1. **Reorder fields** for optimal packing
2. **Add padding** for alignment
3. **Change layout** between compiler versions

```rust
// Default Rust representation
struct Order {
    id: u64,           // 8 bytes, align 8
    price: f64,        // 8 bytes, align 8
    quantity: f32,     // 4 bytes, align 4
    side: OrderSide,   // 1 byte, align 1
    filled: bool,      // 1 byte, align 1
}

// Rust might reorder to:
// [id: u64][price: f64][quantity: f32][side: u8][filled: bool][padding: 2]
// Total: 32 bytes with optimal packing
```

### How Rust Optimizes

Rust's default algorithm (simplified):
1. Sort fields by alignment (largest first)
2. Sort fields of same alignment by size (largest first)
3. Add padding as needed

```rust
struct Example {
    a: u8,     // align 1, size 1
    b: u64,    // align 8, size 8
    c: u16,    // align 2, size 2
    d: u32,    // align 4, size 4
}

// Rust reorders to:
// [b: u64][d: u32][c: u16][a: u8][padding: 1]
// Offsets: 0      8       12      14      15
// Total: 16 bytes (vs 24 bytes if unoptimized!)
```

### When Default is Best

✅ Use default `repr(Rust)` when:
- Internal Rust-only structs
- No FFI requirements
- No serialization format requirements
- Want compiler to optimize automatically

---

## 3. repr(C) - C-Compatible Layout

### What repr(C) Does

```rust
#[repr(C)]
struct Order {
    id: u64,       // Offset 0, always first
    price: f64,    // Offset 8, always second
    side: u8,      // Offset 16, always third
    // Padding added to align next field or struct end
}
```

**Guarantees:**
1. Fields are laid out in **declaration order**
2. Padding follows **C struct rules**
3. Layout is **stable** across compiler versions
4. Compatible with C/C++ structs

### C Padding Rules

```rust
#[repr(C)]
struct COrder {
    a: u8,     // Offset 0 (align 1)
    // [padding: 7 bytes to align next field]
    b: u64,    // Offset 8 (align 8)
    c: u16,    // Offset 16 (align 2)
    // [padding: 6 bytes to make total size multiple of largest alignment (8)]
}
// Size: 24 bytes
// Alignment: 8 (largest field alignment)
```

Visual layout:
```
Offset:  0   1-7    8-15         16-17  18-23
Field:  [a][pad][   b   ][  c  ][pad]
Bytes:   1   7      8       2      6
```

### Calculating Size and Padding

**Algorithm:**
1. Each field is placed at the next available offset that satisfies its alignment
2. Padding is inserted before the field if needed
3. Final padding is added to make struct size a multiple of its alignment

```rust
#[repr(C)]
struct Example {
    a: u8,     // Offset 0, size 1, align 1
    b: u32,    // Offset 4 (0+1 rounded up to 4), size 4, align 4
    c: u16,    // Offset 8, size 2, align 2
}
// Struct alignment = max(1, 4, 2) = 4
// Current size after 'c' = 10
// Round up to multiple of 4 = 12
// Total size: 12 bytes
```

### When to Use repr(C)

✅ **Use repr(C) when:**
- Calling C/C++ functions (FFI)
- Memory-mapped I/O
- Binary protocol parsing
- Interop with other languages
- Serialization with fixed layout requirements

```rust
// Example: Calling C library
#[repr(C)]
struct COrder {
    symbol: [u8; 16],
    price: f64,
    quantity: f64,
}

extern "C" {
    fn process_order(order: *const COrder) -> i32;
}

fn submit_order() {
    let order = COrder {
        symbol: *b"XAUUSD\0\0\0\0\0\0\0\0\0\0",
        price: 2000.0,
        quantity: 10.0,
    };
    unsafe { process_order(&order) };
}
```

---

## 4. repr(Rust) - Optimized Layout

### Explicit repr(Rust)

```rust
#[repr(Rust)]  // Usually implicit, but can be explicit
struct OptimizedOrder {
    side: u8,      // Declared first
    quantity: f32,
    price: f64,
    id: u64,
}

// Rust may reorder to:
// [id: u64][price: f64][quantity: f32][side: u8][padding: 3]
// Better packing than declaration order!
```

### Zero-Sized Types (ZST)

Rust optimizes zero-sized types completely:

```rust
struct Empty;  // ZST, size = 0

struct WithZST {
    data: u64,
    marker: Empty,  // Takes no space!
}
// Size: 8 bytes (not 9!)

// Useful for type-level programming
use std::marker::PhantomData;

struct TypedId<T> {
    id: u64,
    _phantom: PhantomData<T>,  // Zero size!
}
// Size: 8 bytes regardless of T
```

---

## 5. repr(packed) - Remove Padding

### What packed Does

```rust
#[repr(packed)]
struct PackedOrder {
    a: u8,    // Offset 0
    b: u64,   // Offset 1 (no padding!)
    c: u16,   // Offset 9 (no padding!)
}
// Size: 11 bytes (no padding at all)
```

Visual comparison:
```
Normal (#[repr(C)]):
[a][pad x7][  b (8 bytes)  ][  c  ][pad x6]
 1    7           8             2       6     = 24 bytes

Packed (#[repr(packed)]):
[a][  b (8 bytes)  ][  c  ]
 1         8            2                     = 11 bytes
```

### The Danger of Misaligned Access

**⚠️ WARNING**: Packed structs can cause **undefined behavior** or **performance penalties**!

```rust
#[repr(packed)]
struct Packed {
    a: u8,
    b: u64,  // Misaligned! At offset 1, not 8-aligned
}

fn dangerous() {
    let p = Packed { a: 1, b: 42 };

    // ❌ UNDEFINED BEHAVIOR: Taking reference to misaligned field
    // let ptr = &p.b;  // Compile error!

    // ✅ Safe: Copy the value
    let value = p.b;  // OK, copies the value

    // ✅ Safe: Use ptr::addr_of
    let ptr = std::ptr::addr_of!(p.b);
    let value = unsafe { ptr.read_unaligned() };
}
```

### Why Misalignment is Bad

**Performance**: Modern CPUs are optimized for aligned access:
- **Aligned access**: 1 CPU cycle
- **Misaligned access**: Multiple cycles, possibly crosses cache lines
- **Some architectures**: Crash on misaligned access (ARM, SPARC)

```rust
// Example: Reading u64 at misaligned address
// Aligned (offset 0, 8, 16...):
//   [--------- u64 --------]
//   Cache Line 1
//   Fast: Single load instruction

// Misaligned (offset 1, 9, 17...):
//   [u8][---- u64 ---][u8]
//   Cache Line 1 | Cache Line 2
//   Slow: Two loads + bit shifting + masking
```

### When to Use packed

✅ **Use packed when:**
- Binary protocol requires exact layout (no padding)
- Extreme memory constraints
- You KNOW what you're doing

⚠️ **Restrictions:**
- Can't take references to fields (use `ptr::addr_of!`)
- Must use `read_unaligned()` / `write_unaligned()`
- Performance penalty on access

```rust
// Binary protocol example
#[repr(packed)]
struct FIXMessageHeader {
    begin_string: [u8; 8],    // FIX.4.4
    body_length: u32,          // Message length
    msg_type: u8,              // Message type
}
// Total: 13 bytes exactly, no padding

fn parse_header(data: &[u8]) -> FIXMessageHeader {
    unsafe {
        std::ptr::read_unaligned(data.as_ptr() as *const FIXMessageHeader)
    }
}
```

### packed with Alignment

```rust
// Pack but maintain specific alignment
#[repr(packed(4))]  // Align to 4 bytes, remove other padding
struct SemiPacked {
    a: u8,    // Offset 0
    b: u64,   // Offset 4 (4-aligned, but not 8-aligned!)
    c: u16,   // Offset 12
}
// Size: 14 bytes (vs 11 for fully packed, 24 for normal)
```

---

## 6. repr(align) - Control Alignment

### Force Alignment

```rust
#[repr(align(64))]  // Align to 64 bytes (cache line)
struct CacheAligned {
    data: u64,
}
// Size: 64 bytes (padded from 8)
// Alignment: 64

// Useful for avoiding false sharing
#[repr(align(64))]
struct Counter {
    value: AtomicU64,  // 8 bytes
    // Padding to 64 bytes prevents false sharing
}
```

### False Sharing Problem

**False Sharing**: When multiple threads access different variables on the same cache line:

```rust
// ❌ BAD: False sharing
struct Counters {
    thread1_count: AtomicU64,  // Offset 0
    thread2_count: AtomicU64,  // Offset 8 (same cache line!)
}
// Both counters in same 64-byte cache line
// Every write invalidates other thread's cache!

// ✅ GOOD: Separate cache lines
#[repr(align(64))]
struct AlignedCounter {
    count: AtomicU64,
}

struct SeparatedCounters {
    thread1_count: AlignedCounter,  // Offset 0-63
    thread2_count: AlignedCounter,  // Offset 64-127
}
// Each counter on its own cache line
// No false sharing!
```

Performance impact:
```
False Sharing:    ~50ns per operation (cache invalidation)
Separated:        ~5ns per operation (10x faster!)
```

### When to Use align

✅ **Use align when:**
- Avoiding false sharing between threads
- SIMD alignment requirements (16, 32 bytes)
- Hardware requirements (DMA buffers)

```rust
// SIMD example
#[repr(align(32))]  // AVX requires 32-byte alignment
struct SIMDBuffer {
    data: [f32; 8],
}

// DMA buffer example
#[repr(align(4096))]  // Page-aligned for DMA
struct DMABuffer {
    data: [u8; 4096],
}
```

---

## 7. repr(transparent) - Zero-Cost Wrapper

### What transparent Does

```rust
#[repr(transparent)]
struct OrderId(u64);
// Same memory layout as u64
// Can be transmuted between OrderId and u64

// Valid:
let id = OrderId(42);
let raw: u64 = unsafe { std::mem::transmute(id) };  // Safe!
```

### Requirements

Must have exactly **one non-zero-sized field**:

```rust
// ✅ Valid
#[repr(transparent)]
struct Wrapper(u64);

// ✅ Valid - PhantomData is zero-sized
#[repr(transparent)]
struct TypedWrapper<T> {
    value: u64,
    _marker: PhantomData<T>,
}

// ❌ Invalid - two non-zero-sized fields
#[repr(transparent)]
struct Invalid {
    a: u64,
    b: u32,  // Error!
}
```

### Use Cases

**Type-safe FFI wrappers:**

```rust
#[repr(transparent)]
struct FileDescriptor(i32);

extern "C" {
    fn close(fd: i32) -> i32;
}

impl FileDescriptor {
    fn close(self) {
        unsafe { close(self.0) };
    }
}
// Can pass FileDescriptor directly to C
```

**NewType pattern without overhead:**

```rust
#[repr(transparent)]
struct Seconds(f64);

#[repr(transparent)]
struct Meters(f64);

// Type safety with zero runtime cost
fn calculate_speed(distance: Meters, time: Seconds) -> f64 {
    distance.0 / time.0
}
```

---

## 8. Enum Memory Layout

### Default Enum Layout

```rust
enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
}
// Size: 1 byte (discriminant only)

enum OrderType {
    Market,
    Limit { price: f64 },
    Stop { stop_price: f64, limit_price: f64 },
}
// Size: 24 bytes
// Layout: [discriminant: 8][largest variant: 16]
```

Visual layout of `OrderType`:
```
Memory: [discriminant][---- variant data ----]
Bytes:       8               16              = 24 total

Market:     [0][unused........................]
Limit:      [1][   price   ][unused........]
Stop:       [2][stop_price][limit_price]
```

### repr(C) for Enums

```rust
#[repr(C)]
enum CEnum {
    Variant1,
    Variant2(u32),
}
// C-compatible tag + union layout
```

### repr(u8/u16/u32) - Explicit Discriminant Size

```rust
#[repr(u8)]
enum CompactStatus {
    Pending = 0,
    Filled = 1,
    Cancelled = 2,
}
// Size: 1 byte (explicit discriminant size)

#[repr(u32)]
enum LargeEnum {
    A = 0,
    B = 1000000,
}
// Size: 4 bytes
```

### Optimization: Niche Optimization

Rust can use "niche" values to eliminate discriminant:

```rust
enum Option<T> {
    None,
    Some(T),
}

// For Option<&T>:
// Size: 8 bytes (same as &T!)
// None is represented as null pointer
// No separate discriminant needed!

// Similar optimization for:
Option<Box<T>>       // Uses null pointer
Option<NonZeroU32>   // Uses 0 as None
```

---

## 9. Practical Examples for Trading Systems

### Example 1: Order Book Optimization

```rust
// ❌ BAD: Poor layout (48 bytes)
struct BadOrder {
    id: OrderId,              // 16 bytes (String internally)
    price: Decimal,           // 16 bytes
    quantity: Decimal,        // 16 bytes
    side: OrderSide,          // 1 byte
    timestamp: i64,           // 8 bytes
    // Padding to align struct
}

// ✅ GOOD: Optimized layout (32 bytes)
#[repr(C)]
struct GoodOrder {
    // Hot path fields first (most frequently accessed)
    price: u64,               // 8 bytes (fixed-point instead of Decimal)
    quantity: u32,            // 4 bytes (sufficient for most orders)
    timestamp: u32,           // 4 bytes (seconds since epoch)
    id: u64,                  // 8 bytes (numeric ID)
    side: u8,                 // 1 byte
    status: u8,               // 1 byte
    _padding: [u8; 6],        // Explicit padding for clarity
}
// Total: 32 bytes, fits in half a cache line!
```

### Example 2: Cache-Aligned Trading Engine State

```rust
// Separate cache lines for multi-threaded components
#[repr(align(64))]
struct OrderBookState {
    bid_count: AtomicU32,
    ask_count: AtomicU32,
    last_trade_price: AtomicU64,
    // Padding to 64 bytes
    _pad: [u8; 52],
}

#[repr(align(64))]
struct MatchingEngineState {
    matched_orders: AtomicU64,
    total_volume: AtomicU64,
    // Padding to 64 bytes
    _pad: [u8; 48],
}

struct TradingEngine {
    order_book: OrderBookState,    // Cache line 0
    matching: MatchingEngineState, // Cache line 1
}
// No false sharing between components!
```

### Example 3: Binary Protocol Message

```rust
// FIX protocol header
#[repr(packed)]
#[derive(Copy, Clone)]
struct FIXHeader {
    begin_string: [u8; 8],  // "FIX.4.4\0"
    body_length: u32,        // Message body length
    msg_type: u8,            // 'D' for order, etc.
    sender_comp_id: [u8; 8], // Sender ID
    target_comp_id: [u8; 8], // Target ID
    msg_seq_num: u32,        // Sequence number
}
// Total: 33 bytes exactly, no padding

fn parse_fix_message(data: &[u8]) -> Option<FIXHeader> {
    if data.len() < std::mem::size_of::<FIXHeader>() {
        return None;
    }

    Some(unsafe {
        std::ptr::read_unaligned(data.as_ptr() as *const FIXHeader)
    })
}
```

### Example 4: High-Frequency Tick Data

```rust
// Store millions of ticks efficiently
#[repr(C, packed(4))]
struct CompactTick {
    timestamp_us: u64,   // Microseconds since epoch (8 bytes)
    price: u32,          // Fixed-point price (4 bytes)
    volume: u32,         // Volume (4 bytes)
    flags: u8,           // Bid/ask + other flags (1 byte)
}
// Total: 17 bytes per tick

// 1 million ticks:
// - Unoptimized: ~64 MB
// - Optimized:   ~17 MB (3.7x smaller!)
// - Better cache utilization
// - Faster iteration

fn process_ticks(ticks: &[CompactTick]) {
    // Entire dataset more likely to fit in cache
    for tick in ticks {
        // Fast sequential access
        let price = tick.price;
        let volume = tick.volume;
        // Process...
    }
}
```

### Example 5: Zero-Copy Order ID

```rust
// Type-safe order ID with zero overhead
#[repr(transparent)]
struct OrderId(u64);

impl OrderId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}

// Can be used in FFI without conversion
extern "C" {
    fn cancel_order_c(order_id: u64) -> i32;
}

fn cancel_order(id: OrderId) -> i32 {
    unsafe {
        // Can pass directly - same layout as u64
        cancel_order_c(std::mem::transmute(id))
    }
}
```

---

## 10. Performance Impact

### Benchmark: Different Layouts

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Poorly laid out struct
struct BadStruct {
    a: u8,
    b: u64,
    c: u8,
    d: u64,
}

// Well laid out struct
#[repr(C)]
struct GoodStruct {
    b: u64,
    d: u64,
    a: u8,
    c: u8,
}

fn benchmark_access(c: &mut Criterion) {
    let bad = vec![BadStruct { a: 1, b: 2, c: 3, d: 4 }; 10000];
    let good = vec![GoodStruct { a: 1, b: 2, c: 3, d: 4 }; 10000];

    c.bench_function("bad_layout", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for s in &bad {
                sum += s.b + s.d;
            }
            black_box(sum)
        })
    });

    c.bench_function("good_layout", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for s in &good {
                sum += s.b + s.d;
            }
            black_box(sum)
        })
    });
}

// Results (typical):
// bad_layout:  15.2 µs
// good_layout: 12.1 µs
// 25% faster due to better cache utilization!
```

### Cache Miss Impact

```rust
// Benchmark: Cache aligned vs not
#[repr(align(64))]
struct CacheAligned {
    counter: AtomicU64,
}

struct NotAligned {
    counter: AtomicU64,
}

// In multi-threaded scenario:
// NotAligned:    ~50ns per increment (false sharing)
// CacheAligned:  ~5ns per increment (no false sharing)
// 10x performance difference!
```

---

## 11. Inspecting Memory Layout

### Using std::mem

```rust
use std::mem;

struct Order {
    id: u64,
    price: f64,
    quantity: f32,
    side: u8,
}

fn inspect_layout() {
    println!("Size: {}", mem::size_of::<Order>());
    println!("Alignment: {}", mem::align_of::<Order>());

    // Field offsets (requires nightly)
    #[cfg(feature = "offset_of")]
    {
        println!("id offset: {}", mem::offset_of!(Order, id));
        println!("price offset: {}", mem::offset_of!(Order, price));
    }
}

// Output might be:
// Size: 24
// Alignment: 8
```

### Using memoffset Crate

```rust
use memoffset::offset_of;

#[repr(C)]
struct Order {
    id: u64,       // Offset 0
    price: f64,    // Offset 8
    side: u8,      // Offset 16
}

fn main() {
    assert_eq!(offset_of!(Order, id), 0);
    assert_eq!(offset_of!(Order, price), 8);
    assert_eq!(offset_of!(Order, side), 16);

    println!("Order size: {}", std::mem::size_of::<Order>());
    // Output: Order size: 24 (16 + 1 + 7 padding)
}
```

### Compiler Explorer (godbolt.org)

View actual assembly to see memory layout:

```rust
#[repr(C)]
pub struct Order {
    pub id: u64,
    pub price: f64,
}

pub fn get_price(order: &Order) -> f64 {
    order.price
}

// Assembly shows:
// movsd   xmm0, qword ptr [rdi + 8]
//                              ^^^ offset 8
```

### dbg_hex Macro

```rust
macro_rules! print_hex {
    ($val:expr) => {{
        let ptr = &$val as *const _ as *const u8;
        let size = std::mem::size_of_val(&$val);
        let bytes = unsafe { std::slice::from_raw_parts(ptr, size) };
        println!("{}: {:02x?}", stringify!($val), bytes);
    }};
}

fn main() {
    #[repr(C)]
    struct Example {
        a: u8,
        b: u16,
    }

    let ex = Example { a: 0x42, b: 0x1234 };
    print_hex!(ex);
    // Output: ex: [42, 00, 34, 12]
    //              ^a  pad  ^b (little-endian)
}
```

---

## 12. Decision Matrix

### When to Use Each repr

| repr | Use When | Avoid When | Size | Predictable |
|------|----------|------------|------|-------------|
| **Rust** (default) | Internal Rust structs, optimal packing | FFI, fixed layout needed | Optimized | No |
| **C** | FFI, binary protocols, stable layout | Pure Rust, optimal packing priority | Predictable | Yes |
| **packed** | Binary protocols, extreme memory savings | Hot path access, needs references | Minimal | Yes |
| **packed(N)** | Semi-optimized packing with some alignment | Fully aligned or fully packed | Moderate | Yes |
| **align(N)** | False sharing prevention, SIMD, hardware | Memory constrained | Larger | Yes |
| **transparent** | Zero-cost wrappers, FFI | Multiple fields | Same as inner | Yes |

### Quick Decision Flow

```
┌─────────────────────────────────┐
│  Need to call C/C++ code?       │
│  or fixed binary layout?        │
└────────┬────────────────────────┘
         │ Yes
         ├─────────────────> Use #[repr(C)]
         │
         │ No
         ▼
┌─────────────────────────────────┐
│  Need zero-cost wrapper?        │
└────────┬────────────────────────┘
         │ Yes
         ├─────────────────> Use #[repr(transparent)]
         │
         │ No
         ▼
┌─────────────────────────────────┐
│  Multi-threaded hot path with   │
│  separate counters?              │
└────────┬────────────────────────┘
         │ Yes
         ├─────────────────> Use #[repr(align(64))]
         │
         │ No
         ▼
┌─────────────────────────────────┐
│  Parsing binary protocol?       │
│  Need exact byte layout?        │
└────────┬────────────────────────┘
         │ Yes
         ├─────────────────> Use #[repr(packed)]
         │                    ⚠️ Warning: Performance cost!
         │ No
         ▼
┌─────────────────────────────────┐
│  Use default (let Rust optimize)│
└─────────────────────────────────┘
```

### Checklist for Production Code

```markdown
## Memory Layout Checklist

### Hot Path Structs (accessed frequently)
- [ ] Group frequently accessed fields together
- [ ] Order fields by size (largest first) or use repr(C)
- [ ] Ensure hot fields fit in one cache line (64 bytes)
- [ ] Avoid padding in critical structs

### Multi-threaded State
- [ ] Use #[repr(align(64))] for atomic counters
- [ ] Separate thread-local data to avoid false sharing
- [ ] Profile for cache misses

### FFI Structs
- [ ] Always use #[repr(C)]
- [ ] Match exact C struct layout
- [ ] Test with C code before production

### Binary Protocols
- [ ] Use #[repr(packed)] if protocol requires it
- [ ] Use ptr::addr_of! instead of references
- [ ] Document alignment requirements
- [ ] Test on target architecture

### NewType Wrappers
- [ ] Use #[repr(transparent)] for zero-cost abstraction
- [ ] Verify transmute safety if used

### Memory Constrained
- [ ] Measure actual size with std::mem::size_of
- [ ] Consider packed representations
- [ ] Profile performance impact
- [ ] Document trade-offs
```

---

## Summary

### Key Takeaways

1. **Default Rust layout** optimizes automatically but is unpredictable
2. **repr(C)** provides stability and FFI compatibility at cost of potential padding
3. **repr(packed)** eliminates padding but causes performance penalties and safety issues
4. **repr(align)** prevents false sharing but increases memory usage
5. **repr(transparent)** enables zero-cost wrappers

### Performance Rules of Thumb

- **Cache line = 64 bytes**: Keep hot data together
- **Alignment matters**: Misaligned access is slow (or crashes!)
- **Padding is not free**: Wasted memory and cache pressure
- **False sharing kills performance**: Separate atomic counters by cache line

### When in Doubt

1. **Start with default (repr(Rust))** - let compiler optimize
2. **Measure before optimizing** - use benchmarks
3. **repr(C) for FFI** - always, no exceptions
4. **Avoid packed** unless you have a very good reason
5. **Use align(64)** for thread-local atomics

### Tools for Verification

- `std::mem::size_of<T>()` - Check struct size
- `std::mem::align_of<T>()` - Check alignment
- `memoffset::offset_of!` - Check field offsets
- Compiler Explorer - View actual memory layout
- `cargo bench` - Measure performance impact

---

## Further Reading

- [Rust Reference: Type Layout](https://doc.rust-lang.org/reference/type-layout.html)
- [Compiler Explorer](https://godbolt.org/)
- [What Every Programmer Should Know About Memory](https://people.freebsd.org/~lstewart/articles/cpumemory.pdf)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
