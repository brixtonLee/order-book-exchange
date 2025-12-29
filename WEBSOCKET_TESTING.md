# WebSocket Testing Guide

## Overview
This guide shows how to test the FIX-to-WebSocket bridge using Postman.

## Architecture

```
FIX Market Data (cTrader)
          ↓
   CTraderFixClient
          ↓
   MarketTick Channel
          ↓
  FixToWebSocketBridge
          ↓
      Broadcaster
          ↓
   WebSocket Clients
```

## Running the Server

### Option 1: Mock Server (For Testing)
```bash
cargo run --bin ws_mock_server
```
This generates fake market data for XAUUSD and EURUSD every second.

### Option 2: Real FIX Server
```bash
cargo run --bin ctrader_ws_server
```
Connects to real cTrader FIX API (requires valid credentials).

## Server Endpoints

- **WebSocket**: `ws://localhost:3000/ws`
- **HTTP API**: `http://localhost:3000`
- **Swagger UI**: `http://localhost:3000/swagger-ui`
- **Health Check**: `http://localhost:3000/health`

## Testing with Postman

### Step 1: Create WebSocket Request
1. Open Postman
2. Click "New" → "WebSocket Request"
3. Enter URL: `ws://localhost:3000/ws`
4. Click "Connect"

### Step 2: Subscribe to Ticker Updates

#### Subscribe to XAUUSD (Gold)
```json
{
  "action": "subscribe",
  "channel": "ticker",
  "symbol": "XAUUSD"
}
```

#### Subscribe to EURUSD
```json
{
  "action": "subscribe",
  "channel": "ticker",
  "symbol": "EURUSD"
}
```

#### Subscribe to All Tickers (Wildcard)
```json
{
  "action": "subscribe",
  "channel": "ticker",
  "symbol": "*"
}
```

### Step 3: Watch Real-Time Updates

You should receive messages like:
```json
{
  "type": "subscribed",
  "channel": "ticker",
  "symbol": "XAUUSD"
}
```

Followed by continuous ticker updates:
```json
{
  "type": "ticker",
  "symbol": "XAUUSD",
  "best_bid": "2650.50",
  "best_ask": "2651.00",
  "spread": "0.50",
  "mid_price": "2650.75",
  "timestamp": "2025-12-22T14:24:30.123Z"
}
```

### Step 4: Unsubscribe (Optional)
```json
{
  "action": "unsubscribe",
  "channel": "ticker",
  "symbol": "XAUUSD"
}
```

### Step 5: Ping/Pong (Heartbeat)
```json
{
  "action": "ping"
}
```

Response:
```json
{
  "type": "pong",
  "timestamp": "2025-12-22T14:24:30.123Z"
}
```

## Message Types

### Client → Server (Requests)

| Action | Description |
|--------|-------------|
| `subscribe` | Subscribe to a channel |
| `unsubscribe` | Unsubscribe from a channel |
| `ping` | Heartbeat check |

### Server → Client (Responses)

| Type | Description |
|------|-------------|
| `subscribed` | Subscription confirmation |
| `unsubscribed` | Unsubscription confirmation |
| `ticker` | Real-time price update |
| `ping` | Server heartbeat (every 30s) |
| `pong` | Response to client ping |
| `error` | Error message |

## Available Channels

| Channel | Symbol Required | Description |
|---------|----------------|-------------|
| `ticker` | Yes | Best bid/ask prices |
| `orderbook` | Yes | Full order book snapshot |
| `trades` | Optional | Trade executions |

## Available Symbols

The mock server generates data for:
- `XAUUSD` (Gold/USD) - Symbol ID: 41
- `EURUSD` (Euro/USD) - Symbol ID: 1
- `GBPUSD` (British Pound/USD) - Symbol ID: 2
- `USDJPY` (USD/Japanese Yen) - Symbol ID: 3

## Data Mapping

The bridge converts FIX `MarketTick` to WebSocket `Ticker` messages:

| MarketTick Field | WsMessage::Ticker Field |
|-----------------|------------------------|
| `symbol_id` | `symbol` (after name lookup) |
| `bid_price` | `best_bid` |
| `ask_price` | `best_ask` |
| Calculated | `spread` (ask - bid) |
| Calculated | `mid_price` ((bid + ask) / 2) |
| `timestamp` | `timestamp` |

## Troubleshooting

### Connection Refused
- Ensure the server is running: `cargo run --bin ws_mock_server`
- Check the port is not in use: `lsof -i :3000`

### No Messages Received
- Verify you sent a subscribe message
- Check the symbol name matches exactly (case-sensitive)
- Look at server logs for broadcast confirmations

### Connection Drops
- Server sends ping every 30 seconds
- Postman should auto-respond with pong
- Check network stability

## Testing with Other Tools

### wscat (CLI)
```bash
npm install -g wscat
wscat -c ws://localhost:3000/ws

# Then send:
{"action":"subscribe","channel":"ticker","symbol":"XAUUSD"}
```

### curl (HTTP endpoints)
```bash
# Health check
curl http://localhost:3000/health

# Get order book via REST
curl http://localhost:3000/api/v1/orderbook/AAPL
```

### Browser JavaScript
```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  console.log('Connected');
  ws.send(JSON.stringify({
    action: 'subscribe',
    channel: 'ticker',
    symbol: 'XAUUSD'
  }));
};

ws.onmessage = (event) => {
  console.log('Received:', JSON.parse(event.data));
};
```

## Performance Notes

- Mock server generates ticks every 1 second
- Real FIX server can stream hundreds of ticks per second
- Each WebSocket client maintains its own subscription list
- Broadcaster uses DashMap for concurrent access
- No message queuing - real-time streaming only

## Next Steps

1. Add more symbols to the bridge mapping
2. Implement order book snapshots (full depth)
3. Add trade execution streaming
4. Implement message rate limiting
5. Add authentication/authorization
6. Add metrics and monitoring
