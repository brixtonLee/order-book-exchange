# Elite HFT Firms: What Jane Street & Jump Trading Do

> **A microsecond-level breakdown of high-frequency trading and what it takes to impress the world's top trading firms.**

---

## Table of Contents

1. [The Microsecond Timeline](#the-microsecond-timeline)
2. [What They Actually Trade](#what-they-actually-trade)
3. [The Technology Stack](#the-technology-stack)
4. [What Would Actually Impress Them](#what-would-actually-impress-them)
5. [Concrete Projects to Build](#concrete-projects-to-build)
6. [The Interview Bar](#the-interview-bar)

---

## The Microsecond Timeline

### A Complete Trade at Jane Street (5-50μs Total)

```
┌─────────────────────────────────────────────────────────────────────────┐
│           A Trade Lifecycle at Jane Street (Total: ~5-50μs)              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  0μs ────── Market data arrives at NIC (Network Interface Card)         │
│             • DPDK bypasses kernel (kernel adds 10-20μs!)                │
│             • Hardware timestamp captured                                │
│             • Interrupt coalescing disabled                              │
│                                                                          │
│  0.5μs ──── NIC DMA to userspace buffer (zero-copy)                     │
│             • Packet in ring buffer                                      │
│             • No system calls, no context switches                       │
│             • Memory pre-allocated (no page faults)                      │
│                                                                          │
│  1μs ────── FIX/SBE message parsing                                     │
│             • Custom binary protocol (NOT JSON!)                         │
│             • Zero-allocation deserialization                            │
│             • SIMD string parsing (4-8 bytes at once)                    │
│             • Perfect hash table for field lookup                        │
│                                                                          │
│  2μs ────── Risk checks (pre-trade)                                     │
│             • Position limits: lock-free read from shared memory         │
│             • Credit check: atomic compare-and-swap                      │
│             • Symbol validation: perfect hash table lookup               │
│             • NO database queries (all in-memory)                        │
│                                                                          │
│  3μs ────── Strategy decision                                           │
│             • Statistical model inference (pre-computed)                 │
│             • No branching (branchless programming)                      │
│             • All data in L1/L2 cache (~4 cycles = 1-2ns)               │
│             • Lookup tables instead of if/else                           │
│                                                                          │
│  4μs ────── Order generation                                            │
│             • Object pool (pre-allocated)                                │
│             • Stack allocation only (NO heap!)                           │
│             • Serialize to binary (bytemuck or custom)                   │
│             • Checksum calculated in hardware                            │
│                                                                          │
│  5μs ────── Send to exchange                                            │
│             • Kernel bypass (DPDK/Solarflare/Mellanox)                   │
│             • Direct NIC queue write                                     │
│             • Hardware checksum offload                                  │
│             • Hardware TX timestamp                                      │
│                                                                          │
│  ════════════════════════════════════════════════════════════════       │
│                                                                          │
│  Network propagation to exchange: 50-500μs (speed of light limit!)      │
│  - NYC to Chicago: 6.5ms round trip (fiber)                             │
│  - NYC to Chicago: 4.0ms round trip (microwave)                         │
│  - Co-located in same DC: 50-200μs                                      │
│                                                                          │
│  TOTAL PROCESSING TIME: 5μs                                             │
│                                                                          │
│  Targets:                                                                │
│  • Jane Street: 3-5μs                                                   │
│  • Jump Trading: 1-3μs                                                  │
│  • Citadel Securities: <1μs                                             │
│  • Tower Research: <500ns (FPGA-assisted)                               │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## What They Actually Trade

### 1. Market Making (Jane Street's Core Business)

**Definition:** Continuously post buy and sell quotes, capturing the spread.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Market Making Explained                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Current AAPL Market:                                                   │
│    Best Bid: $150.00 (from Citadel)                                    │
│    Best Ask: $150.05 (from Virtu)                                      │
│    Spread: 5 cents                                                      │
│                                                                          │
│  Jane Street Posts (100μs later):                                       │
│    Bid: $150.01 for 1,000 shares  ← Better than Citadel                │
│    Ask: $150.04 for 1,000 shares  ← Better than Virtu                  │
│    New Spread: 3 cents                                                  │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ If both orders fill:                                           │     │
│  │   Buy 1000 @ $150.01  = -$150,010                             │     │
│  │   Sell 1000 @ $150.04 = +$150,040                             │     │
│  │   Profit: $30 (3¢ per share)                                   │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                                                                          │
│  The Challenge:                                                          │
│    • Must update quotes 10,000+ times per second                        │
│    • Across 1,000s of symbols simultaneously                            │
│    • Adjust for inventory (don't accumulate too much AAPL)              │
│    • React to volatility (widen spread when risky)                      │
│    • Detect adverse selection (informed traders picking you off)        │
│                                                                          │
│  Being 1μs slower = Citadel's quote appears first = You lose           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Inventory Management Example:**

```rust
pub struct MarketMaker {
    inventory: HashMap<String, Decimal>,  // symbol -> position
    target_inventory: Decimal,             // Ideal position (usually 0)
    max_inventory: Decimal,                // Risk limit
}

impl MarketMaker {
    /// Adjust quotes based on inventory
    pub fn calculate_quotes(&self, symbol: &str, mid_price: Decimal) -> Quotes {
        let inventory = self.inventory.get(symbol).copied().unwrap_or(Decimal::ZERO);
        let base_spread = self.calculate_spread(symbol);

        // Inventory skew: penalize the side that increases inventory
        let skew = (inventory / self.max_inventory) * dec!(0.01); // 1bp per 100% inventory

        Quotes {
            bid: mid_price - base_spread / dec!(2) - skew,  // Lower bid if long
            ask: mid_price + base_spread / dec!(2) + skew,  // Raise ask if long
            bid_size: self.calculate_size(inventory, OrderSide::Buy),
            ask_size: self.calculate_size(inventory, OrderSide::Sell),
        }
    }

    fn calculate_size(&self, inventory: Decimal, side: OrderSide) -> Decimal {
        let base_size = dec!(1000);

        match side {
            OrderSide::Buy if inventory > self.target_inventory => {
                // Already long, reduce buy size
                base_size * dec!(0.5)
            }
            OrderSide::Sell if inventory < self.target_inventory => {
                // Already short, reduce sell size
                base_size * dec!(0.5)
            }
            _ => base_size
        }
    }
}
```

**Annual Volume:** Jane Street trades $1+ trillion per year, making ~0.01% per trade = $100M+ profit.

---

### 2. Latency Arbitrage (Jump Trading's Specialty)

**Definition:** Exploit speed advantage to trade on stale prices before they update.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Latency Arbitrage Timeline                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Time 0μs:   Trade on NYSE: AAPL = $150.00 → $150.05                   │
│              ┌────────────────────────────────────────┐                  │
│              │ Jump Trading sees this FIRST (fastest  │                  │
│              │ network, co-located server)            │                  │
│              └────────────────────────────────────────┘                  │
│                                                                          │
│  Time 5μs:   Jump generates buy order for BATS exchange                │
│              • BATS still shows $150.02 ask (stale)                     │
│              • Jump knows it will update to $150.05                     │
│                                                                          │
│  Time 50μs:  Jump's order arrives at BATS                               │
│              • Buys 10,000 shares @ $150.02                             │
│              • Cost: $1,500,200                                         │
│                                                                          │
│  Time 100μs: BATS receives NYSE feed update                             │
│              • Best ask updates to $150.05                              │
│              • Jump already bought at $150.02!                          │
│                                                                          │
│  Time 150μs: Jump sells on NYSE at $150.05                              │
│              • Revenue: $1,500,500                                      │
│              • Profit: $300 (3¢ per share)                              │
│                                                                          │
│  ════════════════════════════════════════════════════════════════       │
│                                                                          │
│  If competitor is 10μs slower:                                          │
│    • Jump's order already filled at $150.02                             │
│    • Competitor arrives, price already $150.05                          │
│    • Competitor gets 0% fill rate                                       │
│                                                                          │
│  This is why EVERY MICROSECOND matters!                                 │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why Microwave Networks:**

```
NYC to Chicago distance: ~710 miles

Fiber optic (standard):
  • Speed: ~124,000 miles/second (2/3 speed of light in glass)
  • Latency: 710 / 124,000 = 5.7ms one-way
  • Round trip: 11.4ms

Microwave (line-of-sight):
  • Speed: ~186,000 miles/second (speed of light in air)
  • Latency: 710 / 186,000 = 3.8ms one-way
  • Round trip: 7.6ms

Advantage: 3.8ms faster = worth $100M+ to build towers
```

---

### 3. Statistical Arbitrage

**Definition:** Exploit statistical relationships between correlated assets.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Statistical Arbitrage Example                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Historical Correlation: AAPL and MSFT move together                    │
│  (Tech stocks, similar market cap, both in indices)                     │
│                                                                          │
│  Time 0:                                                                 │
│    AAPL: $150.00                                                        │
│    MSFT: $300.00                                                        │
│    Ratio: MSFT/AAPL = 2.00 (normal)                                    │
│                                                                          │
│  Time 100μs: News hits about chip shortage (affects both)               │
│    AAPL: $149.25 (-0.5%)   ← Dropped fast                              │
│    MSFT: $300.00 (0%)      ← Hasn't moved yet                          │
│    Ratio: MSFT/AAPL = 2.017 (abnormal!)                                │
│                                                                          │
│  Jane Street's Trade (within 10μs):                                     │
│    1. Buy AAPL @ $149.25 (it's "cheap" relative to MSFT)               │
│    2. Short MSFT @ $300.00 (it will likely drop too)                   │
│                                                                          │
│  Time 2,000μs (2ms): MSFT price updates                                │
│    AAPL: $149.50 (+0.17%)                                               │
│    MSFT: $298.50 (-0.5%)                                                │
│    Ratio: MSFT/AAPL = 1.996 (back to normal)                           │
│                                                                          │
│  Close Positions:                                                        │
│    • Sell AAPL @ $149.50:  +$0.25 per share                            │
│    • Cover MSFT @ $298.50: +$1.50 per share                            │
│                                                                          │
│  On 10,000 share positions:                                             │
│    AAPL profit: $2,500                                                  │
│    MSFT profit: $15,000                                                 │
│    Total: $17,500 in 2 milliseconds                                    │
│                                                                          │
│  ════════════════════════════════════════════════════════════════       │
│                                                                          │
│  The Challenge:                                                          │
│    • Must monitor 1000s of pairs simultaneously                         │
│    • Correlation can break (biggest risk!)                              │
│    • Must execute BOTH trades atomically                                │
│    • Competition: if you're slow, profit disappears                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Rust Implementation:**

```rust
pub struct StatArb {
    pairs: Vec<TradingPair>,
    correlation_window: Duration,
}

pub struct TradingPair {
    asset_a: String,
    asset_b: String,
    historical_ratio: Decimal,
    ratio_std_dev: Decimal,
    current_ratio: Decimal,
}

impl StatArb {
    /// Check if pair has diverged from historical relationship
    pub fn check_divergence(&self, pair: &TradingPair) -> Option<Trade> {
        let z_score = (pair.current_ratio - pair.historical_ratio) / pair.ratio_std_dev;

        if z_score > dec!(2.0) {
            // Asset B is expensive relative to A
            Some(Trade::new(
                Signal::Buy(pair.asset_a.clone()),
                Signal::Sell(pair.asset_b.clone()),
            ))
        } else if z_score < dec!(-2.0) {
            // Asset A is expensive relative to B
            Some(Trade::new(
                Signal::Sell(pair.asset_a.clone()),
                Signal::Buy(pair.asset_b.clone()),
            ))
        } else {
            None
        }
    }
}
```

---

## The Technology Stack

### What Jane Street & Jump Actually Use

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Technology Stack                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  LANGUAGE CHOICES:                                                       │
│                                                                          │
│  Jane Street:                                                            │
│    • OCaml (95% of codebase)                                            │
│      - Functional programming (immutability = correctness)               │
│      - Type safety (catch bugs at compile time)                         │
│      - GC tuned for low-latency (incremental GC)                        │
│                                                                          │
│  Jump Trading:                                                           │
│    • C++ (70% of codebase)                                              │
│      - Zero-cost abstractions                                            │
│      - Manual memory management                                          │
│      - Template metaprogramming                                          │
│    • Rust (growing adoption for new systems)                            │
│      - Memory safety without GC                                          │
│      - Fearless concurrency                                              │
│                                                                          │
│  Citadel Securities:                                                     │
│    • C++ (dominant)                                                     │
│    • Python (research/backtesting)                                      │
│    • Rust (new critical path systems)                                   │
│                                                                          │
│  ════════════════════════════════════════════════════════════════       │
│                                                                          │
│  NETWORK:                                                                │
│    • 10G/40G/100G Ethernet                                              │
│    • Solarflare/Mellanox NICs (kernel bypass)                           │
│    • DPDK (Data Plane Development Kit)                                  │
│    • RDMA over Converged Ethernet (RoCE)                                │
│    • Custom microwave networks                                          │
│                                                                          │
│  HARDWARE:                                                               │
│    • Intel Xeon (Cascade Lake / Ice Lake)                               │
│    • AMD EPYC (cheaper, more cores)                                     │
│    • FPGAs (Xilinx/Intel) for ultra-low latency                         │
│    • Custom ASICs (for market data parsing)                             │
│    • NVMe SSDs for persistence (Intel Optane)                           │
│                                                                          │
│  PROTOCOLS:                                                              │
│    • FIX (Financial Information eXchange)                               │
│    • SBE (Simple Binary Encoding)                                       │
│    • Custom binary protocols                                             │
│    • NO JSON in production (too slow)                                   │
│                                                                          │
│  PERSISTENCE:                                                            │
│    • Memory-mapped files                                                 │
│    • Write-ahead log (WAL)                                              │
│    • Distributed replicas (Raft/Paxos)                                  │
│    • NOT traditional databases                                           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## What Would Actually Impress Them

### Tier 1: CPU Cache Mastery (Must-Have)

**Problem:** Your order struct causes 2 cache misses instead of 1.

```rust
// ❌ BAD: Spans multiple cache lines
struct Order {
    id: Uuid,              // 16 bytes
    symbol: String,        // 24 bytes (heap pointer + len + cap)
    side: OrderSide,       // 1 byte
    order_type: OrderType, // 1 byte
    price: Decimal,        // 16 bytes
    quantity: Decimal,     // 16 bytes
    user_id: String,       // 24 bytes
    timestamp: DateTime,   // 12 bytes
    metadata: HashMap,     // 24 bytes
    // Total: ~134 bytes → spans 3 cache lines (64 bytes each)!
}

// When CPU loads this:
// - Cache miss 1: Fetch bytes 0-63 (100 cycles = 30ns)
// - Cache miss 2: Fetch bytes 64-127 (100 cycles = 30ns)
// - Cache miss 3: Fetch bytes 128-134 (100 cycles = 30ns)
// Total: 90ns just to load the struct!

// ✅ GOOD: Hot/cold separation
#[repr(C, align(64))]  // Align to cache line boundary
struct OrderHot {
    // Only fields needed for matching (32 bytes total)
    price: u64,        // 8 bytes (fixed-point: price × 10^8)
    quantity: u64,     // 8 bytes
    order_id: u64,     // 8 bytes (hash of UUID)
    flags: u64,        // 8 bytes (side, type, status packed as bits)
    // Total: 32 bytes → fits in HALF a cache line!
}

struct OrderCold {
    // Fields rarely accessed
    user_id: u64,           // Hash, not String
    timestamp: u64,         // Nanos since epoch
    metadata_ptr: *const u8, // Pointer to cold storage
    original_id: Uuid,      // Full UUID
}

// Now cache miss count: 1 instead of 3 = 3x faster!
```

**What They Ask:**
> "Your matching engine runs at 500ns per order. Profile it and identify the bottleneck."

**Impressive Answer:**

```bash
# Use perf to profile
perf stat -e cache-misses,cache-references,branch-misses,branches ./matching_engine

# Output shows:
#   cache-misses: 24 per order (too high!)
#   branch-misses: 15 per order (too high!)

# Deep dive with perf record
perf record -g ./matching_engine
perf report

# Findings:
#   40% of time: Cache misses from Order struct spanning 3 lines
#   30% of time: Branch mispredictions from if/else chains
#   20% of time: malloc/free in order creation
#   10% of time: Actual matching logic
```

**Solution:**

```rust
// 1. Fix cache misses: hot/cold split (saves 200ns)
// 2. Fix branches: use branchless comparison (saves 150ns)
// 3. Fix allocations: use object pool (saves 100ns)
// New latency: 50ns (90% improvement!)
```

---

### Tier 2: Lock-Free Programming (Senior Level)

**They want to see:** Understanding of memory ordering and lock-free data structures.

```rust
use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::cell::UnsafeCell;

/// Lock-free SPSC queue (Lamport's algorithm)
pub struct LamportQueue<T> {
    buffer: Vec<UnsafeCell<MaybeUninit<T>>>,

    // Cache-line padding to prevent false sharing
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,

    capacity: usize,
}

#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

unsafe impl<T: Send> Send for LamportQueue<T> {}
unsafe impl<T: Send> Sync for LamportQueue<T> {}

impl<T> LamportQueue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            buffer,
            head: CachePadded { value: AtomicUsize::new(0) },
            tail: CachePadded { value: AtomicUsize::new(0) },
            capacity,
        }
    }

    /// Producer: push
    pub fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.value.load(Ordering::Relaxed);  // Relaxed OK!
        let head = self.head.value.load(Ordering::Acquire);  // Acquire barrier

        let next_tail = (tail + 1) & (self.capacity - 1);

        // Queue full?
        if next_tail == head {
            return Err(value);
        }

        // Write data
        unsafe {
            (*self.buffer[tail].get()).write(value);
        }

        // Publish tail (Release ensures write happens before this)
        self.tail.value.store(next_tail, Ordering::Release);

        Ok(())
    }

    /// Consumer: pop
    pub fn pop(&self) -> Option<T> {
        let head = self.head.value.load(Ordering::Relaxed);
        let tail = self.tail.value.load(Ordering::Acquire);  // Acquire barrier

        // Queue empty?
        if head == tail {
            return None;
        }

        // Read data
        let value = unsafe {
            (*self.buffer[head].get()).assume_init_read()
        };

        let next_head = (head + 1) & (self.capacity - 1);

        // Publish head
        self.head.value.store(next_head, Ordering::Release);

        Some(value)
    }
}
```

**Why This Impresses:**

1. **Memory Ordering:** Shows you understand `Acquire`, `Release`, and `Relaxed`
2. **False Sharing:** Cache-line padding prevents CPU cores from invalidating each other's cache
3. **Lock-Free:** No mutexes = guaranteed progress = no priority inversion
4. **Power-of-2 Capacity:** Enables fast modulo with bitwise AND

**Interview Question:**
> "Why is tail loaded with `Relaxed` but head loaded with `Acquire`?"

**Answer:**
```
Producer (push):
  - tail.load(Relaxed): We're the only writer, no need for synchronization
  - head.load(Acquire): Synchronize with consumer's Release store

Consumer (pop):
  - head.load(Relaxed): We're the only reader, no need for synchronization
  - tail.load(Acquire): Synchronize with producer's Release store

Acquire/Release form a "happens-before" relationship that ensures:
  - Data written before Release is visible after Acquire
  - No data races
```

---

### Tier 3: Network Stack Expertise (Expert Level)

**Problem:** Kernel network stack adds 10-20μs latency.

```rust
// ❌ Standard approach (slow)
use std::net::TcpStream;

let mut stream = TcpStream::connect("exchange.com:5000")?;
stream.write_all(&order_bytes)?;  // Syscall overhead: ~5-10μs

// Path: Your code → System call → Kernel → NIC → Network
//       ~5μs       ~5μs          ~2μs     ~1μs

// ✅ Kernel bypass with DPDK (fast)
use dpdk::*;

pub struct DpdkNetworking {
    port_id: u16,
    tx_queue: u16,
    rx_queue: u16,
    packet_pool: *mut rte_mempool,
}

impl DpdkNetworking {
    pub fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        unsafe {
            // Allocate packet from pool (no syscall!)
            let mbuf = rte_pktmbuf_alloc(self.packet_pool);

            // Zero-copy: write directly to NIC memory
            let pkt_data = rte_pktmbuf_mtod(mbuf, *mut u8);
            std::ptr::copy_nonoverlapping(data.as_ptr(), pkt_data, data.len());

            // Submit to NIC queue (no syscall!)
            rte_eth_tx_burst(self.port_id, self.tx_queue, &mbuf, 1);
        }

        Ok(())
    }

    pub fn recv_packets(&self) -> Vec<Packet> {
        let mut packets = Vec::with_capacity(32);

        unsafe {
            // Poll NIC directly (no interrupts, no syscalls!)
            let nb_rx = rte_eth_rx_burst(
                self.port_id,
                self.rx_queue,
                packets.as_mut_ptr() as *mut *mut rte_mbuf,
                32,
            );

            packets.set_len(nb_rx as usize);
        }

        packets
    }
}

// Path: Your code → NIC → Network
//       ~1μs       ~1μs
// Total: ~2μs (5x faster!)
```

**What They Care About:**

1. **Understanding the full stack:**
   ```
   Application → Socket API → TCP/IP Stack → NIC Driver → Hardware
                  ↑ 5μs      ↑ 5μs         ↑ 2μs        ↑ 1μs

   DPDK bypasses first 3 layers:
   Application → DPDK → Hardware
                  ↑ 1μs  ↑ 1μs
   ```

2. **Poll Mode vs Interrupt Mode:**
   ```
   Interrupt Mode (standard):
     - NIC raises interrupt when packet arrives
     - Context switch to kernel (500ns)
     - Kernel processes interrupt (1μs)
     - Copy to userspace (500ns)
     Total: ~2μs + syscall overhead

   Poll Mode (DPDK):
     - CPU continuously polls NIC ring buffer
     - No interrupts, no context switches
     - Zero-copy access
     Total: ~100ns (20x faster!)
   ```

3. **TCP offload:**
   ```rust
   // Hardware handles TCP checksum, segmentation
   let mut txq_conf = rte_eth_txconf::default();
   txq_conf.offloads = DEV_TX_OFFLOAD_TCP_CKSUM | DEV_TX_OFFLOAD_TCP_TSO;
   ```

---

### Tier 4: Hardware Timestamping

**Problem:** Software timestamps have 1-5μs jitter.

```rust
// ❌ Software timestamp (jitter from scheduling)
let timestamp = SystemTime::now();  // Jitter: ±5μs

// ✅ Hardware timestamp (from NIC)
pub struct HardwareTimestamp {
    raw_cycles: u64,    // TSC value from NIC
    nanos: u64,         // Calibrated nanoseconds
    sequence: u64,      // Sequence number
}

impl HardwareTimestamp {
    /// Extract timestamp from NIC metadata
    pub fn from_packet(packet: &RxPacket) -> Self {
        // Most NICs write timestamp to packet descriptor
        let descriptor = packet.descriptor();

        Self {
            raw_cycles: descriptor.timestamp_raw,
            nanos: cycles_to_nanos(descriptor.timestamp_raw),
            sequence: descriptor.sequence,
        }
    }

    /// Calibrate TSC to wall clock
    pub fn calibrate() -> CycleCalibration {
        let start_wall = get_ptp_time();  // From PTP daemon
        let start_tsc = unsafe { core::arch::x86_64::_rdtsc() };

        std::thread::sleep(Duration::from_secs(1));

        let end_wall = get_ptp_time();
        let end_tsc = unsafe { core::arch::x86_64::_rdtsc() };

        let cycles_per_nano = (end_tsc - start_tsc) as f64
                              / (end_wall - start_wall).as_nanos() as f64;

        CycleCalibration { cycles_per_nano }
    }
}

fn cycles_to_nanos(cycles: u64) -> u64 {
    // Use pre-calibrated conversion
    static CALIBRATION: CycleCalibration = /* ... */;
    (cycles as f64 / CALIBRATION.cycles_per_nano) as u64
}
```

**Clock Synchronization:**

```
PTP (Precision Time Protocol):
  • Synchronizes clocks across network
  • Accuracy: <100ns between servers
  • Required for regulatory compliance

GPS Clock:
  • Absolute time reference
  • Accuracy: <50ns to UTC
  • Used by all major exchanges

Why it matters:
  • Prove order timestamps for regulations
  • Measure true end-to-end latency
  • Debug race conditions
```

---

## Concrete Projects to Build

### Project 1: Latency Breakdown Dashboard

```rust
use hdrhistogram::Histogram;

pub struct LatencyBreakdown {
    pub network_rx: Histogram<u64>,      // NIC → buffer
    pub deserialization: Histogram<u64>, // Bytes → Order
    pub validation: Histogram<u64>,      // Risk checks
    pub matching: Histogram<u64>,        // Core algorithm
    pub serialization: Histogram<u64>,   // Trade → bytes
    pub network_tx: Histogram<u64>,      // Buffer → NIC

    pub total: Histogram<u64>,           // End-to-end
}

impl LatencyBreakdown {
    pub fn record(&mut self, breakdown: &OrderLatency) {
        self.network_rx.record(breakdown.network_rx).ok();
        self.deserialization.record(breakdown.deserialization).ok();
        self.validation.record(breakdown.validation).ok();
        self.matching.record(breakdown.matching).ok();
        self.serialization.record(breakdown.serialization).ok();
        self.network_tx.record(breakdown.network_tx).ok();

        let total = breakdown.network_rx
                  + breakdown.deserialization
                  + breakdown.validation
                  + breakdown.matching
                  + breakdown.serialization
                  + breakdown.network_tx;

        self.total.record(total).ok();
    }

    pub fn print_report(&self) {
        println!("Latency Breakdown (all times in nanoseconds):");
        println!("─────────────────────────────────────────────");

        let components = [
            ("Network RX", &self.network_rx),
            ("Deserialize", &self.deserialization),
            ("Validation", &self.validation),
            ("Matching", &self.matching),
            ("Serialize", &self.serialization),
            ("Network TX", &self.network_tx),
            ("TOTAL", &self.total),
        ];

        for (name, hist) in components {
            println!("{:12} | p50: {:6}ns | p99: {:6}ns | p99.9: {:6}ns | max: {:6}ns",
                     name,
                     hist.value_at_quantile(0.50),
                     hist.value_at_quantile(0.99),
                     hist.value_at_quantile(0.999),
                     hist.max());
        }
    }
}
```

**Output Example:**

```
Latency Breakdown (all times in nanoseconds):
─────────────────────────────────────────────
Network RX   | p50:   500ns | p99:  1200ns | p99.9:  2500ns | max:  5000ns
Deserialize  | p50:   200ns | p99:   450ns | p99.9:   800ns | max:  1500ns
Validation   | p50:   300ns | p99:   600ns | p99.9:  1200ns | max:  3000ns
Matching     | p50:   800ns | p99:  2000ns | p99.9:  5000ns | max: 15000ns ← BOTTLENECK
Serialize    | p50:   150ns | p99:   300ns | p99.9:   600ns | max:  1200ns
Network TX   | p50:   400ns | p99:   900ns | p99.9:  2000ns | max:  4000ns
TOTAL        | p50:  2350ns | p99:  5450ns | p99.9: 12100ns | max: 29700ns
```

**What This Shows:**
- Matching is the bottleneck (800ns median, but 15μs worst case!)
- Focus optimization efforts there
- p99.9 is what matters in production (not median)

---

### Project 2: Market Maker Simulation

```rust
pub struct MarketMakerBot {
    symbol: String,
    inventory: Decimal,
    target_inventory: Decimal,
    max_inventory: Decimal,

    // Strategy parameters
    base_spread_bps: Decimal,      // Base spread in basis points
    min_spread_bps: Decimal,        // Minimum spread
    vol_multiplier: Decimal,        // Widen spread in volatility

    // Performance tracking
    trades_won: usize,              // Quotes that filled
    trades_lost: usize,             // Adverse selection
    pnl: Decimal,                   // Profit/loss
}

impl MarketMakerBot {
    /// Generate two-sided quote every 100μs
    pub fn generate_quote(&mut self, market: &MarketData) -> TwoSidedQuote {
        let mid = market.mid_price();

        // Calculate spread based on volatility
        let volatility = self.estimate_volatility(market);
        let spread = self.base_spread_bps * (dec!(1) + volatility * self.vol_multiplier);
        let spread = spread.max(self.min_spread_bps);

        // Inventory skew
        let skew = self.calculate_inventory_skew();

        TwoSidedQuote {
            bid: mid - spread / dec!(2) - skew,
            ask: mid + spread / dec!(2) + skew,
            bid_size: self.calculate_quote_size(OrderSide::Buy),
            ask_size: self.calculate_quote_size(OrderSide::Sell),
        }
    }

    fn calculate_inventory_skew(&self) -> Decimal {
        // Penalize side that increases inventory
        let inventory_pct = self.inventory / self.max_inventory;

        // 1bp skew per 10% inventory
        inventory_pct * dec!(0.001)
    }

    fn calculate_quote_size(&self, side: OrderSide) -> Decimal {
        let base_size = dec!(100);

        match side {
            OrderSide::Buy if self.inventory > self.target_inventory => {
                // Already long, reduce buy size
                base_size * dec!(0.5)
            }
            OrderSide::Sell if self.inventory < self.target_inventory => {
                // Already short, reduce sell size
                base_size * dec!(0.5)
            }
            _ => base_size,
        }
    }

    fn estimate_volatility(&self, market: &MarketData) -> Decimal {
        // Simple: spread as % of price
        let spread_bps = ((market.best_ask - market.best_bid) / market.mid_price())
                         * dec!(10000);

        // Normalize to 0-1 range
        (spread_bps / dec!(100)).min(dec!(1))
    }

    pub fn on_fill(&mut self, fill: &Fill) {
        // Update inventory
        match fill.side {
            OrderSide::Buy => self.inventory += fill.quantity,
            OrderSide::Sell => self.inventory -= fill.quantity,
        }

        // Update P&L
        let mid = fill.price; // Simplification
        let edge = (fill.price - mid).abs();
        self.pnl += edge * fill.quantity;

        self.trades_won += 1;
    }

    pub fn performance_report(&self) {
        let win_rate = self.trades_won as f64
                       / (self.trades_won + self.trades_lost) as f64;

        println!("Market Maker Performance:");
        println!("  Trades Won: {}", self.trades_won);
        println!("  Trades Lost: {}", self.trades_lost);
        println!("  Win Rate: {:.2}%", win_rate * 100.0);
        println!("  P&L: ${:.2}", self.pnl);
        println!("  Inventory: {}", self.inventory);
    }
}
```

---

### Project 3: Pre-Trade Risk System

```rust
pub struct PreTradeRisk {
    // Position limits
    max_position: HashMap<String, Decimal>,
    current_position: Arc<DashMap<String, AtomicDecimal>>,

    // Notional limits
    max_notional_per_symbol: Decimal,
    max_notional_total: Decimal,
    current_notional: AtomicDecimal,

    // Order limits
    max_order_size: Decimal,
    max_orders_per_second: usize,

    // Rate limiting
    order_count: Arc<AtomicUsize>,
    window_start: Arc<AtomicU64>,
}

impl PreTradeRisk {
    /// Check order against all risk limits (lock-free!)
    pub fn check_order(&self, order: &Order) -> Result<(), RiskError> {
        // 1. Order size limit
        if order.quantity > self.max_order_size {
            return Err(RiskError::OrderTooLarge);
        }

        // 2. Rate limit
        self.check_rate_limit()?;

        // 3. Position limit
        self.check_position_limit(order)?;

        // 4. Notional limit
        self.check_notional_limit(order)?;

        Ok(())
    }

    fn check_rate_limit(&self) -> Result<(), RiskError> {
        let now = Utc::now().timestamp_millis() as u64;
        let window_start = self.window_start.load(Ordering::Acquire);

        // New 1-second window?
        if now - window_start >= 1000 {
            self.window_start.store(now, Ordering::Release);
            self.order_count.store(1, Ordering::Release);
            return Ok(());
        }

        // Increment counter
        let count = self.order_count.fetch_add(1, Ordering::AcqRel);

        if count >= self.max_orders_per_second {
            return Err(RiskError::RateLimitExceeded);
        }

        Ok(())
    }

    fn check_position_limit(&self, order: &Order) -> Result<(), RiskError> {
        let max = self.max_position.get(&order.symbol)
            .copied()
            .unwrap_or(Decimal::MAX);

        let current = self.current_position
            .entry(order.symbol.clone())
            .or_insert_with(|| AtomicDecimal::new(Decimal::ZERO));

        let delta = match order.side {
            OrderSide::Buy => order.quantity,
            OrderSide::Sell => -order.quantity,
        };

        loop {
            let current_val = current.load(Ordering::Acquire);
            let new_val = current_val + delta;

            if new_val.abs() > max {
                return Err(RiskError::PositionLimitExceeded);
            }

            // Try to reserve capacity (CAS)
            if current.compare_exchange(
                current_val,
                new_val,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                return Ok(());
            }

            // CAS failed, retry
            std::hint::spin_loop();
        }
    }

    fn check_notional_limit(&self, order: &Order) -> Result<(), RiskError> {
        let order_notional = order.price.unwrap_or(Decimal::ZERO) * order.quantity;

        loop {
            let current = self.current_notional.load(Ordering::Acquire);
            let new_total = current + order_notional;

            if new_total > self.max_notional_total {
                return Err(RiskError::NotionalLimitExceeded);
            }

            if self.current_notional.compare_exchange(
                current,
                new_total,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                return Ok(());
            }

            std::hint::spin_loop();
        }
    }
}

#[derive(Debug)]
pub enum RiskError {
    OrderTooLarge,
    RateLimitExceeded,
    PositionLimitExceeded,
    NotionalLimitExceeded,
}
```

---

## The Interview Bar

### Junior Level (0-2 years)

**Technical Bar:**
- Implement basic order book with <1ms latency
- Understand Big-O notation and data structure choices
- Basic profiling with `perf` or similar tools
- Read research papers (Hasbrouck, O'Hara)

**Interview Question:**
> "Design an order book. What data structures would you use and why?"

**Good Answer:**
```
Price-time priority requires:
  1. Fast price lookup: BTreeMap<Decimal, PriceLevel>
     - O(log n) insert/delete
     - O(1) access to best bid/ask

  2. FIFO within price level: VecDeque<Order>
     - O(1) push_back
     - O(1) pop_front

  3. Fast cancel by ID: HashMap<Uuid, Order>
     - O(1) lookup
     - Store pointer to position in VecDeque

Expected latency: 500ns - 1μs per operation
```

---

### Mid Level (2-5 years)

**Technical Bar:**
- Order book with <100μs latency
- Lock-free data structures
- Memory ordering (Acquire/Release)
- Kernel tuning (CPU pinning, huge pages)

**Interview Question:**
> "Your order book update takes 10μs. Profile it and fix it."

**Good Answer:**
```
1. Profile with perf:
   perf stat -e cache-misses,branch-misses ./order_book

2. Findings:
   - 5μs: Mutex contention (8 cores fighting for lock)
   - 3μs: Cache misses (Order struct spans 3 cache lines)
   - 2μs: Hash map lookups (poor cache locality)

3. Fixes:
   - Replace Mutex with SPSC queue → saves 4μs
   - Hot/cold split Order struct → saves 2μs
   - Object pool for orders → saves 1μs

New latency: 3μs (70% improvement)
```

---

### Senior Level (5-10 years)

**Technical Bar:**
- Order book with <10μs latency
- Kernel bypass (DPDK/io_uring)
- SIMD optimizations
- Hardware timestamping
- Production experience

**Interview Question:**
> "Design a market maker that quotes 1000 symbols simultaneously with <5μs latency per quote update."

**Good Answer:**
```
Architecture:
  1. One matching thread per symbol (pinned to cores)
     - Lock-free SPSC queue for order submission
     - Lock-free order book (RCU pattern)

  2. Shared market data cache (Seqlock)
     - All threads read without blocking
     - Single writer updates from market data feed

  3. Risk checks before submission
     - Lock-free atomic counters
     - No database queries

  4. Network: DPDK for kernel bypass
     - Poll mode (no interrupts)
     - Batch 32 packets at once

Expected latency:
  - Market data update → quote decision: 1μs
  - Quote serialization: 0.5μs
  - Network TX: 1μs
  Total: 2.5μs per symbol

Scalability: 1000 symbols × 10,000 updates/sec = 10M quotes/sec
```

---

### Principal Level (10+ years)

**Technical Bar:**
- Sub-microsecond latency
- FPGA experience
- Distributed systems (cross-datacenter)
- Team leadership
- Production war stories

**Interview Question:**
> "Our latency is 5μs but competitors are at 1μs. You have unlimited budget. What do you do?"

**Excellent Answer:**
```
Phase 1: Quick wins (2 weeks)
  1. Profile every component → find the 20% taking 80% time
  2. SIMD optimizations for hot paths
  3. Object pools → zero allocations
  4. Better CPU pinning (isolate matching cores)
  Expected: 5μs → 3μs

Phase 2: Architecture (1 month)
  1. Move matching to FPGA
     - Parse FIX in 100ns (vs 1μs in software)
     - Deterministic latency (no cache misses)
  2. Kernel bypass everywhere (DPDK)
  3. Hardware timestamping
  Expected: 3μs → 800ns

Phase 3: Infrastructure (3 months)
  1. Co-locate in exchange datacenter
     - Reduce network latency from 200μs to 50μs
  2. Direct market data feed (not consolidated)
  3. Microwave network for cross-exchange arb
  Expected: 800ns processing + 50μs network

Phase 4: Question assumptions
  "Is 1μs even the right target? Our edge might be:
   - Better models (not faster execution)
   - More capital (bigger quotes)
   - Better risk management

  Speed is necessary but not sufficient."
```

---

## Summary Table

| Level | Latency Target | Key Skills | What Impresses |
|-------|---------------|------------|----------------|
| **Junior** | <1ms | Data structures, profiling | Working order book, paper understanding |
| **Mid** | <100μs | Lock-free, memory ordering | Latency breakdown, optimization story |
| **Senior** | <10μs | Kernel bypass, SIMD | Production experience, war stories |
| **Principal** | <1μs | FPGA, distributed | Systems thinking, leadership |

---

## Final Advice

### What Jane Street Actually Cares About

1. **Problem Solving > Speed**
   - They care MORE about correctness than raw performance
   - "Fast and wrong" is worse than "slow and right"
   - Can you reason about edge cases?

2. **Communication**
   - Explain technical concepts clearly
   - Write design docs
   - Code review skills

3. **Curiosity**
   - "Why does this take 10μs?" not "it's fast enough"
   - Read research papers
   - Stay current with industry

### What Jump Trading Actually Cares About

1. **Speed Obsession**
   - Every nanosecond matters
   - Hardware awareness
   - Kernel/network knowledge

2. **Quantitative Skills**
   - Statistical models
   - Probability theory
   - Fast mental math

3. **Production Mindset**
   - What happens when it breaks?
   - Monitoring, alerting, recovery
   - Risk management

---

## Recommended Next Steps

1. **Implement Phase 3 features from your roadmap**
   - Lock-free SPSC queue
   - Binary protocol
   - Latency breakdown tracking

2. **Build latency intuition**
   - Benchmark EVERYTHING
   - Use `perf`, `flamegraph`, `cachegrind`
   - Build a mental model: "HashMap lookup = 50ns, cache miss = 100ns"

3. **Read the classics**
   - "Market Microstructure in Practice" (Lehalle & Laruelle)
   - "The Quants" (Scott Patterson)
   - OCaml docs (Jane Street's weapon of choice)
   - Rust Atomics and Locks (Mara Bos)

4. **Practice interviews**
   - LeetCode Hard (to pass HR screen)
   - System design (to pass technical round)
   - Live coding (implement order book in 45 min)

**Remember:** Jane Street gets 20,000 applications per year and hires ~100. Jump Trading gets 10,000 applications and hires ~50. The bar is EXTREMELY high, but achievable with dedication.

Good luck! 🚀
