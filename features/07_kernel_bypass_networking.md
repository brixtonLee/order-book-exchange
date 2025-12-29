# Kernel-Bypass Network Stack (Ultra-Low Latency)

## Purpose

**Kernel-bypass networking** allows applications to send and receive network packets without going through the operating system kernel. This dramatically reduces latency - a critical advantage in high-frequency trading where microseconds matter.

### The Problem with Standard Networking

```
Normal network path:
Application → Kernel → Network Driver → NIC → Wire
           ↑                                    ↓
           ← Kernel ← Network Driver ← NIC ← Wire

Latency: ~50-200 microseconds (2 context switches, syscalls, interrupts)
```

### Kernel-Bypass Solution

```
Kernel-bypass path:
Application → User-space Driver → NIC → Wire
           ↑                          ↓
           ← User-space Driver ← NIC ← Wire

Latency: ~1-10 microseconds (direct DMA to user-space memory)
```

### Technologies

1. **DPDK** (Data Plane Development Kit): Poll-mode drivers, batching
2. **io_uring**: Modern Linux async I/O (kernel-assisted, not full bypass)
3. **AF_XDP**: Linux socket type for zero-copy packet processing
4. **eBPF**: In-kernel packet filtering and forwarding

---

## Technology Stack

### Core Dependencies

```toml
[dependencies]
# io_uring (easiest to start with)
io-uring = "0.6"
tokio-uring = "0.4"  # Tokio integration

# Low-level networking
libc = "0.2"
nix = "0.27"         # Unix system call wrappers

# Packet parsing
etherparse = "0.14"  # Ethernet/IP/TCP/UDP parsing
pnet = "0.34"        # Alternative packet library

# Memory management
memmap2 = "0.9"      # Memory-mapped I/O
crossbeam = "0.8"    # Lock-free queues

# Monitoring
metrics = "0.21"
tracing = "0.1"

[dev-dependencies]
criterion = "0.5"
```

### System Requirements

