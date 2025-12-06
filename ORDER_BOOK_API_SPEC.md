# Rust Order Book API - Technical Specification

## Project Overview

A high-performance order matching engine and REST API built in Rust to demonstrate understanding of:
- Order matching algorithms and market microstructure
- Spread dynamics and liquidity metrics
- Exchange profit optimization through fee structures
- Suitable for quant developer interviews at firms like HRT, Jane Street, Citadel, etc.

## Technology Stack

- **Language**: Rust (latest stable)
- **Web Framework**: Axum (async, high-performance)
- **Serialization**: Serde (JSON)
- **Time**: Chrono
- **UUID**: uuid crate for order IDs
- **Testing**: Built-in Rust test framework + criterion for benchmarks

## Project Structure

```
order-book-api/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs                 # Entry point, server setup
│   ├── lib.rs                  # Library exports
│   ├── models/
│   │   ├── mod.rs
│   │   ├── order.rs            # Order types and enums
│   │   ├── trade.rs            # Trade execution records
│   │   └── orderbook.rs        # Order book state
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── matching.rs         # Core matching logic
│   │   ├── orderbook.rs        # Order book implementation
│   │   └── fees.rs             # Fee calculation logic
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs           # Route definitions
│   │   ├── handlers.rs         # Request handlers
│   │   └── responses.rs        # Response types
│   ├── metrics/
│   │   ├── mod.rs
│   │   ├── spread.rs           # Spread calculations
│   │   └── analytics.rs        # Market analytics
│   └── utils/
│       ├── mod.rs
│       └── validation.rs       # Input validation
├── tests/
│   ├── integration_tests.rs
│   └── performance_tests.rs
└── benches/
    └── matching_benchmark.rs
```

## Core Data Models

### Order Structure

```rust
// src/models/order.rs

pub struct Order {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,  // None for market orders
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
}

pub enum OrderSide {
    Buy,
    Sell,
}

pub enum OrderType {
    Limit,
    Market,
}

pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
}
```

### Trade Structure

```rust
// src/models/trade.rs

pub struct Trade {
    pub id: Uuid,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_id: String,
    pub seller_id: String,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub timestamp: DateTime<Utc>,
}
```

### Order Book Structure

```rust
// src/models/orderbook.rs

pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<Decimal, PriceLevel>,  // Buy orders (descending)
    pub asks: BTreeMap<Decimal, PriceLevel>,  // Sell orders (ascending)
    pub orders: HashMap<Uuid, Order>,          // Quick order lookup
    pub trades: Vec<Trade>,                    // Trade history
}

pub struct PriceLevel {
    pub price: Decimal,
    pub total_quantity: Decimal,
    pub orders: VecDeque<Uuid>,  // FIFO queue for price-time priority
}
```

## Core Engine Features

### Phase 1: MVP (Build This First)

#### 1. Order Book Engine (`src/engine/orderbook.rs`)

**Methods:**
- `new(symbol: String) -> OrderBook`
- `add_order(order: Order) -> Result<Vec<Trade>, OrderBookError>`
- `cancel_order(order_id: Uuid) -> Result<Order, OrderBookError>`
- `get_order(order_id: Uuid) -> Option<&Order>`
- `get_best_bid() -> Option<Decimal>`
- `get_best_ask() -> Option<Decimal>`
- `get_spread() -> Option<Decimal>`
- `get_mid_price() -> Option<Decimal>`

**Features:**
- Price-time priority matching
- Automatic matching on order insertion
- Support for limit orders only (Phase 1)
- Partial fill support

#### 2. Matching Engine (`src/engine/matching.rs`)

**Core Logic:**
- Match incoming buy orders against ask side (ascending prices)
- Match incoming sell orders against bid side (descending prices)
- Execute trades at resting order prices (maker gets their price)
- Handle partial fills when quantity exceeds available liquidity
- Update order statuses after matching

**Key Function:**
```rust
pub fn match_order(
    orderbook: &mut OrderBook,
    incoming_order: &mut Order,
) -> Result<Vec<Trade>, MatchingError>
```

#### 3. Fee Calculator (`src/engine/fees.rs`)

**Fee Structure:**
- Maker fee: 0.10% (adds liquidity to book)
- Taker fee: 0.20% (removes liquidity from book)
- Calculate fees per trade
- Track total exchange profit

**Functions:**
```rust
pub fn calculate_maker_fee(trade_value: Decimal) -> Decimal
pub fn calculate_taker_fee(trade_value: Decimal) -> Decimal
pub fn calculate_exchange_profit(trades: &[Trade]) -> Decimal
```

### Phase 2: Enhanced Features

