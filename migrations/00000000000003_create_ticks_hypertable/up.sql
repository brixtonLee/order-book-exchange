-- Enable TimescaleDB extension (if not already enabled)
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Convert ticks table to TimescaleDB hypertable
-- Partition by tick_time with 1-day chunks for optimal query performance
SELECT create_hypertable('ticks', 'tick_time',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create index on hypertable (TimescaleDB-optimized)
-- Note: Primary key on (tick_time, id) for better partitioning
-- The UNIQUE constraint will be recreated on the hypertable
CREATE INDEX IF NOT EXISTS idx_ticks_hypertable_symbol_time
    ON ticks(symbol_id, tick_time DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_ticks_hypertable_name_time
    ON ticks(symbol_name, tick_time DESC, id DESC);

-- Set compression policy (compress chunks older than 7 days)
SELECT add_compression_policy('ticks', INTERVAL '7 days');

-- Add comment
COMMENT ON TABLE ticks IS 'TimescaleDB hypertable for market data ticks (1-day chunks, 7-day compression)';
