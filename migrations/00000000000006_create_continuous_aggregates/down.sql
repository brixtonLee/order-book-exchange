-- Drop continuous aggregate policies
SELECT remove_continuous_aggregate_policy('ohlc_1m', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_5m', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_15m', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_30m', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_1h', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_4h', if_exists => true);
SELECT remove_continuous_aggregate_policy('ohlc_1d', if_exists => true);

-- Drop indexes on continuous aggregates
DROP INDEX IF EXISTS idx_ohlc_1d_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_4h_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_1h_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_30m_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_15m_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_5m_symbol_time;
DROP INDEX IF EXISTS idx_ohlc_1m_symbol_time;

-- Drop continuous aggregate materialized views
DROP MATERIALIZED VIEW IF EXISTS ohlc_1d CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_4h CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_1h CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_30m CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_15m CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_5m CASCADE;
DROP MATERIALIZED VIEW IF EXISTS ohlc_1m CASCADE;

-- Drop helper function
DROP FUNCTION IF EXISTS mid_price(NUMERIC, NUMERIC);
