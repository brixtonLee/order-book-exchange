# Rust Order Book API

A high-performance order matching engine and REST API built in Rust, demonstrating:
- Order matching algorithms with price-time priority
- Market microstructure and spread dynamics
- Exchange profit optimization through maker-taker fee structures
- Suitable for quant developer interviews at top trading firms

## Features

### Phase 1 (Implemented)

- **Core Order Book Engine**
  - Price-time priority matching
  - Support for limit orders
  - Partial fill support
  - Real-time order matching

- **Fee Structure**
  - Maker fee: 0.10%
  - Taker fee: 0.20%
  - Fee calculation and profit tracking

- **REST API Endpoints**
  - Submit orders
  - Cancel orders
  - Query order status
  - View order book with depth
  - Get spread metrics
  - View recent trades
  - Exchange metrics

- **Market Metrics**
  - Best bid/ask prices
  - Spread calculation (absolute, percentage, basis points)
  - Mid price calculation
  - Order book depth
  - Trade history

## Technology Stack

- **Language**: Rust (2021 edition)
- **Web Framework**: Axum 0.7 (async, high-performance)
- **Serialization**: Serde (JSON)
- **Time**: Chrono
- **UUID**: uuid crate for order IDs
- **Decimal**: rust_decimal for precise financial calculations
- **API Documentation**: utoipa + utoipa-swagger-ui (OpenAPI/Swagger)
- **Testing**: Built-in Rust test framework

## Quick Start

### Prerequisites

