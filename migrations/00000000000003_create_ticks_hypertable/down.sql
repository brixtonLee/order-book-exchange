-- Remove compression policy
SELECT remove_compression_policy('ticks', if_exists => true);

-- Drop TimescaleDB-specific indexes
DROP INDEX IF EXISTS idx_ticks_hypertable_name_time;
DROP INDEX IF EXISTS idx_ticks_hypertable_symbol_time;

-- Note: Converting back from hypertable to regular table is complex
-- For simplicity, we'll just drop and recreate as regular table in down migration
-- In production, you might want to preserve data using pg_dump before rollback

-- This is a destructive operation - use with caution
-- If you need to preserve data, manually export before running down migration
