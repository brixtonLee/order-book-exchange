-- Create TimescaleDB continuous aggregates for OHLC candles
-- These materialized views automatically aggregate tick data into OHLC candles
-- Refresh policies keep them up-to-date in near real-time

-- Helper function to calculate mid price for OHLC aggregation
-- Using (bid + ask) / 2 as the price for candles
CREATE OR REPLACE FUNCTION mid_price(bid_price NUMERIC, ask_price NUMERIC)
RETURNS NUMERIC AS $$
BEGIN
    RETURN (bid_price + ask_price) / 2;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- 1-Minute Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_1m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 minute'::interval, tick_time) AS open_time,
    time_bucket('1 minute'::interval, tick_time) + '1 minute'::interval AS close_time,
    symbol_id,
    symbol_name,
    '1m' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('1 minute'::interval, tick_time), symbol_id, symbol_name;

-- Refresh policy: Update every 1 minute, keep last 2 hours materialized
SELECT add_continuous_aggregate_policy('ohlc_1m',
    start_offset => INTERVAL '2 hours',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');

-- 5-Minute Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_5m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('5 minutes'::interval, tick_time) AS open_time,
    time_bucket('5 minutes'::interval, tick_time) + '5 minutes'::interval AS close_time,
    symbol_id,
    symbol_name,
    '5m' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('5 minutes'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_5m',
    start_offset => INTERVAL '10 hours',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

-- 15-Minute Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_15m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('15 minutes'::interval, tick_time) AS open_time,
    time_bucket('15 minutes'::interval, tick_time) + '15 minutes'::interval AS close_time,
    symbol_id,
    symbol_name,
    '15m' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('15 minutes'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_15m',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

-- 30-Minute Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_30m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('30 minutes'::interval, tick_time) AS open_time,
    time_bucket('30 minutes'::interval, tick_time) + '30 minutes'::interval AS close_time,
    symbol_id,
    symbol_name,
    '30m' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('30 minutes'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_30m',
    start_offset => INTERVAL '2 days',
    end_offset => INTERVAL '30 minutes',
    schedule_interval => INTERVAL '30 minutes');

-- 1-Hour Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_1h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour'::interval, tick_time) AS open_time,
    time_bucket('1 hour'::interval, tick_time) + '1 hour'::interval AS close_time,
    symbol_id,
    symbol_name,
    '1h' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('1 hour'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_1h',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- 4-Hour Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_4h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('4 hours'::interval, tick_time) AS open_time,
    time_bucket('4 hours'::interval, tick_time) + '4 hours'::interval AS close_time,
    symbol_id,
    symbol_name,
    '4h' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('4 hours'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_4h',
    start_offset => INTERVAL '30 days',
    end_offset => INTERVAL '4 hours',
    schedule_interval => INTERVAL '4 hours');

-- 1-Day Candles Continuous Aggregate
CREATE MATERIALIZED VIEW ohlc_1d
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day'::interval, tick_time) AS open_time,
    time_bucket('1 day'::interval, tick_time) + '1 day'::interval AS close_time,
    symbol_id,
    symbol_name,
    '1d' AS timeframe,
    FIRST(mid_price(bid_price, ask_price), tick_time) AS open_price,
    MAX(mid_price(bid_price, ask_price)) AS high_price,
    MIN(mid_price(bid_price, ask_price)) AS low_price,
    LAST(mid_price(bid_price, ask_price), tick_time) AS close_price,
    SUM((bid_volume + ask_volume) / 2) AS volume,
    COUNT(*) AS tick_count
FROM ticks
GROUP BY time_bucket('1 day'::interval, tick_time), symbol_id, symbol_name;

SELECT add_continuous_aggregate_policy('ohlc_1d',
    start_offset => INTERVAL '90 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

-- Create indexes on continuous aggregates for faster queries
CREATE INDEX idx_ohlc_1m_symbol_time ON ohlc_1m(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_5m_symbol_time ON ohlc_5m(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_15m_symbol_time ON ohlc_15m(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_30m_symbol_time ON ohlc_30m(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_1h_symbol_time ON ohlc_1h(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_4h_symbol_time ON ohlc_4h(symbol_id, open_time DESC);
CREATE INDEX idx_ohlc_1d_symbol_time ON ohlc_1d(symbol_id, open_time DESC);

-- Add comments
COMMENT ON MATERIALIZED VIEW ohlc_1m IS 'TimescaleDB continuous aggregate: 1-minute OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_5m IS 'TimescaleDB continuous aggregate: 5-minute OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_15m IS 'TimescaleDB continuous aggregate: 15-minute OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_30m IS 'TimescaleDB continuous aggregate: 30-minute OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_1h IS 'TimescaleDB continuous aggregate: 1-hour OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_4h IS 'TimescaleDB continuous aggregate: 4-hour OHLC candles';
COMMENT ON MATERIALIZED VIEW ohlc_1d IS 'TimescaleDB continuous aggregate: 1-day OHLC candles';
