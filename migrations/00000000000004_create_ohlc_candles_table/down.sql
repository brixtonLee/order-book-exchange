-- Drop OHLC candles table and all associated indexes
DROP INDEX IF EXISTS idx_ohlc_time;
DROP INDEX IF EXISTS idx_ohlc_symbol_name_timeframe_time;
DROP INDEX IF EXISTS idx_ohlc_symbol_timeframe_time;
DROP TABLE IF EXISTS ohlc_candles;
