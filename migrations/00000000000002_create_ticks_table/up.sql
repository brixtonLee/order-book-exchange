-- Create ticks table in TimescaleDB (timeseries database)
-- This table stores market data ticks from FIX feed
-- Will be converted to hypertable in next migration

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

    -- Composite unique constraint: symbol_id + symbol_name + tick_time
    CONSTRAINT ticks_unique_tick UNIQUE (symbol_id, symbol_name, tick_time)
);

-- Create indexes (will be optimized after converting to hypertable)
CREATE INDEX idx_ticks_symbol_id_time ON ticks(symbol_id, tick_time DESC);
CREATE INDEX idx_ticks_symbol_name_time ON ticks(symbol_name, tick_time DESC);
CREATE INDEX idx_ticks_time ON ticks(tick_time DESC);

-- Add comments to table
COMMENT ON TABLE ticks IS 'Market data ticks from cTrader FIX feed (TimescaleDB hypertable)';

-- Add comments to columns
COMMENT ON COLUMN ticks.symbol_id IS 'Foreign key to symbols.symbol_id';
COMMENT ON COLUMN ticks.symbol_name IS 'Denormalized symbol name for easier querying';
COMMENT ON COLUMN ticks.tick_time IS 'Timestamp of the tick (hypertable partition key)';
COMMENT ON COLUMN ticks.bid_price IS 'Best bid price';
COMMENT ON COLUMN ticks.ask_price IS 'Best ask/offer price';
COMMENT ON COLUMN ticks.bid_volume IS 'Volume available at bid price';
COMMENT ON COLUMN ticks.ask_volume IS 'Volume available at ask price';
