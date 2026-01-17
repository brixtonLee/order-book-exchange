-- Drop symbols table and all associated indexes
DROP INDEX IF EXISTS idx_symbols_last_synced;
DROP INDEX IF EXISTS idx_symbols_name;
DROP TABLE IF EXISTS symbols;