#### 4. Market Orders
- Execute immediately at best available prices
- Walk the book across multiple price levels
- Calculate weighted average execution price

#### 5. Advanced Analytics (`src/metrics/analytics.rs`)
- VWAP calculation
- Order book depth (top N levels)
- Volume at each price level
- Order book imbalance ratio
- Liquidity metrics

#### 6. Trade History Management
- Store last N trades (configurable)
- Query trades by time range
- Calculate trading statistics

### Phase 3: Advanced Features

#### 7. Smart Order Routing
- Optimal execution strategies for large orders
- Minimize market impact
- TWAP/VWAP execution algorithms

#### 8. Performance Optimization
- Lock-free data structures
- Memory pooling
- Zero-copy operations where possible

## REST API Endpoints

### Phase 1 Endpoints

#### Submit Order
```
POST /api/v1/orders
Content-Type: application/json

Request:
{
  "symbol": "AAPL",
  "side": "buy",
  "order_type": "limit",
  "price": 150.50,
  "quantity": 100,
  "user_id": "user123"
}

Response (201 Created):
{
  "order_id": "uuid-here",
  "status": "new",
  "filled_quantity": 0,
  "trades": [],
  "timestamp": "2025-11-12T10:30:00Z"
}

Response with immediate match (201 Created):
{
  "order_id": "uuid-here",
  "status": "filled",
  "filled_quantity": 100,
  "trades": [
    {
      "trade_id": "trade-uuid",
      "price": 150.45,
      "quantity": 100,
      "maker_fee": 0.15045,
      "taker_fee": 0.3009,
      "timestamp": "2025-11-12T10:30:00Z"
    }
  ],
  "timestamp": "2025-11-12T10:30:00Z"
}
```

#### Cancel Order
```
DELETE /api/v1/orders/{order_id}

Response (200 OK):
{
  "order_id": "uuid-here",
  "status": "cancelled",
  "filled_quantity": 0,
  "remaining_quantity": 100
}
```

#### Get Order Status
```
GET /api/v1/orders/{order_id}

Response (200 OK):
{
  "order_id": "uuid-here",
  "symbol": "AAPL",
  "side": "buy",
  "order_type": "limit",
  "price": 150.50,
  "quantity": 100,
  "filled_quantity": 50,
  "status": "partially_filled",
  "timestamp": "2025-11-12T10:30:00Z"
}
```

#### Get Order Book
```
GET /api/v1/orderbook/{symbol}?depth=10

Response (200 OK):
{
  "symbol": "AAPL",
  "timestamp": "2025-11-12T10:30:00Z",
  "bids": [
    {"price": 150.50, "quantity": 1000, "orders": 5},
    {"price": 150.45, "quantity": 500, "orders": 3},
    {"price": 150.40, "quantity": 2000, "orders": 8}
  ],
  "asks": [
    {"price": 150.55, "quantity": 800, "orders": 4},
    {"price": 150.60, "quantity": 1200, "orders": 6},
    {"price": 150.65, "quantity": 500, "orders": 2}
  ],
  "best_bid": 150.50,
  "best_ask": 150.55,
  "spread": 0.05,
  "spread_bps": 3.32,
  "mid_price": 150.525
}
```

#### Get Spread Metrics
```
GET /api/v1/orderbook/{symbol}/spread

Response (200 OK):
{
  "symbol": "AAPL",
  "best_bid": 150.50,
  "best_ask": 150.55,
  "spread_absolute": 0.05,
  "spread_percentage": 0.0332,
  "spread_bps": 3.32,
  "mid_price": 150.525,
  "bid_depth": 3500,
  "ask_depth": 2500,
  "timestamp": "2025-11-12T10:30:00Z"
}
```

#### Get Recent Trades
```
GET /api/v1/trades/{symbol}?limit=50

Response (200 OK):
{
  "symbol": "AAPL",
  "trades": [
    {
      "trade_id": "uuid-here",
      "price": 150.50,
      "quantity": 100,
      "buyer_id": "user123",
      "seller_id": "user456",
      "maker_fee": 0.1505,
      "taker_fee": 0.301,
      "timestamp": "2025-11-12T10:30:00Z"
    }
  ],
  "count": 50
}
```

#### Get Exchange Metrics
```
GET /api/v1/metrics/exchange

Response (200 OK):
{
  "total_trades": 15234,
  "total_volume": 1500000.00,
  "total_fees_collected": 2250.50,
  "maker_fees": 900.20,
  "taker_fees": 1350.30,
  "active_orders": 523,
  "symbols": ["AAPL", "GOOGL", "MSFT"]
}
```

### Phase 2 Endpoints

