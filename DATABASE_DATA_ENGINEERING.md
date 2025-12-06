# Database & Data Engineering Roadmap

> A comprehensive guide to persistence, analytics, and data infrastructure for high-frequency trading systems in Rust.

---

## Table of Contents

1. [Overview: Why Data Engineering Matters](#1-overview-why-data-engineering-matters)
2. [Database Selection Guide](#2-database-selection-guide)
3. [Time-Series Data Storage](#3-time-series-data-storage)
4. [Event Sourcing Architecture](#4-event-sourcing-architecture)
5. [Real-Time Analytics Pipeline](#5-real-time-analytics-pipeline)
6. [Historical Data Warehouse](#6-historical-data-warehouse)
7. [Market Data Feed Handler](#7-market-data-feed-handler)
8. [Backtesting Infrastructure](#8-backtesting-infrastructure)
9. [Data Quality & Monitoring](#9-data-quality--monitoring)
10. [Production Deployment Patterns](#10-production-deployment-patterns)

---

## 1. Overview: Why Data Engineering Matters

### The Data Challenge in Trading

```
┌─────────────────────────────────────────────────────────────────┐
│ Trading System Data Flows                                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  External Data                    Internal Data                  │
│  ─────────────                    ─────────────                  │
│  • Market feeds (1M+ msg/sec)     • Orders (100K/sec)           │
│  • Reference data                 • Trades (10K/sec)            │
│  • News/sentiment                 • Position updates            │
│  • Economic indicators            • Risk metrics                │
│                                                                  │
│                    ┌───────────────┐                            │
│                    │ Order Book    │                            │
│  ─────────────────▶│ Engine        │─────────────────▶          │
│                    └───────────────┘                            │
│                           │                                      │
│                           ▼                                      │
│              ┌────────────────────────┐                         │
│              │  Data Infrastructure   │                         │
│              ├────────────────────────┤                         │
│              │ • Write-Ahead Log      │ ◄── Durability          │
│              │ • Time-Series DB       │ ◄── Analytics           │
│              │ • Event Store          │ ◄── Audit/Replay        │
│              │ • Data Warehouse       │ ◄── Historical          │
│              │ • Cache Layer          │ ◄── Performance         │
│              └────────────────────────┘                         │
│                                                                  │
│  Key Requirements:                                               │
│  • Sub-millisecond write latency (don't slow down trading)     │
│  • High throughput (millions of events/second)                  │
│  • Durability (never lose data)                                 │
│  • Query performance (real-time analytics)                      │
│  • Cost efficiency (data grows fast!)                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Categories in Trading

| Category | Volume | Latency Req | Retention | Primary Use |
|----------|--------|-------------|-----------|-------------|
| Order events | Very High | < 1ms write | 7+ years | Audit, replay |
| Trade executions | High | < 1ms write | Forever | Settlement, reporting |
| Market data (L1) | Extreme | < 100μs | 1-3 years | Strategy, analytics |
| Market data (L2/L3) | Extreme | < 100μs | 30-90 days | Research |
| Position/PnL | Medium | < 10ms | Forever | Risk, reporting |
| Reference data | Low | < 100ms | Forever | All systems |

---

## 2. Database Selection Guide

### Decision Matrix

```
┌─────────────────────────────────────────────────────────────────┐
│ Database Selection for Trading Systems                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Use Case → Recommended Database                                  │
│                                                                  │
│ ┌──────────────────────┬────────────────────────────────────┐   │
│ │ Hot Path (WAL)       │ Custom binary + mmap               │   │
│ │                      │ or RocksDB/LMDB                    │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Time-Series (OHLCV)  │ QuestDB, TimescaleDB, InfluxDB     │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Event Store          │ EventStoreDB, Kafka, Custom        │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Order/Trade History  │ PostgreSQL, ScyllaDB               │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Analytics/Warehouse  │ ClickHouse, DuckDB, Snowflake      │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Cache Layer          │ Redis, Dragonfly, memcached        │   │
│ ├──────────────────────┼────────────────────────────────────┤   │
│ │ Reference Data       │ PostgreSQL, SQLite                 │   │
│ └──────────────────────┴────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Detailed Comparison

#### For Write-Ahead Log (Hot Path)

| Database | Write Latency | Throughput | Rust Support | Notes |
|----------|---------------|------------|--------------|-------|
| Custom WAL | ~1μs | 10M+/sec | Native | Most control, most work |
| LMDB | ~5μs | 1M+/sec | `lmdb` crate | Memory-mapped, ACID |
| RocksDB | ~10μs | 500K+/sec | `rocksdb` crate | LSM tree, good compression |
| SQLite WAL | ~100μs | 50K/sec | `rusqlite` crate | Simple, single-writer |

**Recommendation:** Start with RocksDB for balance of performance and features. Move to custom WAL only if you need <5μs latency.

#### For Time-Series Data

| Database | Ingestion | Query Speed | Compression | Rust Support |
|----------|-----------|-------------|-------------|--------------|
| QuestDB | 1M+ rows/sec | Very Fast | Good | HTTP/ILP |
| TimescaleDB | 500K rows/sec | Fast | Excellent | `tokio-postgres` |
| InfluxDB | 1M+ points/sec | Fast | Good | `influxdb` crate |
| ClickHouse | 1M+ rows/sec | Excellent | Excellent | `clickhouse-rs` |

**Recommendation:** QuestDB for highest ingestion rates, TimescaleDB if you need SQL compatibility.

#### For Event Sourcing

| System | Throughput | Ordering | Replay | Rust Support |
|--------|------------|----------|--------|--------------|
| EventStoreDB | 100K+/sec | Per-stream | Excellent | `eventstore` crate |
| Apache Kafka | 1M+/sec | Per-partition | Good | `rdkafka` crate |
| Redpanda | 1M+/sec | Per-partition | Good | Kafka compatible |
| Custom | 10M+/sec | Full control | Full control | Native |

**Recommendation:** Kafka/Redpanda for high throughput, EventStoreDB for rich event sourcing features.

---

## 3. Time-Series Data Storage

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Time-Series Storage Architecture                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Incoming Data                    Storage Tiers                  │
│  ─────────────                    ─────────────                  │
│                                                                  │
│  ┌──────────┐     ┌─────────────────────────────────────────┐   │
│  │ Trades   │────▶│ Hot Tier (RAM + SSD)                    │   │
│  │ Quotes   │     │ • Last 24 hours                         │   │
│  │ OHLCV    │     │ • Uncompressed for fast access          │   │
│  └──────────┘     │ • ~100GB for active symbols             │   │
│                   └────────────────┬────────────────────────┘   │
│                                    │ Roll after 24h             │
│                                    ▼                            │
│                   ┌─────────────────────────────────────────┐   │
│                   │ Warm Tier (SSD)                         │   │
│                   │ • Last 30-90 days                       │   │
│                   │ • Compressed (LZ4/ZSTD)                 │   │
│                   │ • ~1TB                                  │   │
│                   └────────────────┬────────────────────────┘   │
│                                    │ Roll after 90 days         │
│                                    ▼                            │
│                   ┌─────────────────────────────────────────┐   │
│                   │ Cold Tier (Object Storage / HDD)        │   │
│                   │ • Historical (years)                    │   │
│                   │ • Highly compressed                     │   │
│                   │ • Parquet format                        │   │
│                   │ • ~10TB+ per year                       │   │
│                   └─────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation: QuestDB Integration

**File: `src/storage/timeseries.rs`**

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tokio::net::TcpStream;
use std::io::Write;

/// QuestDB client using InfluxDB Line Protocol (ILP)
pub struct TimeSeriesClient {
    /// TCP connection to QuestDB ILP endpoint
    stream: TcpStream,

    /// Buffer for batching writes
    buffer: Vec<u8>,

    /// Max buffer size before flush
    max_buffer_size: usize,
}

impl TimeSeriesClient {
    pub async fn connect(host: &str, port: u16) -> std::io::Result<Self> {
        let stream = TcpStream::connect((host, port)).await?;

        Ok(Self {
            stream,
            buffer: Vec::with_capacity(65536),
            max_buffer_size: 65536,
        })
    }

    /// Write a trade to the time-series database
    pub async fn write_trade(&mut self, trade: &Trade) -> std::io::Result<()> {
        // InfluxDB Line Protocol format:
        // measurement,tag1=value1,tag2=value2 field1=value1,field2=value2 timestamp

        let line = format!(
            "trades,symbol={},side={} price={},quantity={},buyer_id=\"{}\",seller_id=\"{}\" {}\n",
            trade.symbol,
            if trade.buyer_order_id < trade.seller_order_id { "buy" } else { "sell" },
            trade.price,
            trade.quantity,
            trade.buyer_id,
            trade.seller_id,
            trade.timestamp.timestamp_nanos_opt().unwrap_or(0)
        );

        self.buffer.extend_from_slice(line.as_bytes());

        if self.buffer.len() >= self.max_buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// Write OHLCV candle data
    pub async fn write_ohlcv(&mut self, candle: &OhlcvCandle) -> std::io::Result<()> {
        let line = format!(
            "ohlcv,symbol={},interval={} open={},high={},low={},close={},volume={},trade_count={} {}\n",
            candle.symbol,
            candle.interval,
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
            candle.trade_count,
            candle.timestamp.timestamp_nanos_opt().unwrap_or(0)
        );

        self.buffer.extend_from_slice(line.as_bytes());

        if self.buffer.len() >= self.max_buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// Write order book snapshot
    pub async fn write_book_snapshot(&mut self, snapshot: &BookSnapshot) -> std::io::Result<()> {
        let line = format!(
            "book_snapshots,symbol={} \
             best_bid={},best_ask={},\
             bid_volume={},ask_volume={},\
             spread_bps={},mid_price={},\
             imbalance={} {}\n",
            snapshot.symbol,
            snapshot.best_bid,
            snapshot.best_ask,
            snapshot.bid_volume,
            snapshot.ask_volume,
            snapshot.spread_bps,
            snapshot.mid_price,
            snapshot.imbalance,
            snapshot.timestamp.timestamp_nanos_opt().unwrap_or(0)
        );

        self.buffer.extend_from_slice(line.as_bytes());

        if self.buffer.len() >= self.max_buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush buffer to database
    pub async fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        use tokio::io::AsyncWriteExt;
        self.stream.write_all(&self.buffer).await?;
        self.buffer.clear();

        Ok(())
    }
}

/// OHLCV candle data
#[derive(Debug, Clone)]
pub struct OhlcvCandle {
    pub symbol: String,
    pub interval: String,  // "1m", "5m", "1h", "1d"
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trade_count: u64,
}

/// Order book snapshot for time-series storage
#[derive(Debug, Clone)]
pub struct BookSnapshot {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub best_bid: Decimal,
    pub best_ask: Decimal,
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
    pub spread_bps: Decimal,
    pub mid_price: Decimal,
    pub imbalance: Decimal,
}
```

### Candle Aggregation Service

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Real-time OHLCV candle aggregator
pub struct CandleAggregator {
    /// Current candles being built, keyed by (symbol, interval)
    candles: HashMap<(String, String), OhlcvCandle>,

    /// Intervals to track
    intervals: Vec<CandleInterval>,

    /// Channel to receive trades
    trade_rx: mpsc::Receiver<Trade>,

    /// Channel to emit completed candles
    candle_tx: mpsc::Sender<OhlcvCandle>,
}

#[derive(Clone)]
pub struct CandleInterval {
    pub name: String,          // "1m", "5m", etc.
    pub duration_secs: u64,
}

impl CandleAggregator {
    pub fn new(
        intervals: Vec<CandleInterval>,
        trade_rx: mpsc::Receiver<Trade>,
        candle_tx: mpsc::Sender<OhlcvCandle>,
    ) -> Self {
        Self {
            candles: HashMap::new(),
            intervals,
            trade_rx,
            candle_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(trade) = self.trade_rx.recv().await {
            for interval in &self.intervals {
                self.update_candle(&trade, interval).await;
            }
        }
    }

    async fn update_candle(&mut self, trade: &Trade, interval: &CandleInterval) {
        let candle_start = self.get_candle_start(trade.timestamp, interval.duration_secs);
        let key = (trade.symbol.clone(), interval.name.clone());

        let candle = self.candles.entry(key.clone()).or_insert_with(|| {
            OhlcvCandle {
                symbol: trade.symbol.clone(),
                interval: interval.name.clone(),
                timestamp: candle_start,
                open: trade.price,
                high: trade.price,
                low: trade.price,
                close: trade.price,
                volume: Decimal::ZERO,
                trade_count: 0,
            }
        });

        // Check if we've moved to a new candle period
        if candle.timestamp != candle_start {
            // Emit completed candle
            let completed = candle.clone();
            let _ = self.candle_tx.send(completed).await;

            // Start new candle
            *candle = OhlcvCandle {
                symbol: trade.symbol.clone(),
                interval: interval.name.clone(),
                timestamp: candle_start,
                open: trade.price,
                high: trade.price,
                low: trade.price,
                close: trade.price,
                volume: Decimal::ZERO,
                trade_count: 0,
            };
        }

        // Update candle with trade
        candle.high = candle.high.max(trade.price);
        candle.low = candle.low.min(trade.price);
        candle.close = trade.price;
        candle.volume += trade.quantity;
        candle.trade_count += 1;
    }

    fn get_candle_start(&self, timestamp: DateTime<Utc>, duration_secs: u64) -> DateTime<Utc> {
        let ts = timestamp.timestamp() as u64;
        let candle_ts = (ts / duration_secs) * duration_secs;
        DateTime::from_timestamp(candle_ts as i64, 0).unwrap()
    }
}
```

---

## 4. Event Sourcing Architecture

### Why Event Sourcing for Trading

```
┌─────────────────────────────────────────────────────────────────┐
│ Event Sourcing vs State Storage                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Traditional (State Storage):                                     │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Order Table                                                  │ │
│ │ ┌────────────────────────────────────────────────────────┐  │ │
│ │ │ id: ABC123                                             │  │ │
│ │ │ status: PARTIALLY_FILLED  ← Current state only!        │  │ │
│ │ │ filled_qty: 500                                        │  │ │
│ │ │ updated_at: 2024-01-15 10:30:00                       │  │ │
│ │ └────────────────────────────────────────────────────────┘  │ │
│ │ Problem: Lost history of HOW we got here                    │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Event Sourcing:                                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Event Stream for Order ABC123                                │ │
│ │ ┌────────────────────────────────────────────────────────┐  │ │
│ │ │ 1. OrderSubmitted { qty: 1000, price: 100.50, ... }    │  │ │
│ │ │ 2. OrderAccepted { timestamp: ... }                    │  │ │
│ │ │ 3. OrderPartiallyFilled { fill_qty: 200, price: 100.50}│  │ │
│ │ │ 4. OrderPartiallyFilled { fill_qty: 300, price: 100.50}│  │ │
│ │ │ ← Can replay to ANY point in time!                     │  │ │
│ │ └────────────────────────────────────────────────────────┘  │ │
│ │ Benefits:                                                    │ │
│ │ • Complete audit trail                                       │ │
│ │ • Regulatory compliance                                      │ │
│ │ • Debugging (replay exact sequence)                          │ │
│ │ • Backtesting (replay with strategy)                         │ │
│ │ • Recovery (rebuild state from events)                       │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation

**File: `src/storage/event_store.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All possible events in the trading system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TradingEvent {
    // Order lifecycle events
    OrderSubmitted(OrderSubmittedEvent),
    OrderAccepted(OrderAcceptedEvent),
    OrderRejected(OrderRejectedEvent),
    OrderPartiallyFilled(OrderFilledEvent),
    OrderFilled(OrderFilledEvent),
    OrderCancelled(OrderCancelledEvent),
    OrderExpired(OrderExpiredEvent),

    // Trade events
    TradeExecuted(TradeExecutedEvent),

    // Market data events
    BookUpdated(BookUpdatedEvent),
    QuoteUpdated(QuoteUpdatedEvent),

    // System events
    TradingHalted(TradingHaltedEvent),
    TradingResumed(TradingResumedEvent),
}

/// Metadata attached to every event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub stream_id: String,      // e.g., "orders-BTC-USD" or "order-{order_id}"
    pub correlation_id: Uuid,   // Link related events
    pub causation_id: Uuid,     // What caused this event
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSubmittedEvent {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<String>,  // Decimal as string for JSON
    pub quantity: String,
    pub user_id: String,
    pub time_in_force: String,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFilledEvent {
    pub order_id: Uuid,
    pub trade_id: Uuid,
    pub fill_price: String,
    pub fill_quantity: String,
    pub cumulative_quantity: String,
    pub remaining_quantity: String,
    pub fee: String,
    pub fee_currency: String,
    pub liquidity_flag: String,  // "maker" or "taker"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecutedEvent {
    pub trade_id: Uuid,
    pub symbol: String,
    pub price: String,
    pub quantity: String,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: String,
    pub seller_user_id: String,
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub maker_fee: String,
    pub taker_fee: String,
}

// ... other event types ...

/// Event store interface
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to a stream
    async fn append(
        &self,
        stream_id: &str,
        events: Vec<TradingEvent>,
        expected_version: Option<u64>,
    ) -> Result<u64, EventStoreError>;

    /// Read events from a stream
    async fn read_stream(
        &self,
        stream_id: &str,
        start: u64,
        count: usize,
    ) -> Result<Vec<(EventMetadata, TradingEvent)>, EventStoreError>;

    /// Read all events (for rebuilding state)
    async fn read_all(
        &self,
        start: u64,
        count: usize,
    ) -> Result<Vec<(EventMetadata, TradingEvent)>, EventStoreError>;

    /// Subscribe to new events
    async fn subscribe(
        &self,
        stream_pattern: &str,
    ) -> Result<EventSubscription, EventStoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("Optimistic concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: u64, actual: u64 },

    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

pub struct EventSubscription {
    rx: tokio::sync::mpsc::Receiver<(EventMetadata, TradingEvent)>,
}
```

### Kafka-Based Event Store

```rust
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::consumer::{StreamConsumer, Consumer};
use rdkafka::Message;

pub struct KafkaEventStore {
    producer: FutureProducer,
    brokers: String,
}

impl KafkaEventStore {
    pub fn new(brokers: &str) -> Self {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("acks", "all")  // Wait for all replicas
            .set("enable.idempotence", "true")  // Exactly-once semantics
            .create()
            .expect("Failed to create Kafka producer");

        Self {
            producer,
            brokers: brokers.to_string(),
        }
    }

    pub async fn append_event(
        &self,
        topic: &str,
        key: &str,
        event: &TradingEvent,
    ) -> Result<(), rdkafka::error::KafkaError> {
        let payload = serde_json::to_vec(event).unwrap();

        self.producer
            .send(
                FutureRecord::to(topic)
                    .key(key)
                    .payload(&payload),
                std::time::Duration::from_secs(5),
            )
            .await
            .map_err(|(e, _)| e)?;

        Ok(())
    }

    pub async fn create_consumer(
        &self,
        group_id: &str,
        topics: &[&str],
    ) -> StreamConsumer {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")  // Manual commit for exactly-once
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Failed to create consumer");

        consumer.subscribe(topics).expect("Failed to subscribe");
        consumer
    }
}

/// Event processor that consumes and processes events
pub struct EventProcessor {
    consumer: StreamConsumer,
    handlers: Vec<Box<dyn EventHandler>>,
}

#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &TradingEvent) -> Result<(), Box<dyn std::error::Error>>;
}

impl EventProcessor {
    pub async fn run(&self) {
        use futures::StreamExt;

        let mut stream = self.consumer.stream();

        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        if let Ok(event) = serde_json::from_slice::<TradingEvent>(payload) {
                            for handler in &self.handlers {
                                if let Err(e) = handler.handle(&event).await {
                                    tracing::error!("Event handler error: {}", e);
                                }
                            }
                        }
                    }
                    // Commit offset after processing
                    self.consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async)
                        .expect("Failed to commit");
                }
                Err(e) => {
                    tracing::error!("Kafka error: {}", e);
                }
            }
        }
    }
}
```

### State Projection (CQRS Read Model)

```rust
/// Builds read-optimized views from events
pub struct OrderProjection {
    /// Current state of all orders
    orders: DashMap<Uuid, ProjectedOrder>,

    /// Database for persistence
    db: sqlx::PgPool,
}

#[derive(Debug, Clone)]
pub struct ProjectedOrder {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub status: OrderStatus,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub average_fill_price: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub fills: Vec<Fill>,fn parse_message(&self, raw: &str) -> Result<MarketDataEvent, Box<dyn std::error::Error>> {
    // Exchange-specific parsing
    // Example for generic JSON format:
    let json: serde_json::Value = serde_json::from_str(raw)?;

    match json.get("type").and_then(|t| t.as_str()) {
    Some("trade") => {
    Ok(MarketDataEvent::Trade {
    exchange: self.config.name.clone(),
    symbol: json["symbol"].as_str().unwrap().to_string(),
    price: json["price"].as_str().unwrap().parse()?,
    quantity: json["quantity"].as_str().unwrap().parse()?,
    side: if json["side"] == "buy" { OrderSide::Buy } else { OrderSide::Sell },
    timestamp: DateTime::parse_from_rfc3339(json["timestamp"].as_str().unwrap())?
    .with_timezone(&Utc),
    sequence: json["sequence"].as_u64().unwrap_or(0),
    })
    }
    // ... other message types
    _ => Err("Unknown message type".into()),
    }
    }
}

#[async_trait::async_trait]
impl EventHandler for OrderProjection {
    async fn handle(&self, event: &TradingEvent) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            TradingEvent::OrderSubmitted(e) => {
                let order = ProjectedOrder {
                    id: e.order_id,
                    symbol: e.symbol.clone(),
                    side: e.side.parse()?,
                    status: OrderStatus::New,
                    price: e.price.as_ref().map(|p| p.parse()).transpose()?,
                    quantity: e.quantity.parse()?,
                    filled_quantity: Decimal::ZERO,
                    average_fill_price: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    fills: Vec::new(),
                };

                self.orders.insert(e.order_id, order.clone());
                self.persist_order(&order).await?;
            }

            TradingEvent::OrderPartiallyFilled(e) | TradingEvent::OrderFilled(e) => {
                if let Some(mut order) = self.orders.get_mut(&e.order_id) {
                    order.filled_quantity = e.cumulative_quantity.parse()?;
                    order.status = if e.remaining_quantity.parse::<Decimal>()? == Decimal::ZERO {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };
                    order.updated_at = Utc::now();

                    // Update average fill price
                    order.fills.push(Fill {
                        price: e.fill_price.parse()?,
                        quantity: e.fill_quantity.parse()?,
                        timestamp: Utc::now(),
                    });

                    let total_value: Decimal = order.fills.iter()
                        .map(|f| f.price * f.quantity)
                        .sum();
                    order.average_fill_price = Some(total_value / order.filled_quantity);

                    self.persist_order(&order).await?;
                }
            }

            TradingEvent::OrderCancelled(e) => {
                if let Some(mut order) = self.orders.get_mut(&e.order_id) {
                    order.status = OrderStatus::Cancelled;
                    order.updated_at = Utc::now();
                    self.persist_order(&order).await?;
                }
            }

            _ => {}
        }

        Ok(())
    }
}

impl OrderProjection {
    async fn persist_order(&self, order: &ProjectedOrder) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO orders (id, symbol, side, status, price, quantity, filled_quantity,
                              average_fill_price, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                filled_quantity = EXCLUDED.filled_quantity,
                average_fill_price = EXCLUDED.average_fill_price,
                updated_at = EXCLUDED.updated_at
            "#,
            order.id,
            order.symbol,
            order.side.to_string(),
            order.status.to_string(),
            order.price.map(|p| p.to_string()),
            order.quantity.to_string(),
            order.filled_quantity.to_string(),
            order.average_fill_price.map(|p| p.to_string()),
            order.created_at,
            order.updated_at,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
```

---

## 5. Real-Time Analytics Pipeline

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Real-Time Analytics Pipeline                                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐                                                │
│  │ Order Book  │                                                │
│  │ Engine      │                                                │
│  └──────┬──────┘                                                │
│         │ Events                                                 │
│         ▼                                                        │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Kafka / Redpanda                      │    │
│  │  Topics: trades, orders, book-updates, metrics           │    │
│  └─────────────────────────┬───────────────────────────────┘    │
│                            │                                     │
│         ┌──────────────────┼──────────────────────┐             │
│         │                  │                      │             │
│         ▼                  ▼                      ▼             │
│  ┌─────────────┐   ┌─────────────┐      ┌─────────────┐        │
│  │ Candle      │   │ Metrics     │      │ Alert       │        │
│  │ Aggregator  │   │ Calculator  │      │ Engine      │        │
│  └──────┬──────┘   └──────┬──────┘      └──────┬──────┘        │
│         │                  │                    │                │
│         ▼                  ▼                    ▼                │
│  ┌─────────────┐   ┌─────────────┐      ┌─────────────┐        │
│  │ QuestDB     │   │ Prometheus  │      │ PagerDuty   │        │
│  │ (OHLCV)     │   │ (Metrics)   │      │ (Alerts)    │        │
│  └─────────────┘   └─────────────┘      └─────────────┘        │
│         │                  │                                     │
│         └──────────────────┼──────────────────┐                 │
│                            ▼                   │                 │
│                    ┌─────────────┐             │                 │
│                    │ Grafana     │◄────────────┘                 │
│                    │ Dashboards  │                               │
│                    └─────────────┘                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Real-Time Metrics Calculator

```rust
use prometheus::{Registry, Counter, Gauge, Histogram, HistogramOpts};
use std::sync::Arc;

/// Real-time trading metrics
pub struct TradingMetrics {
    // Volume metrics
    trade_count: Counter,
    trade_volume: Counter,
    trade_notional: Counter,

    // Order metrics
    orders_submitted: Counter,
    orders_filled: Counter,
    orders_cancelled: Counter,
    orders_rejected: Counter,

    // Latency metrics
    matching_latency: Histogram,
    order_to_fill_latency: Histogram,

    // Market metrics
    spread_bps: Gauge,
    book_imbalance: Gauge,
    mid_price: Gauge,

    // Per-symbol metrics
    symbol_volumes: DashMap<String, Counter>,
}

impl TradingMetrics {
    pub fn new(registry: &Registry) -> Self {
        let trade_count = Counter::new("trades_total", "Total number of trades")
            .expect("metric creation failed");
        registry.register(Box::new(trade_count.clone())).unwrap();

        let trade_volume = Counter::new("trade_volume_total", "Total trade volume")
            .expect("metric creation failed");
        registry.register(Box::new(trade_volume.clone())).unwrap();

        let matching_latency = Histogram::with_opts(
            HistogramOpts::new("matching_latency_seconds", "Matching engine latency")
                .buckets(vec![0.000001, 0.00001, 0.0001, 0.001, 0.01, 0.1])
        ).expect("metric creation failed");
        registry.register(Box::new(matching_latency.clone())).unwrap();

        // ... initialize other metrics ...

        Self {
            trade_count,
            trade_volume,
            trade_notional: Counter::new("trade_notional_total", "Total notional value").unwrap(),
            orders_submitted: Counter::new("orders_submitted_total", "Orders submitted").unwrap(),
            orders_filled: Counter::new("orders_filled_total", "Orders filled").unwrap(),
            orders_cancelled: Counter::new("orders_cancelled_total", "Orders cancelled").unwrap(),
            orders_rejected: Counter::new("orders_rejected_total", "Orders rejected").unwrap(),
            matching_latency,
            order_to_fill_latency: Histogram::with_opts(
                HistogramOpts::new("order_to_fill_seconds", "Time from order to fill")
            ).unwrap(),
            spread_bps: Gauge::new("spread_bps", "Current spread in basis points").unwrap(),
            book_imbalance: Gauge::new("book_imbalance", "Order book imbalance").unwrap(),
            mid_price: Gauge::new("mid_price", "Current mid price").unwrap(),
            symbol_volumes: DashMap::new(),
        }
    }

    pub fn record_trade(&self, trade: &Trade) {
        self.trade_count.inc();
        self.trade_volume.inc_by(trade.quantity.to_f64().unwrap_or(0.0));
        self.trade_notional.inc_by(
            (trade.price * trade.quantity).to_f64().unwrap_or(0.0)
        );
    }

    pub fn record_matching_latency(&self, latency_ns: u64) {
        self.matching_latency.observe(latency_ns as f64 / 1_000_000_000.0);
    }

    pub fn update_market_metrics(&self, spread_bps: f64, imbalance: f64, mid: f64) {
        self.spread_bps.set(spread_bps);
        self.book_imbalance.set(imbalance);
        self.mid_price.set(mid);
    }
}

/// HTTP endpoint for Prometheus scraping
pub async fn metrics_handler(
    metrics: Arc<TradingMetrics>,
    registry: Arc<Registry>,
) -> impl axum::response::IntoResponse {
    use prometheus::Encoder;

    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        buffer,
    )
}
```

---

## 6. Historical Data Warehouse

### Schema Design

```sql
-- PostgreSQL / TimescaleDB schema for historical data

-- Raw trade data (partitioned by time)
CREATE TABLE trades (
    id UUID PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    price DECIMAL(20, 8) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    side VARCHAR(4) NOT NULL,
    buyer_order_id UUID NOT NULL,
    seller_order_id UUID NOT NULL,
    buyer_user_id VARCHAR(50) NOT NULL,
    seller_user_id VARCHAR(50) NOT NULL,
    maker_fee DECIMAL(20, 8) NOT NULL,
    taker_fee DECIMAL(20, 8) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);

-- Convert to TimescaleDB hypertable
SELECT create_hypertable('trades', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes
CREATE INDEX idx_trades_symbol_time ON trades (symbol, timestamp DESC);
CREATE INDEX idx_trades_buyer ON trades (buyer_user_id, timestamp DESC);
CREATE INDEX idx_trades_seller ON trades (seller_user_id, timestamp DESC);

-- Pre-aggregated OHLCV data
CREATE TABLE ohlcv (
    symbol VARCHAR(20) NOT NULL,
    interval VARCHAR(10) NOT NULL,  -- '1m', '5m', '1h', '1d'
    timestamp TIMESTAMPTZ NOT NULL,
    open DECIMAL(20, 8) NOT NULL,
    high DECIMAL(20, 8) NOT NULL,
    low DECIMAL(20, 8) NOT NULL,
    close DECIMAL(20, 8) NOT NULL,
    volume DECIMAL(20, 8) NOT NULL,
    quote_volume DECIMAL(20, 8) NOT NULL,
    trade_count INTEGER NOT NULL,
    PRIMARY KEY (symbol, interval, timestamp)
);

SELECT create_hypertable('ohlcv', 'timestamp', chunk_time_interval => INTERVAL '1 month');

-- Continuous aggregates for automatic rollups
CREATE MATERIALIZED VIEW ohlcv_1h
WITH (timescaledb.continuous) AS
SELECT
    symbol,
    time_bucket('1 hour', timestamp) AS timestamp,
    first(open, timestamp) AS open,
    max(high) AS high,
    min(low) AS low,
    last(close, timestamp) AS close,
    sum(volume) AS volume,
    sum(quote_volume) AS quote_volume,
    sum(trade_count) AS trade_count
FROM ohlcv
WHERE interval = '1m'
GROUP BY symbol, time_bucket('1 hour', timestamp);

-- Compression policy for old data
SELECT add_compression_policy('trades', INTERVAL '7 days');
SELECT add_compression_policy('ohlcv', INTERVAL '30 days');

-- Retention policy (optional)
SELECT add_retention_policy('trades', INTERVAL '2 years');
```

### Data Warehouse Queries

```rust
use sqlx::postgres::PgPool;
use chrono::{DateTime, Utc};

pub struct DataWarehouse {
    pool: PgPool,
}

impl DataWarehouse {
    /// Get OHLCV data for charting
    pub async fn get_ohlcv(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<OhlcvCandle>, sqlx::Error> {
        sqlx::query_as!(
            OhlcvCandle,
            r#"
            SELECT
                symbol, interval, timestamp,
                open, high, low, close,
                volume, trade_count
            FROM ohlcv
            WHERE symbol = $1
              AND interval = $2
              AND timestamp >= $3
              AND timestamp < $4
            ORDER BY timestamp ASC
            "#,
            symbol,
            interval,
            start,
            end,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Get user's trade history
    pub async fn get_user_trades(
        &self,
        user_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<Trade>, sqlx::Error> {
        sqlx::query_as!(
            Trade,
            r#"
            SELECT *
            FROM trades
            WHERE (buyer_user_id = $1 OR seller_user_id = $1)
              AND timestamp >= $2
              AND timestamp < $3
            ORDER BY timestamp DESC
            LIMIT $4
            "#,
            user_id,
            start,
            end,
            limit,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Calculate daily volume statistics
    pub async fn get_daily_volume_stats(
        &self,
        symbol: &str,
        days: i32,
    ) -> Result<Vec<DailyStats>, sqlx::Error> {
        sqlx::query_as!(
            DailyStats,
            r#"
            SELECT
                date_trunc('day', timestamp) as date,
                COUNT(*) as trade_count,
                SUM(quantity) as volume,
                SUM(price * quantity) as notional,
                AVG(price) as avg_price,
                MIN(price) as min_price,
                MAX(price) as max_price
            FROM trades
            WHERE symbol = $1
              AND timestamp >= NOW() - ($2 || ' days')::INTERVAL
            GROUP BY date_trunc('day', timestamp)
            ORDER BY date DESC
            "#,
            symbol,
            days.to_string(),
        )
        .fetch_all(&self.pool)
        .await
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DailyStats {
    pub date: DateTime<Utc>,
    pub trade_count: i64,
    pub volume: Decimal,
    pub notional: Decimal,
    pub avg_price: Decimal,
    pub min_price: Decimal,
    pub max_price: Decimal,
}
```

---

## 7. Market Data Feed Handler

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Market Data Feed Handler                                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  External Feeds                    Internal Processing           │
│  ──────────────                    ───────────────────           │
│                                                                  │
│  ┌─────────────┐                   ┌─────────────────────────┐  │
│  │ Binance WS  │──┐                │ Market Data Normalizer  │  │
│  └─────────────┘  │                │                         │  │
│  ┌─────────────┐  │   ┌────────┐   │ • Normalize formats     │  │
│  │ Coinbase WS │──┼──▶│ Ring   │──▶│ • Validate data        │  │
│  └─────────────┘  │   │ Buffer │   │ • Detect anomalies     │  │
│  ┌─────────────┐  │   └────────┘   │ • Sequence ordering    │  │
│  │ Kraken WS   │──┘                │                         │  │
│  └─────────────┘                   └───────────┬─────────────┘  │
│                                                │                 │
│                                    ┌───────────┴───────────┐    │
│                                    │                       │    │
│                            ┌───────▼───────┐      ┌────────▼───┐│
│                            │ Time-Series   │      │ Order Book ││
│                            │ Storage       │      │ Engine     ││
│                            └───────────────┘      └────────────┘│
│                                                                  │
│  Feed Recovery:                                                  │
│  • Automatic reconnection with exponential backoff              │
│  • Sequence gap detection and recovery                          │
│  • Snapshot request on reconnect                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation

**File: `src/feeds/handler.rs`**

```rust
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use std::time::Duration;

/// Normalized market data event
#[derive(Debug, Clone)]
pub enum MarketDataEvent {
    Trade {
        exchange: String,
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        side: OrderSide,
        timestamp: DateTime<Utc>,
        sequence: u64,
    },
    Quote {
        exchange: String,
        symbol: String,
        bid: Decimal,
        bid_size: Decimal,
        ask: Decimal,
        ask_size: Decimal,
        timestamp: DateTime<Utc>,
    },
    BookUpdate {
        exchange: String,
        symbol: String,
        bids: Vec<(Decimal, Decimal)>,
        asks: Vec<(Decimal, Decimal)>,
        timestamp: DateTime<Utc>,
        sequence: u64,
    },
}

/// Configuration for a market data feed
pub struct FeedConfig {
    pub name: String,
    pub ws_url: String,
    pub symbols: Vec<String>,
    pub channels: Vec<String>,
    pub reconnect_delay: Duration,
    pub max_reconnect_attempts: u32,
}

/// Market data feed handler
pub struct FeedHandler {
    config: FeedConfig,
    event_tx: mpsc::Sender<MarketDataEvent>,
    last_sequence: HashMap<String, u64>,
}

impl FeedHandler {
    pub fn new(config: FeedConfig, event_tx: mpsc::Sender<MarketDataEvent>) -> Self {
        Self {
            config,
            event_tx,
            last_sequence: HashMap::new(),
        }
    }

    pub async fn run(&mut self) {
        let mut reconnect_count = 0;

        loop {
            match self.connect_and_process().await {
                Ok(_) => {
                    tracing::info!("{}: Connection closed normally", self.config.name);
                    reconnect_count = 0;
                }
                Err(e) => {
                    tracing::error!("{}: Connection error: {}", self.config.name, e);
                    reconnect_count += 1;

                    if reconnect_count >= self.config.max_reconnect_attempts {
                        tracing::error!("{}: Max reconnect attempts reached", self.config.name);
                        break;
                    }
                }
            }

            // Exponential backoff
            let delay = self.config.reconnect_delay * 2u32.pow(reconnect_count.min(5));
            tracing::info!("{}: Reconnecting in {:?}", self.config.name, delay);
            tokio::time::sleep(delay).await;
        }
    }

    async fn connect_and_process(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.config.ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to channels
        let subscribe_msg = self.build_subscribe_message();
        write.send(Message::Text(subscribe_msg)).await?;

        // Process messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    self.process_message(&text).await?;
                }
                Message::Ping(data) => {
                    write.send(Message::Pong(data)).await?;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_message(&mut self, raw: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Parse based on exchange format
        // This would be specialized per exchange
        let event = self.parse_message(raw)?;

        // Check for sequence gaps
        if let Some(seq) = self.get_sequence(&event) {
            let key = self.get_sequence_key(&event);
            if let Some(&last) = self.last_sequence.get(&key) {
                if seq != last + 1 {
                    tracing::warn!(
                        "{}: Sequence gap detected. Expected {}, got {}",
                        self.config.name,
                        last + 1,
                        seq
                    );
                    // Handle gap: request snapshot, buffer, etc.
                }
            }
            self.last_sequence.insert(key, seq);
        }

        // Send to processing pipeline
        self.event_tx.send(event).await?;

        Ok(())
    }

    // Strongly typed struct to be used in production system
    #[derive(Deserialize)]
    struct TradeMessage {
        #[serde(rename = "type")]
        msg_type: String,
        symbol: String,
        price: String,
        quantity: String,
        side: String,
        timestamp: String,
        sequence: u64,
    }

    fn parse_message(&self, raw: &str) -> Result<MarketDataEvent, Box<dyn std::error::Error>> {
        let msg: TradeMessage = serde_json::from_str(raw)?;

        // Now all fields are guaranteed to exist and have correct types!
        Ok(MarketDataEvent::Trade {
            exchange: self.config.name.clone(),
            symbol: msg.symbol,
            price: msg.price.parse()?,
            quantity: msg.quantity.parse()?,
            side: if msg.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
            timestamp: DateTime::parse_from_rfc3339(&msg.timestamp)?
                .with_timezone(&Utc),
            sequence: msg.sequence,
        })
    }

    fn build_subscribe_message(&self) -> String {
        // Exchange-specific subscription message
        serde_json::json!({
            "type": "subscribe",
            "channels": self.config.channels,
            "symbols": self.config.symbols,
        }).to_string()
    }

    fn get_sequence(&self, event: &MarketDataEvent) -> Option<u64> {
        // *sequence, the asterisk is the dereference operator. It converts &u64 -> u64 by copying the value
        // Some(*sequence) - Wraps the dereferenced u64 value in Some to return Option<u64>.
        match event {
            MarketDataEvent::Trade { sequence, .. } => Some(*sequence),
            MarketDataEvent::BookUpdate { sequence, .. } => Some(*sequence),
            _ => None,
        }
    }

    // This function creates a unique identifier string for tracking sequences per exchange-symbol pair.
    // .. means ignore all other fields
    fn get_sequence_key(&self, event: &MarketDataEvent) -> String {
        match event {
            MarketDataEvent::Trade { exchange, symbol, .. } |
            MarketDataEvent::BookUpdate { exchange, symbol, .. } |
            MarketDataEvent::Quote { exchange, symbol, .. } => {
                // as_str borrows existing data (zero-cost, not allocation)
                // format! creates new String by allocating memory and copying data
                format!("{}:{}", exchange, symbol)
            }
        }
    }
}

/// Feed aggregator that combines multiple feeds
pub struct FeedAggregator {
    feeds: Vec<FeedHandler>,
    event_rx: mpsc::Receiver<MarketDataEvent>,
    event_tx: mpsc::Sender<MarketDataEvent>,
}

impl FeedAggregator {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100_000);
        Self {
            feeds: Vec::new(),
            event_rx: rx,
            event_tx: tx,
        }
    }

    pub fn add_feed(&mut self, config: FeedConfig) {
        let handler = FeedHandler::new(config, self.event_tx.clone());
        self.feeds.push(handler);
    }

    pub async fn run(mut self) -> mpsc::Receiver<MarketDataEvent> {
        // Spawn feed handlers
        for mut feed in self.feeds {
            tokio::spawn(async move {
                feed.run().await;
            });
        }

        self.event_rx
    }
}
```

---

## 8. Backtesting Infrastructure

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Backtesting Infrastructure                                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Historical Data Store                                    │    │
│  │ • Trade ticks                                           │    │
│  │ • Order book snapshots                                  │    │
│  │ • L2/L3 data                                           │    │
│  └───────────────────────────┬─────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Event Replay Engine                                      │    │
│  │ • Chronological event replay                            │    │
│  │ • Configurable speed (1x, 10x, max)                     │    │
│  │ • Time-travel debugging                                 │    │
│  └───────────────────────────┬─────────────────────────────┘    │
│                              │                                   │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
│  │ Simulated     │  │ Strategy      │  │ Risk          │       │
│  │ Order Book    │  │ Engine        │  │ Model         │       │
│  └───────────────┘  └───────────────┘  └───────────────┘       │
│              │               │               │                  │
│              └───────────────┼───────────────┘                  │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Performance Analytics                                    │    │
│  │ • PnL calculation                                       │    │
│  │ • Sharpe ratio, drawdown, etc.                         │    │
│  │ • Trade-by-trade analysis                              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation

**File: `src/backtest/engine.rs`**

```rust
use chrono::{DateTime, Utc};
use std::collections::BinaryHeap;

/// BinaryHeap in Rust is a max heap means largest element is at root
/// Event for replay
#[derive(Debug, Clone)]
pub struct ReplayEvent {
    pub timestamp: DateTime<Utc>,
    pub event: MarketDataEvent,
}

impl Ord for ReplayEvent {
    /// This line is to reverse the order for min-heap behavior, earlier timestamps appear "larger" to the heap, so they get popped first.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for min-heap behavior
        other.timestamp.cmp(&self.timestamp)
    }
}

impl PartialOrd for ReplayEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ReplayEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for ReplayEvent {}

/// Backtesting engine
pub struct BacktestEngine {
    /// Historical data source
    data_source: Box<dyn HistoricalDataSource>,

    /// Simulated order book
    order_book: SimulatedOrderBook,

    /// Strategy to test
    strategy: Box<dyn Strategy>,

    /// Portfolio/position tracker
    portfolio: Portfolio,

    /// Current simulation time
    current_time: DateTime<Utc>,

    /// Event queue for time-ordered processing
    event_queue: BinaryHeap<ReplayEvent>,

    /// Performance tracker
    performance: PerformanceTracker,
}

#[async_trait::async_trait]
pub trait HistoricalDataSource: Send + Sync {
    async fn get_trades(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Trade>, Box<dyn std::error::Error>>;

    async fn get_book_snapshots(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        interval: Duration,
    ) -> Result<Vec<BookSnapshot>, Box<dyn std::error::Error>>;
}

pub trait Strategy: Send + Sync {
    fn on_trade(&mut self, trade: &Trade) -> Vec<OrderRequest>;
    fn on_book_update(&mut self, book: &BookSnapshot) -> Vec<OrderRequest>;
    fn on_fill(&mut self, fill: &Fill);
}

impl BacktestEngine {
    pub async fn new(
        data_source: Box<dyn HistoricalDataSource>,
        strategy: Box<dyn Strategy>,
        config: BacktestConfig,
    ) -> Self {
        Self {
            data_source,
            order_book: SimulatedOrderBook::new(config.slippage_model),
            strategy,
            portfolio: Portfolio::new(config.initial_capital),
            current_time: config.start_time,
            event_queue: BinaryHeap::new(),
            performance: PerformanceTracker::new(),
        }
    }

    // Load data → Queue events → Pop events chronologically → Strategy reacts → Execute trades → Track performance.
    pub async fn run(&mut self, config: BacktestConfig) -> BacktestResult {
        // Load historical data
        let trades = self.data_source
            .get_trades(&config.symbol, config.start_time, config.end_time)
            .await
            .expect("Failed to load trades");

        // Queue all events
        for trade in trades {
            self.event_queue.push(ReplayEvent {
                timestamp: trade.timestamp,
                event: MarketDataEvent::Trade {
                    exchange: "backtest".to_string(),
                    symbol: trade.symbol.clone(),
                    price: trade.price,
                    quantity: trade.quantity,
                    side: trade.side,
                    timestamp: trade.timestamp,
                    sequence: 0,
                },
            });
        }

        // Process events in chronological order
        while let Some(replay_event) = self.event_queue.pop() {
            self.current_time = replay_event.timestamp;

            match replay_event.event {
                MarketDataEvent::Trade { price, quantity, side, symbol, .. } => {
                    let trade = Trade {
                        symbol,
                        price,
                        quantity,
                        timestamp: self.current_time,
                        // ... other fields
                        ..Default::default()
                    };

                    // Update simulated book
                    self.order_book.on_trade(&trade);

                    // Get strategy signals
                    let orders = self.strategy.on_trade(&trade);

                    // Execute orders against simulated book
                    for order in orders {
                        if let Some(fill) = self.order_book.execute_order(order, self.current_time) {
                            self.portfolio.apply_fill(&fill);
                            self.strategy.on_fill(&fill);
                            self.performance.record_trade(&fill);
                        }
                    }

                    // Record portfolio state
                    let mid_price = self.order_book.mid_price();
                    self.performance.record_equity(
                        self.current_time,
                        self.portfolio.equity(mid_price),
                    );
                }
                // ... handle other event types
                _ => {}
            }
        }

        self.performance.calculate_results()
    }
}

/// Simulated order book with slippage model
pub struct SimulatedOrderBook {
    best_bid: Option<Decimal>,
    best_ask: Option<Decimal>,
    slippage_model: SlippageModel,
}

#[derive(Clone)]
pub enum SlippageModel {
    /// No slippage (fills at quoted price)
    None,
    /// Fixed basis points slippage
    FixedBps(Decimal),
    /// Volume-dependent slippage
    VolumeImpact { coefficient: Decimal },
}

impl SimulatedOrderBook {
    pub fn execute_order(&mut self, order: OrderRequest, timestamp: DateTime<Utc>) -> Option<Fill> {
        let base_price = match order.side {
            OrderSide::Buy => self.best_ask?,
            OrderSide::Sell => self.best_bid?,
        };

        // Apply slippage
        let fill_price = match &self.slippage_model {
            SlippageModel::None => base_price,
            SlippageModel::FixedBps(bps) => {
                let slippage = base_price * bps / dec!(10000);
                match order.side {
                    OrderSide::Buy => base_price + slippage,
                    OrderSide::Sell => base_price - slippage,
                }
            }
            SlippageModel::VolumeImpact { coefficient } => {
                let impact = coefficient * order.quantity.sqrt();
                match order.side {
                    OrderSide::Buy => base_price * (Decimal::ONE + impact),
                    OrderSide::Sell => base_price * (Decimal::ONE - impact),
                }
            }
        };

        Some(Fill {
            order_id: order.id,
            symbol: order.symbol,
            side: order.side,
            price: fill_price,
            quantity: order.quantity,
            timestamp,
            fee: fill_price * order.quantity * dec!(0.001), // 10bps fee
        })
    }
}

/// Performance tracking and analytics
pub struct PerformanceTracker {
    trades: Vec<Fill>,
    equity_curve: Vec<(DateTime<Utc>, Decimal)>,
    peak_equity: Decimal,
    max_drawdown: Decimal,
}

impl PerformanceTracker {
    pub fn calculate_results(&self) -> BacktestResult {
        let total_return = self.calculate_return();
        let sharpe = self.calculate_sharpe();
        let max_drawdown = self.max_drawdown;
        let win_rate = self.calculate_win_rate();
        let profit_factor = self.calculate_profit_factor();

        BacktestResult {
            total_return,
            sharpe_ratio: sharpe,
            max_drawdown,
            win_rate,
            profit_factor,
            total_trades: self.trades.len(),
            equity_curve: self.equity_curve.clone(),
        }
    }

    fn calculate_sharpe(&self) -> Decimal {
        // Calculate daily returns and Sharpe ratio
        // Assumes 252 trading days, risk-free rate of 0
        let returns: Vec<Decimal> = self.equity_curve
            .windows(2)
            .map(|w| (w[1].1 - w[0].1) / w[0].1)
            .collect();

        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let mean: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let variance: Decimal = returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<Decimal>() / Decimal::from(returns.len());

        let std_dev = variance.sqrt().unwrap_or(Decimal::ONE);

        if std_dev.is_zero() {
            return Decimal::ZERO;
        }

        (mean / std_dev) * Decimal::from(252).sqrt().unwrap_or(Decimal::ONE)
    }

    // ... other metric calculations
}

#[derive(Debug)]
pub struct BacktestResult {
    pub total_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub profit_factor: Decimal,
    pub total_trades: usize,
    pub equity_curve: Vec<(DateTime<Utc>, Decimal)>,
}
```

---

## 9. Data Quality & Monitoring

### Data Validation Pipeline

```rust
/// Data quality validator
pub struct DataValidator {
    rules: Vec<Box<dyn ValidationRule>>,
    anomaly_detector: AnomalyDetector,
}

pub trait ValidationRule: Send + Sync {
    fn validate(&self, event: &MarketDataEvent) -> ValidationResult;
    fn name(&self) -> &str;
}

pub enum ValidationResult {
    Valid,
    Warning(String),
    Error(String),
}

/// Price sanity check
pub struct PriceSanityRule {
    max_price_change_pct: Decimal,
    last_prices: DashMap<String, Decimal>,
}

impl ValidationRule for PriceSanityRule {
    fn validate(&self, event: &MarketDataEvent) -> ValidationResult {
        if let MarketDataEvent::Trade { symbol, price, .. } = event {
            if let Some(last) = self.last_prices.get(symbol) {
                let change_pct = ((price - *last) / *last).abs() * dec!(100);

                if change_pct > self.max_price_change_pct {
                    return ValidationResult::Error(format!(
                        "Price change {}% exceeds threshold {}%",
                        change_pct, self.max_price_change_pct
                    ));
                }
            }
            self.last_prices.insert(symbol.clone(), *price);
        }
        ValidationResult::Valid
    }

    fn name(&self) -> &str {
        "PriceSanityRule"
    }
}

/// Timestamp validation
pub struct TimestampRule {
    max_future_offset: Duration,
    max_past_offset: Duration,
}

impl ValidationRule for TimestampRule {
    fn validate(&self, event: &MarketDataEvent) -> ValidationResult {
        let event_time = match event {
            MarketDataEvent::Trade { timestamp, .. } => timestamp,
            MarketDataEvent::Quote { timestamp, .. } => timestamp,
            MarketDataEvent::BookUpdate { timestamp, .. } => timestamp,
        };

        let now = Utc::now();

        if *event_time > now + self.max_future_offset {
            return ValidationResult::Error(format!(
                "Event timestamp {} is too far in the future",
                event_time
            ));
        }

        if *event_time < now - self.max_past_offset {
            return ValidationResult::Warning(format!(
                "Event timestamp {} is stale",
                event_time
            ));
        }

        ValidationResult::Valid
    }

    fn name(&self) -> &str {
        "TimestampRule"
    }
}

/// Statistical anomaly detection
pub struct AnomalyDetector {
    /// Rolling statistics per symbol
    stats: DashMap<String, RollingStats>,
    /// Number of standard deviations for anomaly
    threshold_sigmas: f64,
}

struct RollingStats {
    count: u64,
    mean: f64,
    m2: f64,  // For Welford's algorithm
}

impl RollingStats {
    fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn std_dev(&self) -> f64 {
        if self.count < 2 {
            return f64::MAX;
        }
        (self.m2 / (self.count - 1) as f64).sqrt()
    }

    fn is_anomaly(&self, value: f64, threshold_sigmas: f64) -> bool {
        let z_score = (value - self.mean).abs() / self.std_dev();
        z_score > threshold_sigmas
    }
}

impl AnomalyDetector {
    pub fn check_price_anomaly(&self, symbol: &str, price: Decimal) -> bool {
        let price_f64 = price.to_f64().unwrap_or(0.0);

        let mut stats = self.stats
            .entry(symbol.to_string())
            .or_insert(RollingStats { count: 0, mean: 0.0, m2: 0.0 });

        let is_anomaly = stats.count > 100 && stats.is_anomaly(price_f64, self.threshold_sigmas);

        stats.update(price_f64);

        is_anomaly
    }
}
```

---

## 10. Production Deployment Patterns

### Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Production Data Infrastructure                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Load Balancer (HAProxy / Nginx)                          │   │
│  └────────────────────────┬─────────────────────────────────┘   │
│                           │                                      │
│  ┌────────────────────────┼─────────────────────────────────┐   │
│  │ Application Layer      │                                  │   │
│  │ ┌──────────┐ ┌──────────┐ ┌──────────┐                   │   │
│  │ │ Order    │ │ Order    │ │ Order    │  (3+ replicas)    │   │
│  │ │ Book 1   │ │ Book 2   │ │ Book 3   │                   │   │
│  │ └────┬─────┘ └────┬─────┘ └────┬─────┘                   │   │
│  └──────┼────────────┼────────────┼─────────────────────────┘   │
│         └────────────┼────────────┘                              │
│                      ▼                                           │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Message Queue (Kafka/Redpanda Cluster)                    │  │
│  │ • 3+ brokers, replication factor 3                        │  │
│  │ • Topics: orders, trades, book-updates, events            │  │
│  └───────────────────────┬───────────────────────────────────┘  │
│                          │                                       │
│         ┌────────────────┼────────────────┐                     │
│         ▼                ▼                ▼                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ TimescaleDB │  │ ClickHouse  │  │ Redis       │             │
│  │ (OLTP)      │  │ (Analytics) │  │ (Cache)     │             │
│  │ Primary +   │  │ 3 node      │  │ Cluster     │             │
│  │ 2 Replicas  │  │ cluster     │  │             │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│         │                │                                       │
│         └────────────────┼──────────────────┐                   │
│                          ▼                   │                   │
│  ┌───────────────────────────────────────────┴──────────────┐   │
│  │ Object Storage (S3 / MinIO)                              │   │
│  │ • Historical data archives                               │   │
│  │ • Parquet files for cold data                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Monitoring:                                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │Prometheus│ │ Grafana  │ │ Jaeger   │ │ PagerDuty│          │
│  │          │ │          │ │ (traces) │ │ (alerts) │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Docker Compose for Development

```yaml
# docker-compose.yml
version: '3.8'

services:
  # Message Queue
  redpanda:
    image: redpandadata/redpanda:latest
    command:
      - redpanda start
      - --smp 1
      - --memory 1G
      - --overprovisioned
      - --kafka-addr PLAINTEXT://0.0.0.0:9092
    ports:
      - "9092:9092"
    volumes:
      - redpanda-data:/var/lib/redpanda/data

  # Time-Series Database
  questdb:
    image: questdb/questdb:latest
    ports:
      - "9000:9000"  # Web console
      - "9009:9009"  # InfluxDB line protocol
      - "8812:8812"  # PostgreSQL wire protocol
    volumes:
      - questdb-data:/var/lib/questdb

  # Relational Database
  postgres:
    image: timescale/timescaledb:latest-pg15
    environment:
      POSTGRES_USER: trading
      POSTGRES_PASSWORD: trading
      POSTGRES_DB: trading
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data

  # Analytics Database
  clickhouse:
    image: clickhouse/clickhouse-server:latest
    ports:
      - "8123:8123"  # HTTP
      - "9000:9000"  # Native
    volumes:
      - clickhouse-data:/var/lib/clickhouse

  # Cache
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data

  # Monitoring
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana

volumes:
  redpanda-data:
  questdb-data:
  postgres-data:
  clickhouse-data:
  redis-data:
  grafana-data:
```

---

## Summary: Implementation Roadmap

| Phase | Components | Timeline | Priority |
|-------|------------|----------|----------|
| **Phase 1** | RocksDB WAL, Basic Event Store | 2 weeks | Critical |
| **Phase 2** | Time-Series (QuestDB), Candle Aggregation | 2 weeks | High |
| **Phase 3** | Kafka Integration, Event Sourcing | 3 weeks | High |
| **Phase 4** | Analytics Pipeline, Prometheus | 2 weeks | Medium |
| **Phase 5** | Data Warehouse, Historical Queries | 3 weeks | Medium |
| **Phase 6** | Backtesting Infrastructure | 4 weeks | Medium |
| **Phase 7** | Market Data Feed Handler | 2 weeks | Low |
| **Phase 8** | Data Quality, Monitoring | 2 weeks | Low |

---

## Dependencies

```toml
[dependencies]
# Databases
rocksdb = "0.22"
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "decimal", "chrono", "uuid"] }
redis = "0.24"

# Message Queue
rdkafka = { version = "0.36", features = ["cmake-build"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Time-Series
questdb-rs = "0.2"  # or use TCP directly

# Monitoring
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
dashmap = "6.0"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1.33", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

## Next Steps

1. Start with **Phase 1**: Implement RocksDB-based WAL for durability
2. Add **QuestDB** for time-series analytics
3. Integrate with your existing order book via the event patterns shown
4. See [HFT_FEATURES_ROADMAP.md](./HFT_FEATURES_ROADMAP.md) for trading features
5. See [LOCK_FREE_ARCHITECTURE.md](./LOCK_FREE_ARCHITECTURE.md) for performance optimization
