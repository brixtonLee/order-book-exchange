# PostgreSQL and TimescaleDB Integration - Setup Complete

## Overview
Successfully integrated dual-database architecture with PostgreSQL for reference data and TimescaleDB for time-series market data.

## Database Architecture

### PostgreSQL (port 5432)
- **Database**: `orderbook_metadata`
- **Purpose**: Reference data (symbols metadata)
- **Tables**:
  - `symbols` - Symbol definitions from FIX feed

### TimescaleDB (port 5433)
- **Database**: `orderbook_timeseries`
- **Purpose**: Time-series market data
- **Hypertables**:
  - `ticks` - Market tick data (1-day chunks, 7-day compression)
  - `ohlc_candles` - OHLC candles (7-day chunks, 30-day compression)
- **Continuous Aggregates** (7 timeframes):
  - `ohlc_1m` - 1-minute candles
  - `ohlc_5m` - 5-minute candles
  - `ohlc_15m` - 15-minute candles
  - `ohlc_30m` - 30-minute candles
  - `ohlc_1h` - 1-hour candles
  - `ohlc_4h` - 4-hour candles
  - `ohlc_1d` - 1-day candles

## Migration Status

All migrations completed successfully:

1. ✅ `create_symbols_table` - Symbol metadata table
2. ✅ `create_ticks_table` - Tick data table
3. ✅ `create_ticks_hypertable` - Convert ticks to TimescaleDB hypertable
4. ✅ `create_ohlc_candles_table` - OHLC candle table
5. ✅ `create_ohlc_hypertable` - Convert OHLC to TimescaleDB hypertable
6. ✅ `create_continuous_aggregates` - Manual continuous aggregates (see below)
7. ✅ `create_retention_policies` - Data retention policies

### Important Note: Continuous Aggregates

TimescaleDB continuous aggregates cannot be created inside transaction blocks, but Diesel CLI wraps migrations in transactions. Therefore:

- Migration 6 is a **placeholder** in Diesel
- Actual continuous aggregates are created via: `migrations/manual_continuous_aggregates.sql`
- Run manually: `psql postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries -f migrations/manual_continuous_aggregates.sql`
- **Already completed** - all 7 continuous aggregates are active

## Schema Highlights

### Ticks Table
```sql
CREATE TABLE ticks (
    id BIGSERIAL,
    symbol_id BIGINT NOT NULL,
    symbol_name VARCHAR(50) NOT NULL,
    tick_time TIMESTAMPTZ NOT NULL,
    bid_price NUMERIC(20, 8) NOT NULL,
    ask_price NUMERIC(20, 8) NOT NULL,
    bid_volume NUMERIC(20, 8) NOT NULL,
    ask_volume NUMERIC(20, 8) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (symbol_id, symbol_name, tick_time)
);
```
- **Composite Primary Key**: Required by TimescaleDB for hypertables
- **Partition Column**: `tick_time` (1-day chunks)
- **Compression**: Enabled after 7 days

### OHLC Candles Table
```sql
CREATE TABLE ohlc_candles (
    id BIGSERIAL,
    symbol_id BIGINT NOT NULL,
    symbol_name VARCHAR(50) NOT NULL,
    timeframe VARCHAR(10) NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    close_time TIMESTAMPTZ NOT NULL,
    open_price NUMERIC(20, 8) NOT NULL,
    high_price NUMERIC(20, 8) NOT NULL,
    low_price NUMERIC(20, 8) NOT NULL,
    close_price NUMERIC(20, 8) NOT NULL,
    volume NUMERIC(20, 8) NOT NULL DEFAULT 0,
    tick_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (symbol_id, timeframe, open_time)
);
```
- **Composite Primary Key**: (symbol_id, timeframe, open_time)
- **Partition Column**: `open_time` (7-day chunks)
- **Compression**: Enabled after 30 days

## Application Features

### Tick Persistence
- **Batch Processing**: Ticks are batched in memory (1000 ticks or 100ms flush interval)
- **Fan-out Architecture**: Ticks broadcast to:
  1. WebSocket clients
  2. RabbitMQ publisher
  3. Database persister (PostgreSQL + TimescaleDB)
- **High Performance**: Async batch inserts minimize database round-trips

### Cron Scheduler
- **Symbol Sync Job**: Runs every 5 minutes
- **Purpose**: Sync symbols from FIX feed to PostgreSQL
- **Auto-starts**: Initialized on server startup

### New API Endpoints

```
GET  /api/v1/symbols                        # List all symbols
GET  /api/v1/symbols/{symbol_id}            # Get symbol by ID
GET  /api/v1/ticks/{symbol_id}              # Get ticks for symbol
GET  /api/v1/ohlc/{symbol_id}?timeframe=5m  # Get OHLC candles
```

## Repository Pattern (SOLID Principles)

