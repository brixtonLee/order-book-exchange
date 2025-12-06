use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::select;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use super::{
    broadcaster::{topics, Broadcaster},
    messages::{ClientMessage, WsMessage},
};
use crate::engine::OrderBookEngine;

/// WebSocket connection state
pub struct WsState {
    pub broadcaster: Broadcaster,
    pub engine: Arc<OrderBookEngine>,
}

/// Handle WebSocket upgrade request
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscriptions for this client
    let mut subscriptions: Vec<(String, tokio::sync::broadcast::Receiver<WsMessage>)> = Vec::new();

    // Heartbeat interval
    let mut heartbeat = interval(Duration::from_secs(30));

    info!("WebSocket client connected");

    loop {
        select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_client_message(
                            &text,
                            &mut subscriptions,
                            &mut sender,
                            &state,
                        ).await {
                            error!("Error handling client message: {}", e);
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }

            // Handle broadcast messages from subscribed channels
            _ = async {
                for (_, rx) in &mut subscriptions {
                    if let Ok(ws_msg) = rx.try_recv() {
                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                return Err(());
                            }
                        }
                    }
                }
                Ok::<(), ()>(())
            } => {}

            // Send heartbeat
            _ = heartbeat.tick() => {
                let heartbeat_msg = WsMessage::Ping {
                    timestamp: chrono::Utc::now(),
                };
                if let Ok(json) = serde_json::to_string(&heartbeat_msg) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket connection closed");
}

/// Handle client messages (subscribe/unsubscribe)
async fn handle_client_message(
    text: &str,
    subscriptions: &mut Vec<(String, tokio::sync::broadcast::Receiver<WsMessage>)>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    state: &Arc<WsState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client_msg: ClientMessage = serde_json::from_str(text)?;

    match client_msg {
        ClientMessage::Subscribe { channel, symbol } => {
            let topic = build_topic(&channel, symbol.as_deref())?;
            let rx = state.broadcaster.subscribe(&topic);

            subscriptions.push((topic.clone(), rx));

            // Send subscription confirmation
            let response = WsMessage::Subscribed {
                channel: channel.clone(),
                symbol: symbol.clone(),
            };

            // Send snapshot if it's an orderbook subscription
            if channel == "orderbook" {
                if let Some(sym) = &symbol {
                    send_orderbook_snapshot(sym, sender, &state.engine).await?;
                }
            }

            let json = serde_json::to_string(&response)?;
            sender.send(Message::Text(json)).await?;

            info!("Client subscribed to: {}", topic);
        }
        ClientMessage::Unsubscribe { channel, symbol } => {
            let topic = build_topic(&channel, symbol.as_deref())?;

            subscriptions.retain(|(t, _)| t != &topic);

            let response = WsMessage::Unsubscribed {
                channel: channel.clone(),
                symbol: symbol.clone(),
            };
            let json = serde_json::to_string(&response)?;
            sender.send(Message::Text(json)).await?;

            info!("Client unsubscribed from: {}", topic);
        }
        ClientMessage::Ping => {
            let response = WsMessage::Pong {
                timestamp: chrono::Utc::now(),
            };
            let json = serde_json::to_string(&response)?;
            sender.send(Message::Text(json)).await?;
        }
    }

    Ok(())
}

/// Build topic string from channel and symbol
fn build_topic(channel: &str, symbol: Option<&str>) -> Result<String, String> {
    match channel {
        "orderbook" => symbol
            .map(topics::orderbook)
            .ok_or_else(|| "orderbook channel requires symbol".to_string()),
        "trades" => {
            if let Some(sym) = symbol {
                Ok(topics::trades(sym))
            } else {
                Ok(topics::all_trades().to_string())
            }
        }
        "ticker" => symbol
            .map(topics::ticker)
            .ok_or_else(|| "ticker channel requires symbol".to_string()),
        _ => Err(format!("Unknown channel: {}", channel)),
    }
}

/// Send order book snapshot to client
async fn send_orderbook_snapshot(
    symbol: &str,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    engine: &OrderBookEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let book = engine.get_order_book(symbol);

    // Build snapshot
    let bids: Vec<super::messages::PriceLevel> = book
        .bids
        .iter()
        .rev()
        .take(20)
        .map(|(_, level)| super::messages::PriceLevel {
            price: level.price,
            quantity: level.total_quantity,
        })
        .collect();

    let asks: Vec<super::messages::PriceLevel> = book
        .asks
        .iter()
        .take(20)
        .map(|(_, level)| super::messages::PriceLevel {
            price: level.price,
            quantity: level.total_quantity,
        })
        .collect();

    let snapshot = WsMessage::OrderBookSnapshot {
        symbol: symbol.to_string(),
        timestamp: chrono::Utc::now(),
        bids,
        asks,
    };

    let json = serde_json::to_string(&snapshot)?;
    sender.send(Message::Text(json)).await?;

    Ok(())
}