#### Market Order Support
```
POST /api/v1/orders
{
  "symbol": "AAPL",
  "side": "buy",
  "order_type": "market",
  "quantity": 100,
  "user_id": "user123"
}
```

#### Get Market Analytics
```
GET /api/v1/analytics/{symbol}?interval=1h

Response:
{
  "symbol": "AAPL",
  "vwap": 150.45,
  "high": 151.00,
  "low": 149.50,
  "volume": 125000,
  "trades": 1234,
  "interval": "1h"
}
```

## Error Handling

### Error Types
```rust
pub enum OrderBookError {
    OrderNotFound(Uuid),
    InvalidPrice,
    InvalidQuantity,
    InsufficientLiquidity,
    SelfTrade,
    DuplicateOrder,
    InvalidSymbol,
}
```

### HTTP Status Codes
- 200: Success
- 201: Created (new order)
- 400: Bad Request (invalid input)
- 404: Not Found (order doesn't exist)
- 409: Conflict (duplicate order)
- 500: Internal Server Error

## Performance Requirements

### Latency Targets (Phase 3)
- Order insertion: < 10 microseconds (p99)
- Order matching: < 50 microseconds (p99)
- Order cancellation: < 5 microseconds (p99)
- API response time: < 1 millisecond (p99)

### Throughput Targets
- 100,000+ orders per second (single symbol)
- 50,000+ matches per second

## Testing Strategy

### Unit Tests
- Order book operations
- Matching logic correctness
- Fee calculations
- Spread calculations
- Edge cases (empty book, partial fills, etc.)

### Integration Tests
- Full order lifecycle (submit → match → fill)
- Multiple concurrent orders
- API endpoint testing
- Error handling

### Performance Tests
- Benchmark matching speed
- Memory usage profiling
- Load testing API endpoints

## Implementation Phases

### Phase 1: Core MVP (Week 1)
- [ ] Project setup and dependencies
- [ ] Core data models (Order, Trade, OrderBook)
- [ ] Basic order book with limit orders only
- [ ] Price-time priority matching engine
- [ ] Fee calculation logic
- [ ] REST API with basic endpoints
- [ ] Unit tests for core logic

### Phase 2: Enhanced Features (Week 2)
- [ ] Market order support
- [ ] Advanced spread metrics
- [ ] Trade history management
- [ ] Order book depth visualization
- [ ] Market analytics (VWAP, volume)
- [ ] Integration tests
- [ ] API documentation

### Phase 3: Optimization & Polish (Week 3)
- [ ] Performance benchmarking
- [ ] Optimization of data structures
- [ ] Smart order routing logic
- [ ] WebSocket support for real-time updates
- [ ] Market maker simulation
- [ ] Comprehensive documentation
- [ ] Demo scenarios for interviews

## Interview Talking Points

### Technical Decisions
1. **Why BTreeMap for price levels?**
   - O(log n) insertion/deletion/lookup
   - Maintains sorted order automatically
   - Efficient iteration over price ranges

2. **Why VecDeque for orders at same price?**
   - FIFO semantics for price-time priority
   - O(1) push/pop from both ends
   - Efficient memory layout

3. **Decimal vs float for prices?**
   - Avoid floating-point precision issues
   - Critical for financial calculations
   - Deterministic rounding

4. **Fee structure optimizes exchange profit?**
   - Maker-taker model incentivizes liquidity provision
   - Taker pays more (removes liquidity)
   - Creates sustainable exchange business model

### Market Microstructure Understanding
- Price-time priority vs pro-rata matching
- Impact of spread on market efficiency
- Role of market makers in liquidity provision
- Order book imbalance as market direction indicator
- Hidden liquidity (iceberg orders)

### Scaling Considerations
- Multiple symbols (separate order books)
- Concurrent access (lock-free structures)
- Persistence (write-ahead logging)
- Distributed matching (sharding by symbol)
- Market data distribution

## Dependencies (Cargo.toml)

```toml
[package]
name = "order-book-api"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = "1.33"
rust_decimal_macros = "1.33"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
criterion = "0.5"
```

## Getting Started

1. Initialize project: `cargo new order-book-api`
2. Copy this spec to project root
3. Update Cargo.toml with dependencies
4. Create module structure
5. Implement Phase 1 features
6. Write tests as you go
7. Benchmark performance
8. Document your design decisions

## Success Criteria

- All Phase 1 features working correctly
- Comprehensive test coverage (>80%)
- Clean, idiomatic Rust code
- Clear documentation
- Performance benchmarks showing sub-millisecond latency
- Ability to explain design decisions in interview context

---

**Note**: This specification prioritizes correctness and clarity over premature optimization. Focus on getting Phase 1 working correctly, then optimize in Phase 3 based on benchmarks.
