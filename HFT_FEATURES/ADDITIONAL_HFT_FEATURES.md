# Additional HFT Features for Advanced Trading Systems

> Comprehensive guide to additional features beyond the core roadmap that will enhance your Rust order book exchange into a production-grade HFT system.

---

## Table of Contents

1. [Market Data Feed Handler](#1-market-data-feed-handler)
2. [Order Routing & Smart Order Routing (SOR)](#2-order-routing--smart-order-routing-sor)
3. [Position & Risk Management](#3-position--risk-management)
4. [Order Matching Engine Enhancements](#4-order-matching-engine-enhancements)
5. [Market Making Strategies](#5-market-making-strategies)
6. [Historical Data Storage & Query](#6-historical-data-storage--query)
7. [Backtesting Engine](#7-backtesting-engine)
8. [Network Optimization](#8-network-optimization)
9. [Reconnection & Session Management](#9-reconnection--session-management)
10. [Admin & Monitoring Dashboard](#10-admin--monitoring-dashboard)
11. [Advanced Order Types](#11-advanced-order-types)
12. [FIX Protocol Implementation](#12-fix-protocol-implementation)
13. [Configuration Hot Reload](#13-configuration-hot-reload)
14. [Order Book Delta Compression](#14-order-book-delta-compression)
15. [Multi-Threading & Work Stealing](#15-multi-threading--work-stealing)
16. [Trade Reporting & Compliance](#16-trade-reporting--compliance)
17. [Advanced Features (Expert Level)](#17-advanced-features-expert-level)

---

## 1. Market Data Feed Handler

### Why This Matters

In production HFT systems, you need to consume market data from multiple sources simultaneously:
- Multiple exchanges (Binance, Coinbase, Kraken)
- Redundant feeds for reliability
- Different protocols (WebSocket, UDP multicast, FIX)

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                Multi-Source Feed Aggregation                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Exchange A (WebSocket)        Exchange B (WebSocket)           │
│       │                              │                          │
│       ▼                              ▼                          │
│  ┌──────────┐                  ┌──────────┐                     │
│  │ Decoder  │                  │ Decoder  │                     │
│  │ Thread 1 │                  │ Thread 2 │                     │
│  └────┬─────┘                  └────┬─────┘                     │
│       │                              │                          │
│       └──────────┬───────────────────┘                          │
│                  ▼                                               │
│         ┌────────────────────┐                                  │
│         │ Sequence Validator │ ─────> Gap detected?            │
│         │ & Normalizer       │         │                        │
│         └────────┬───────────┘         ▼                        │
│                  │              ┌─────────────────┐             │
│                  │              │ Recovery Request│             │
│                  │              │ (resend missing)│             │
│                  │              └─────────────────┘             │
│                  ▼                                               │
│         ┌────────────────────┐                                  │
│         │  Unified Market    │                                  │
│         │  Data Stream       │                                  │
│         └────────────────────┘                                  │
│                  │                                               │
│                  ▼                                               │
│         [Order Book Updates]                                    │
│         [Trade Events]                                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/feeds/mod.rs`**

```rust
pub mod websocket;
pub mod normalizer;
pub mod sequencer;
pub mod aggregator;
```

**File: `src/feeds/websocket.rs`**

```rust
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Generic market data event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataEvent {
    Trade {
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        timestamp: DateTime<Utc>,
        side: OrderSide,
        sequence: u64,
    },
    QuoteUpdate {
        symbol: String,
        bid_price: Decimal,
        bid_quantity: Decimal,
        ask_price: Decimal,
        ask_quantity: Decimal,
        timestamp: DateTime<Utc>,
        sequence: u64,
    },
    BookSnapshot {
        symbol: String,
        bids: Vec<(Decimal, Decimal)>, // (price, quantity)
        asks: Vec<(Decimal, Decimal)>,
        sequence: u64,
        timestamp: DateTime<Utc>,
    },
    BookDelta {
        symbol: String,
        side: OrderSide,
        price: Decimal,
        quantity: Decimal, // 0 = delete level
        sequence: u64,
        timestamp: DateTime<Utc>,
    },
}

/// WebSocket feed handler
pub struct WebSocketFeedHandler {
    url: String,
    exchange_id: String,
    event_sender: mpsc::UnboundedSender<MarketDataEvent>,
    reconnect_delay: Duration,
    max_reconnect_delay: Duration,
}

impl WebSocketFeedHandler {
    pub fn new(
        url: String,
        exchange_id: String,
        event_sender: mpsc::UnboundedSender<MarketDataEvent>,
    ) -> Self {
        Self {
            url,
            exchange_id,
            event_sender,
            reconnect_delay: Duration::from_millis(100),
            max_reconnect_delay: Duration::from_secs(30),
        }
    }

    /// Main event loop with auto-reconnect
    pub async fn run(&mut self) {
        let mut current_delay = self.reconnect_delay;

        loop {
            match self.connect_and_stream().await {
                Ok(_) => {
                    // Connection closed gracefully - reset backoff
                    current_delay = self.reconnect_delay;
                }
                Err(e) => {
                    eprintln!(
                        "[{}] Feed error: {:?}. Reconnecting in {:?}",
                        self.exchange_id, e, current_delay
                    );
                    tokio::time::sleep(current_delay).await;

                    // Exponential backoff
                    current_delay = (current_delay * 2).min(self.max_reconnect_delay);
                }
            }
        }
    }

    async fn connect_and_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        println!("[{}] Connected to feed", self.exchange_id);

        let (mut write, mut read) = ws_stream.split();

        // Send subscription message
        let subscribe_msg = self.create_subscription_message();
        write.send(Message::Text(subscribe_msg)).await?;

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Some(event) = self.parse_message(&text) {
                        let _ = self.event_sender.send(event);
                    }
                }
                Message::Binary(data) => {
                    if let Some(event) = self.parse_binary(&data) {
                        let _ = self.event_sender.send(event);
                    }
                }
                Message::Ping(payload) => {
                    write.send(Message::Pong(payload)).await?;
                }
                Message::Close(_) => {
                    println!("[{}] Connection closed by server", self.exchange_id);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn create_subscription_message(&self) -> String {
        // Exchange-specific subscription format
        // Example for Binance:
        r#"{"method":"SUBSCRIBE","params":["btcusdt@trade","btcusdt@depth"],"id":1}"#.to_string()
    }

    fn parse_message(&self, text: &str) -> Option<MarketDataEvent> {
        // Exchange-specific parsing logic
        // Convert exchange format to unified MarketDataEvent

        // Example stub:
        None
    }

    fn parse_binary(&self, data: &[u8]) -> Option<MarketDataEvent> {
        // Binary protocol parsing (e.g., for custom feeds)
        None
    }
}
```

**File: `src/feeds/sequencer.rs`**

```rust
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;

/// Detects sequence gaps and triggers recovery
pub struct SequenceValidator {
    /// Expected next sequence per symbol
    next_sequence: HashMap<String, u64>,

    /// Buffer for out-of-order messages
    pending_messages: HashMap<String, VecDeque<MarketDataEvent>>,

    /// Maximum gap before requesting snapshot
    max_gap: u64,

    /// Output channel for validated events
    output: mpsc::UnboundedSender<MarketDataEvent>,

    /// Recovery request channel
    recovery_requests: mpsc::UnboundedSender<RecoveryRequest>,
}

#[derive(Debug)]
pub struct RecoveryRequest {
    pub symbol: String,
    pub from_sequence: u64,
    pub to_sequence: u64,
}

impl SequenceValidator {
    pub fn new(
        output: mpsc::UnboundedSender<MarketDataEvent>,
        recovery_requests: mpsc::UnboundedSender<RecoveryRequest>,
    ) -> Self {
        Self {
            next_sequence: HashMap::new(),
            pending_messages: HashMap::new(),
            max_gap: 100,
            output,
            recovery_requests,
        }
    }

    pub fn process_event(&mut self, event: MarketDataEvent) {
        let (symbol, sequence) = match &event {
            MarketDataEvent::Trade { symbol, sequence, .. } => (symbol, *sequence),
            MarketDataEvent::QuoteUpdate { symbol, sequence, .. } => (symbol, *sequence),
            MarketDataEvent::BookSnapshot { symbol, sequence, .. } => {
                // Snapshot resets sequence
                self.next_sequence.insert(symbol.clone(), sequence + 1);
                self.pending_messages.remove(symbol);
                let _ = self.output.send(event);
                return;
            }
            MarketDataEvent::BookDelta { symbol, sequence, .. } => (symbol, *sequence),
        };

        let expected = self.next_sequence.get(symbol).copied().unwrap_or(sequence);

        match sequence.cmp(&expected) {
            std::cmp::Ordering::Equal => {
                // In-order message
                let _ = self.output.send(event);
                self.next_sequence.insert(symbol.clone(), sequence + 1);

                // Check if we can drain pending messages
                self.drain_pending(symbol);
            }
            std::cmp::Ordering::Greater => {
                // Future message - gap detected
                let gap = sequence - expected;

                if gap > self.max_gap {
                    // Request snapshot
                    println!("Gap too large for {}: {} missing. Requesting snapshot.", symbol, gap);
                    let _ = self.recovery_requests.send(RecoveryRequest {
                        symbol: symbol.clone(),
                        from_sequence: expected,
                        to_sequence: sequence,
                    });
                } else {
                    // Buffer for later
                    self.pending_messages
                        .entry(symbol.clone())
                        .or_insert_with(VecDeque::new)
                        .push_back(event);
                }
            }
            std::cmp::Ordering::Less => {
                // Duplicate or late message - ignore
                println!("Ignoring late message for {}: seq {} (expected {})", symbol, sequence, expected);
            }
        }
    }

    fn drain_pending(&mut self, symbol: &str) {
        let Some(pending) = self.pending_messages.get_mut(symbol) else {
            return;
        };

        let expected = self.next_sequence.get(symbol).copied().unwrap_or(0);

        while let Some(event) = pending.front() {
            let seq = self.get_sequence(event);

            if seq == expected {
                let event = pending.pop_front().unwrap();
                let _ = self.output.send(event);
                self.next_sequence.insert(symbol.to_string(), expected + 1);
            } else {
                break;
            }
        }
    }

    fn get_sequence(&self, event: &MarketDataEvent) -> u64 {
        match event {
            MarketDataEvent::Trade { sequence, .. } => *sequence,
            MarketDataEvent::QuoteUpdate { sequence, .. } => *sequence,
            MarketDataEvent::BookSnapshot { sequence, .. } => *sequence,
            MarketDataEvent::BookDelta { sequence, .. } => *sequence,
        }
    }
}
```

### Rust Skills You'll Learn

- **Async WebSocket handling** with `tokio-tungstenite`
- **Channel-based architecture** with `mpsc` for fan-out
- **Exponential backoff** for reconnection
- **Sequence validation** and gap detection
- **Out-of-order message buffering** with `VecDeque`
- **Multi-threaded event processing**

---

## 2. Order Routing & Smart Order Routing (SOR)

### Why This Matters

Smart Order Routing optimizes execution by:
- Finding the best price across multiple venues
- Splitting large orders to minimize market impact
- Considering fees, rebates, and fill probabilities

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                  Smart Order Routing (SOR)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Incoming Order: Buy 10,000 BTC @ Market                        │
│                                                                  │
│         ┌──────────────────────────────────────┐                │
│         │  Venue Analysis                      │                │
│         ├──────────────────────────────────────┤                │
│         │ Exchange A: 3,000 @ $100.05 (-0.1%)  │                │
│         │ Exchange B: 5,000 @ $100.06 (-0.05%) │                │
│         │ Exchange C: 2,000 @ $100.08 (+0.0%)  │                │
│         │ Exchange D: 4,000 @ $100.10 (rebate) │                │
│         └──────────────────────────────────────┘                │
│                         │                                        │
│                         ▼                                        │
│         ┌──────────────────────────────────────┐                │
│         │  Routing Algorithm                   │                │
│         │  - Calculate effective price         │                │
│         │    (price + fees - rebates)          │                │
│         │  - Optimize fill probability         │                │
│         │  - Consider latency to each venue    │                │
│         └──────────────────────────────────────┘                │
│                         │                                        │
│                         ▼                                        │
│         ┌──────────────────────────────────────┐                │
│         │  Order Split Decision                │                │
│         ├──────────────────────────────────────┤                │
│         │ Route 1: Exchange A → 3,000 shares   │                │
│         │ Route 2: Exchange B → 5,000 shares   │                │
│         │ Route 3: Exchange D → 2,000 shares   │                │
│         └──────────────────────────────────────┘                │
│                         │                                        │
│                         ▼                                        │
│            [Simultaneous Execution]                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/routing/sor.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Venue quote with fees
#[derive(Debug, Clone)]
pub struct VenueQuote {
    pub venue_id: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub maker_fee_bps: Decimal,  // Basis points (e.g., 10 = 0.1%)
    pub taker_fee_bps: Decimal,
    pub rebate_bps: Decimal,      // Maker rebate
    pub latency_ms: f64,          // Expected latency
    pub fill_probability: f64,    // Historical fill rate
}

impl VenueQuote {
    /// Calculate effective price including fees
    pub fn effective_price(&self, side: OrderSide, is_maker: bool) -> Decimal {
        let fee_bps = if is_maker {
            self.maker_fee_bps - self.rebate_bps
        } else {
            self.taker_fee_bps
        };

        let fee_multiplier = Decimal::ONE + (fee_bps / Decimal::from(10000));

        match side {
            OrderSide::Buy => self.price * fee_multiplier,
            OrderSide::Sell => self.price / fee_multiplier,
        }
    }

    /// Score for ranking (lower is better for buys)
    pub fn score(&self, side: OrderSide) -> Decimal {
        let effective = self.effective_price(side, false);

        // Penalize for low fill probability and high latency
        let latency_penalty = Decimal::from_f64(self.latency_ms / 1000.0).unwrap_or(Decimal::ZERO);
        let fill_penalty = Decimal::from_f64(1.0 - self.fill_probability).unwrap_or(Decimal::ZERO);

        effective + latency_penalty + fill_penalty
    }
}

/// Smart Order Router
pub struct SmartOrderRouter {
    /// Available venues
    venues: HashMap<String, Box<dyn VenueConnector>>,

    /// Historical fill statistics
    fill_stats: HashMap<String, FillStatistics>,
}

#[derive(Debug, Clone, Default)]
pub struct FillStatistics {
    pub total_orders: u64,
    pub filled_orders: u64,
    pub avg_fill_time_ms: f64,
}

impl FillStatistics {
    pub fn fill_rate(&self) -> f64 {
        if self.total_orders == 0 {
            0.0
        } else {
            self.filled_orders as f64 / self.total_orders as f64
        }
    }
}

/// Routing decision
#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub venue_id: String,
    pub quantity: Decimal,
    pub expected_price: Decimal,
}

impl SmartOrderRouter {
    pub fn new() -> Self {
        Self {
            venues: HashMap::new(),
            fill_stats: HashMap::new(),
        }
    }

    pub fn register_venue(&mut self, venue_id: String, connector: Box<dyn VenueConnector>) {
        self.venues.insert(venue_id, connector);
    }

    /// Find optimal routing for an order
    pub async fn route_order(
        &self,
        symbol: &str,
        side: OrderSide,
        quantity: Decimal,
        limit_price: Option<Decimal>,
    ) -> Vec<RouteDecision> {
        // Gather quotes from all venues
        let mut quotes = Vec::new();

        for (venue_id, connector) in &self.venues {
            if let Some(quote) = connector.get_quote(symbol, side, quantity).await {
                let fill_prob = self.fill_stats
                    .get(venue_id)
                    .map(|s| s.fill_rate())
                    .unwrap_or(0.5);

                quotes.push(VenueQuote {
                    venue_id: venue_id.clone(),
                    fill_probability: fill_prob,
                    ..quote
                });
            }
        }

        // Sort by effective price (best first)
        quotes.sort_by(|a, b| {
            let score_a = a.score(side);
            let score_b = b.score(side);

            match side {
                OrderSide::Buy => score_a.cmp(&score_b),      // Lower is better
                OrderSide::Sell => score_b.cmp(&score_a),     // Higher is better
            }
        });

        // Allocate quantity across venues
        let mut routes = Vec::new();
        let mut remaining = quantity;

        for quote in quotes {
            if remaining <= Decimal::ZERO {
                break;
            }

            // Check price limit
            if let Some(limit) = limit_price {
                match side {
                    OrderSide::Buy if quote.price > limit => continue,
                    OrderSide::Sell if quote.price < limit => continue,
                    _ => {}
                }
            }

            let allocation = remaining.min(quote.quantity);

            routes.push(RouteDecision {
                venue_id: quote.venue_id,
                quantity: allocation,
                expected_price: quote.effective_price(side, false),
            });

            remaining -= allocation;
        }

        routes
    }

    /// Update fill statistics after order completion
    pub fn record_fill(&mut self, venue_id: &str, filled: bool, fill_time_ms: f64) {
        let stats = self.fill_stats.entry(venue_id.to_string()).or_default();

        stats.total_orders += 1;
        if filled {
            stats.filled_orders += 1;
        }

        // Exponential moving average
        let alpha = 0.1;
        stats.avg_fill_time_ms = alpha * fill_time_ms + (1.0 - alpha) * stats.avg_fill_time_ms;
    }
}

/// Trait for venue connectivity
#[async_trait::async_trait]
pub trait VenueConnector: Send + Sync {
    async fn get_quote(&self, symbol: &str, side: OrderSide, quantity: Decimal) -> Option<VenueQuote>;
    async fn submit_order(&self, order: Order) -> Result<String, VenueError>;
    async fn cancel_order(&self, order_id: &str) -> Result<(), VenueError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VenueError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),

    #[error("Venue unavailable")]
    Unavailable,
}
```

### Rust Skills You'll Learn

- **Trait objects** for polymorphic venue connectors
- **Async trait methods** with `async_trait`
- **Complex sorting algorithms** with custom comparators
- **Statistical tracking** with exponential moving averages
- **Resource allocation** algorithms

---

## 3. Position & Risk Management

### Why This Matters

Risk management prevents catastrophic losses:
- Real-time P&L tracking
- Position limit enforcement
- Exposure monitoring
- Margin calculations

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                 Position & Risk Management                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Position Tracker                                         │   │
│  │ ┌──────────────┬──────────────┬──────────────┐          │   │
│  │ │ Symbol       │ Net Position │ Avg Price    │          │   │
│  │ ├──────────────┼──────────────┼──────────────┤          │   │
│  │ │ BTC-USD      │ +150.5       │ $99,850.00   │          │   │
│  │ │ ETH-USD      │ -200.0       │ $3,500.50    │          │   │
│  │ │ SOL-USD      │ +1,000.0     │ $125.75      │          │   │
│  │ └──────────────┴──────────────┴──────────────┘          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ P&L Calculator                                           │   │
│  │                                                          │   │
│  │  Realized P&L:   $12,450.00 (from closed trades)        │   │
│  │  Unrealized P&L: -$3,200.00 (mark-to-market)            │   │
│  │  Total P&L:      $9,250.00                              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Risk Checks (Pre-Trade)                                  │   │
│  │                                                          │   │
│  │  ✓ Position limit:     150/500 BTC (OK)                 │   │
│  │  ✓ Gross exposure:     $45M / $100M limit (OK)          │   │
│  │  ✗ Max loss today:     -$51,000 / -$50,000 (REJECT!)    │   │
│  │  ✓ Margin available:   $250,000 / $100,000 (OK)         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/risk/position.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Position for a single symbol
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,

    /// Net quantity (positive = long, negative = short)
    pub net_quantity: Decimal,

    /// Average entry price
    pub avg_price: Decimal,

    /// Realized P&L (from closed trades)
    pub realized_pnl: Decimal,

    /// Total buy quantity (for FIFO/LIFO tracking)
    pub total_bought: Decimal,

    /// Total sell quantity
    pub total_sold: Decimal,

    /// Last update time
    pub last_updated: DateTime<Utc>,
}

impl Position {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            net_quantity: Decimal::ZERO,
            avg_price: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            total_bought: Decimal::ZERO,
            total_sold: Decimal::ZERO,
            last_updated: Utc::now(),
        }
    }

    /// Apply a trade to the position
    pub fn apply_trade(&mut self, side: OrderSide, quantity: Decimal, price: Decimal) {
        let signed_qty = match side {
            OrderSide::Buy => quantity,
            OrderSide::Sell => -quantity,
        };

        let old_qty = self.net_quantity;
        let new_qty = old_qty + signed_qty;

        // Check if trade closes or flips position
        if old_qty.signum() != new_qty.signum() && !old_qty.is_zero() {
            // Position flip - realize P&L on closed portion
            let closed_qty = old_qty.abs().min(quantity);
            self.realized_pnl += self.calculate_pnl(old_qty, closed_qty, price);

            // Remaining quantity opens new position
            let remaining = quantity - closed_qty;
            if remaining > Decimal::ZERO {
                self.avg_price = price;
            }
        } else if new_qty.abs() > old_qty.abs() {
            // Increasing position - update average price
            let added_value = quantity * price;
            let old_value = old_qty.abs() * self.avg_price;
            self.avg_price = (old_value + added_value) / new_qty.abs();
        } else {
            // Reducing position - realize P&L
            self.realized_pnl += self.calculate_pnl(old_qty, quantity, price);
        }

        self.net_quantity = new_qty;
        self.last_updated = Utc::now();

        match side {
            OrderSide::Buy => self.total_bought += quantity,
            OrderSide::Sell => self.total_sold += quantity,
        }
    }

    fn calculate_pnl(&self, position_qty: Decimal, trade_qty: Decimal, trade_price: Decimal) -> Decimal {
        let sign = position_qty.signum();
        (trade_price - self.avg_price) * trade_qty * sign
    }

    /// Calculate unrealized P&L at current market price
    pub fn unrealized_pnl(&self, market_price: Decimal) -> Decimal {
        (market_price - self.avg_price) * self.net_quantity
    }

    /// Total P&L
    pub fn total_pnl(&self, market_price: Decimal) -> Decimal {
        self.realized_pnl + self.unrealized_pnl(market_price)
    }
}

/// Portfolio-level position manager
pub struct PositionManager {
    positions: HashMap<String, Position>,
    market_prices: HashMap<String, Decimal>,
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            market_prices: HashMap::new(),
        }
    }

    pub fn update_market_price(&mut self, symbol: &str, price: Decimal) {
        self.market_prices.insert(symbol.to_string(), price);
    }

    pub fn apply_trade(&mut self, symbol: &str, side: OrderSide, quantity: Decimal, price: Decimal) {
        let position = self.positions
            .entry(symbol.to_string())
            .or_insert_with(|| Position::new(symbol.to_string()));

        position.apply_trade(side, quantity, price);
    }

    pub fn get_position(&self, symbol: &str) -> Option<&Position> {
        self.positions.get(symbol)
    }

    /// Total unrealized P&L across all positions
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions
            .iter()
            .map(|(symbol, pos)| {
                let market_price = self.market_prices
                    .get(symbol)
                    .copied()
                    .unwrap_or(pos.avg_price);
                pos.unrealized_pnl(market_price)
            })
            .sum()
    }

    /// Total realized P&L
    pub fn total_realized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.realized_pnl).sum()
    }

    /// Gross exposure (sum of absolute notional values)
    pub fn gross_exposure(&self) -> Decimal {
        self.positions
            .iter()
            .map(|(symbol, pos)| {
                let market_price = self.market_prices
                    .get(symbol)
                    .copied()
                    .unwrap_or(pos.avg_price);
                pos.net_quantity.abs() * market_price
            })
            .sum()
    }
}
```

**File: `src/risk/limits.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Risk limits configuration
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Max position size per symbol
    pub max_position_per_symbol: HashMap<String, Decimal>,

    /// Max gross exposure (notional)
    pub max_gross_exposure: Decimal,

    /// Max net exposure
    pub max_net_exposure: Decimal,

    /// Max daily loss
    pub max_daily_loss: Decimal,

    /// Max order size
    pub max_order_size: Decimal,

    /// Minimum margin required
    pub min_margin_required: Decimal,
}

/// Risk checker
pub struct RiskChecker {
    limits: RiskLimits,
    position_manager: PositionManager,
    daily_pnl_start: Decimal,
}

#[derive(Debug, thiserror::Error)]
pub enum RiskViolation {
    #[error("Position limit exceeded for {symbol}: {current} > {limit}")]
    PositionLimitExceeded {
        symbol: String,
        current: Decimal,
        limit: Decimal,
    },

    #[error("Gross exposure limit exceeded: {current} > {limit}")]
    GrossExposureLimitExceeded {
        current: Decimal,
        limit: Decimal,
    },

    #[error("Daily loss limit exceeded: {current} > {limit}")]
    DailyLossLimitExceeded {
        current: Decimal,
        limit: Decimal,
    },

    #[error("Order size too large: {size} > {limit}")]
    OrderSizeTooLarge {
        size: Decimal,
        limit: Decimal,
    },

    #[error("Insufficient margin: {available} < {required}")]
    InsufficientMargin {
        available: Decimal,
        required: Decimal,
    },
}

impl RiskChecker {
    /// Check if an order would violate risk limits
    pub fn check_order(
        &self,
        symbol: &str,
        side: OrderSide,
        quantity: Decimal,
        price: Decimal,
    ) -> Result<(), RiskViolation> {
        // 1. Check order size
        if quantity > self.limits.max_order_size {
            return Err(RiskViolation::OrderSizeTooLarge {
                size: quantity,
                limit: self.limits.max_order_size,
            });
        }

        // 2. Check position limit (projected after this order)
        let current_pos = self.position_manager
            .get_position(symbol)
            .map(|p| p.net_quantity)
            .unwrap_or(Decimal::ZERO);

        let signed_qty = match side {
            OrderSide::Buy => quantity,
            OrderSide::Sell => -quantity,
        };

        let projected_pos = (current_pos + signed_qty).abs();

        if let Some(&limit) = self.limits.max_position_per_symbol.get(symbol) {
            if projected_pos > limit {
                return Err(RiskViolation::PositionLimitExceeded {
                    symbol: symbol.to_string(),
                    current: projected_pos,
                    limit,
                });
            }
        }

        // 3. Check daily loss limit
        let current_pnl = self.position_manager.total_realized_pnl()
            + self.position_manager.total_unrealized_pnl();
        let daily_pnl = current_pnl - self.daily_pnl_start;

        if daily_pnl < -self.limits.max_daily_loss {
            return Err(RiskViolation::DailyLossLimitExceeded {
                current: daily_pnl.abs(),
                limit: self.limits.max_daily_loss,
            });
        }

        // 4. Check gross exposure (would need to calculate projected exposure)
        // ... implementation omitted for brevity

        Ok(())
    }
}
```

### Rust Skills You'll Learn

- **Financial calculations** with decimal precision
- **FIFO/LIFO accounting** logic
- **HashMap-based state management**
- **Error types** with `thiserror`
- **Pre-trade risk checks**

---

## 4. Order Matching Engine Enhancements

### 4a. Pro-Rata Matching Algorithm

Beyond FIFO (First In First Out), professional exchanges use:

**Pro-Rata**: Fills are distributed proportionally based on order size

```
Price Level: $100.00
Orders at this level:
  Order A: 1,000 shares (20%)
  Order B: 3,000 shares (60%)
  Order C: 1,000 shares (20%)
  Total:   5,000 shares

Incoming sell: 2,000 shares
Pro-rata fills:
  Order A: 400 shares (20% of 2,000)
  Order B: 1,200 shares (60% of 2,000)
  Order C: 400 shares (20% of 2,000)
```

**File: `src/engine/pro_rata.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::VecDeque;

pub struct ProRataMatchingEngine {
    // ... existing fields
}

impl ProRataMatchingEngine {
    /// Match using pro-rata algorithm
    pub fn match_pro_rata(
        &mut self,
        incoming: &mut Order,
        level: &mut PriceLevel,
    ) -> Vec<Trade> {
        let mut trades = Vec::new();
        let incoming_remaining = incoming.remaining_quantity();

        if incoming_remaining.is_zero() || level.total_quantity.is_zero() {
            return trades;
        }

        // Calculate each order's proportion
        let mut allocations: Vec<(Uuid, Decimal)> = level.orders
            .iter()
            .map(|order_id| {
                let order = self.orders.get(order_id).unwrap();
                let proportion = order.visible_quantity() / level.total_quantity;
                let allocation = (proportion * incoming_remaining).floor();
                (*order_id, allocation)
            })
            .collect();

        // Distribute remaining fractional shares (if any) by time priority
        let allocated_total: Decimal = allocations.iter().map(|(_, qty)| qty).sum();
        let mut remainder = incoming_remaining - allocated_total;

        for (order_id, allocation) in allocations.iter_mut() {
            if remainder.is_zero() {
                break;
            }

            *allocation += Decimal::ONE;
            remainder -= Decimal::ONE;
        }

        // Execute trades
        for (order_id, qty) in allocations {
            if qty.is_zero() {
                continue;
            }

            let resting = self.orders.get_mut(&order_id).unwrap();
            let trade = self.execute_trade(incoming, resting, qty);
            trades.push(trade);

            if resting.is_filled() {
                level.orders.retain(|id| *id != order_id);
            }

            if incoming.is_filled() {
                break;
            }
        }

        level.total_quantity = level.orders
            .iter()
            .map(|id| self.orders.get(id).unwrap().visible_quantity())
            .sum();

        trades
    }
}
```

### 4b. Auction Mechanism

**Opening Auction**: Batch match all orders at a single equilibrium price

```
┌─────────────────────────────────────────────────────────────────┐
│                    Opening Auction Process                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Pre-Open: Collect orders (no matching)                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Buy Orders          │ Sell Orders                        │   │
│  │ 1,000 @ $100.10     │ 500 @ $99.90                       │   │
│  │ 2,000 @ $100.05     │ 1,500 @ $100.00                    │   │
│  │ 3,000 @ $100.00     │ 2,000 @ $100.05                    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Find Equilibrium Price (maximize volume):                      │
│  Try $100.00:                                                    │
│    - Buyable: 6,000 (all buy orders >= $100.00)                │
│    - Sellable: 2,000 (sell orders <= $100.00)                  │
│    - Volume: min(6000, 2000) = 2,000 ✓                         │
│                                                                  │
│  Try $100.05:                                                    │
│    - Buyable: 3,000                                             │
│    - Sellable: 4,000                                            │
│    - Volume: 3,000 ✓✓ (higher, choose this!)                   │
│                                                                  │
│  Uncrossing: Match all at $100.05                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**File: `src/engine/auction.rs`**

```rust
use rust_decimal::Decimal;
use std::collections::BTreeMap;

pub struct AuctionEngine {
    /// Pending buy orders
    bids: BTreeMap<Decimal, Vec<Order>>,  // price -> orders

    /// Pending sell orders
    asks: BTreeMap<Decimal, Vec<Order>>,

    /// Auction state
    state: AuctionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionState {
    Collecting,    // Accepting orders, no matching
    Uncrossing,    // Calculating equilibrium
    Continuous,    // Normal trading
}

impl AuctionEngine {
    /// Find equilibrium price that maximizes matched volume
    pub fn find_equilibrium_price(&self) -> Option<Decimal> {
        let mut max_volume = Decimal::ZERO;
        let mut equilibrium_price = None;

        // Get all unique prices
        let mut all_prices: Vec<Decimal> = self.bids.keys()
            .chain(self.asks.keys())
            .copied()
            .collect();
        all_prices.sort();

        for price in all_prices {
            let buy_volume = self.cumulative_buy_volume_at(price);
            let sell_volume = self.cumulative_sell_volume_at(price);
            let volume = buy_volume.min(sell_volume);

            if volume > max_volume {
                max_volume = volume;
                equilibrium_price = Some(price);
            }
        }

        equilibrium_price
    }

    fn cumulative_buy_volume_at(&self, price: Decimal) -> Decimal {
        self.bids
            .range(price..)  // All bids >= price
            .flat_map(|(_, orders)| orders)
            .map(|o| o.quantity)
            .sum()
    }

    fn cumulative_sell_volume_at(&self, price: Decimal) -> Decimal {
        self.asks
            .range(..=price)  // All asks <= price
            .flat_map(|(_, orders)| orders)
            .map(|o| o.quantity)
            .sum()
    }

    /// Uncross the auction at equilibrium price
    pub fn uncross(&mut self) -> Vec<Trade> {
        let Some(equilibrium) = self.find_equilibrium_price() else {
            return Vec::new();
        };

        let mut trades = Vec::new();

        // Collect all matchable orders
        let mut buy_orders: Vec<Order> = self.bids
            .range(equilibrium..)
            .flat_map(|(_, orders)| orders.clone())
            .collect();

        let mut sell_orders: Vec<Order> = self.asks
            .range(..=equilibrium)
            .flat_map(|(_, orders)| orders.clone())
            .collect();

        // Sort by time priority
        buy_orders.sort_by_key(|o| o.timestamp);
        sell_orders.sort_by_key(|o| o.timestamp);

        // Match orders
        let mut buy_idx = 0;
        let mut sell_idx = 0;

        while buy_idx < buy_orders.len() && sell_idx < sell_orders.len() {
            let buy = &mut buy_orders[buy_idx];
            let sell = &mut sell_orders[sell_idx];

            let match_qty = buy.remaining_quantity().min(sell.remaining_quantity());

            trades.push(Trade {
                id: Uuid::new_v4(),
                symbol: buy.symbol.clone(),
                price: equilibrium,
                quantity: match_qty,
                buyer_order_id: buy.id,
                seller_order_id: sell.id,
                timestamp: Utc::now(),
                is_buyer_maker: false,  // Auction trades have no maker/taker
            });

            buy.filled_quantity += match_qty;
            sell.filled_quantity += match_qty;

            if buy.is_filled() {
                buy_idx += 1;
            }
            if sell.is_filled() {
                sell_idx += 1;
            }
        }

        // Clear matched orders from book
        self.bids.clear();
        self.asks.clear();

        // Re-add unfilled orders
        for order in buy_orders.into_iter().chain(sell_orders) {
            if !order.is_filled() {
                self.add_order_to_book(order);
            }
        }

        self.state = AuctionState::Continuous;
        trades
    }

    fn add_order_to_book(&mut self, order: Order) {
        let map = match order.side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        map.entry(order.price.unwrap())
            .or_insert_with(Vec::new)
            .push(order);
    }
}
```

### Rust Skills You'll Learn

- **BTreeMap range queries** for efficient price level scanning
- **Equilibrium calculation** algorithms
- **Batch processing** vs continuous matching
- **Custom sorting** with multiple criteria

---

## 5. Market Making Strategies

### Why This Matters

Market makers provide liquidity by:
- Quoting both bid and ask prices
- Capturing the spread
- Managing inventory risk
- Adjusting quotes based on market conditions

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                  Market Making Strategy                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Fair Value Estimation:                                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Microprice: $100.025 (from order book imbalance)        │   │
│  │ Last Trade: $100.030                                     │   │
│  │ Fair Value: $100.027 (weighted average)                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Inventory Skewing:                                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Current Inventory: +50 BTC (long)                        │   │
│  │ Target Inventory:  0 BTC (neutral)                       │   │
│  │ Skew:              -0.01% (lean towards selling)         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Quote Calculation:                                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Base Spread:       0.05% ($50)                           │   │
│  │ Inventory Skew:    -$10 (widen ask, tighten bid)        │   │
│  │ Volatility Adj:    +$5 (wider in volatile markets)      │   │
│  │                                                          │   │
│  │ Final Quotes:                                            │   │
│  │   Bid: $100.002  (100.027 - 0.025% - 0.005%)            │   │
│  │   Ask: $100.057  (100.027 + 0.025% + 0.005%)            │   │
│  │   Size: 10 BTC each side                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/strategies/market_maker.rs`**

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Market making strategy
pub struct MarketMakerStrategy {
    /// Target inventory (usually 0 for neutral)
    pub target_inventory: Decimal,

    /// Base spread (in percentage)
    pub base_spread_pct: Decimal,

    /// Inventory risk aversion (higher = more aggressive skewing)
    pub inventory_risk_aversion: Decimal,

    /// Quote size
    pub quote_size: Decimal,

    /// Maximum inventory deviation before stopping
    pub max_inventory_deviation: Decimal,

    /// Current position
    current_inventory: Decimal,
}

impl MarketMakerStrategy {
    pub fn new(quote_size: Decimal) -> Self {
        Self {
            target_inventory: Decimal::ZERO,
            base_spread_pct: dec!(0.05),  // 5 bps
            inventory_risk_aversion: dec!(0.5),
            quote_size,
            max_inventory_deviation: dec!(100.0),
            current_inventory: Decimal::ZERO,
        }
    }

    /// Calculate bid/ask quotes
    pub fn calculate_quotes(
        &self,
        fair_value: Decimal,
        current_volatility: Decimal,
    ) -> Option<(Decimal, Decimal)> {
        // Check if inventory is within limits
        let inventory_deviation = (self.current_inventory - self.target_inventory).abs();
        if inventory_deviation > self.max_inventory_deviation {
            return None;  // Stop quoting if too much inventory
        }

        // Calculate inventory skew
        let inventory_diff = self.current_inventory - self.target_inventory;
        let skew = -inventory_diff * self.inventory_risk_aversion * dec!(0.0001);

        // Adjust spread based on volatility
        let vol_adjustment = current_volatility * dec!(0.5);
        let adjusted_spread = self.base_spread_pct + vol_adjustment;

        // Calculate half-spread
        let half_spread = fair_value * (adjusted_spread / dec!(200.0));

        // Apply skew
        let bid_price = fair_value - half_spread - (fair_value * skew);
        let ask_price = fair_value + half_spread - (fair_value * skew);

        Some((bid_price, ask_price))
    }

    /// Update inventory after a fill
    pub fn on_fill(&mut self, side: OrderSide, quantity: Decimal) {
        match side {
            OrderSide::Buy => self.current_inventory += quantity,
            OrderSide::Sell => self.current_inventory -= quantity,
        }
    }

    /// Calculate expected profit per round-trip
    pub fn expected_profit_per_round_trip(&self, fair_value: Decimal) -> Decimal {
        fair_value * (self.base_spread_pct / dec!(100.0)) * self.quote_size
    }
}
```

**File: `src/strategies/fair_value.rs`**

```rust
use rust_decimal::Decimal;

/// Fair value estimator
pub struct FairValueEstimator {
    /// Weight for microprice
    pub microprice_weight: Decimal,

    /// Weight for last trade price
    pub last_trade_weight: Decimal,
}

impl FairValueEstimator {
    pub fn new() -> Self {
        Self {
            microprice_weight: dec!(0.7),
            last_trade_weight: dec!(0.3),
        }
    }

    pub fn estimate(
        &self,
        microprice: Decimal,
        last_trade_price: Decimal,
    ) -> Decimal {
        microprice * self.microprice_weight + last_trade_price * self.last_trade_weight
    }
}
```

### Rust Skills You'll Learn

- **Financial strategy modeling**
- **Inventory management** algorithms
- **Dynamic pricing** calculations
- **Risk-adjusted quoting**

---

## 6. Historical Data Storage & Query

### Why This Matters

Storing and querying historical data efficiently enables:
- Backtesting strategies
- Performance analysis
- Regulatory compliance
- Market research

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                Time-Series Data Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Write Path (Hot Data):                                          │
│  ┌──────────┐    ┌──────────┐    ┌──────────────┐              │
│  │  Trade   │───▶│ In-Memory│───▶│ Write-Ahead  │              │
│  │  Event   │    │  Buffer   │    │ Log (WAL)    │              │
│  └──────────┘    └──────────┘    └──────────────┘              │
│                         │                                        │
│                         ▼                                        │
│                  ┌──────────────┐                                │
│                  │ Flush to Disk│ (every 1s or 10k trades)      │
│                  │ (Parquet)    │                                │
│                  └──────────────┘                                │
│                                                                  │
│  Storage Layout:                                                 │
│  data/                                                           │
│    ├── trades/                                                   │
│    │   ├── 2025-01-15/                                          │
│    │   │   ├── BTC-USD.parquet    (columnar, compressed)       │
│    │   │   └── ETH-USD.parquet                                 │
│    │   └── 2025-01-16/                                          │
│    ├── quotes/                                                   │
│    └── orderbook_snapshots/                                     │
│                                                                  │
│  Query Path:                                                     │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ Range Query  │───▶│ Load Parquet │───▶│ Arrow Arrays │      │
│  │ (time range) │    │ Files        │    │ (zero-copy)  │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/storage/timeseries.rs`**

```rust
use parquet::file::writer::SerializedFileWriter;
use parquet::file::reader::FileReader;
use arrow::array::{Int64Array, Float64Array, StringArray};
use arrow::datatypes::{Schema, Field, DataType};
use arrow::record_batch::RecordBatch;
use std::fs::File;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Trade record for storage
#[derive(Debug, Clone)]
pub struct StoredTrade {
    pub timestamp_ns: i64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub side: String,
    pub trade_id: String,
}

/// Time-series storage engine
pub struct TimeSeriesStorage {
    base_path: PathBuf,
}

impl TimeSeriesStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Write trades to Parquet file
    pub fn write_trades(
        &self,
        date: DateTime<Utc>,
        symbol: &str,
        trades: Vec<StoredTrade>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create directory structure
        let dir = self.base_path
            .join("trades")
            .join(date.format("%Y-%m-%d").to_string());
        std::fs::create_dir_all(&dir)?;

        let file_path = dir.join(format!("{}.parquet", symbol));
        let file = File::create(file_path)?;

        // Define schema
        let schema = Schema::new(vec![
            Field::new("timestamp_ns", DataType::Int64, false),
            Field::new("symbol", DataType::Utf8, false),
            Field::new("price", DataType::Float64, false),
            Field::new("quantity", DataType::Float64, false),
            Field::new("side", DataType::Utf8, false),
            Field::new("trade_id", DataType::Utf8, false),
        ]);

        // Build Arrow arrays
        let timestamps: Vec<i64> = trades.iter().map(|t| t.timestamp_ns).collect();
        let symbols: Vec<&str> = trades.iter().map(|t| t.symbol.as_str()).collect();
        let prices: Vec<f64> = trades.iter().map(|t| t.price).collect();
        let quantities: Vec<f64> = trades.iter().map(|t| t.quantity).collect();
        let sides: Vec<&str> = trades.iter().map(|t| t.side.as_str()).collect();
        let ids: Vec<&str> = trades.iter().map(|t| t.trade_id.as_str()).collect();

        let batch = RecordBatch::try_new(
            Arc::new(schema.clone()),
            vec![
                Arc::new(Int64Array::from(timestamps)),
                Arc::new(StringArray::from(symbols)),
                Arc::new(Float64Array::from(prices)),
                Arc::new(Float64Array::from(quantities)),
                Arc::new(StringArray::from(sides)),
                Arc::new(StringArray::from(ids)),
            ],
        )?;

        // Write to Parquet
        let props = parquet::file::properties::WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();

        let mut writer = SerializedFileWriter::new(file, Arc::new(schema), Arc::new(props))?;

        // Convert Arrow to Parquet (actual conversion code omitted for brevity)
        // ...

        writer.close()?;
        Ok(())
    }

    /// Query trades in a time range
    pub fn query_trades(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<StoredTrade>, Box<dyn std::error::Error>> {
        let mut all_trades = Vec::new();

        // Iterate over dates in range
        let mut current_date = start.date_naive();
        let end_date = end.date_naive();

        while current_date <= end_date {
            let file_path = self.base_path
                .join("trades")
                .join(current_date.format("%Y-%m-%d").to_string())
                .join(format!("{}.parquet", symbol));

            if file_path.exists() {
                let trades = self.read_parquet_file(&file_path, start, end)?;
                all_trades.extend(trades);
            }

            current_date = current_date.succ_opt().unwrap();
        }

        Ok(all_trades)
    }

    fn read_parquet_file(
        &self,
        path: &PathBuf,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<StoredTrade>, Box<dyn std::error::Error>> {
        // Read Parquet file and filter by timestamp
        // Actual implementation omitted for brevity
        Ok(Vec::new())
    }
}
```

**Alternative: Using `sled` for embedded database**

```rust
use sled::{Db, IVec};
use bincode;

/// Simple key-value storage using sled
pub struct SledStorage {
    db: Db,
}

impl SledStorage {
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Store a trade
    pub fn store_trade(&self, trade: &StoredTrade) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}:{}", trade.symbol, trade.timestamp_ns);
        let value = bincode::serialize(trade)?;
        self.db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// Query trades by symbol and time range
    pub fn query_trades(
        &self,
        symbol: &str,
        start_ns: i64,
        end_ns: i64,
    ) -> Result<Vec<StoredTrade>, Box<dyn std::error::Error>> {
        let prefix = format!("{}:", symbol);
        let mut trades = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if let Some(timestamp_str) = key_str.split(':').nth(1) {
                let timestamp: i64 = timestamp_str.parse()?;

                if timestamp >= start_ns && timestamp <= end_ns {
                    let trade: StoredTrade = bincode::deserialize(&value)?;
                    trades.push(trade);
                }
            }
        }

        Ok(trades)
    }
}
```

### Dependencies

```toml
[dependencies]
parquet = "53.0"
arrow = "53.0"
sled = "0.34"
bincode = "1.3"
```

### Rust Skills You'll Learn

- **Apache Arrow** for columnar data processing
- **Parquet** file format for efficient storage
- **Embedded databases** (`sled`)
- **File I/O** and directory management
- **Serialization** with `bincode`
- **Range queries** and indexing

---

## 7. Backtesting Engine

### Why This Matters

Backtesting validates strategies against historical data:
- Test strategies before risking real capital
- Optimize parameters
- Measure performance metrics (Sharpe ratio, max drawdown)
- Realistic fill simulation (slippage, latency)

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Backtesting Architecture                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Historical Data → Event Stream → Strategy → Simulated Exchange │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ Load trades  │───▶│ Market Event │───▶│ Strategy     │      │
│  │ & quotes     │    │ (chronological)│  │ Logic        │      │
│  └──────────────┘    └──────────────┘    └──────┬───────┘      │
│                                                  │              │
│                                                  ▼              │
│                                          ┌──────────────┐       │
│                                          │ Order        │       │
│                                          └──────┬───────┘       │
│                                                  │              │
│                                                  ▼              │
│                                          ┌──────────────┐       │
│                                          │ Simulated    │       │
│                                          │ Matching     │       │
│                                          │ Engine       │       │
│                                          └──────┬───────┘       │
│                                                  │              │
│                                                  ▼              │
│                                          ┌──────────────┐       │
│                                          │ Fill Event   │       │
│                                          │ (+ slippage) │       │
│                                          └──────┬───────┘       │
│                                                  │              │
│                                                  ▼              │
│                                          ┌──────────────┐       │
│                                          │ P&L Tracker  │       │
│                                          └──────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Code

**File: `src/backtest/engine.rs`**

```rust
use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Historical market event
#[derive(Debug, Clone)]
pub enum HistoricalEvent {
    Trade {
        timestamp: DateTime<Utc>,
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        side: OrderSide,
    },
    Quote {
        timestamp: DateTime<Utc>,
        symbol: String,
        bid: Decimal,
        ask: Decimal,
        bid_size: Decimal,
        ask_size: Decimal,
    },
}

/// Backtest configuration
pub struct BacktestConfig {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub commission_bps: Decimal,
    pub slippage_bps: Decimal,
    pub latency_ms: u64,
}

/// Backtesting engine
pub struct BacktestEngine {
    config: BacktestConfig,

    /// Simulated order book
    order_book: HashMap<String, SimulatedOrderBook>,

    /// Pending orders
    orders: HashMap<Uuid, Order>,

    /// Position tracker
    positions: PositionManager,

    /// Performance metrics
    metrics: PerformanceMetrics,

    /// Current simulation time
    current_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub total_pnl: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig) -> Self {
        Self {
            config,
            order_book: HashMap::new(),
            orders: HashMap::new(),
            positions: PositionManager::new(),
            metrics: PerformanceMetrics::default(),
            current_time: config.start_time,
        }
    }

    /// Run backtest over historical events
    pub fn run<S: TradingStrategy>(
        &mut self,
        strategy: &mut S,
        events: Vec<HistoricalEvent>,
    ) -> PerformanceMetrics {
        for event in events {
            self.current_time = event.timestamp();

            // Update market state
            self.process_market_event(&event);

            // Let strategy react
            let signals = strategy.on_event(&event, &self.positions);

            // Process trading signals
            for signal in signals {
                self.submit_order(signal);
            }

            // Check for fills
            self.check_pending_orders();
        }

        self.metrics.clone()
    }

    fn process_market_event(&mut self, event: &HistoricalEvent) {
        match event {
            HistoricalEvent::Trade { symbol, price, .. } => {
                self.positions.update_market_price(symbol, *price);
            }
            HistoricalEvent::Quote { symbol, bid, ask, .. } => {
                let book = self.order_book
                    .entry(symbol.clone())
                    .or_insert_with(SimulatedOrderBook::new);

                book.update_quote(*bid, *ask);
            }
        }
    }

    fn submit_order(&mut self, order: Order) {
        // Simulate latency
        let execution_time = self.current_time + Duration::milliseconds(self.config.latency_ms as i64);

        // Store order for later execution
        self.orders.insert(order.id, order);
    }

    fn check_pending_orders(&mut self) {
        let current_time = self.current_time;
        let mut filled_orders = Vec::new();

        for (id, order) in &self.orders {
            // Check if order should fill
            if let Some(fill_price) = self.simulate_fill(order) {
                // Apply slippage
                let slippage = fill_price * (self.config.slippage_bps / Decimal::from(10000));
                let actual_price = match order.side {
                    OrderSide::Buy => fill_price + slippage,
                    OrderSide::Sell => fill_price - slippage,
                };

                // Apply commission
                let commission = order.quantity * actual_price * (self.config.commission_bps / Decimal::from(10000));

                // Update position
                self.positions.apply_trade(
                    &order.symbol,
                    order.side,
                    order.quantity,
                    actual_price,
                );

                // Record metrics
                self.metrics.total_trades += 1;

                filled_orders.push(*id);
            }
        }

        // Remove filled orders
        for id in filled_orders {
            self.orders.remove(&id);
        }
    }

    fn simulate_fill(&self, order: &Order) -> Option<Decimal> {
        let book = self.order_book.get(&order.symbol)?;

        match order.order_type {
            OrderType::Market => {
                // Market orders fill at current quote
                match order.side {
                    OrderSide::Buy => Some(book.ask),
                    OrderSide::Sell => Some(book.bid),
                }
            }
            OrderType::Limit => {
                // Limit orders fill if price crossed
                match order.side {
                    OrderSide::Buy if book.ask <= order.price? => Some(book.ask),
                    OrderSide::Sell if book.bid >= order.price? => Some(book.bid),
                    _ => None,
                }
            }
        }
    }

    fn calculate_sharpe_ratio(&self) -> f64 {
        // Calculate Sharpe ratio from P&L history
        // Implementation omitted for brevity
        0.0
    }
}

/// Trading strategy trait
pub trait TradingStrategy {
    fn on_event(
        &mut self,
        event: &HistoricalEvent,
        positions: &PositionManager,
    ) -> Vec<Order>;
}

/// Simple simulated order book (just top of book)
#[derive(Debug, Clone)]
struct SimulatedOrderBook {
    bid: Decimal,
    ask: Decimal,
}

impl SimulatedOrderBook {
    fn new() -> Self {
        Self {
            bid: Decimal::ZERO,
            ask: Decimal::MAX,
        }
    }

    fn update_quote(&mut self, bid: Decimal, ask: Decimal) {
        self.bid = bid;
        self.ask = ask;
    }
}

impl HistoricalEvent {
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            HistoricalEvent::Trade { timestamp, .. } => *timestamp,
            HistoricalEvent::Quote { timestamp, .. } => *timestamp,
        }
    }
}
```

**Example strategy:**

```rust
/// Simple moving average crossover strategy
pub struct MovingAverageCrossover {
    fast_period: usize,
    slow_period: usize,
    prices: VecDeque<Decimal>,
}

impl TradingStrategy for MovingAverageCrossover {
    fn on_event(
        &mut self,
        event: &HistoricalEvent,
        positions: &PositionManager,
    ) -> Vec<Order> {
        let mut orders = Vec::new();

        if let HistoricalEvent::Trade { price, symbol, timestamp, .. } = event {
            self.prices.push_back(*price);
            if self.prices.len() > self.slow_period {
                self.prices.pop_front();
            }

            if self.prices.len() == self.slow_period {
                let fast_ma = self.moving_average(self.fast_period);
                let slow_ma = self.moving_average(self.slow_period);

                let position = positions.get_position(symbol);
                let current_qty = position.map(|p| p.net_quantity).unwrap_or(Decimal::ZERO);

                // Golden cross - buy signal
                if fast_ma > slow_ma && current_qty <= Decimal::ZERO {
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        symbol: symbol.clone(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Market,
                        quantity: dec!(1.0),
                        timestamp: *timestamp,
                        ..Default::default()
                    });
                }
                // Death cross - sell signal
                else if fast_ma < slow_ma && current_qty >= Decimal::ZERO {
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        symbol: symbol.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Market,
                        quantity: dec!(1.0),
                        timestamp: *timestamp,
                        ..Default::default()
                    });
                }
            }
        }

        orders
    }
}

impl MovingAverageCrossover {
    fn moving_average(&self, period: usize) -> Decimal {
        let sum: Decimal = self.prices.iter().rev().take(period).sum();
        sum / Decimal::from(period)
    }
}
```

### Rust Skills You'll Learn

- **Event-driven simulation**
- **Trait-based strategy pattern**
- **Time-series calculations** (moving averages)
- **Performance metrics** computation
- **Fill simulation** with slippage and latency

---

Due to length constraints, I'll provide a condensed version of the remaining features:

## 8. Network Optimization

**Key Technologies:**
- `io_uring` for async I/O (Linux)
- Kernel bypass with `DPDK`
- `mio` for cross-platform event loops
- TCP_NODELAY socket options

**Rust Skills:** Unsafe FFI, `libc` bindings, raw sockets

---

## 9. Reconnection & Session Management

**Implementation:**
- Exponential backoff with `tokio::time`
- Session state persistence
- Heartbeat/keepalive timers
- Connection pooling

**Rust Skills:** State machines, timer-based logic

---

## 10. Admin & Monitoring Dashboard

**Tech Stack:**
- `axum` for REST API
- WebSocket for live updates
- `prometheus` crate for metrics
- `serde_json` for serialization

**Rust Skills:** Web framework usage, async HTTP handlers

---

## 11. Advanced Order Types

**Types to Implement:**
- One-Cancels-Other (OCO)
- Bracket orders (entry + SL + TP)
- Fill-or-Kill (FOK)
- Pegged orders (dynamic pricing)

**Rust Skills:** Complex order lifecycle management

---

## 12. FIX Protocol Implementation

**Components:**
- FIX message parsing (tag-value pairs)
- Session management (logon/logout)
- Sequence number tracking
- Checksum validation

**Rust Skills:** Text parsing, protocol state machines

---

## 13. Configuration Hot Reload

```rust
use notify::{Watcher, RecursiveMode};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HotReloadConfig {
    config: Arc<RwLock<AppConfig>>,
}

impl HotReloadConfig {
    pub async fn watch_file(&self, path: &str) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::watcher(tx, Duration::from_secs(1)).unwrap();
        watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

        loop {
            match rx.recv() {
                Ok(event) => {
                    // Reload config
                    let new_config = load_config(path).await;
                    *self.config.write().await = new_config;
                }
                Err(e) => break,
            }
        }
    }
}
```

---

## 14. Order Book Delta Compression

**Techniques:**
- Delta encoding (only send changes)
- Run-length encoding for repeated prices
- Checksum for integrity

**Rust Skills:** Compression algorithms, differential updates

---

## 15. Multi-Threading & Work Stealing

```rust
use rayon::prelude::*;
use crossbeam::channel;

