# Custom Memory Allocator for Order Book

## Purpose

A **custom memory allocator** is a specialized component that manages heap memory allocation and deallocation. While Rust's default allocator (typically `jemalloc` or system malloc) is general-purpose, building a custom allocator allows you to:

1. **Optimize for specific access patterns**: Order books have predictable allocation patterns (many small, short-lived orders)
2. **Reduce fragmentation**: Keep related data together in memory
3. **Improve cache locality**: Allocate from the same memory regions for better CPU cache usage
4. **Control performance**: Eliminate non-deterministic latency spikes
5. **Learn unsafe Rust**: Understand memory layout, pointers, and lifetime guarantees at the lowest level

### Why Custom Allocators for Trading?

In high-frequency trading, every nanosecond counts:
- **Standard allocator**: 50-200ns per allocation
- **Arena allocator**: 5-10ns per allocation (10-40x faster!)
- **Slab allocator**: ~3ns per allocation for fixed-size objects

---

## Technology Stack

### Core Libraries

```toml
[dependencies]
# Memory management
libc = "0.2"              # Low-level system calls

# Optional helpers
spin = "0.9"              # Spinlocks for lock-free allocators
crossbeam-epoch = "0.9"   # Epoch-based memory reclamation

[dev-dependencies]
criterion = "0.5"         # Benchmarking
mimalloc = "0.1"          # Comparison baseline
```

### No External Allocator Libraries!

We'll build everything from scratch to learn the internals.

---

## Implementation Guide

### Phase 1: Arena Allocator (Bump Allocator)

The simplest allocator - just increment a pointer. Perfect for short-lived objects.

#### Step 1: Basic Arena

```rust
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::ptr;

pub struct Arena {
    // Raw memory region
    data: *mut u8,

    // Size of the arena
    size: usize,

    // Current allocation offset
    offset: Cell<usize>,
}

impl Arena {
    /// Create a new arena with specified size
    pub fn new(size: usize) -> Self {
        unsafe {
            // Allocate raw memory using system allocator
            let layout = Layout::from_size_align(size, 16).unwrap();
            let data = std::alloc::alloc(layout);

            if data.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            Self {
                data,
                size,
                offset: Cell::new(0),
            }
        }
    }

    /// Allocate memory from the arena
    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        // Calculate aligned offset
        let offset = self.offset.get();
        let aligned_offset = align_up(offset, layout.align());

        // Check if we have space
        let new_offset = aligned_offset + layout.size();
        if new_offset > self.size {
            return ptr::null_mut();  // Out of memory
        }

        // Bump the offset
        self.offset.set(new_offset);

        // Return pointer to allocated memory
        unsafe { self.data.add(aligned_offset) }
    }

    /// Reset the arena (deallocate everything at once)
    pub fn reset(&self) {
        self.offset.set(0);
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(self.size, 16).unwrap();
            std::alloc::dealloc(self.data, layout);
        }
    }
}

// Helper: align value up to alignment
fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

// Make it safe to share across threads (we'll add synchronization later)
unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}
```

**Key concepts:**
- **Bump allocation**: Just increment a pointer (super fast!)
- **Alignment**: CPU requires certain addresses for types (e.g., `u64` must be 8-byte aligned)
- **No individual deallocation**: Free everything at once by resetting

---

#### Step 2: Thread-Safe Arena with Atomics

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ThreadSafeArena {
    data: *mut u8,
    size: usize,
    offset: AtomicUsize,  // Atomic for thread safety
}

impl ThreadSafeArena {
    pub fn new(size: usize) -> Self {
        unsafe {
            let layout = Layout::from_size_align(size, 16).unwrap();
            let data = std::alloc::alloc(layout);

            if data.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            Self {
                data,
                size,
                offset: AtomicUsize::new(0),
            }
        }
    }

    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let offset = self.offset.load(Ordering::Relaxed);
            let aligned_offset = align_up(offset, layout.align());
            let new_offset = aligned_offset + layout.size();

            if new_offset > self.size {
                return ptr::null_mut();
            }

            // Try to atomically update offset
            if self.offset.compare_exchange(
                offset,
                new_offset,
                Ordering::Release,  // Synchronize with other threads
                Ordering::Relaxed,
            ).is_ok() {
                // Success! Return the pointer
                return unsafe { self.data.add(aligned_offset) };
            }

            // CAS failed, retry
        }
    }

    pub fn reset(&self) {
        self.offset.store(0, Ordering::Release);
    }
}

unsafe impl Send for ThreadSafeArena {}
unsafe impl Sync for ThreadSafeArena {}
```

**Atomic compare-and-swap (CAS):**
- Multiple threads try to allocate simultaneously
- Only one succeeds per iteration
- Others retry (lock-free!)

---

### Phase 2: Slab Allocator (Fixed-Size Pools)

Perfect for order books where most allocations are the same size (Order struct).

#### Step 3: Free List Implementation

```rust
use std::ptr::NonNull;

/// A slab for allocating fixed-size objects
pub struct Slab {
    // Pointer to the start of the memory region
    memory: *mut u8,

    // Size of each object
    object_size: usize,

