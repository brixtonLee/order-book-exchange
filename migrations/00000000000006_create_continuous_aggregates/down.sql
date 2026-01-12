-- ⚠️  PLACEHOLDER for continuous aggregates rollback
--
-- Since continuous aggregates are created manually outside Diesel migrations,
-- they must also be dropped manually if needed.
--
-- To rollback continuous aggregates, run:
--   psql postgresql://orderbook:orderbook123@localhost:5433/orderbook_timeseries
--
-- Then execute:
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_1d CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_4h CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_1h CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_30m CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_15m CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_5m CASCADE;
--   DROP MATERIALIZED VIEW IF EXISTS ohlc_1m CASCADE;
--   DROP FUNCTION IF EXISTS mid_price(NUMERIC, NUMERIC);

SELECT 1 AS continuous_aggregates_rollback_placeholder;
