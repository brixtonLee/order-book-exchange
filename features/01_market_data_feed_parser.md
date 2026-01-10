# Market Data Feed Parser & Time-Series Database

## Purpose

A **Market Data Feed Parser** ingests real-time financial market data from external sources (exchanges like Binance, Coinbase, Kraken) and stores it efficiently in a time-series database. This system is crucial for:

- **Historical analysis**: Backtesting trading strategies
- **Real-time analytics**: Computing indicators, detecting patterns
- **Compliance**: Regulatory requirements for trade surveillance
- **Research**: Market microstructure studies

The time-series database is optimized for:
- High write throughput (thousands to millions of ticks per second)
- Fast range queries by time
- Efficient compression (financial data is highly compressible)
- Low-latency reads for real-time applications

---

## Technology Stack

### Core Libraries

```toml
[dependencies]
# WebSocket clients for exchange APIs
tokio-tungstenite = "0.21"  # Async WebSocket
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"              # Binary serialization
rkyv = "0.7"                 # Zero-copy deserialization

# Time-series storage
mmap-rs = "0.6"              # Memory-mapped files
zstd = "0.13"                # Compression
chrono = "0.4"

# Performance
crossbeam = "0.8"            # Lock-free channels
parking_lot = "0.12"         # Faster mutexes
rayon = "1.8"                # Parallel iterators
```

### Exchange APIs Used
- **Binance WebSocket API**: Real-time tick data
- **Coinbase WebSocket Feed**: Level 2 order book updates
- **Alpha Vantage / Polygon.io**: Stock market data

---

## Implementation Guide

### Phase 1: WebSocket Feed Parser

#### Step 1: Define Market Data Types

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single market tick (trade)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,  // Buy or Sell
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order book snapshot (top N levels)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<(Decimal, Decimal)>,  // (price, quantity)
    pub asks: Vec<(Decimal, Decimal)>,
}
```

**Why these types?**
- `DateTime<Utc>`: Standardize all timestamps to UTC
- `Decimal`: Avoid floating-point precision errors in finance
- `OrderSide`: Explicitly track aggressor side (important for market impact analysis)

---

#### Step 2: WebSocket Client for Binance

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{StreamExt, SinkExt};
use url::Url;

pub struct BinanceFeedParser {
    symbol: String,
    tx: tokio::sync::mpsc::Sender<Tick>,
}

impl BinanceFeedParser {
    pub fn new(symbol: String, tx: tokio::sync::mpsc::Sender<Tick>) -> Self {
        Self { symbol, tx }
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Binance WebSocket URL for trade stream
        let url = format!(
            "wss://stream.binance.com:9443/ws/{}@trade",
            self.symbol.to_lowercase()
        );

        let (ws_stream, _) = connect_async(Url::parse(&url)?).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to trade stream
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [format!("{}@trade", self.symbol.to_lowercase())],
            "id": 1
        });
        write.send(Message::Text(subscribe_msg.to_string())).await?;

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(tick) = self.parse_binance_trade(&text) {
                        self.tx.send(tick).await?;
                    }
                }
                Message::Ping(data) => {
                    write.send(Message::Pong(data)).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_binance_trade(&self, json: &str) -> Result<Tick, serde_json::Error> {
        #[derive(Deserialize)]
        struct BinanceTrade {
            #[serde(rename = "E")]
            event_time: i64,
            #[serde(rename = "p")]
            price: String,
            #[serde(rename = "q")]
            quantity: String,
            #[serde(rename = "m")]
            is_buyer_maker: bool,
        }

        let trade: BinanceTrade = serde_json::from_str(json)?;

        Ok(Tick {
            symbol: self.symbol.clone(),
            timestamp: DateTime::from_timestamp_millis(trade.event_time)
                .unwrap_or_else(|| Utc::now()),
            price: trade.price.parse().unwrap_or_default(),
            quantity: trade.quantity.parse().unwrap_or_default(),
            side: if trade.is_buyer_maker { OrderSide::Sell } else { OrderSide::Buy },
        })
    }
}
```

**Key Design Decisions:**
- **Channels for decoupling**: Parser sends to a channel, database writer consumes
- **Error isolation**: Connection errors don't crash the whole system
- **Heartbeat handling**: Respond to pings to keep connection alive

---

### Phase 2: Time-Series Database Engine

#### Step 3: In-Memory Buffer with Batch Writes

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam::channel::{bounded, Receiver, Sender};

pub struct TimeSeriesDB {
    // In-memory buffer for recent ticks
    buffer: Arc<RwLock<Vec<Tick>>>,

    // File-based storage path
    data_dir: String,

    // Channel for async writes
    write_tx: Sender<Vec<Tick>>,
}