- Rust 1.70+ (install from https://rustup.rs/)

### Build and Run

```bash
# Clone or navigate to the project
cd rust-order-book

# Build the project
cargo build

# Run tests
cargo test

# Run the server
cargo run
```

The server will start on `http://127.0.0.1:3000`

## 📚 Interactive API Documentation

This project includes comprehensive **Swagger/OpenAPI documentation** with an interactive UI!

### Access Swagger UI
**URL:** http://127.0.0.1:3000/swagger-ui

Features:
- 🌐 **Interactive API testing** - Try endpoints directly from your browser
- 📖 **Complete documentation** - All endpoints, schemas, and examples
- 🔄 **Version selection** - Switch between API versions (v1.0, v2.0)
- 📋 **Request/Response examples** - See exactly what to send and expect
- 🎯 **Schema browser** - Explore all data models and types

### OpenAPI Specifications
- **v1.0 Spec:** http://127.0.0.1:3000/api-docs/v1/openapi.json
- **v2.0 Spec:** http://127.0.0.1:3000/api-docs/v2/openapi.json

Import these into Postman, Insomnia, or use OpenAPI Generator for client code!

See [SWAGGER_DOCS.md](SWAGGER_DOCS.md) for detailed documentation.

## API Usage Examples

### Health Check

```bash
curl http://127.0.0.1:3000/health
```

### Submit a Sell Order

```bash
curl -X POST http://127.0.0.1:3000/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "sell",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 100,
    "user_id": "seller1"
  }'
```

Response:
```json
{
  "order_id": "uuid-here",
  "status": "new",
  "filled_quantity": "0",
  "trades": [],
  "timestamp": "2025-11-12T10:30:00Z"
}
```

### Submit a Buy Order (Matching)

```bash
curl -X POST http://127.0.0.1:3000/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "buy",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 50,
    "user_id": "buyer1"
  }'
```

Response (with trade):
```json
{
  "order_id": "uuid-here",
  "status": "filled",
  "filled_quantity": "50",
  "trades": [
    {
      "trade_id": "trade-uuid",
      "price": "150.50",
      "quantity": "50",
      "maker_fee": "7.5250",
      "taker_fee": "15.0500",
      "timestamp": "2025-11-12T10:30:01Z"
    }
  ],
  "timestamp": "2025-11-12T10:30:01Z"
}
```

### Get Order Book

```bash
curl "http://127.0.0.1:3000/api/v1/orderbook/AAPL?depth=10"
```

Response:
```json
{
  "symbol": "AAPL",
  "timestamp": "2025-11-12T10:30:00Z",
  "bids": [
    {"price": "150.45", "quantity": "500", "orders": 3}
  ],
  "asks": [
    {"price": "150.55", "quantity": "800", "orders": 4}
  ],
  "best_bid": "150.45",
  "best_ask": "150.55",
  "spread": "0.10",
  "spread_bps": "6.64",
  "mid_price": "150.50"
}
```

### Get Spread Metrics

```bash
curl http://127.0.0.1:3000/api/v1/orderbook/AAPL/spread
```

### Get Recent Trades

```bash
curl "http://127.0.0.1:3000/api/v1/trades/AAPL?limit=50"
```

### Get Order Status

```bash
curl http://127.0.0.1:3000/api/v1/orders/AAPL/{order_id}
```

### Cancel an Order

```bash
curl -X DELETE http://127.0.0.1:3000/api/v1/orders/AAPL/{order_id}
```

### Get Exchange Metrics

```bash
curl http://127.0.0.1:3000/api/v1/metrics/exchange
```

Response:
```json
{
  "total_trades": 1234,
  "total_volume": "1500000.00",
  "total_fees_collected": "2250.50",
  "active_orders": 523,
  "symbols": ["AAPL", "GOOGL", "MSFT"]
}
```

## Project Structure

```
order-book-api/
├── Cargo.toml
├── README.md
├── ORDER_BOOK_API_SPEC.md
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
│   │   ├── orderbook.rs        # Order book engine
│   │   └── fees.rs             # Fee calculation logic
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs           # Route definitions
│   │   ├── handlers.rs         # Request handlers
│   │   └── responses.rs        # Response types
│   ├── metrics/
│   │   ├── mod.rs
│   │   └── spread.rs           # Spread calculations
│   └── utils/
│       ├── mod.rs
│       └── validation.rs       # Input validation
└── tests/
```

## Architecture

### Order Matching Logic

The engine implements **price-time priority** matching:

1. **Buy orders** are matched against the ask side (lowest prices first)
2. **Sell orders** are matched against the bid side (highest prices first)
3. Trades execute at the **resting order price** (maker gets their price)
4. Orders at the same price level are matched in **FIFO order** (first in, first out)

### Data Structures

- **BTreeMap** for price levels: O(log n) operations, maintains sorted order
- **VecDeque** for orders at same price: O(1) push/pop, FIFO semantics
- **HashMap** for quick order lookup: O(1) average case
- **rust_decimal::Decimal** for precise financial calculations

### Fee Structure

The maker-taker model incentivizes liquidity provision:
- **Makers** (add liquidity): 0.10% fee
- **Takers** (remove liquidity): 0.20% fee

This creates sustainable exchange revenue while encouraging market makers.

## Testing

Run all tests:
```bash
cargo test
```

Run with output:
```bash
cargo test -- --nocapture
```

Current test coverage:
- Order creation and lifecycle
- Matching engine correctness
- Fee calculations
- Spread metrics
- Order book operations
- Trade execution

## Performance Characteristics

Current implementation (Phase 1):
- Order insertion: < 1ms
- Order matching: < 1ms
- Order cancellation: < 1ms

Future optimization targets (Phase 3):
- Order insertion: < 10 microseconds (p99)
- Order matching: < 50 microseconds (p99)
- Throughput: 100,000+ orders/second

## Design Decisions

### Why BTreeMap for price levels?
- O(log n) insertion/deletion/lookup
- Maintains sorted order automatically (crucial for matching)
- Efficient iteration over price ranges

### Why VecDeque for orders at same price?
- FIFO semantics for price-time priority
- O(1) push/pop from both ends
- Efficient memory layout

### Why Decimal instead of float?
- Avoids floating-point precision errors
- Critical for financial calculations
- Deterministic rounding

### Why maker-taker fees?
- Incentivizes liquidity provision
- Creates sustainable business model
- Industry standard approach

## Future Enhancements

### Phase 2
- Market order support
- Advanced analytics (VWAP, volume analysis)
- Order book imbalance metrics
- WebSocket support for real-time updates

### Phase 3
- Performance optimization
- Lock-free data structures
- Smart order routing
- Benchmark suite with criterion

## Interview Talking Points

This project demonstrates understanding of:

1. **Market Microstructure**
   - Price-time priority vs pro-rata matching
   - Role of spread in market efficiency
   - Market maker incentives

2. **System Design**
   - High-performance data structure selection
   - Thread-safe concurrent access patterns
   - API design for trading systems

3. **Financial Engineering**
   - Precise decimal arithmetic
   - Fee structure optimization
   - Risk management (self-trade prevention)

4. **Software Engineering**
   - Idiomatic Rust code
   - Comprehensive testing
   - Clear separation of concerns
   - Type-safe error handling

## License

MIT