    // Number of objects in the slab
    capacity: usize,

    // Free list (linked list of available slots)
    free_list: Cell<*mut u8>,
}

impl Slab {
    pub fn new(object_size: usize, capacity: usize) -> Self {
        unsafe {
            // Ensure object_size is at least pointer-sized (for free list)
            let object_size = object_size.max(std::mem::size_of::<*mut u8>());

            let total_size = object_size * capacity;
            let layout = Layout::from_size_align(total_size, 16).unwrap();
            let memory = std::alloc::alloc(layout);

            if memory.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            // Initialize free list
            let mut current = memory;
            for _ in 0..capacity - 1 {
                let next = current.add(object_size);
                *(current as *mut *mut u8) = next;
                current = next;
            }
            // Last element points to null
            *(current as *mut *mut u8) = ptr::null_mut();

            Self {
                memory,
                object_size,
                capacity,
                free_list: Cell::new(memory),
            }
        }
    }

    /// Allocate an object from the slab
    pub fn alloc(&self) -> *mut u8 {
        let free = self.free_list.get();

        if free.is_null() {
            return ptr::null_mut();  // Slab is full
        }

        unsafe {
            // Get the next free slot
            let next_free = *(free as *const *mut u8);
            self.free_list.set(next_free);
            free
        }
    }

    /// Deallocate an object back to the slab
    pub fn dealloc(&self, ptr: *mut u8) {
        unsafe {
            let current_free = self.free_list.get();

            // Add to front of free list
            *(ptr as *mut *mut u8) = current_free;
            self.free_list.set(ptr);
        }
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe {
            let total_size = self.object_size * self.capacity;
            let layout = Layout::from_size_align(total_size, 16).unwrap();
            std::alloc::dealloc(self.memory, layout);
        }
    }
}

unsafe impl Send for Slab {}
```

**Free list:**
- Each free slot stores a pointer to the next free slot
- Allocation = pop from free list (O(1))
- Deallocation = push to free list (O(1))
- Uses the freed memory itself for storage (no extra overhead!)

---

### Phase 3: Global Allocator Integration

#### Step 4: Implement `GlobalAlloc` Trait

```rust
use std::alloc::{GlobalAlloc, Layout};
use parking_lot::Mutex;

pub struct OrderBookAllocator {
    // Different slabs for different sizes
    small_slab: Mutex<Slab>,   // 1-64 bytes
    medium_slab: Mutex<Slab>,  // 65-256 bytes
    large_slab: Mutex<Slab>,   // 257-1024 bytes

    // Fallback to system allocator for huge allocations
}

impl OrderBookAllocator {
    pub const fn new() -> Self {
        // Note: Can't use Slab::new in const fn, so we initialize lazily
        Self {
            small_slab: Mutex::new(Slab::empty()),
            medium_slab: Mutex::new(Slab::empty()),
            large_slab: Mutex::new(Slab::empty()),
        }
    }
}

unsafe impl GlobalAlloc for OrderBookAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match layout.size() {
            0..=64 => {
                let slab = self.small_slab.lock();
                slab.alloc()
            }
            65..=256 => {
                let slab = self.medium_slab.lock();
                slab.alloc()
            }
            257..=1024 => {
                let slab = self.large_slab.lock();
                slab.alloc()
            }
            _ => {
                // Fallback to system allocator
                std::alloc::System.alloc(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match layout.size() {
            0..=64 => {
                let slab = self.small_slab.lock();
                slab.dealloc(ptr);
            }
            65..=256 => {
                let slab = self.medium_slab.lock();
                slab.dealloc(ptr);
            }
            257..=1024 => {
                let slab = self.large_slab.lock();
                slab.dealloc(ptr);
            }
            _ => {
                std::alloc::System.dealloc(ptr, layout);
            }
        }
    }
}

// Use it as the global allocator
#[global_allocator]
static GLOBAL: OrderBookAllocator = OrderBookAllocator::new();
```

**Size classes:**
- Group similar-sized allocations together
- Reduces fragmentation
- Trade-off: some internal fragmentation (64-byte request uses 64-byte slot)

---

### Phase 4: Advanced - Lock-Free Slab

#### Step 5: Lock-Free Free List with Atomics

```rust
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct LockFreeSlab {
    memory: *mut u8,
    object_size: usize,
    capacity: usize,
    free_list: AtomicPtr<u8>,  // Atomic pointer for lock-free access
}

impl LockFreeSlab {
    pub fn alloc(&self) -> *mut u8 {
        loop {
            let free = self.free_list.load(Ordering::Acquire);

            if free.is_null() {
                return ptr::null_mut();
            }

            unsafe {
                let next_free = *(free as *const *mut u8);

                // Try to atomically update free list
                if self.free_list.compare_exchange(
                    free,
                    next_free,
                    Ordering::Release,
                    Ordering::Acquire,
                ).is_ok() {
                    return free;
                }
            }

            // CAS failed, retry
        }
    }

