-- Remove compression policy
SELECT remove_compression_policy('ohlc_candles', if_exists => true);

-- Drop TimescaleDB-specific indexes
DROP INDEX IF EXISTS idx_ohlc_hypertable_name_timeframe;
DROP INDEX IF EXISTS idx_ohlc_hypertable_symbol_timeframe;

-- Note: Converting back from hypertable to regular table is complex
-- For simplicity, rollback would recreate as regular table
-- In production, export data before rolling back
