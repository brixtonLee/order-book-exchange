# Data Structures & Algorithms: Complete Guide with Rust

**Comprehensive DSA guide with visualizations, implementations, and dry runs for trading systems**

---

## Table of Contents

1. [Arrays & Vectors](#1-arrays--vectors)
2. [Linked Lists](#2-linked-lists)
3. [Stacks](#3-stacks)
4. [Queues](#4-queues)
5. [Hash Tables](#5-hash-tables)
6. [Trees](#6-trees)
7. [Heaps](#7-heaps)
8. [Graphs](#8-graphs)
9. [Tries](#9-tries)
10. [Sorting Algorithms](#10-sorting-algorithms)
11. [Searching Algorithms](#11-searching-algorithms)
12. [Dynamic Programming](#12-dynamic-programming)
13. [Greedy Algorithms](#13-greedy-algorithms)
14. [Graph Algorithms](#14-graph-algorithms)
15. [Trading System Applications](#15-trading-system-applications)

---

## 1. Arrays & Vectors

### Concept

Arrays are contiguous memory locations storing elements of the same type. Vectors are dynamic arrays that can grow.

### Visual Representation

```
Array: [10][20][30][40][50]
Index:  0   1   2   3   4

Memory Layout:
┌────┬────┬────┬────┬────┐
│ 10 │ 20 │ 30 │ 40 │ 50 │
└────┴────┴────┴────┴────┘
 0x00 0x08 0x10 0x18 0x20  (memory addresses)
```

### Time Complexity

| Operation | Array | Vector |
|-----------|-------|--------|
| Access | O(1) | O(1) |
| Search | O(n) | O(n) |
| Insert (end) | N/A | O(1) amortized |
| Insert (middle) | N/A | O(n) |
| Delete (end) | N/A | O(1) |
| Delete (middle) | N/A | O(n) |

### Implementation

```rust
// Fixed-size array
fn array_example() {
    let arr: [i32; 5] = [10, 20, 30, 40, 50];

    // Access - O(1)
    println!("Element at index 2: {}", arr[2]);  // 30

    // Iterate - O(n)
    for (i, &val) in arr.iter().enumerate() {
        println!("arr[{}] = {}", i, val);
    }

    // Search - O(n)
    let target = 30;
    let position = arr.iter().position(|&x| x == target);
    println!("Found {} at position: {:?}", target, position);
}

// Dynamic vector
fn vector_example() {
    let mut vec = Vec::new();

    // Push (append) - O(1) amortized
    vec.push(10);
    vec.push(20);
    vec.push(30);
    println!("Vector: {:?}", vec);  // [10, 20, 30]

    // Pop (remove last) - O(1)
    let last = vec.pop();
    println!("Popped: {:?}", last);  // Some(30)

    // Insert at position - O(n)
    vec.insert(1, 15);  // Insert 15 at index 1
    println!("After insert: {:?}", vec);  // [10, 15, 20]

    // Remove at position - O(n)
    vec.remove(1);  // Remove element at index 1
    println!("After remove: {:?}", vec);  // [10, 20]

    // Access - O(1)
    println!("Element at index 1: {}", vec[1]);  // 20

    // Capacity management
    vec.reserve(100);  // Reserve space for 100 more elements
    println!("Capacity: {}", vec.capacity());
}
```

### Dry Run Example

```rust
// Problem: Find two numbers that sum to target
fn two_sum(nums: Vec<i32>, target: i32) -> Option<(usize, usize)> {
    for i in 0..nums.len() {
        for j in (i + 1)..nums.len() {
            if nums[i] + nums[j] == target {
                return Some((i, j));
            }
        }
    }
    None
}

// Dry run:
let nums = vec![2, 7, 11, 15];
let target = 9;

// Iteration 1: i=0, nums[0]=2
//   j=1: nums[1]=7, 2+7=9 ✓ Found!
//   Return (0, 1)

let result = two_sum(nums, target);
println!("{:?}", result);  // Some((0, 1))
```

### Trading System Application

```rust
// Order book price level array
struct PriceLevel {
    price: i32,
    volume: i32,
}

fn find_best_price(levels: &[PriceLevel], side: &str) -> Option<i32> {
    match side {
        "buy" => levels.iter().map(|l| l.price).max(),
        "sell" => levels.iter().map(|l| l.price).min(),
        _ => None,
    }
}

// Example
let bids = vec![
    PriceLevel { price: 100, volume: 10 },
    PriceLevel { price: 99, volume: 20 },
    PriceLevel { price: 98, volume: 15 },
];

let best_bid = find_best_price(&bids, "buy");
println!("Best bid: {:?}", best_bid);  // Some(100)
```

---

## 2. Linked Lists

### Concept

A linked list is a linear data structure where elements are stored in nodes. Each node contains data and a pointer to the next node.

### Visual Representation

```
Singly Linked List:
┌──────┬────┐   ┌──────┬────┐   ┌──────┬────┐   ┌──────┬────┐
│ 10   │ ●──┼──→│ 20   │ ●──┼──→│ 30   │ ●──┼──→│ 40   │ X  │
└──────┴────┘   └──────┴────┘   └──────┴────┘   └──────┴────┘
  Head                                              Tail

Doubly Linked List:
     ┌──────┬────┬────┐   ┌──────┬────┬────┐   ┌──────┬────┬────┐
NULL │  X   │ 10 │ ●──┼──→│  ●   │ 20 │ ●──┼──→│  ●   │ 30 │ X  │
     └──────┴────┴────┘   └──────┴────┴────┘   └──────┴────┴────┘
      Head    ↑               ↑      ↑              ↑       Tail
              └───────────────┘      └──────────────┘
```

### Time Complexity

| Operation | Singly | Doubly |
|-----------|--------|--------|
| Access | O(n) | O(n) |
| Search | O(n) | O(n) |
| Insert (head) | O(1) | O(1) |
| Insert (tail) | O(1)* | O(1) |
| Insert (middle) | O(n) | O(n) |
| Delete (head) | O(1) | O(1) |
| Delete (tail) | O(n) | O(1) |
| Delete (middle) | O(n) | O(n) |

*Requires tail pointer

### Implementation

```rust
// Singly Linked List
#[derive(Debug)]
struct Node<T> {
    data: T,
    next: Option<Box<Node<T>>>,
}

#[derive(Debug)]
struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

impl<T> LinkedList<T> {
    fn new() -> Self {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    // Insert at head - O(1)
    fn push_front(&mut self, data: T) {
        let new_node = Box::new(Node {
            data,
            next: self.head.take(),
        });
        self.head = Some(new_node);
        self.size += 1;
    }

    // Remove from head - O(1)
    fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|node| {
            self.head = node.next;
            self.size -= 1;
            node.data
        })
    }

    // Peek at head - O(1)
    fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.data)
    }

    // Check if empty - O(1)
    fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    // Get size - O(1)
    fn len(&self) -> usize {
        self.size
    }

    // Iterate
    fn iter(&self) -> LinkedListIter<T> {
        LinkedListIter {
            next: self.head.as_deref(),
        }
    }
}

struct LinkedListIter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for LinkedListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_deref();
            &node.data
        })
    }
}

// Usage example
fn linked_list_example() {
    let mut list = LinkedList::new();

    // Push elements
    list.push_front(30);
    list.push_front(20);
    list.push_front(10);

    println!("List size: {}", list.len());  // 3

    // Iterate
    for value in list.iter() {
        println!("{}", value);  // 10, 20, 30
    }

    // Pop elements
    while let Some(val) = list.pop_front() {
        println!("Popped: {}", val);  // 10, 20, 30
    }
}
```

### Dry Run Example

```rust
// Problem: Reverse a linked list

fn reverse_list<T>(mut list: LinkedList<T>) -> LinkedList<T> {
    let mut reversed = LinkedList::new();

    while let Some(data) = list.pop_front() {
        reversed.push_front(data);
    }

    reversed
}

// Dry run:
// Original: 10 -> 20 -> 30 -> None
//
// Step 1: Pop 10, push to reversed
//   Original: 20 -> 30 -> None
//   Reversed: 10 -> None
//
// Step 2: Pop 20, push to reversed
//   Original: 30 -> None
//   Reversed: 20 -> 10 -> None
//
// Step 3: Pop 30, push to reversed
//   Original: None
//   Reversed: 30 -> 20 -> 10 -> None
//
// Result: 30 -> 20 -> 10 -> None
```

### Trading System Application

```rust
// Order queue using linked list (FIFO)
struct OrderQueue {
    orders: LinkedList<Order>,
}

#[derive(Debug, Clone)]
struct Order {
    id: String,
    price: i32,
    quantity: i32,
}

impl OrderQueue {
    fn new() -> Self {
        OrderQueue {
            orders: LinkedList::new(),
        }
    }

    fn add_order(&mut self, order: Order) {
        self.orders.push_front(order);  // Add to end
    }

    fn process_next_order(&mut self) -> Option<Order> {
        self.orders.pop_front()  // Process from front
    }
}
```

---

## 3. Stacks

### Concept

A stack is a LIFO (Last In, First Out) data structure. Like a stack of plates - you add and remove from the top.

### Visual Representation

```
Stack Operations:

Initial:  Empty
          ┌─────┐
          │     │
          └─────┘

Push(10): ┌─────┐
          │ 10  │ ← Top
          └─────┘

Push(20): ┌─────┐
          │ 20  │ ← Top
          ├─────┤
          │ 10  │
          └─────┘

Push(30): ┌─────┐
          │ 30  │ ← Top
          ├─────┤
          │ 20  │
          ├─────┤
          │ 10  │
          └─────┘

Pop():    ┌─────┐
          │ 20  │ ← Top (30 removed)
          ├─────┤
          │ 10  │
          └─────┘
```

### Time Complexity

| Operation | Time |
|-----------|------|
| Push | O(1) |
| Pop | O(1) |
| Peek/Top | O(1) |
| IsEmpty | O(1) |
| Size | O(1) |

### Implementation

```rust
// Stack using Vec
struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Stack { items: Vec::new() }
    }

    // Push element - O(1)
    fn push(&mut self, item: T) {
        self.items.push(item);
    }

    // Pop element - O(1)
    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    // Peek at top - O(1)
    fn peek(&self) -> Option<&T> {
        self.items.last()
    }

    // Check if empty - O(1)
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // Get size - O(1)
    fn size(&self) -> usize {
        self.items.len()
    }
}

fn stack_example() {
    let mut stack = Stack::new();

    // Push elements
    stack.push(10);
    stack.push(20);
    stack.push(30);

    println!("Top: {:?}", stack.peek());  // Some(30)
    println!("Size: {}", stack.size());   // 3

    // Pop elements (LIFO order)
    while let Some(val) = stack.pop() {
        println!("Popped: {}", val);  // 30, 20, 10
    }
}
```

### Dry Run Example

```rust
// Problem: Balanced Parentheses

fn is_balanced(expr: &str) -> bool {
    let mut stack = Stack::new();

    for ch in expr.chars() {
        match ch {
            '(' | '{' | '[' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            _ => {}
        }
    }

    stack.is_empty()
}

// Dry run: "{[()]}"
//
// char='{': push '{' → stack: ['{']
// char='[': push '[' → stack: ['{', '[']
// char='(': push '(' → stack: ['{', '[', '(']
// char=')': pop '(' ✓ matches → stack: ['{', '[']
// char=']': pop '[' ✓ matches → stack: ['{']
// char='}': pop '{' ✓ matches → stack: []
// stack.is_empty() = true ✓

let result = is_balanced("{[()]}");
println!("Balanced: {}", result);  // true
```

### Trading System Application

```rust
// Undo/Redo functionality for order modifications
struct OrderHistory {
    undo_stack: Stack<OrderAction>,
    redo_stack: Stack<OrderAction>,
}

#[derive(Debug, Clone)]
enum OrderAction {
    Create(Order),
    Modify { old: Order, new: Order },
    Cancel(Order),
}

impl OrderHistory {
    fn new() -> Self {
        OrderHistory {
            undo_stack: Stack::new(),
            redo_stack: Stack::new(),
        }
    }

    fn execute(&mut self, action: OrderAction) {
        self.undo_stack.push(action);
        self.redo_stack = Stack::new();  // Clear redo on new action
    }

    fn undo(&mut self) -> Option<OrderAction> {
        if let Some(action) = self.undo_stack.pop() {
            self.redo_stack.push(action.clone());
            Some(action)
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<OrderAction> {
        if let Some(action) = self.redo_stack.pop() {
            self.undo_stack.push(action.clone());
            Some(action)
        } else {
            None
        }
    }
}

// Expression evaluation (for pricing formulas)
fn evaluate_postfix(expr: &str) -> i32 {
    let mut stack = Stack::new();

    for token in expr.split_whitespace() {
        match token.parse::<i32>() {
            Ok(num) => stack.push(num),
            Err(_) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let result = match token {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    _ => 0,
                };
                stack.push(result);
            }
        }
    }

    stack.pop().unwrap()
}

// Example: "3 4 + 2 *" = (3+4)*2 = 14
let result = evaluate_postfix("3 4 + 2 *");
println!("Result: {}", result);  // 14
```

---

## 4. Queues

### Concept

A queue is a FIFO (First In, First Out) data structure. Like a line at a ticket counter - first person in line is served first.

### Visual Representation

```
Queue Operations:

Enqueue(10):
Front → [10] ← Rear

Enqueue(20):
Front → [10][20] ← Rear

Enqueue(30):
Front → [10][20][30] ← Rear

Dequeue():
Front → [20][30] ← Rear  (10 removed)

Circular Queue:
     ┌───┬───┬───┬───┐
     │ 3 │ 4 │ 1 │ 2 │
     └───┴───┴───┴───┘
       ↑       ↑
     Front   Rear
```

### Time Complexity

| Operation | Simple Queue | Circular Queue |
|-----------|--------------|----------------|
| Enqueue | O(1) | O(1) |
| Dequeue | O(1) | O(1) |
| Front | O(1) | O(1) |
| IsEmpty | O(1) | O(1) |
| IsFull | N/A | O(1) |

### Implementation

```rust
// Simple Queue using VecDeque
use std::collections::VecDeque;

struct Queue<T> {
    items: VecDeque<T>,
}

impl<T> Queue<T> {
    fn new() -> Self {
        Queue {
            items: VecDeque::new(),
        }
    }

    // Enqueue (add to rear) - O(1)
    fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }

    // Dequeue (remove from front) - O(1)
    fn dequeue(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    // Peek at front - O(1)
    fn front(&self) -> Option<&T> {
        self.items.front()
    }

    // Check if empty - O(1)
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // Get size - O(1)
    fn size(&self) -> usize {
        self.items.len()
    }
}

fn queue_example() {
    let mut queue = Queue::new();

    // Enqueue elements
    queue.enqueue(10);
    queue.enqueue(20);
    queue.enqueue(30);

    println!("Front: {:?}", queue.front());  // Some(10)
    println!("Size: {}", queue.size());      // 3

    // Dequeue elements (FIFO order)
    while let Some(val) = queue.dequeue() {
        println!("Dequeued: {}", val);  // 10, 20, 30
    }
}

// Circular Queue (Fixed Size)
struct CircularQueue<T: Copy + Default> {
    data: Vec<T>,
    front: usize,
    rear: usize,
    size: usize,
    capacity: usize,
}

impl<T: Copy + Default> CircularQueue<T> {
    fn new(capacity: usize) -> Self {
        CircularQueue {
            data: vec![T::default(); capacity],
            front: 0,
            rear: 0,
            size: 0,
            capacity,
        }
    }

    fn is_full(&self) -> bool {
        self.size == self.capacity
    }

    fn is_empty(&self) -> bool {
        self.size == 0
    }

    fn enqueue(&mut self, item: T) -> Result<(), &'static str> {
        if self.is_full() {
            return Err("Queue is full");
        }

        self.data[self.rear] = item;
        self.rear = (self.rear + 1) % self.capacity;
        self.size += 1;
        Ok(())
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let item = self.data[self.front];
        self.front = (self.front + 1) % self.capacity;
        self.size -= 1;
        Some(item)
    }
}

fn circular_queue_example() {
    let mut queue = CircularQueue::new(3);

    queue.enqueue(10).unwrap();
    queue.enqueue(20).unwrap();
    queue.enqueue(30).unwrap();
    // Queue is now full

    println!("Dequeued: {:?}", queue.dequeue());  // Some(10)
    queue.enqueue(40).unwrap();  // Now has space

    while let Some(val) = queue.dequeue() {
        println!("{}", val);  // 20, 30, 40
    }
}
```

### Priority Queue

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Eq, PartialEq, Debug)]
struct PriorityOrder {
    price: i32,
    quantity: i32,
    timestamp: u64,
}

// Higher price = higher priority for buy orders
impl Ord for PriorityOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price.cmp(&other.price)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
    }
}

impl PartialOrd for PriorityOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn priority_queue_example() {
    let mut pq = BinaryHeap::new();

    pq.push(PriorityOrder {
        price: 100,
        quantity: 10,
        timestamp: 1,
    });

    pq.push(PriorityOrder {
        price: 105,
        quantity: 5,
        timestamp: 2,
    });

    pq.push(PriorityOrder {
        price: 102,
        quantity: 8,
        timestamp: 3,
    });

    // Pop in priority order (highest price first)
    while let Some(order) = pq.pop() {
        println!("Price: {}, Qty: {}", order.price, order.quantity);
        // 105, 102, 100
    }
}
```

### Dry Run Example

```rust
// Problem: Sliding Window Maximum

fn sliding_window_max(nums: Vec<i32>, k: usize) -> Vec<i32> {
    let mut result = Vec::new();
    let mut window: VecDeque<usize> = VecDeque::new();

    for i in 0..nums.len() {
        // Remove elements outside window
        while !window.is_empty() && window[0] <= i.saturating_sub(k) {
            window.pop_front();
        }

        // Remove smaller elements (won't be max)
        while !window.is_empty() && nums[*window.back().unwrap()] < nums[i] {
            window.pop_back();
        }

        window.push_back(i);

        // Add to result if window is complete
        if i >= k - 1 {
            result.push(nums[window[0]]);
        }
    }

    result
}

// Dry run: nums=[1,3,-1,-3,5,3,6,7], k=3
//
// i=0: window=[0], result=[]
// i=1: window=[1] (remove 0, 1>3), result=[]
// i=2: window=[1,2], result=[3] (max in [1,3,-1])
// i=3: window=[1,2,3], result=[3,3] (max in [3,-1,-3])
// i=4: window=[4] (remove all, 5 is largest), result=[3,3,5]
// i=5: window=[4,5], result=[3,3,5,5]
// i=6: window=[6] (remove all, 6 is largest), result=[3,3,5,5,6]
// i=7: window=[7] (remove all, 7 is largest), result=[3,3,5,5,6,7]
```

### Trading System Application

```rust
// Order matching queue (FIFO within price level)
struct PriceLevel {
    price: i32,
    orders: Queue<Order>,
}

impl PriceLevel {
    fn new(price: i32) -> Self {
        PriceLevel {
            price,
            orders: Queue::new(),
        }
    }

    fn add_order(&mut self, order: Order) {
        self.orders.enqueue(order);
    }

    fn match_against(&mut self, incoming_qty: i32) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_qty = incoming_qty;

        while remaining_qty > 0 && !self.orders.is_empty() {
            if let Some(mut resting_order) = self.orders.dequeue() {
                let trade_qty = remaining_qty.min(resting_order.quantity);

                trades.push(Trade {
                    price: self.price,
                    quantity: trade_qty,
                });

                remaining_qty -= trade_qty;
                resting_order.quantity -= trade_qty;

                if resting_order.quantity > 0 {
                    // Put back if not fully filled
                    self.orders.enqueue(resting_order);
                    break;
                }
            }
        }

        trades
    }
}

#[derive(Debug, Clone)]
struct Trade {
    price: i32,
    quantity: i32,
}
```

---

## 5. Hash Tables

### Concept

Hash tables store key-value pairs and use a hash function to compute an index for fast access.

### Visual Representation

```
Hash Table (Separate Chaining):

Hash Function: hash(key) = key % 10

   Index  Bucket
   ┌───┬───────────────────────┐
 0 │ 0 │ → [10→"A"]             │
   ├───┼───────────────────────┤
 1 │ 1 │ → [11→"B"] → [21→"C"]  │ (Collision!)
   ├───┼───────────────────────┤
 2 │ 2 │ → [12→"D"]             │
   ├───┼───────────────────────┤
 3 │ 3 │ → Empty                │
   ├───┼───────────────────────┤
 4 │ 4 │ → [14→"E"]             │
   └───┴───────────────────────┘

Hash Table (Open Addressing):
   ┌───┬─────┐
 0 │10 │ "A" │
   ├───┼─────┤
 1 │11 │ "B" │
   ├───┼─────┤
 2 │21 │ "C" │ ← Collision, probed to next slot
   ├───┼─────┤
 3 │12 │ "D" │
   └───┴─────┘
```

### Time Complexity

| Operation | Average | Worst |
|-----------|---------|-------|
| Insert | O(1) | O(n) |
| Delete | O(1) | O(n) |
| Search | O(1) | O(n) |

### Implementation

```rust
use std::collections::HashMap;

fn hashmap_example() {
    let mut map = HashMap::new();

    // Insert - O(1) average
    map.insert("XAUUSD", 2000);
    map.insert("EURUSD", 1);
    map.insert("BTCUSD", 45000);

    // Access - O(1) average
    if let Some(&price) = map.get("XAUUSD") {
        println!("XAUUSD price: {}", price);  // 2000
    }

    // Update
    map.insert("XAUUSD", 2050);  // Overwrites existing value

    // Delete - O(1) average
    map.remove("EURUSD");

    // Check existence - O(1) average
    if map.contains_key("BTCUSD") {
        println!("BTCUSD exists");
    }

    // Iterate
    for (symbol, price) in &map {
        println!("{}: {}", symbol, price);
    }

    // Entry API (for efficient upserts)
    map.entry("GBPUSD")
        .and_modify(|price| *price += 1)
        .or_insert(1);
}

// Custom Hash Table Implementation
struct SimpleHashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    size: usize,
}

impl<K: std::hash::Hash + Eq, V> SimpleHashMap<K, V> {
    fn new() -> Self {
        SimpleHashMap {
            buckets: vec![Vec::new(); 16],
            size: 0,
        }
    }

    fn hash(&self, key: &K) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.buckets.len()
    }

    fn insert(&mut self, key: K, value: V) {
        let index = self.hash(&key);
        let bucket = &mut self.buckets[index];

        // Update if exists
        for (k, v) in bucket.iter_mut() {
            if k == &key {
                *v = value;
                return;
            }
        }

        // Insert new
        bucket.push((key, value));
        self.size += 1;
    }

    fn get(&self, key: &K) -> Option<&V> {
        let index = self.hash(key);
        let bucket = &self.buckets[index];

        for (k, v) in bucket {
            if k == key {
                return Some(v);
            }
        }

        None
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.hash(key);
        let bucket = &mut self.buckets[index];

        if let Some(pos) = bucket.iter().position(|(k, _)| k == key) {
            self.size -= 1;
            Some(bucket.remove(pos).1)
        } else {
            None
        }
    }
}
```

### Dry Run Example

```rust
// Problem: Two Sum using HashMap

fn two_sum_hash(nums: Vec<i32>, target: i32) -> Option<(usize, usize)> {
    let mut map = HashMap::new();

    for (i, &num) in nums.iter().enumerate() {
        let complement = target - num;

        if let Some(&j) = map.get(&complement) {
            return Some((j, i));
        }

        map.insert(num, i);
    }

    None
}

// Dry run: nums=[2,7,11,15], target=9
//
// i=0, num=2:
//   complement = 9-2 = 7
//   map.get(7) = None
//   map.insert(2, 0) → map={2:0}
//
// i=1, num=7:
//   complement = 9-7 = 2
//   map.get(2) = Some(0) ✓ Found!
//   Return (0, 1)
//
// Time: O(n), Space: O(n)
// Much faster than O(n²) nested loops!

let result = two_sum_hash(vec![2, 7, 11, 15], 9);
println!("{:?}", result);  // Some((0, 1))
```

### Trading System Application

```rust
// Order Book using HashMap
use std::collections::HashMap;

struct OrderBook {
    bids: HashMap<i32, Vec<Order>>,  // price -> orders
    asks: HashMap<i32, Vec<Order>>,
}

impl OrderBook {
    fn new() -> Self {
        OrderBook {
            bids: HashMap::new(),
            asks: HashMap::new(),
        }
    }

    fn add_order(&mut self, order: Order) {
        let book = match order.side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        book.entry(order.price)
            .or_insert_with(Vec::new)
            .push(order);
    }

    fn get_orders_at_price(&self, side: Side, price: i32) -> Option<&Vec<Order>> {
        match side {
            Side::Buy => self.bids.get(&price),
            Side::Sell => self.asks.get(&price),
        }
    }

    fn remove_order(&mut self, order_id: &str) -> Option<Order> {
        // Search in both bids and asks
        for orders in self.bids.values_mut() {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                return Some(orders.remove(pos));
            }
        }

        for orders in self.asks.values_mut() {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                return Some(orders.remove(pos));
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
enum Side {
    Buy,
    Sell,
}

// Symbol price cache
struct PriceCache {
    cache: HashMap<String, CachedPrice>,
}

struct CachedPrice {
    price: i32,
    timestamp: u64,
}

impl PriceCache {
    fn new() -> Self {
        PriceCache {
            cache: HashMap::new(),
        }
    }

    fn update(&mut self, symbol: String, price: i32, timestamp: u64) {
        self.cache.insert(symbol, CachedPrice { price, timestamp });
    }

    fn get(&self, symbol: &str) -> Option<i32> {
        self.cache.get(symbol).map(|cp| cp.price)
    }

    fn is_stale(&self, symbol: &str, max_age: u64) -> bool {
        if let Some(cached) = self.cache.get(symbol) {
            let age = current_timestamp() - cached.timestamp;
            age > max_age
        } else {
            true
        }
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

---

## 6. Trees

### Concept

Trees are hierarchical data structures with a root node and child nodes forming a parent-child relationship.

### Visual Representation

```
Binary Tree:
           10
          /  \
         5    15
        / \   / \
       3   7 12  20

Binary Search Tree (BST):
- Left subtree < root
- Right subtree > root

         50
        /  \
      30    70
     / \    / \
   20  40 60  80
```

### Time Complexity

| Operation | BST (Average) | BST (Worst) | Balanced Tree |
|-----------|---------------|-------------|---------------|
| Search | O(log n) | O(n) | O(log n) |
| Insert | O(log n) | O(n) | O(log n) |
| Delete | O(log n) | O(n) | O(log n) |
| Min/Max | O(log n) | O(n) | O(log n) |

### Implementation

```rust
// Binary Search Tree
#[derive(Debug)]
struct TreeNode<T> {
    value: T,
    left: Option<Box<TreeNode<T>>>,
    right: Option<Box<TreeNode<T>>>,
}

#[derive(Debug)]
struct BinarySearchTree<T> {
    root: Option<Box<TreeNode<T>>>,
}

impl<T: Ord> BinarySearchTree<T> {
    fn new() -> Self {
        BinarySearchTree { root: None }
    }

    fn insert(&mut self, value: T) {
        self.root = Self::insert_node(self.root.take(), value);
    }

    fn insert_node(node: Option<Box<TreeNode<T>>>, value: T) -> Option<Box<TreeNode<T>>> {
        match node {
            None => Some(Box::new(TreeNode {
                value,
                left: None,
                right: None,
            })),
            Some(mut n) => {
                if value < n.value {
                    n.left = Self::insert_node(n.left, value);
                } else {
                    n.right = Self::insert_node(n.right, value);
                }
                Some(n)
            }
        }
    }

    fn search(&self, value: &T) -> bool {
        Self::search_node(&self.root, value)
    }

    fn search_node(node: &Option<Box<TreeNode<T>>>, value: &T) -> bool {
        match node {
            None => false,
            Some(n) => {
                if value == &n.value {
                    true
                } else if value < &n.value {
                    Self::search_node(&n.left, value)
                } else {
                    Self::search_node(&n.right, value)
                }
            }
        }
    }

    // In-order traversal (sorted order)
    fn inorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::inorder_traversal(&self.root, &mut result);
        result
    }

    fn inorder_traversal<'a>(node: &'a Option<Box<TreeNode<T>>>, result: &mut Vec<&'a T>) {
        if let Some(n) = node {
            Self::inorder_traversal(&n.left, result);
            result.push(&n.value);
            Self::inorder_traversal(&n.right, result);
        }
    }
}

fn bst_example() {
    let mut bst = BinarySearchTree::new();

    // Insert
    bst.insert(50);
    bst.insert(30);
    bst.insert(70);
    bst.insert(20);
    bst.insert(40);
    bst.insert(60);
    bst.insert(80);

    // Search
    println!("Contains 40: {}", bst.search(&40));  // true
    println!("Contains 90: {}", bst.search(&90));  // false

    // In-order traversal (sorted)
    let sorted = bst.inorder();
    println!("Sorted: {:?}", sorted);  // [20, 30, 40, 50, 60, 70, 80]
}
```

### Dry Run Example

```rust
// Problem: Validate Binary Search Tree

fn is_valid_bst<T: Ord>(node: &Option<Box<TreeNode<T>>>, min: Option<&T>, max: Option<&T>) -> bool {
    match node {
        None => true,
        Some(n) => {
            if let Some(min_val) = min {
                if &n.value <= min_val {
                    return false;
                }
            }

            if let Some(max_val) = max {
                if &n.value >= max_val {
                    return false;
                }
            }

            is_valid_bst(&n.left, min, Some(&n.value))
                && is_valid_bst(&n.right, Some(&n.value), max)
        }
    }
}

// Dry run:
//        10
//       /  \
//      5    15
//     / \
//    3   7
//
// is_valid_bst(10, None, None):
//   value=10, min=None ✓, max=None ✓
//   Check left: is_valid_bst(5, None, Some(10))
//     value=5, min=None ✓, max=10 ✓
//     Check left: is_valid_bst(3, None, Some(5))
//       value=3, min=None ✓, max=5 ✓
//       left=None ✓, right=None ✓ → true
//     Check right: is_valid_bst(7, Some(5), Some(10))
//       value=7, min=5 ✓, max=10 ✓
//       left=None ✓, right=None ✓ → true
//     → true
//   Check right: is_valid_bst(15, Some(10), None)
//     value=15, min=10 ✓, max=None ✓
//     left=None ✓, right=None ✓ → true
//   → true ✓ Valid BST!
```

### Trading System Application

```rust
// Price level tree for order book
use std::collections::BTreeMap;

struct OrderBookTree {
    bids: BTreeMap<i32, Vec<Order>>,  // Descending order
    asks: BTreeMap<i32, Vec<Order>>,  // Ascending order
}

impl OrderBookTree {
    fn new() -> Self {
        OrderBookTree {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    fn add_bid(&mut self, price: i32, order: Order) {
        self.bids.entry(price)
            .or_insert_with(Vec::new)
            .push(order);
    }

    fn add_ask(&mut self, price: i32, order: Order) {
        self.asks.entry(price)
            .or_insert_with(Vec::new)
            .push(order);
    }

    fn best_bid(&self) -> Option<i32> {
        self.bids.keys().next_back().copied()  // Highest price
    }

    fn best_ask(&self) -> Option<i32> {
        self.asks.keys().next().copied()  // Lowest price
    }

    fn spread(&self) -> Option<i32> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
}

// Example
let mut book = OrderBookTree::new();
book.add_bid(100, Order { id: "1".into(), quantity: 10 });
book.add_bid(99, Order { id: "2".into(), quantity: 20 });
book.add_ask(101, Order { id: "3".into(), quantity: 15 });
book.add_ask(102, Order { id: "4".into(), quantity: 25 });

println!("Best bid: {:?}", book.best_bid());  // Some(100)
println!("Best ask: {:?}", book.best_ask());  // Some(101)
println!("Spread: {:?}", book.spread());      // Some(1)
```

---

Due to length constraints, I'll summarize the remaining sections. Would you like me to continue with the remaining data structures (Heaps, Graphs, Tries) and algorithms (Sorting, Searching, DP, Greedy, Graph Algorithms) in full detail?

The guide covers:
- **Complete implementations** with explanations
- **Visual diagrams** for each data structure
- **Time/Space complexity** analysis
- **Dry run examples** with step-by-step execution
- **Trading system applications** for each concept

Should I continue with sections 7-15 in the same comprehensive detail?
