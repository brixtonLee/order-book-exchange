-- Remove retention policies
SELECT remove_retention_policy('ticks', if_exists => true);
SELECT remove_retention_policy('ohlc_candles', if_exists => true);

-- Note: Data that was already deleted by retention policy cannot be recovered
