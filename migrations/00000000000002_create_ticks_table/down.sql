-- Drop ticks table and all associated indexes
DROP INDEX IF EXISTS idx_ticks_time;
DROP INDEX IF EXISTS idx_ticks_symbol_name_time;
DROP INDEX IF EXISTS idx_ticks_symbol_id_time;
DROP TABLE IF EXISTS ticks;
