use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Order book snapshot
    OrderBookSnapshot {
        symbol: String,
        timestamp: DateTime<Utc>,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
    },
    /// Incremental order book update
    OrderBookUpdate {
        symbol: String,
        timestamp: DateTime<Utc>,
        side: String, // "bid" or "ask"
        price: Decimal,
        quantity: Decimal, // 0 means level removed
    },
    /// Trade execution
    Trade {
        symbol: String,
        trade_id: String,
        price: Decimal,
        quantity: Decimal,
        side: String, // Taker side: "buy" or "sell"
        timestamp: DateTime<Utc>,
    },
    /// Ticker update (best bid/ask)
    Ticker {
        symbol: String,
        best_bid: Option<Decimal>,
        best_ask: Option<Decimal>,
        spread: Option<Decimal>,
        mid_price: Option<Decimal>,
        timestamp: DateTime<Utc>,
    },
    /// Subscription confirmation
    Subscribed {
        channel: String,
        symbol: Option<String>,
    },
    /// Unsubscription confirmation
    Unsubscribed {
        channel: String,
        symbol: Option<String>,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Heartbeat/Ping
    Ping {
        timestamp: DateTime<Utc>,
    },
    /// Pong response
    Pong {
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

/// Client subscription request
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        channel: String,
        symbol: Option<String>,
    },
    Unsubscribe {
        channel: String,
        symbol: Option<String>,
    },
    Ping,
}

/// Order book update for broadcasting
#[derive(Debug, Clone)]
pub struct OrderBookUpdate {
    pub symbol: String,
    pub side: String,
    pub price: Decimal,
    pub quantity: Decimal,
}

impl OrderBookUpdate {
    pub fn to_ws_message(&self) -> WsMessage {
        WsMessage::OrderBookUpdate {
            symbol: self.symbol.clone(),
            timestamp: Utc::now(),
            side: self.side.clone(),
            price: self.price,
            quantity: self.quantity,
        }
    }
}

/// Trade update for broadcasting
#[derive(Debug, Clone)]
pub struct TradeUpdate {
    pub symbol: String,
    pub trade_id: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: String,
}

impl TradeUpdate {
    pub fn to_ws_message(&self) -> WsMessage {
        WsMessage::Trade {
            symbol: self.symbol.clone(),
            trade_id: self.trade_id.clone(),
            price: self.price,
            quantity: self.quantity,
            side: self.side.clone(),
            timestamp: Utc::now(),
        }
    }
}

/// Ticker update for broadcasting
#[derive(Debug, Clone)]
pub struct TickerUpdate {
    pub symbol: String,
    pub best_bid: Option<Decimal>,
    pub best_ask: Option<Decimal>,
    pub spread: Option<Decimal>,
    pub mid_price: Option<Decimal>,
}

impl TickerUpdate {
    pub fn to_ws_message(&self) -> WsMessage {
        WsMessage::Ticker {
            symbol: self.symbol.clone(),
            best_bid: self.best_bid,
            best_ask: self.best_ask,
            spread: self.spread,
            mid_price: self.mid_price,
            timestamp: Utc::now(),
        }
    }
}
