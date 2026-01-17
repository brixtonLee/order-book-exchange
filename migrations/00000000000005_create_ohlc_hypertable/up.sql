-- Convert ohlc_candles table to TimescaleDB hypertable
-- Partition by open_time with 7-day chunks (OHLC data accessed less frequently than raw ticks)
SELECT create_hypertable('ohlc_candles', 'open_time',
    chunk_time_interval => INTERVAL '7 days',
    if_not_exists => TRUE
);

-- Create TimescaleDB-optimized indexes
CREATE INDEX IF NOT EXISTS idx_ohlc_hypertable_symbol_timeframe
    ON ohlc_candles(symbol_id, timeframe, open_time DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_ohlc_hypertable_name_timeframe
    ON ohlc_candles(symbol_name, timeframe, open_time DESC, id DESC);

-- Enable compression on the hypertable
ALTER TABLE ohlc_candles SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'symbol_id, symbol_name, timeframe',
    timescaledb.compress_orderby = 'open_time DESC'
);

-- Set compression policy (compress chunks older than 30 days)
-- OHLC data is more valuable long-term than raw ticks
SELECT add_compression_policy('ohlc_candles', INTERVAL '30 days');

-- Add comment
COMMENT ON TABLE ohlc_candles IS 'TimescaleDB hypertable for OHLC candles (7-day chunks, 30-day compression)';