    pub fn dealloc(&self, ptr: *mut u8) {
        loop {
            let current_free = self.free_list.load(Ordering::Acquire);

            unsafe {
                // Set this slot's next pointer to current head
                *(ptr as *mut *mut u8) = current_free;

                // Try to make this slot the new head
                if self.free_list.compare_exchange(
                    current_free,
                    ptr,
                    Ordering::Release,
                    Ordering::Acquire,
                ).is_ok() {
                    return;
                }
            }

            // CAS failed, retry
        }
    }
}
```

**ABA Problem:**
- Thread A reads `free = X`
- Thread B pops X, pops Y, pushes X back
- Thread A's CAS succeeds (X == X), but Y was lost!

**Solution:** Use epoch-based reclamation or tagged pointers.

---

### Phase 5: Benchmarking

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_allocators(c: &mut Criterion) {
    c.bench_function("system_alloc", |b| {
        b.iter(|| {
            let v = vec![0u64; 100];
            black_box(v);
        });
    });

    c.bench_function("arena_alloc", |b| {
        let arena = Arena::new(1024 * 1024);
        b.iter(|| {
            let ptr = arena.alloc(Layout::from_size_align(800, 8).unwrap());
            black_box(ptr);
        });
    });

    c.bench_function("slab_alloc", |b| {
        let slab = Slab::new(64, 1000);
        b.iter(|| {
            let ptr = slab.alloc();
            slab.dealloc(ptr);
            black_box(ptr);
        });
    });
}

criterion_group!(benches, benchmark_allocators);
criterion_main!(benches);
```

---

## Advantages

1. **Performance**
   - 10-40x faster than system allocator for specific patterns
   - Predictable latency (no system calls)
   - Better cache locality

2. **Control**
   - Customize for your workload
   - No hidden allocations or fragmentation

3. **Learning**
   - Deep understanding of memory management
   - Master unsafe Rust
   - Understand CPU architecture (cache lines, alignment)

4. **Debugging**
   - Easy to track all allocations
   - Can add instrumentation for leak detection

---

## Disadvantages

1. **Complexity**
   - Easy to introduce memory bugs (use-after-free, double-free)
   - Requires extensive testing

2. **Limited Generality**
   - Optimized for specific patterns
   - Poor performance for general-purpose use

3. **Memory Overhead**
   - Slab allocators waste memory if size classes don't match
   - Arena allocators can't free individual objects

4. **Unsafe Code**
   - Large unsafe blocks are hard to audit
   - Miri (Rust's UB detector) may not catch all bugs

---

## Limitations

1. **No Multi-Slab Growth**
   - Fixed capacity per slab
   - Need to add slab chaining for dynamic growth

2. **No Coalescing**
   - Can't merge adjacent free blocks
   - Fragmentation over time

3. **Single-Threaded Slabs**
   - Lock-free version is complex
   - Easier to use per-thread allocators

4. **No Realloc Support**
   - Can't resize allocations efficiently
   - Need to allocate-copy-free

---

## Alternatives

### 1. **jemalloc** (Facebook's Allocator)
- **Pros**: Excellent general-purpose, low fragmentation
- **Cons**: Not optimized for specific patterns
- **Use**: Production default for most apps

### 2. **mimalloc** (Microsoft)
- **Pros**: Faster than jemalloc, good security
- **Cons**: Less mature
- **Use**: Drop-in replacement for jemalloc

### 3. **tcmalloc** (Google)
- **Pros**: Thread-local caching, good for multi-threaded
- **Cons**: Can use more memory
- **Use**: High-throughput servers

### 4. **rpmalloc** (Rampant Pixels)
- **Pros**: Blazing fast, lock-free
- **Cons**: More memory overhead
- **Use**: Games, real-time systems

### 5. **bumpalo** (Rust Crate)
- **Pros**: Safe arena allocator
- **Cons**: Less flexible than custom
- **Use**: Quick prototyping

---

## When to Build Custom Allocator

**DO build custom:**
- ✅ Predictable allocation patterns
- ✅ Performance-critical hot path
- ✅ Learning exercise
- ✅ Embedded systems (no OS allocator)

**DON'T build custom:**
- ❌ General-purpose applications
- ❌ Unknown allocation patterns
- ❌ Limited unsafe Rust experience
- ❌ Time-constrained projects

---

## Recommended Learning Path

1. **Week 1**: Implement basic arena allocator
2. **Week 2**: Add alignment and safety checks
3. **Week 3**: Build slab allocator with free list
4. **Week 4**: Integrate with `GlobalAlloc`
5. **Week 5**: Add lock-free version
6. **Week 6**: Benchmark and optimize
7. **Week 7**: Add epoch-based memory reclamation

---

## Further Reading

- [Rust Nomicon - Allocators](https://doc.rust-lang.org/nomicon/vec/vec-alloc.html)
- [Writing an OS in Rust - Allocators](https://os.phil-opp.com/allocator-designs/)
- [dlmalloc Design](http://gee.cs.oswego.edu/dl/html/malloc.html)
- [jemalloc Paper](https://people.freebsd.org/~jasone/jemalloc/bsdcan2006/jemalloc.pdf)
- [Lock-Free Data Structures](https://www.1024cores.net/home/lock-free-algorithms)