### Interfaces (Traits)
- `SymbolRepository` - Symbol CRUD operations
- `TickRepository` - Tick persistence and queries
- `OhlcRepository` - OHLC candle queries

### Implementations
- `SymbolRepositoryImpl` - PostgreSQL implementation
- `TickRepositoryImpl` - TimescaleDB implementation
- `OhlcRepositoryImpl` - TimescaleDB continuous aggregate queries

### Dependency Injection
Connection pools are injected via closures:
```rust
let symbol_repo = Arc::new(SymbolRepositoryImpl::new(move || {
    pools.get_metadata_conn()
}));
```

## Connection Pooling

### Configuration
- **Pool Size**: 20 connections per database
- **Implementation**: r2d2 with diesel::pg::PgConnection
- **Thread-Safe**: Arc-wrapped pools shared across application

### Usage in Code
```rust
pub struct DatabasePools {
    pub metadata_pool: Arc<PgPool>,
    pub timeseries_pool: Arc<PgPool>,
}
```

## Diesel CLI Commands

### Apply Migrations
```bash
# TimescaleDB (timeseries data)
diesel migration run --database-url postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries

# PostgreSQL (metadata)
diesel migration run --database-url postgresql://orderbook:orderbook123@localhost:5432/orderbook_metadata
```

### Revert Migrations
```bash
diesel migration revert --database-url postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries
```

### Generate Schema
```bash
diesel print-schema --database-url postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries > src/database/schema.rs
```

## Docker Commands

### Start Databases
```bash
docker-compose up -d postgres timescaledb
```

### View Logs
```bash
docker logs order-book-timescaledb -f
docker logs order-book-postgres -f
```

### Connect to Database
```bash
# TimescaleDB
psql postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries

# PostgreSQL
psql postgresql://orderbook:orderbook123@localhost:5432/orderbook_metadata
```

### Drop and Recreate
```bash
docker-compose down
docker volume prune  # Warning: deletes all data
docker-compose up -d
```

## Verification Commands

### Check Hypertables
```sql
SELECT hypertable_name, num_chunks
FROM timescaledb_information.hypertables;
```

### Check Continuous Aggregates
```sql
SELECT view_name, materialization_hypertable_name
FROM timescaledb_information.continuous_aggregates;
```

### Check Compression Policies
```sql
SELECT hypertable_name, compress_after
FROM timescaledb_information.compression_settings;
```

### Check Retention Policies
```sql
SELECT hypertable_name, drop_after
FROM timescaledb_information.data_retention_policies;
```

## Performance Optimizations

### TimescaleDB Features
1. **Chunking**: Automatic time-based partitioning
   - Ticks: 1-day chunks
   - OHLC: 7-day chunks

2. **Compression**: Columnar compression for old data
   - Ticks: Compress after 7 days
   - OHLC: Compress after 30 days

3. **Continuous Aggregates**: Pre-computed OHLC candles
   - Auto-refresh every 1-5 minutes
   - Query against materialized views (fast)

4. **Retention Policies**: Automatic data cleanup
   - Ticks: Drop after 90 days
   - OHLC: Drop after 730 days (2 years)

### Application Optimizations
1. **Batch Inserts**: Up to 1000 ticks per batch
2. **Connection Pooling**: Reuse database connections
3. **Async I/O**: Non-blocking database operations
4. **Decimal Precision**: rust_decimal for financial accuracy

## Next Steps

### Testing
1. Connect to cTrader FIX feed
2. Verify tick ingestion and batching
3. Monitor continuous aggregate refreshes
4. Query OHLC candles via API

### Production Readiness
1. Configure environment variables for credentials
2. Set up database backups (pg_dump, WAL archiving)
3. Monitor connection pool metrics
4. Add database health check endpoints
5. Configure TimescaleDB memory settings for production load

## Environment Variables

```bash
# PostgreSQL
DATABASE_URL=postgresql://orderbook:orderbook123@localhost:5432/orderbook_metadata

# TimescaleDB
TIMESCALEDB_URL=postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries

# Connection Pool
DATABASE_POOL_SIZE=20

# Tick Persister
TICK_BATCH_SIZE=1000
TICK_FLUSH_INTERVAL_MS=100
```

## Troubleshooting

### Migration Errors
- **"cannot create unique index without partition column"**: Ensure primary key includes partition column
- **"columnstore not enabled"**: Add `ALTER TABLE ... SET (timescaledb.compress)` before compression policy
- **"cannot run inside transaction block"**: Continuous aggregates must be created outside Diesel migrations

### Connection Issues
- Check Docker containers are running: `docker ps`
- Verify port availability: `lsof -i :5432` and `lsof -i :5433`
- Test direct connection: `psql postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries`

### Performance Issues
- Monitor connection pool usage
- Check chunk size vs query patterns
- Review compression and retention policies
- Analyze query plans with EXPLAIN

---

**Status**: ✅ All migrations complete, databases operational, server running on http://127.0.0.1:3000