- **Linux 5.1+** for io_uring
- **Linux 5.3+** for AF_XDP
- **Root privileges** for DPDK/AF_XDP (or CAP_NET_RAW)
- **Dedicated network interface** (can't share with kernel)

---

## Implementation Guide

### Phase 1: io_uring Networking (Easiest Entry Point)

io_uring is not full kernel bypass, but it's much faster than traditional syscalls and a good starting point.

#### Step 1: Basic io_uring TCP Server

```rust
use io_uring::{opcode, types, IoUring};
use std::net::TcpListener;
use std::os::unix::io::AsRawFd;

pub struct IoUringServer {
    ring: IoUring,
    listener: TcpListener,
}

impl IoUringServer {
    pub fn new(addr: &str, queue_depth: u32) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let ring = IoUring::new(queue_depth)?;

        Ok(Self { ring, listener })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        // Submit accept operation
        self.submit_accept()?;

        loop {
            // Submit queued operations
            self.ring.submit_and_wait(1)?;

            // Process completions
            while let Some(cqe) = self.ring.completion().next() {
                self.handle_completion(cqe.user_data(), cqe.result())?;
            }
        }
    }

    fn submit_accept(&mut self) -> std::io::Result<()> {
        let accept_op = opcode::Accept::new(
            types::Fd(self.listener.as_raw_fd()),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        let entry = accept_op
            .build()
            .user_data(1);  // ID for this operation

        unsafe {
            self.ring.submission().push(&entry)?;
        }

        Ok(())
    }

    fn handle_completion(&mut self, user_data: u64, result: i32) -> std::io::Result<()> {
        match user_data {
            1 => {
                // Accept completed
                if result < 0 {
                    return Err(std::io::Error::from_raw_os_error(-result));
                }

                let client_fd = result;
                println!("New connection: fd {}", client_fd);

                // Submit read operation for this client
                self.submit_read(client_fd)?;

                // Re-submit accept for next connection
                self.submit_accept()?;
            }
            _ => {
                // Handle read/write completions
                println!("Operation {} completed with {}", user_data, result);
            }
        }

        Ok(())
    }

    fn submit_read(&mut self, fd: i32) -> std::io::Result<()> {
        // Allocate buffer (in real code, use a buffer pool)
        let mut buf = vec![0u8; 4096];
        let buf_ptr = buf.as_mut_ptr();

        let read_op = opcode::Read::new(
            types::Fd(fd),
            buf_ptr,
            4096,
        );

        let entry = read_op
            .build()
            .user_data(fd as u64);  // Use fd as ID

        unsafe {
            self.ring.submission().push(&entry)?;
        }

        // Prevent buffer from being dropped
        std::mem::forget(buf);

        Ok(())
    }
}
```

**Key advantages of io_uring:**
- Batched syscalls (submit multiple operations at once)
- No context switches per operation
- Kernel polls for completions (no interrupts)
- Zero-copy for some operations

---

### Phase 2: AF_XDP (True Zero-Copy)

AF_XDP allows user-space to directly access network interface queues.

#### Step 2: AF_XDP Socket Setup

```rust
use nix::sys::socket::{socket, bind, SockaddrStorage, AddressFamily, SockType, SockFlag};
use nix::sys::socket::{setsockopt, sockopt};
use std::os::unix::io::RawFd;

const XDP_ZEROCOPY: i32 = 1 << 2;
const XDP_COPY: i32 = 1 << 1;

pub struct XdpSocket {
    fd: RawFd,
    umem: UmemArea,  // User-space memory for packets
    rx_ring: XskRing,
    tx_ring: XskRing,
}

/// User-space memory area for packet buffers
pub struct UmemArea {
    addr: *mut u8,
    size: usize,
    frame_size: usize,
    frame_count: usize,
}

impl UmemArea {
    pub fn new(frame_size: usize, frame_count: usize) -> std::io::Result<Self> {
        use memmap2::MmapMut;

        let size = frame_size * frame_count;

        // Allocate page-aligned memory
        let mmap = MmapMut::map_anon(size)?;
        let addr = mmap.as_ptr() as *mut u8;

        // Lock pages in memory (prevent swapping)
        unsafe {
            libc::mlock(addr as *const libc::c_void, size);
        }

        std::mem::forget(mmap);  // We manage this memory manually

        Ok(Self {
            addr,
            size,
            frame_size,
            frame_count,
        })
    }

    pub fn get_frame(&self, index: usize) -> *mut u8 {
        assert!(index < self.frame_count);
        unsafe { self.addr.add(index * self.frame_size) }
    }
}

/// XDP ring buffer (shared with kernel)
pub struct XskRing {
    producer: *mut u32,
    consumer: *mut u32,
    ring: *mut u64,
    size: u32,
}

impl XdpSocket {
    pub fn new(interface: &str, queue_id: u32) -> std::io::Result<Self> {
        // Create AF_XDP socket
        let fd = socket(
            AddressFamily::Xdp,
            SockType::Raw,
            SockFlag::empty(),
            None,
        )?;

        // Allocate UMEM (user-space memory)
        let umem = UmemArea::new(2048, 4096)?;

        // Register UMEM with kernel
        Self::register_umem(fd, &umem)?;

        // Bind socket to interface and queue
        Self::bind_socket(fd, interface, queue_id)?;

        // Setup RX and TX rings
        let rx_ring = Self::setup_ring(fd, true)?;
        let tx_ring = Self::setup_ring(fd, false)?;

        Ok(Self {
            fd,
            umem,
            rx_ring,
            tx_ring,
        })
    }

    fn register_umem(fd: RawFd, umem: &UmemArea) -> std::io::Result<()> {
        #[repr(C)]
        struct XdpUmemReg {
            addr: u64,
            len: u64,
            chunk_size: u32,
            headroom: u32,
            flags: u32,
        }

        let reg = XdpUmemReg {
            addr: umem.addr as u64,
            len: umem.size as u64,
            chunk_size: umem.frame_size as u32,
            headroom: 0,
            flags: 0,
        };

        unsafe {
            let ret = libc::setsockopt(
                fd,
                libc::SOL_XDP,
                libc::XDP_UMEM_REG,
                &reg as *const _ as *const libc::c_void,
                std::mem::size_of::<XdpUmemReg>() as u32,
            );

            if ret < 0 {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok(())
    }

    fn bind_socket(fd: RawFd, interface: &str, queue_id: u32) -> std::io::Result<()> {
        // Get interface index
        let if_index = Self::if_nametoindex(interface)?;

        #[repr(C)]
        struct SockaddrXdp {
            sxdp_family: u16,
            sxdp_flags: u16,
            sxdp_ifindex: u32,
            sxdp_queue_id: u32,
            sxdp_shared_umem_fd: u32,
        }

        let addr = SockaddrXdp {
            sxdp_family: libc::AF_XDP as u16,
            sxdp_flags: XDP_ZEROCOPY as u16,
            sxdp_ifindex: if_index,
            sxdp_queue_id: queue_id,
            sxdp_shared_umem_fd: 0,
        };

        unsafe {
            let ret = libc::bind(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrXdp>() as u32,
            );

            if ret < 0 {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok(())
    }

    /// Receive packets (zero-copy!)
    pub fn receive(&mut self) -> Vec<&[u8]> {
        let mut packets = Vec::new();

        let consumer = unsafe { *self.rx_ring.consumer };
        let producer = unsafe { *self.rx_ring.producer };

        let available = producer.wrapping_sub(consumer);

        for i in 0..available {
            let idx = (consumer + i) % self.rx_ring.size;
            let desc_addr = unsafe { *self.rx_ring.ring.add(idx as usize) };

            // Get packet data directly from UMEM
            let frame_idx = (desc_addr / self.umem.frame_size as u64) as usize;
            let offset = (desc_addr % self.umem.frame_size as u64) as usize;

            let frame_ptr = self.umem.get_frame(frame_idx);
            let packet_ptr = unsafe { frame_ptr.add(offset) };

            // In real code, extract length from descriptor
            let packet = unsafe { std::slice::from_raw_parts(packet_ptr, 1500) };
            packets.push(packet);
        }

        // Update consumer index
        unsafe {
            *self.rx_ring.consumer = consumer + available;
        }

        packets
    }

    fn if_nametoindex(name: &str) -> std::io::Result<u32> {
        use std::ffi::CString;
        let name = CString::new(name)?;

        let index = unsafe { libc::if_nametoindex(name.as_ptr()) };

        if index == 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(index)
    }
}
```

**Zero-copy magic:**
- Packets DMA directly to user-space UMEM
- No kernel memory copy
- No syscall per packet

---

### Phase 3: Order Book with Kernel Bypass

#### Step 3: Fast Order Ingestion

```rust
use etherparse::{SlicedPacket, TransportSlice};

pub struct FastOrderReceiver {
    xdp_socket: XdpSocket,
    order_queue: crossbeam::queue::ArrayQueue<Order>,
}

impl FastOrderReceiver {
    pub fn run(&mut self) {
        loop {
            // Receive batch of packets (zero-copy)
            let packets = self.xdp_socket.receive();

            for packet_bytes in packets {
                if let Some(order) = self.parse_order(packet_bytes) {
                    // Lock-free queue push
                    let _ = self.order_queue.push(order);
                }
            }
        }
    }

    fn parse_order(&self, bytes: &[u8]) -> Option<Order> {
        // Parse Ethernet/IP/UDP headers
        let packet = SlicedPacket::from_ethernet(bytes).ok()?;

        // Extract UDP payload
        let payload = match packet.transport {
            Some(TransportSlice::Udp(udp)) => udp.payload(),
            _ => return None,
        };

        // Custom binary protocol (faster than JSON!)
        // Format: [order_id: u64][price: u64][qty: u64][side: u8]
        if payload.len() < 25 {
            return None;
        }

        let order_id = u64::from_le_bytes(payload[0..8].try_into().ok()?);
        let price = u64::from_le_bytes(payload[8..16].try_into().ok()?);
        let qty = u64::from_le_bytes(payload[16..24].try_into().ok()?);
        let side = payload[24];

        Some(Order {
            id: order_id,
            price: Decimal::from(price) / Decimal::from(100_000_000),  // Fixed-point
            quantity: Decimal::from(qty) / Decimal::from(100_000_000),
            side: if side == 0 { OrderSide::Buy } else { OrderSide::Sell },
        })
    }
}
```

**Performance:**
- Parse millions of packets per second
- Sub-microsecond latency
- No kernel overhead

---

## Advanced Optimizations

### 1. Batching

Process multiple packets before updating consumer index:

```rust
let batch = self.rx_ring.receive_batch(64);
for packet in batch {
    process(packet);
}
self.rx_ring.release_batch(64);  // Single atomic update
```

### 2. CPU Pinning

Pin threads to specific cores to avoid context switches:

```rust
use nix::sched::{sched_setaffinity, CpuSet};
use nix::unistd::Pid;

let mut cpu_set = CpuSet::new();
cpu_set.set(4)?;  // Pin to core 4
sched_setaffinity(Pid::from_raw(0), &cpu_set)?;
```

### 3. Huge Pages

Reduce TLB misses:

```rust
let size = 1 << 21;  // 2MB huge page
let addr = unsafe {
    libc::mmap(
        std::ptr::null_mut(),
        size,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB,
        -1,
        0,
    )
};
```

---

## Advantages

1. **Ultra-Low Latency**
   - 10-100x faster than kernel networking
   - Consistent sub-microsecond latency

2. **High Throughput**
   - Process millions of packets per second
   - Single core can saturate 10Gbps link

3. **Determinism**
   - No kernel interrupts or scheduling
   - Predictable performance

4. **Control**
   - Custom packet processing logic
   - No TCP/IP stack overhead

---

## Disadvantages

1. **Complexity**
   - Requires deep networking knowledge
   - Easy to make mistakes

2. **Hardware Dependency**
   - Needs compatible NICs
   - Driver support required

3. **Operational Burden**
   - Must implement own reliability (if needed)
   - Debugging is harder

4. **Resource Intensive**
   - Dedicated CPU cores for polling
   - Dedicated NICs (can't share with OS)

---

## Limitations

1. **Linux Only**
   - AF_XDP and io_uring are Linux-specific
   - DPDK supports FreeBSD but Rust bindings are limited

2. **Privileges Required**
   - Need root or CAP_NET_RAW
   - Security implications

3. **No TCP Stack**
   - Must implement TCP yourself or use UDP only
   - No congestion control

4. **Limited Protocol Support**
   - Best for custom binary protocols
   - HTTP/WebSocket need full stack

---

## Alternatives

### 1. **Standard io_uring** (Recommended Start)
- **Pros**: Much easier, still very fast
- **Cons**: Not true kernel bypass
- **Latency**: ~5-20μs

### 2. **DPDK with Rust Bindings**
- **Pros**: Battle-tested, widest NIC support
- **Cons**: Complex C API, large dependency
- **Latency**: ~1-5μs

### 3. **Solarflare Onload**
- **Pros**: Commercial support, kernel bypass
- **Cons**: Proprietary, expensive, specific NICs only
- **Latency**: ~1-3μs

### 4. **Mellanox VMA**
- **Pros**: Acceleration for existing apps (LD_PRELOAD)
- **Cons**: Mellanox NICs only
- **Latency**: ~2-5μs

### 5. **F-Stack (FreeBSD Network Stack in User-Space)**
- **Pros**: Full TCP/IP stack
- **Cons**: Based on DPDK, less Rust-friendly
- **Latency**: ~5-10μs

### 6. **kernel-bypass NICs (Solarflare, Mellanox)**
- **Pros**: Hardware offload
- **Cons**: Very expensive ($5k-$50k per NIC)

---

## When to Use Kernel Bypass

**DO use:**
- ✅ High-frequency trading
- ✅ Low-latency messaging
- ✅ Custom protocols
- ✅ Packet processing (DPI, firewall)

**DON'T use:**
- ❌ Web servers (use io_uring instead)
- ❌ Standard TCP applications
- ❌ Cloud environments (no HW access)
- ❌ Learning Rust basics (too advanced)

---

## Recommended Path

1. **Week 1-2**: Master io_uring with TCP
2. **Week 3-4**: Experiment with AF_XDP
3. **Week 5-6**: Build custom binary protocol
4. **Week 7-8**: Optimize with CPU pinning, batching
5. **Week 9-10**: Add eBPF for packet filtering
6. **Week 11-12**: Benchmark and tune

---

## Further Reading

- [io_uring Documentation](https://kernel.dk/io_uring.pdf)
- [AF_XDP Introduction](https://www.kernel.org/doc/html/latest/networking/af_xdp.html)
- [DPDK Programming Guide](https://doc.dpdk.org/guides/prog_guide/)
- [Cloudflare: How to receive a million packets per second](https://blog.cloudflare.com/how-to-receive-a-million-packets/)
- [The Definitive Guide to Linux Network Stack](https://blog.packagecloud.io/eng/2017/02/06/monitoring-tuning-linux-networking-stack-receiving-data/)
