// @generated automatically by Diesel CLI.
// This file will be auto-generated after running diesel migrations
// Run: diesel migration run --database-url=$DATABASE_URL
// Run: diesel migration run --database-url=$TIMESCALEDB_URL

// Temporary schema definitions - will be replaced by `diesel print-schema`
diesel::table! {
    symbols (symbol_id) {
        symbol_id -> Int8,
        symbol_name -> Varchar,
        description -> Nullable<Text>,
        digits -> Int4,
        tick_size -> Numeric,
        contract_size -> Nullable<Numeric>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_synced_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ticks (id) {
        id -> Int8,
        symbol_id -> Int8,
        symbol_name -> Varchar,
        tick_time -> Timestamptz,
        bid_price -> Numeric,
        ask_price -> Numeric,
        bid_volume -> Numeric,
        ask_volume -> Numeric,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ohlc_candles (id) {
        id -> Int8,
        symbol_id -> Int8,
        symbol_name -> Varchar,
        timeframe -> Varchar,
        open_time -> Timestamptz,
        close_time -> Timestamptz,
        open_price -> Numeric,
        high_price -> Numeric,
        low_price -> Numeric,
        close_price -> Numeric,
        volume -> Numeric,
        tick_count -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(symbols, ticks, ohlc_candles,);
