# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A high-performance order matching engine and REST API built in Rust for demonstrating market microstructure, order book dynamics, and HFT concepts. The system implements price-time priority matching with maker-taker fee structures and real-time WebSocket streaming.

## Build and Test Commands

### Build
```bash
# Build the project
cargo build

# Build with optimizations (release mode)
cargo build --release
```

### Run
```bash
# Run the main API server (starts on http://127.0.0.1:3000)
cargo run

# Run the cTrader FIX streaming demo
./run_streaming_demo.sh

# Run specific binary targets
cargo run --bin ctrader_fix_test
cargo run --bin ctrader_streaming_demo
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Run tests in a specific module
cargo test engine::matching

# Run with verbose output
cargo test -- --show-output
```

### API Testing Scripts
```bash
# Test basic API endpoints
./test_api.sh

# Test Phase 2 features (market data, WebSocket)
./test_phase2_features.sh

# Test market data requests
./test_md_request.sh
```

## Architecture

### Core Module Structure

```
src/
├── main.rs                  # Entry point, server initialization
├── lib.rs                   # Public API exports
├── models/                  # Domain models
│   ├── order.rs            # Order, OrderSide, OrderType, OrderStatus, TimeInForce
│   ├── trade.rs            # Trade execution records
│   └── orderbook.rs        # OrderBook and PriceLevel data structures
├── engine/                  # Matching engine core
│   ├── orderbook.rs        # OrderBookEngine - thread-safe book management
│   ├── matching.rs         # match_order() - price-time priority matching
│   ├── fees.rs             # Fee calculation (maker 0.10%, taker 0.20%)
│   ├── validation.rs       # Order validation logic
│   └── errors.rs           # OrderBookError types
├── api/                     # REST API layer
│   ├── routes.rs           # create_router() with Swagger integration
│   ├── handlers.rs         # HTTP request handlers
│   ├── responses.rs        # Response DTOs
│   └── openapi.rs          # Swagger/OpenAPI configuration
├── websocket/              # Real-time streaming
│   ├── broadcaster.rs      # Broadcaster for WebSocket fanout
│   ├── handler.rs          # WebSocket connection handler
│   └── messages.rs         # WsMessage, OrderBookUpdate, TradeUpdate
├── ctrader_fix/            # cTrader FIX protocol integration
│   ├── client.rs           # CTraderFixClient
│   ├── market_data.rs      # MarketTick, MarketDataParser
│   ├── messages.rs         # FIX message definitions
│   └── ws_bridge.rs        # FixToWebSocketBridge
├── metrics/                # Market metrics calculation
│   └── spread.rs           # Spread calculations (abs, %, bps)
└── utils/                  # Shared utilities
```

### Key Design Patterns

#### Thread-Safe Order Book Engine
The `OrderBookEngine` uses `Arc<RwLock<HashMap<String, OrderBook>>>` for thread-safe access to multiple order books (one per symbol). Operations acquire locks, clone the book, perform mutations, then update the shared state.

```rust
// Pattern used throughout engine/orderbook.rs
let mut book = self.get_or_create_book(symbol);  // Clones under read lock
// ... perform matching and mutations on local copy ...
self.update_book(book);                          // Updates under write lock
```

#### Price-Time Priority Matching
- Uses `BTreeMap` for price levels (O(log n) sorted access)
- Uses `VecDeque` for FIFO order queue at each price level
- Trades execute at the **resting order price** (maker gets their price)
- Matching logic in `engine/matching.rs:match_order()`

#### Decimal Precision for Finance
Uses `rust_decimal::Decimal` throughout instead of `f64` to avoid floating-point precision errors critical in financial calculations.

#### WebSocket Broadcasting
The `Broadcaster` uses `DashMap` for concurrent subscriber management and `tokio::sync::mpsc` channels for message distribution to connected clients.

## cTrader FIX Protocol Integration

The `ctrader_fix` module implements FIX protocol support for connecting to cTrader market data feeds:

- **CTraderFixClient**: TCP client with FIX message encoding/decoding
- **MarketDataParser**: Parses FIX market data snapshots and incremental updates
- **FixToWebSocketBridge**: Bridges FIX market data to internal WebSocket broadcaster
- Binaries: `ctrader_fix_test` and `ctrader_streaming_demo` for testing

## API Endpoints

### Core Trading
- `POST /api/v1/orders` - Submit order
- `DELETE /api/v1/orders/{symbol}/{order_id}` - Cancel order
- `GET /api/v1/orders/{symbol}/{order_id}` - Query order status

### Market Data
- `GET /api/v1/orderbook/{symbol}?depth=N` - Order book snapshot
- `GET /api/v1/orderbook/{symbol}/spread` - Spread metrics
- `GET /api/v1/trades/{symbol}?limit=N` - Recent trades
- `GET /api/v1/metrics/exchange` - Exchange-wide metrics

### Real-Time
- `WS /ws` - WebSocket endpoint for live order book and trade updates

### Documentation
- `GET /swagger-ui` - Interactive Swagger UI
- `GET /api-docs/v1/openapi.json` - OpenAPI v1.0 spec
- `GET /api-docs/v2/openapi.json` - OpenAPI v2.0 spec

## Important Implementation Details

### Helper Function Pattern
Helper functions like `remove_order_from_price_level()` and `add_order_to_price_level()` in `engine/orderbook.rs` are **module-level** (not methods) to keep the `OrderBookEngine` API clean and focused. They operate on `OrderBook` state passed as mutable references.

### Order Lifecycle
1. Validation (`engine/validation.rs`)
2. Matching attempt (`engine/matching.rs`)
3. If not fully filled, add to order book
4. Trades generate fees (maker 0.10%, taker 0.20%)
5. Broadcast updates via WebSocket

### Fee Structure
- **Makers** (add liquidity to book): 0.10% fee - encourages limit orders
- **Takers** (remove liquidity from book): 0.20% fee
- Fee calculation in `engine/fees.rs`
- Exchange profit = total fees collected

### State Management
- Order books stored in `HashMap<String, OrderBook>` keyed by symbol
- Each `OrderBook` contains separate `BTreeMap` for bids and asks
- Each `PriceLevel` contains `VecDeque` of order IDs with FIFO semantics

## Dependencies

Key crates:
- **axum 0.7** - Web framework with WebSocket support
- **tokio** - Async runtime
- **serde/serde_json** - Serialization
- **uuid** - Order and trade IDs
- **rust_decimal** - Precise decimal math
- **utoipa + utoipa-swagger-ui** - OpenAPI/Swagger documentation
- **dashmap** - Concurrent HashMap for WebSocket subscribers
- **bytes + tokio-util** - For FIX protocol handling

## Future Enhancements Roadmap

See `HFT_FEATURES/HFT_FEATURES_ROADMAP.md` for detailed plans including:
- Latency measurement infrastructure with HDR histograms
- Stop-loss and stop-limit orders
- Iceberg/hidden orders
- Order book imbalance and microprice calculations
- TWAP/VWAP execution algorithms
- Binary protocol for ultra-low latency
- Write-Ahead Log (WAL) for persistence
- Circuit breakers and risk controls
- Memory-mapped ring buffers (Disruptor pattern)

## Code Style

- Use explicit error handling with `Result<T, E>` (no panics in production code)
- Prefer `&str` for function parameters, `String` for owned data
- Use `#[derive(Debug, Clone, Serialize, Deserialize)]` for data types
- Document public APIs with `///` doc comments
- Use `tracing` for logging (not `println!`)
- Follow standard Rust naming: `snake_case` for functions/variables, `PascalCase` for types
