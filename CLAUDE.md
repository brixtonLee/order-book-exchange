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

#### Development
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Check without building
cargo check
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
│   ├── orderbook.rs        # OrderBook and PriceLevel data structures
│   └── datasource.rs       # Datasource configuration and metrics
├── engine/                  # Matching engine core
│   ├── orderbook.rs        # OrderBookEngine - thread-safe book management
│   ├── matching.rs         # match_order() - price-time priority matching
│   ├── fees.rs             # Fee calculation (maker 0.10%, taker 0.20%)
│   ├── validation.rs       # Order validation logic
│   └── errors.rs           # OrderBookError types
├── api/                     # REST API layer
│   ├── routes.rs           # create_router() with Swagger integration
│   ├── handlers.rs         # HTTP request handlers
│   ├── datasource_handlers.rs  # Datasource control endpoints
│   ├── responses.rs        # Response DTOs
│   └── openapi.rs          # Swagger/OpenAPI configuration
├── websocket/              # Real-time streaming
│   ├── broadcaster.rs      # Broadcaster for WebSocket fanout
│   ├── handler.rs          # WebSocket connection handler
│   └── messages.rs         # WsMessage, OrderBookUpdate, TradeUpdate
├── rabbitmq/               # RabbitMQ messaging integration
│   ├── config.rs           # RabbitMQConfig, ReconnectConfig, RoutingKeyBuilder
│   ├── publisher.rs        # RabbitMQPublisher with connection pooling
│   └── bridge.rs           # FixToRabbitMQBridge - FIX to RabbitMQ streaming
├── ctrader_fix/            # cTrader FIX protocol integration
│   ├── client.rs           # CTraderFixClient - TCP client with FIX encoding/decoding
│   ├── messages.rs         # FIX message definitions
│   ├── ws_bridge.rs        # FixToWebSocketBridge - FIX to WebSocket conversion
│   ├── market_data/        # Market data parsing
│   │   ├── market_tick.rs  # MarketTick model
│   │   └── tick_parser.rs  # Zero-copy FIX market data parser
│   ├── symbol_data/        # Symbol information
│   │   └── symbol_parser.rs # SymbolData parser
│   └── helpers/            # FIX protocol utilities
│       └── fix_helper.rs   # Field extraction and encoding
├── datasource/             # Datasource management
│   └── manager.rs          # DatasourceManager - FIX connection lifecycle
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

The `ctrader_fix` module implements FIX 4.4 protocol support for connecting to cTrader market data feeds:

- **CTraderFixClient**: TCP client with FIX message encoding/decoding, automatic heartbeat management
- **MarketDataParser**: Zero-copy parsing of FIX market data (MsgType=W snapshots, MsgType=X incremental updates)
- **FixToWebSocketBridge**: Bridges FIX market data to internal WebSocket broadcaster with symbol mapping
- **DatasourceManager**: Manages FIX connection lifecycle, heartbeat tracking, and symbol subscriptions
- Binaries: `ctrader_fix_test` and `ctrader_streaming_demo` for testing

See `CTRADER_FIX_STREAMING.md` for detailed integration guide and WebSocket-like streaming architecture.

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
  - Subscribe to ticker: `{"action":"subscribe","channel":"ticker","symbol":"XAUUSD"}`
  - Subscribe to order book: `{"action":"subscribe","channel":"orderbook","symbol":"AAPL"}`
  - Subscribe to trades: `{"action":"subscribe","channel":"trades"}`

### Datasource Control
- `POST /api/v1/datasource/connect` - Connect to FIX datasource
- `GET /api/v1/datasource/status` - Get connection status and metrics
- `POST /api/v1/datasource/disconnect` - Disconnect from FIX datasource
- `POST /api/v1/datasource/symbols/subscribe` - Subscribe to symbol feed
- `POST /api/v1/datasource/symbols/unsubscribe` - Unsubscribe from symbol feed

### RabbitMQ Messaging
- `POST /api/v1/rabbitmq/connect` - Connect to RabbitMQ server
- `GET /api/v1/rabbitmq/status` - Get RabbitMQ connection status and publisher stats
- `POST /api/v1/rabbitmq/disconnect` - Disconnect from RabbitMQ

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

### FIX Datasource Architecture
- **Shared TCP Writer**: `Arc<Mutex<OwnedWriteHalf>>` enables concurrent heartbeat and request sending
- **Channel-based Streaming**: `tokio::mpsc::unbounded_channel` for tick streaming from FIX parser to WebSocket bridge
- **Background Tasks**: Separate async tasks for message reading, heartbeat sending, and tick broadcasting
- **Symbol Mapping**: Maps cTrader symbol IDs to human-readable names (e.g., `1` → `EURUSD`)
- **Zero-copy Parsing**: Minimizes allocations during FIX message parsing for maximum throughput

### RabbitMQ Integration Architecture
- **Dual Output**: FIX ticks are fan-out to both WebSocket and RabbitMQ bridges in parallel
- **Publisher with Auto-Reconnect**: `RabbitMQPublisher` handles connection pooling, exponential backoff, and automatic reconnection
- **Topic Exchange**: Uses `market.data` topic exchange with routing keys like `tick.EURUSD`, `tick.XAUUSD`
- **Publisher Confirms**: Ensures at-least-once delivery with acknowledgments from broker
- **Message Format**: JSON serialization for cross-platform compatibility
- **Docker Integration**: RabbitMQ runs in Docker with management UI at http://localhost:15672
- **Flow**: FIX Client → Fan-out Task → (WebSocket Bridge + RabbitMQ Bridge) → (WebSocket Clients + RabbitMQ Consumers)

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
- **lapin** - Async AMQP 0.9.1 client for RabbitMQ integration

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
- **Exception Handling Pattern**: Let errors propagate naturally using `?` operator - avoid excessive try-catch blocks

## Module Organization

### Helper Functions Pattern
Module-level helper functions (not methods) are used extensively in `engine/orderbook.rs`:
- Functions like `remove_order_from_price_level()` and `add_order_to_price_level()` operate on `OrderBook` passed as mutable references
- Keeps the `OrderBookEngine` API clean and focused on public operations
- These helpers are private to the module and handle internal state mutations

### Self-Trade Prevention (STP)
The matching engine implements comprehensive STP modes in `engine/matching.rs`:
- `None`: Allow self-trades (skip prevention)
- `CancelResting`: Cancel the resting order when self-trade detected
- `CancelIncoming`: Reject the incoming order
- `CancelBoth`: Cancel both orders
- `CancelSmallest`: Cancel whichever order has smaller remaining quantity
- `DecrementBoth`: Reduce both orders by the matched quantity (prevents execution but maintains book presence)
