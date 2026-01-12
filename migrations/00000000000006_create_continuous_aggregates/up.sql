-- ⚠️  IMPORTANT: This migration is a PLACEHOLDER
--
-- TimescaleDB continuous aggregates cannot be created inside transaction blocks,
-- but Diesel CLI wraps all migrations in transactions by default.
--
-- The actual continuous aggregate setup is in:
--   migrations/manual_continuous_aggregates.sql
--
-- To apply this migration, run:
--   psql postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries \
--     -f migrations/manual_continuous_aggregates.sql
--
-- This creates:
--   - 7 continuous aggregates (1m, 5m, 15m, 30m, 1h, 4h, 1d OHLC candles)
--   - Automatic refresh policies for each timeframe
--   - Optimized indexes for query performance
--
-- This migration placeholder allows Diesel to track that this step was acknowledged.

SELECT 1 AS continuous_aggregates_placeholder;
