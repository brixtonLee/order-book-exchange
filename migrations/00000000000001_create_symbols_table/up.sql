-- Create symbols table in PostgreSQL (metadata database)
-- This table stores trading instrument information synced from cTrader FIX

CREATE TABLE symbols (
    symbol_id BIGINT PRIMARY KEY,
    symbol_name VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    digits INTEGER NOT NULL DEFAULT 5,
    tick_size NUMERIC(20, 8) NOT NULL,
    contract_size NUMERIC(20, 8),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_synced_at TIMESTAMPTZ
);

-- Create index on symbol_name for faster lookups
CREATE INDEX idx_symbols_name ON symbols(symbol_name);

-- Create index on last_synced_at for monitoring stale symbols
CREATE INDEX idx_symbols_last_synced ON symbols(last_synced_at);

-- Add comment to table
COMMENT ON TABLE symbols IS 'Trading symbols/instruments metadata synced from cTrader FIX';

-- Add comments to columns
COMMENT ON COLUMN symbols.symbol_id IS 'Unique symbol ID from cTrader FIX protocol';
COMMENT ON COLUMN symbols.symbol_name IS 'Human-readable symbol name (e.g., EURUSD, XAUUSD)';
COMMENT ON COLUMN symbols.digits IS 'Number of decimal places for price precision';
COMMENT ON COLUMN symbols.tick_size IS 'Minimum price increment';
COMMENT ON COLUMN symbols.contract_size IS 'Contract size for CFDs/futures';
COMMENT ON COLUMN symbols.last_synced_at IS 'Last successful sync timestamp from FIX feed';