impl TimeSeriesDB {
    pub fn new(data_dir: String) -> Self {
        let (write_tx, write_rx) = bounded::<Vec<Tick>>(100);

        let db = Self {
            buffer: Arc::new(RwLock::new(Vec::with_capacity(100_000))),
            data_dir: data_dir.clone(),
            write_tx,
        };

        // Spawn background writer thread
        let buffer_clone = db.buffer.clone();
        tokio::spawn(async move {
            Self::writer_loop(write_rx, data_dir).await;
        });

        // Spawn periodic flush task
        let buffer_clone2 = db.buffer.clone();
        let tx_clone = db.write_tx.clone();
        tokio::spawn(async move {
            Self::flush_loop(buffer_clone2, tx_clone).await;
        });

        db
    }

    pub fn insert(&self, tick: Tick) {
        let mut buffer = self.buffer.write();
        buffer.push(tick);
    }

    async fn flush_loop(
        buffer: Arc<RwLock<Vec<Tick>>>,
        tx: Sender<Vec<Tick>>,
    ) {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(1)
        );

        loop {
            interval.tick().await;

            let mut buffer_guard = buffer.write();
            if buffer_guard.len() > 10_000 || !buffer_guard.is_empty() {
                let ticks = std::mem::take(&mut *buffer_guard);
                drop(buffer_guard);

                // Send to writer thread (non-blocking)
                let _ = tx.try_send(ticks);
            }
        }
    }

    async fn writer_loop(rx: Receiver<Vec<Tick>>, data_dir: String) {
        while let Ok(ticks) = rx.recv() {
            if let Err(e) = Self::write_to_disk(&data_dir, &ticks).await {
                eprintln!("Write error: {}", e);
            }
        }
    }

    async fn write_to_disk(
        data_dir: &str,
        ticks: &[Tick],
    ) -> std::io::Result<()> {
        if ticks.is_empty() {
            return Ok(());
        }

        // Group by symbol and day
        use std::collections::HashMap;
        let mut grouped: HashMap<(String, String), Vec<&Tick>> = HashMap::new();

        for tick in ticks {
            let date_key = tick.timestamp.format("%Y-%m-%d").to_string();
            let key = (tick.symbol.clone(), date_key);
            grouped.entry(key).or_default().push(tick);
        }

        // Write each group to separate file
        for ((symbol, date), group) in grouped {
            let file_path = format!("{}/{}/{}.bin", data_dir, symbol, date);

            // Ensure directory exists
            let dir = format!("{}/{}", data_dir, symbol);
            tokio::fs::create_dir_all(&dir).await?;

            // Serialize with bincode (binary format)
            let encoded = bincode::serialize(&group)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            // Compress with zstd
            let compressed = zstd::encode_all(&encoded[..], 3)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            // Append to file
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .await?;

            // Write length prefix (for reading later)
            file.write_u32(compressed.len() as u32).await?;
            file.write_all(&compressed).await?;
            file.flush().await?;
        }

        Ok(())
    }
}
```

**Why this design?**
- **Write buffering**: Amortize I/O cost by batching writes
- **Periodic flushing**: Balance between latency and throughput
- **Partitioning by symbol/date**: Efficient range queries
- **Compression**: Financial data compresses 10-20x with zstd
- **Length prefixes**: Enable reading variable-length compressed chunks

---

#### Step 4: Efficient Querying with Memory-Mapped Files

```rust
use memmap2::Mmap;
use std::fs::File;

