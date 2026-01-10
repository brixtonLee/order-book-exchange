-- Create OHLC candles table in TimescaleDB (timeseries database)
-- This table stores aggregated OHLC candles at various timeframes
-- Will be populated by TimescaleDB continuous aggregates

CREATE TABLE ohlc_candles (
    id BIGSERIAL,
    symbol_id BIGINT NOT NULL,
    symbol_name VARCHAR(50) NOT NULL,
    timeframe VARCHAR(10) NOT NULL,  -- '1m', '5m', '15m', '30m', '1h', '4h', '1d'
    open_time TIMESTAMPTZ NOT NULL,
    close_time TIMESTAMPTZ NOT NULL,
    open_price NUMERIC(20, 8) NOT NULL,
    high_price NUMERIC(20, 8) NOT NULL,
    low_price NUMERIC(20, 8) NOT NULL,
    close_price NUMERIC(20, 8) NOT NULL,
    volume NUMERIC(20, 8) NOT NULL DEFAULT 0,
    tick_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite unique constraint: symbol_id + timeframe + open_time
    CONSTRAINT ohlc_candles_unique_candle UNIQUE (symbol_id, timeframe, open_time),

    -- Check constraint for valid timeframes
    CONSTRAINT ohlc_candles_timeframe_check
        CHECK (timeframe IN ('1m', '5m', '15m', '30m', '1h', '4h', '1d'))
);

-- Create indexes (will be optimized after converting to hypertable)
CREATE INDEX idx_ohlc_symbol_timeframe_time ON ohlc_candles(symbol_id, timeframe, open_time DESC);
CREATE INDEX idx_ohlc_symbol_name_timeframe_time ON ohlc_candles(symbol_name, timeframe, open_time DESC);
CREATE INDEX idx_ohlc_time ON ohlc_candles(open_time DESC);

-- Add comments to table
COMMENT ON TABLE ohlc_candles IS 'OHLC candles at multiple timeframes (populated by TimescaleDB continuous aggregates)';

-- Add comments to columns
COMMENT ON COLUMN ohlc_candles.symbol_id IS 'Foreign key to symbols.symbol_id';
COMMENT ON COLUMN ohlc_candles.timeframe IS 'Candle timeframe: 1m, 5m, 15m, 30m, 1h, 4h, 1d';
COMMENT ON COLUMN ohlc_candles.open_time IS 'Candle open time (hypertable partition key)';
COMMENT ON COLUMN ohlc_candles.close_time IS 'Candle close time';
COMMENT ON COLUMN ohlc_candles.tick_count IS 'Number of ticks aggregated in this candle';
