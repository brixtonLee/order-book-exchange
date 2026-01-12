// @generated automatically by Diesel CLI.

diesel::table! {
    ohlc_candles (symbol_id, timeframe, open_time) {
        id -> Int8,
        symbol_id -> Int8,
        #[max_length = 50]
        symbol_name -> Varchar,
        #[max_length = 10]
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

diesel::table! {
    symbols (symbol_id) {
        symbol_id -> Int8,
        #[max_length = 50]
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
    ticks (symbol_id, symbol_name, tick_time) {
        id -> Int8,
        symbol_id -> Int8,
        #[max_length = 50]
        symbol_name -> Varchar,
        tick_time -> Timestamptz,
        bid_price -> Numeric,
        ask_price -> Numeric,
        bid_volume -> Numeric,
        ask_volume -> Numeric,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    ohlc_candles,
    symbols,
    ticks,
);
