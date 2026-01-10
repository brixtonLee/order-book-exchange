-- Create retention policies for automatic data cleanup
-- This keeps the database size manageable by auto-deleting old data

-- Retention policy for raw ticks: Keep for 90 days
-- After 90 days, raw tick data is automatically deleted
-- OHLC aggregates will still be available for historical analysis
SELECT add_retention_policy('ticks', INTERVAL '90 days');

-- Retention policy for OHLC candles: Keep for 2 years
-- Long-term historical candle data for analysis
SELECT add_retention_policy('ohlc_candles', INTERVAL '2 years');

-- Add comments
COMMENT ON TABLE ticks IS 'TimescaleDB hypertable with 90-day retention policy';
COMMENT ON TABLE ohlc_candles IS 'TimescaleDB hypertable with 2-year retention policy';

-- Note: Retention policies run automatically on a schedule
-- You can manually trigger them with: CALL run_job(<job_id>);
-- View active jobs: SELECT * FROM timescaledb_information.jobs;
