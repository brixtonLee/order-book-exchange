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
        assert!(capacity > 0, "Capacity must be greater than 0");
        assert!(num_consumers > 0, "Must have at least one consumer");

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

    /// Get the current write cursor position
    pub fn write_position(&self) -> u64 {
        self.write_cursor.0.load(Ordering::Relaxed)
    }

    /// Get a consumer's read position
    pub fn read_position(&self, consumer_id: usize) -> u64 {
        self.read_cursors[consumer_id].0.load(Ordering::Relaxed)
    }

    /// Get the capacity of the ring buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the number of consumers
    pub fn num_consumers(&self) -> usize {
        self.read_cursors.len()
    }

    /// Check if a consumer has items available to read
    pub fn has_available(&self, consumer_id: usize) -> bool {
        let read_pos = self.read_cursors[consumer_id].0.load(Ordering::Relaxed);
        let write_pos = self.write_cursor.0.load(Ordering::Acquire);
        read_pos < write_pos
    }

    /// Get number of items available for a consumer
    pub fn available_count(&self, consumer_id: usize) -> u64 {
        let read_pos = self.read_cursors[consumer_id].0.load(Ordering::Relaxed);
        let write_pos = self.write_cursor.0.load(Ordering::Acquire);
        write_pos.saturating_sub(read_pos)
    }
}

/// Example event for demonstrating ring buffer usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEvent {
    NewOrder { order_id: u64, price: i64, quantity: i64 },
    OrderCancelled { order_id: u64 },
    OrderModified { order_id: u64, new_quantity: i64 },
    Trade { order_id: u64, price: i64, quantity: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;

    #[test]
    fn test_ring_buffer_creation() {
        let buffer: RingBuffer<i32> = RingBuffer::new(1024, 1);
        assert_eq!(buffer.capacity(), 1024);
        assert_eq!(buffer.num_consumers(), 1);
    }

    #[test]
    #[should_panic(expected = "Capacity must be power of 2")]
    fn test_non_power_of_two_capacity() {
        let _buffer: RingBuffer<i32> = RingBuffer::new(1000, 1);
    }

    #[test]
    fn test_single_producer_single_consumer() {
        let buffer = Arc::new(RingBuffer::new(1024, 1));
        let buffer_clone = buffer.clone();

        // Producer thread
        let producer = thread::spawn(move || {
            for i in 0..100 {
                buffer_clone.publish(i);
            }
        });

        // Give producer time to start
        thread::sleep(std::time::Duration::from_millis(10));

        // Consumer
        let mut received = Vec::new();
        let mut next_seq = 0;

        while received.len() < 100 {
            if let Some(item) = buffer.read(0, next_seq) {
                received.push(item);
                next_seq += 1;
            }
        }

        producer.join().unwrap();

        assert_eq!(received.len(), 100);
        assert_eq!(received[0], 0);
        assert_eq!(received[99], 99);
    }

    #[test]
    fn test_batch_read() {
        let buffer = Arc::new(RingBuffer::new(1024, 1));

        // Publish some items
        for i in 0..50 {
            buffer.publish(i);
        }

        // Batch read
        let items = buffer.read_batch(0, 25);
        assert_eq!(items.len(), 25);
        assert_eq!(items[0], 0);
        assert_eq!(items[24], 24);

        // Read remaining
        let items = buffer.read_batch(0, 25);
        assert_eq!(items.len(), 25);
        assert_eq!(items[0], 25);
        assert_eq!(items[24], 49);
    }

    #[test]
    fn test_multiple_consumers() {
        let buffer = Arc::new(RingBuffer::new(1024, 2));
        let buffer_clone1 = buffer.clone();
        let buffer_clone2 = buffer.clone();

        // Producer
        let producer = thread::spawn(move || {
            for i in 0..100 {
                buffer.publish(i);
            }
        });

        // Consumer 1
        let consumer1 = thread::spawn(move || {
            let items = buffer_clone1.read_batch(0, 100);
            items
        });

        // Consumer 2
        let consumer2 = thread::spawn(move || {
            let items = buffer_clone2.read_batch(1, 100);
            items
        });

        producer.join().unwrap();
        let items1 = consumer1.join().unwrap();
        let items2 = consumer2.join().unwrap();

        // Both consumers should receive all items
        assert_eq!(items1.len(), 100);
        assert_eq!(items2.len(), 100);
        assert_eq!(items1, items2);
    }

    #[test]
    fn test_order_event() {
        let buffer = Arc::new(RingBuffer::new(1024, 1));

        let event = OrderEvent::NewOrder {
            order_id: 123,
            price: 50000,
            quantity: 100,
        };

        buffer.publish(event);

        let received = buffer.read(0, 0);
        assert_eq!(received, Some(event));
    }

    #[test]
    fn test_available_count() {
        let buffer = RingBuffer::new(1024, 1);

        assert_eq!(buffer.available_count(0), 0);

        buffer.publish(42);
        buffer.publish(43);

        assert_eq!(buffer.available_count(0), 2);

        buffer.read(0, 0);
        assert_eq!(buffer.available_count(0), 1);
    }

    #[test]
    fn test_has_available() {
        let buffer = RingBuffer::new(1024, 1);

        assert!(!buffer.has_available(0));

        buffer.publish(42);

        assert!(buffer.has_available(0));

        buffer.read(0, 0);

        assert!(!buffer.has_available(0));
    }
}