impl TimeSeriesDB {
    pub async fn query(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> std::io::Result<Vec<Tick>> {
        let mut results = Vec::new();

        // Generate file paths for date range
        let mut current_date = start.date_naive();
        let end_date = end.date_naive();

        while current_date <= end_date {
            let file_path = format!(
                "{}/{}/{}.bin",
                self.data_dir,
                symbol,
                current_date.format("%Y-%m-%d")
            );

            if let Ok(ticks) = Self::read_file(&file_path).await {
                // Filter by timestamp
                results.extend(
                    ticks.into_iter()
                        .filter(|t| t.timestamp >= start && t.timestamp <= end)
                );
            }

            current_date = current_date.succ_opt().unwrap();
        }

        Ok(results)
    }

    async fn read_file(file_path: &str) -> std::io::Result<Vec<Tick>> {
        let file = File::open(file_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let mut results = Vec::new();
        let mut offset = 0;

        // Read all chunks in the file
        while offset + 4 <= mmap.len() {
            // Read length prefix
            let length = u32::from_le_bytes([
                mmap[offset],
                mmap[offset + 1],
                mmap[offset + 2],
                mmap[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + length > mmap.len() {
                break;
            }

            // Decompress chunk
            let compressed = &mmap[offset..offset + length];
            let decompressed = zstd::decode_all(compressed)?;

            // Deserialize
            let ticks: Vec<Tick> = bincode::deserialize(&decompressed)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            results.extend(ticks);
            offset += length;
        }

        Ok(results)
    }
}
```

**Memory-mapped files benefits:**
- OS manages caching automatically
- Zero-copy reads (data not copied to user space)
- Lazy loading (only accessed pages loaded into RAM)

---

### Phase 3: Putting It All Together

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database
    let db = Arc::new(TimeSeriesDB::new("./market_data".to_string()));

    // Create channel for ticks
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Tick>(10_000);

    // Start feed parser
    let parser = BinanceFeedParser::new("BTCUSDT".to_string(), tx.clone());
    tokio::spawn(async move {
        if let Err(e) = parser.connect().await {
            eprintln!("Parser error: {}", e);
        }
    });

    // Consume ticks and insert into database
    let db_clone = db.clone();
    tokio::spawn(async move {
        while let Some(tick) = rx.recv().await {
            db_clone.insert(tick);
        }
    });

    // Example query
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);

    let ticks = db.query("BTCUSDT", one_hour_ago, now).await?;
    println!("Retrieved {} ticks from last hour", ticks.len());

    Ok(())
}
```

---

## Advanced Optimizations

### 1. **Zero-Copy Deserialization with `rkyv`**

Replace `bincode` with `rkyv` for 10-100x faster reads:

```rust
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize)]
pub struct Tick {
    pub symbol: String,
    pub timestamp: i64,  // Unix timestamp
    pub price: i64,      // Fixed-point (multiply by 10^8)
    pub quantity: i64,
    pub side: u8,        // 0 = Buy, 1 = Sell
}

// Writing
let bytes = rkyv::to_bytes::<_, 256>(&ticks)?;

// Reading (ZERO COPY!)
let archived = rkyv::check_archived_root::<Vec<Tick>>(&bytes)?;
// Can directly access fields without deserializing!
```

### 2. **Columnar Storage for Analytics**

Store each field in separate files for faster queries:

```
market_data/
  BTCUSDT/
    2024-01-15/
      timestamps.bin  (compressed array of i64)
      prices.bin      (compressed array of i64)
      quantities.bin  (compressed array of i64)
      sides.bin       (compressed bitset)
```

Benefits:
- Only read columns you need
- Better compression (similar values together)
- SIMD-friendly for computations

---

## Advantages

1. **Performance**
   - Handle millions of ticks per second
   - Sub-millisecond query latency for recent data
   - Compression reduces storage by 10-20x

2. **Cost Efficiency**
   - Store years of tick data on modest hardware
   - Lower cloud storage costs

3. **Flexibility**
   - Easy to add new data sources
   - Custom query optimizations

4. **Learning Value**
   - Master async Rust and concurrency
   - Understand database internals
   - Practice zero-copy techniques

---

## Disadvantages

1. **Complexity**
   - More code to maintain than using existing DB
   - Need to handle edge cases (corruption, partial writes)

2. **Limited Query Capabilities**
   - No SQL-like expressions
   - Manual index management

3. **Operational Burden**
   - No built-in replication or backup
   - Manual monitoring required

4. **Bugs**
   - File corruption can lose data
   - Race conditions in concurrent access

---

## Limitations

1. **Single-Machine Only**
   - No distributed storage (without adding clustering)

2. **No Transactional Guarantees**
   - Crashes during writes may lose data
   - No ACID properties

3. **Limited Concurrency**
   - Write-heavy workloads may bottleneck
   - No multi-writer support

4. **Schema Changes Are Hard**
   - Adding fields requires migration scripts

5. **No Built-in Analytics**
   - Need to write custom aggregation functions

---

## Alternatives

### 1. **InfluxDB** (Time-Series DB)
- **Pros**: Production-ready, clustering, SQL-like queries
- **Cons**: Higher latency than custom solution, more resource-intensive
- **When to use**: Production systems, teams without time to build custom

### 2. **TimescaleDB** (PostgreSQL Extension)
- **Pros**: Full SQL, reliable, mature ecosystem
- **Cons**: Slower than specialized solutions
- **When to use**: Need relational features + time-series

### 3. **QuestDB**
- **Pros**: Extremely fast, built for financial data
- **Cons**: Less mature, smaller community
- **When to use**: High-performance trading systems

### 4. **Parquet Files + DuckDB**
- **Pros**: Columnar format, excellent compression, SQL analytics
- **Cons**: Not optimized for real-time writes
- **When to use**: Batch analytics, backtesting

### 5. **Clickhouse**
- **Pros**: Blazing fast analytics, excellent compression
- **Cons**: Complex setup, overkill for small datasets
- **When to use**: Multi-terabyte datasets, OLAP queries

### 6. **Redis TimeSeries**
- **Pros**: Simple, in-memory, fast
- **Cons**: Limited by RAM, no complex queries
- **When to use**: Real-time dashboards, small datasets

---

## Recommended Path

1. **Start Simple**: Build basic version with in-memory buffer + file writes
2. **Add Compression**: Implement zstd compression (easy 10x savings)
3. **Optimize Reads**: Add memory-mapped files
4. **Go Zero-Copy**: Migrate to `rkyv` for hot paths
5. **Add Indexing**: Build secondary indices for non-time queries
6. **Consider Alternatives**: If needs exceed custom solution, migrate to InfluxDB/QuestDB

---

## Further Reading

- [Building a Time-Series Database](https://nakabonne.dev/posts/write-tsdb-from-scratch/)
- [rkyv Zero-Copy Serialization](https://rkyv.org/)
- [Memory-Mapped Files in Rust](https://docs.rs/memmap2)
- [Binance WebSocket API](https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams)
