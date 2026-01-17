# PostgreSQL & TimescaleDB Integration

This document describes the PostgreSQL and TimescaleDB integration for persisting market data, symbols, and OHLC candles.

## Table of Contents
- [Overview](#overview)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Database Setup](#database-setup)
- [Configuration](#configuration)
- [API Endpoints](#api-endpoints)
- [Background Jobs](#background-jobs)
- [Querying Data](#querying-data)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

---

## Overview

### Features
- **Dual Database Setup**:
  - PostgreSQL for reference data (symbols metadata)
  - TimescaleDB for time-series data (ticks, OHLC candles)

- **High-Performance Tick Persistence**:
  - Batch insertion (1000 ticks or 100ms window)
  - Automatic deduplication via unique constraints
  - Non-blocking async processing

- **Automatic OHLC Aggregation**:
  - TimescaleDB continuous aggregates
  - 7 timeframes: 1m, 5m, 15m, 30m, 1h, 4h, 1d
  - Near real-time updates

- **Data Management**:
  - Auto-compression (7 days for ticks, 30 days for OHLC)
  - Auto-retention (90 days for ticks, 2 years for OHLC)
  - Symbol synchronization every 5 minutes

---

## Quick Start

### 1. Start Databases
```bash
docker-compose up -d postgres timescaledb
```

### 2. Install Diesel CLI
```bash
cargo install diesel_cli --no-default-features --features postgres
```

### 3. Run Migrations
```bash
# Migrate metadata database (PostgreSQL)
diesel migration run --database-url=postgres://orderbook:orderbook123@localhost:5432/orderbook_metadata

# Migrate timeseries database (TimescaleDB)
diesel migration run --database-url=postgres://orderbook:orderbook123@localhost:5433/orderbook_timeseries
```

### 4. Configure Environment
```bash
cp .env.example .env
# Edit .env and verify database URLs
```

### 5. Start Server
```bash
cargo run
```

You should see:
```
🗄️  Initializing PostgreSQL and TimescaleDB connections...
✅ Database connections established successfully
✅ Tick persister configured (batch_size=1000, flush_interval=100ms)
✅ Database integration complete
⏰ Initializing cron scheduler...
✅ Cron scheduler started successfully
```

---

## Architecture

### Database Structure

```
┌─────────────────────────────────────┐
│  PostgreSQL (port 5432)             │
│  orderbook_metadata                 │
├─────────────────────────────────────┤
│  • symbols (reference data)         │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│  TimescaleDB (port 5433)            │
│  orderbook_timeseries               │
├─────────────────────────────────────┤
│  • ticks (hypertable, 1-day chunks) │
│  • ohlc_candles (hypertable)        │
│  • ohlc_1m ... ohlc_1d (views)      │
└─────────────────────────────────────┘
```

### Data Flow

```
FIX Feed
   ↓
MarketTick
   ↓
┌──────────────────────────────────────┐
│  Fan-out (DatasourceManager)        │
├──────────────────────────────────────┤
│  1. WebSocket Broadcast              │
│  2. RabbitMQ Publisher               │
│  3. Tick Persister → TimescaleDB     │
└──────────────────────────────────────┘
   ↓
TimescaleDB ticks table
   ↓
Continuous Aggregates (auto)
   ↓
OHLC candles (1m, 5m, 15m, ...)
```

---

## Database Setup

### Tables

#### `symbols` (PostgreSQL)
Stores trading instrument metadata synced from cTrader FIX every 5 minutes.

| Column | Type | Description |
|--------|------|-------------|
| `symbol_id` | BIGINT (PK) | cTrader symbol ID |
| `symbol_name` | VARCHAR(50) | e.g., "EURUSD", "XAUUSD" |
| `description` | TEXT | Symbol description |
| `digits` | INTEGER | Price precision (decimal places) |
| `tick_size` | NUMERIC | Minimum price increment |
| `contract_size` | NUMERIC | Contract size (CFDs/futures) |
| `created_at` | TIMESTAMPTZ | Record creation time |
| `updated_at` | TIMESTAMPTZ | Last update time |
| `last_synced_at` | TIMESTAMPTZ | Last sync from FIX |

#### `ticks` (TimescaleDB Hypertable)
Stores market data ticks with automatic partitioning.

| Column | Type | Description |
|--------|------|-------------|
| `id` | BIGSERIAL | Auto-increment ID |
| `symbol_id` | BIGINT | FK to symbols |
| `symbol_name` | VARCHAR(50) | Denormalized for queries |
| `tick_time` | TIMESTAMPTZ | Tick timestamp (partition key) |
| `bid_price` | NUMERIC(20,8) | Best bid |
| `ask_price` | NUMERIC(20,8) | Best ask |
| `bid_volume` | NUMERIC(20,8) | Bid volume |
| `ask_volume` | NUMERIC(20,8) | Ask volume |
| `created_at` | TIMESTAMPTZ | Insert time |

**Unique Constraint**: `(symbol_id, symbol_name, tick_time)`
**Compression**: After 7 days
**Retention**: 90 days

#### `ohlc_candles` (TimescaleDB Hypertable)
Populated by continuous aggregates (no manual inserts needed).

| Column | Type | Description |
|--------|------|-------------|
| `id` | BIGSERIAL | Auto-increment ID |
| `symbol_id` | BIGINT | FK to symbols |
| `symbol_name` | VARCHAR(50) | Denormalized |
| `timeframe` | VARCHAR(10) | 1m, 5m, 15m, 30m, 1h, 4h, 1d |
| `open_time` | TIMESTAMPTZ | Candle start (partition key) |
| `close_time` | TIMESTAMPTZ | Candle end |
| `open_price` | NUMERIC(20,8) | First tick price |
| `high_price` | NUMERIC(20,8) | Highest price |
| `low_price` | NUMERIC(20,8) | Lowest price |
| `close_price` | NUMERIC(20,8) | Last tick price |
| `volume` | NUMERIC(20,8) | Total volume |
| `tick_count` | BIGINT | Number of ticks aggregated |
| `created_at` | TIMESTAMPTZ | Insert time |

**Compression**: After 30 days
**Retention**: 2 years

---

## Configuration

### Environment Variables

```bash
# PostgreSQL (metadata)
DATABASE_URL=postgres://orderbook:orderbook123@localhost:5432/orderbook_metadata

# TimescaleDB (time-series)
TIMESCALEDB_URL=postgres://orderbook:orderbook123@localhost:5433/orderbook_timeseries

# Connection pool settings
DB_POOL_MIN_SIZE=5
DB_POOL_MAX_SIZE=20
DB_POOL_TIMEOUT_SECONDS=30

# Tick persistence tuning
TICK_BATCH_SIZE=1000          # Buffer size before flush
TICK_FLUSH_INTERVAL_MS=100    # Max time between flushes
```

### Docker Compose

```yaml
services:
  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: orderbook
      POSTGRES_PASSWORD: orderbook123
      POSTGRES_DB: orderbook_metadata

  timescaledb:
    image: timescale/timescaledb:latest-pg16
    ports:
      - "5433:5432"
    environment:
      POSTGRES_USER: orderbook
      POSTGRES_PASSWORD: orderbook123
      POSTGRES_DB: orderbook_timeseries
```

---

## API Endpoints

### Symbol Endpoints

```http
GET /api/v1/symbols
GET /api/v1/symbols/{symbol_id}
GET /api/v1/symbols/name/{symbol_name}
```

**Example**:
```bash
curl http://localhost:3000/api/v1/symbols

# Response:
[
  {
    "symbol_id": 1,
    "symbol_name": "EURUSD",
    "digits": 5,
    "tick_size": "0.00001",
    "created_at": "2024-01-10T12:00:00Z"
  }
]
```

### Tick Endpoints

```http
GET /api/v1/ticks/{symbol_id}?from={rfc3339}&to={rfc3339}&limit=1000
GET /api/v1/ticks/{symbol_id}/latest
```

**Example**:
```bash
# Get last 100 ticks for symbol ID 1
curl "http://localhost:3000/api/v1/ticks/1?limit=100"

# Get ticks in time range
curl "http://localhost:3000/api/v1/ticks/1?from=2024-01-10T00:00:00Z&to=2024-01-10T12:00:00Z"

# Response:
[
  {
    "id": 12345,
    "symbol_id": 1,
    "symbol_name": "EURUSD",
    "tick_time": "2024-01-10T12:00:00.123Z",
    "bid_price": "1.10005",
    "ask_price": "1.10010",
    "bid_volume": "1000000",
    "ask_volume": "1000000"
  }
]
```

### OHLC Endpoints

```http
GET /api/v1/ohlc/{symbol_id}?timeframe={1m|5m|15m|30m|1h|4h|1d}&from={rfc3339}&to={rfc3339}&limit=500
GET /api/v1/ohlc/{symbol_id}/latest?timeframe={timeframe}
```

**Example**:
```bash
# Get 5-minute candles for last 24 hours
curl "http://localhost:3000/api/v1/ohlc/1?timeframe=5m&limit=288"

# Get latest 1-hour candle
curl "http://localhost:3000/api/v1/ohlc/1/latest?timeframe=1h"

# Response:
[
  {
    "symbol_id": 1,
    "symbol_name": "EURUSD",
    "timeframe": "5m",
    "open_time": "2024-01-10T12:00:00Z",
    "close_time": "2024-01-10T12:05:00Z",
    "open_price": "1.10005",
    "high_price": "1.10015",
    "low_price": "1.10000",
    "close_price": "1.10010",
    "volume": "5000000",
    "tick_count": 245
  }
]
```

---

## Background Jobs

### Symbol Sync Job
**Schedule**: Every 5 minutes (`0 */5 * * * *`)

**Purpose**: Synchronizes symbols from cTrader FIX to PostgreSQL

**Behavior**:
- Fetches symbol mapping from `DatasourceManager`
- Upserts symbols to database (insert new, update existing)
- Updates `last_synced_at` timestamp
- Logs sync statistics

**Manual Trigger**: Not exposed via API (cron only)

---

## Querying Data

### Direct SQL Queries

```sql
-- Connect to TimescaleDB
psql postgres://orderbook:orderbook123@localhost:5433/orderbook_timeseries

-- View latest ticks
SELECT symbol_name, tick_time, bid_price, ask_price
FROM ticks
ORDER BY tick_time DESC
LIMIT 10;

-- View 5-minute candles
SELECT * FROM ohlc_5m
WHERE symbol_id = 1
  AND open_time >= NOW() - INTERVAL '1 day'
ORDER BY open_time DESC;

-- Check hypertable info
SELECT * FROM timescaledb_information.hypertables;

-- View continuous aggregate policies
SELECT * FROM timescaledb_information.continuous_aggregates;

-- View retention policies
SELECT * FROM timescaledb_information.jobs WHERE proc_name LIKE '%retention%';
```

### Using Diesel CLI

```bash
# Generate schema after migrations
diesel print-schema --database-url=$DATABASE_URL > src/database/schema_metadata.rs
diesel print-schema --database-url=$TIMESCALEDB_URL > src/database/schema_timeseries.rs
```

---

## Performance Tuning

### Batch Size Tuning

| Throughput | Batch Size | Flush Interval |
|------------|------------|----------------|
| Low (< 100 tps) | 100 | 1000ms |
| Medium (100-1000 tps) | 500 | 500ms |
| High (1000-5000 tps) | 1000 | 100ms |
| Very High (> 5000 tps) | 2000 | 50ms |

```bash
# High throughput configuration
TICK_BATCH_SIZE=2000
TICK_FLUSH_INTERVAL_MS=50
```

### Connection Pool Sizing

```bash
# For high concurrency
DB_POOL_MAX_SIZE=50

# For low memory environments
DB_POOL_MAX_SIZE=10
```

### TimescaleDB Optimizations

```sql
-- Adjust chunk interval for higher write throughput
SELECT set_chunk_time_interval('ticks', INTERVAL '6 hours');

-- Disable compression for very recent data
SELECT remove_compression_policy('ticks');
SELECT add_compression_policy('ticks', INTERVAL '30 days');

-- Manual refresh of continuous aggregates
CALL refresh_continuous_aggregate('ohlc_5m', NULL, NULL);
```

---

## Troubleshooting

### Database Connection Failed

**Error**: `Failed to establish database connections`

**Solution**:
```bash
# Check Docker containers are running
docker ps | grep postgres

# Restart containers
docker-compose restart postgres timescaledb

# Verify connectivity
psql postgres://orderbook:orderbook123@localhost:5432/orderbook_metadata -c "SELECT 1"
```

### Migrations Failed

**Error**: `Migration error: table already exists`

**Solution**:
```bash
# Rollback migrations
diesel migration revert --database-url=$DATABASE_URL

# Re-run
diesel migration run --database-url=$DATABASE_URL
```

### Ticks Not Persisting

**Checks**:
1. Database integration enabled (check startup logs for "✅ Tick persister configured")
2. FIX connection active and receiving ticks
3. Check for database errors in logs
4. Verify unique constraint violations (duplicates are ignored)

```bash
# Check tick count
psql $TIMESCALEDB_URL -c "SELECT COUNT(*) FROM ticks;"

# View recent inserts
psql $TIMESCALEDB_URL -c "SELECT * FROM ticks ORDER BY created_at DESC LIMIT 10;"
```

### OHLC Candles Empty

**Cause**: Continuous aggregates need initial data and time to materialize

**Solution**:
```sql
-- Manually refresh continuous aggregate
CALL refresh_continuous_aggregate('ohlc_1m', NULL, NULL);
CALL refresh_continuous_aggregate('ohlc_5m', NULL, NULL);

-- Check aggregate policies
SELECT * FROM timescaledb_information.continuous_aggregates;
```

### High Memory Usage

**Solution**: Reduce batch size and connection pool

```bash
TICK_BATCH_SIZE=500
DB_POOL_MAX_SIZE=10
```

---

## Development

### Adding New Migrations

```bash
# Create new migration
diesel migration generate add_new_feature --database-url=$TIMESCALEDB_URL

# Edit migrations/XXXXXX_add_new_feature/up.sql and down.sql

# Apply migration
diesel migration run --database-url=$TIMESCALEDB_URL

# Update schema
diesel print-schema --database-url=$TIMESCALEDB_URL > src/database/schema.rs
```

### Testing

```bash
# Run tests (requires test database)
cargo test --features database_tests

# Integration test with real database
cargo test --test integration_test -- --nocapture
```

---

## References

- [Diesel Documentation](https://diesel.rs/)
- [TimescaleDB Documentation](https://docs.timescale.com/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Continuous Aggregates Guide](https://docs.timescale.com/use-timescale/latest/continuous-aggregates/)