// Dedicate thread per symbol
pub fn spawn_symbol_thread(symbol: String) {
    std::thread::Builder::new()
        .name(format!("symbol-{}", symbol))
        .spawn(move || {
            // Pin to specific CPU core
            core_affinity::set_for_current(CoreId { id: 0 });

            // Process orders for this symbol
            loop {
                // ... order processing
            }
        })
        .unwrap();
}

// Work stealing with rayon
pub fn parallel_processing(symbols: Vec<String>) {
    symbols.par_iter().for_each(|symbol| {
        // Process in parallel
    });
}
```

---

## 16. Trade Reporting & Compliance

**Features:**
- MiFID II transaction reporting
- Audit trail generation
- Best execution analytics
- Surveillance (spoofing/layering detection)

**Rust Skills:** Complex event processing, batch reporting

---

## 17. Advanced Features (Expert Level)

### A. FPGA Acceleration
- Offload matching to hardware
- NIC-level timestamping
- Requires HDL knowledge (Verilog/VHDL)

### B. Machine Learning Integration
```rust
use pyo3::prelude::*;

#[pyfunction]
fn predict_price(features: Vec<f64>) -> PyResult<f64> {
    // Call Python ML model from Rust
    Python::with_gil(|py| {
        let model = py.import("model")?;
        let result = model.call_method1("predict", (features,))?;
        result.extract()
    })
}
```

### C. Cross-Exchange Arbitrage
- Detect price discrepancies
- Simultaneous execution
- Latency arbitrage

---

## Recommended Implementation Priority

**Phase 1 - Core Trading:**
1. Market Data Feed Handler
2. Position & Risk Management
3. Order Routing & SOR

**Phase 2 - Performance:**
4. Network Optimization
5. Multi-Threading
6. Binary Protocol (from original roadmap)

**Phase 3 - Analytics:**
7. Historical Data Storage
8. Backtesting Engine
9. Admin Dashboard

**Phase 4 - Production:**
10. Reconnection & Session Management
11. FIX Protocol
12. Trade Reporting

**Phase 5 - Advanced:**
13. Market Making Strategies
14. Auction Mechanisms
15. FPGA/ML Integration

---

## Total Learning Value

Implementing these 28 features will teach you:
- **40+ Rust crates** (tokio, serde, arrow, rayon, crossbeam, etc.)
- **Advanced concurrency** (lock-free, work-stealing, async)
- **Systems programming** (sockets, FFI, memory layout)
- **Financial algorithms** (P&L, risk, VWAP, market making)
- **Data engineering** (Parquet, time-series, compression)
- **Web services** (REST, WebSocket, monitoring)

Each feature builds on Rust's strengths: safety, performance, and expressiveness. Combined with the original 9 features from `HFT_FEATURES_ROADMAP.md`, you'll have a world-class HFT system and deep Rust expertise!
