# Datasource API Documentation

## Overview

The Datasource API provides runtime control over FIX protocol connections to cTrader for live market data streaming. You can start, stop, and monitor FIX connections through REST API endpoints without restarting the server.

## Features

- **Runtime FIX Connection Control**: Start/stop FIX connections via API
- **Automatic Symbol Discovery**: Fetches available symbols from Security List Response
- **Heartbeat Monitoring**: Tracks FIX protocol heartbeats for connection health
- **Health Status**: Real-time health monitoring with degradation detection
- **Symbol Subscription**: Automatically subscribes to all available symbols
- **WebSocket Integration**: Live market data streamed to WebSocket clients

## API Endpoints

### 1. Start FIX Connection

**POST** `/api/v1/datasource/start`

Start a new FIX connection to cTrader.

**Request Body:**
```json
{
  "host": "live-uk-eqx-01.p.c-trader.com",
  "port": 5201,
  "credentials": {
    "sender_comp_id": "live.fxpro.8244184",
    "target_comp_id": "cServer",
    "sender_sub_id": "QUOTE",
    "target_sub_id": "QUOTE",
    "username": "8244184",
    "password": "your_password_here"
  }
}
```

**Response (200 OK):**
```json
{
  "status": "connecting",
  "message": "FIX connection initiated. Fetching symbol list..."
}
```

**Error Responses:**
- `400 Bad Request`: Already connected or invalid credentials
- `500 Internal Server Error`: Connection failure

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/datasource/start \
  -H "Content-Type: application/json" \
  -d '{
    "host": "live-uk-eqx-01.p.c-trader.com",
    "port": 5201,
    "credentials": {
      "sender_comp_id": "live.fxpro.8244184",
      "target_comp_id": "cServer",
      "sender_sub_id": "QUOTE",
      "target_sub_id": "QUOTE",
      "username": "8244184",
      "password": "fixapibrixton"
    }
  }'
```

---

### 2. Stop FIX Connection

**POST** `/api/v1/datasource/stop`

Stop the active FIX connection.

**Response (200 OK):**
```json
{
  "status": "stopped",
  "message": "FIX connection stopped successfully"
}
```

**Error Responses:**
- `400 Bad Request`: No active connection to stop

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/datasource/stop
```

---

### 3. Get Datasource Status

**GET** `/api/v1/datasource/status`

Get detailed status of the FIX connection, including subscribed symbols and heartbeat metrics.

**Response (200 OK) - Disconnected:**
```json
{
  "mode": "disconnected",
  "connected": false,
  "heartbeat_count": 0,
  "symbols_subscribed": [],
  "total_symbols": 0
}
```

**Response (200 OK) - Connected:**
```json
{
  "mode": "connected",
  "connected": true,
  "uptime_seconds": 3456,
  "heartbeat_count": 173,
  "last_heartbeat_seconds_ago": 5,
  "symbols_subscribed": [
    {
      "symbol_id": "1",
      "symbol_name": "EURUSD",
      "symbol_digits": 5
    },
    {
      "symbol_id": "41",
      "symbol_name": "XAUUSD",
      "symbol_digits": 2
    },
    {
      "symbol_id": "2",
      "symbol_name": "GBPUSD",
      "symbol_digits": 5
    }
  ],
  "total_symbols": 78,
  "fix_server": "live-uk-eqx-01.p.c-trader.com:5201",
  "connection_info": {
    "sender_comp_id": "live.fxpro.8244184",
    "target_comp_id": "cServer"
  }
}
```

**Example:**
```bash
curl http://localhost:3000/api/v1/datasource/status | jq '.'
```

---

### 4. Get Health Status

**GET** `/api/v1/health`

Get system health status based on FIX connection and heartbeat activity.

**Response (200 OK) - Healthy:**
```json
{
  "status": "healthy",
  "fix_connection": "connected",
  "heartbeat_status": "active",
  "last_heartbeat_seconds_ago": 5,
  "uptime_seconds": 3456,
  "symbols_count": 78,
  "timestamp": "2025-12-29T10:30:45Z"
}
```

**Response (200 OK) - Degraded:**
```json
{
  "status": "degraded",
  "fix_connection": "connected",
  "heartbeat_status": "stale",
  "last_heartbeat_seconds_ago": 45,
  "uptime_seconds": 3456,
  "symbols_count": 78,
  "warning": "No heartbeat received in 45 seconds",
  "timestamp": "2025-12-29T10:30:45Z"
}
```

**Response (200 OK) - Unhealthy:**
```json
{
  "status": "unhealthy",
  "fix_connection": "disconnected",
  "heartbeat_status": "none",
  "timestamp": "2025-12-29T10:30:45Z"
}
```

**Health Status Logic:**
- **Healthy**: FIX connected + heartbeat within last 30 seconds
- **Degraded**: FIX connected but no heartbeat for 30-60 seconds
- **Unhealthy**: FIX disconnected OR no heartbeat for >60 seconds

**Example:**
```bash
curl http://localhost:3000/api/v1/health | jq '.'
```

---

## Connection Lifecycle

### 1. Server Start (No Connection)
```bash
cargo run
```
Server starts with:
- Mode: `disconnected`
- Heartbeat count: `0`
- Symbols: `[]`

### 2. Start FIX Connection
```bash
curl -X POST http://localhost:3000/api/v1/datasource/start \
  -H "Content-Type: application/json" \
  -d '{ "host": "...", "port": 5201, "credentials": {...} }'
```

Behind the scenes:
1. FIX client connects to server
2. Sends Logon message (MsgType=A)
3. Receives Logon response
4. Sends Security List Request (MsgType=x)
5. Receives Security List Response (MsgType=y) with all symbols
6. Sends Market Data Request (MsgType=V) for all symbols
7. Starts receiving market ticks (MsgType=W/X)
8. Heartbeats exchanged every 30 seconds (MsgType=0)

### 3. Monitor Status
```bash
curl http://localhost:3000/api/v1/datasource/status
```
Returns:
- Connection uptime
- Heartbeat count
- Last heartbeat timestamp
- Full list of subscribed symbols (with ID, name, precision)

### 4. Monitor Health
```bash
curl http://localhost:3000/api/v1/health
```
Returns:
- Overall health status
- Connection state
- Heartbeat freshness
- Warning messages (if any)

### 5. Stop Connection
```bash
curl -X POST http://localhost:3000/api/v1/datasource/stop
```
- Aborts FIX client task
- Aborts bridge task
- Clears connection state
- Resets heartbeat counter

---

## Integration with WebSocket

Once FIX connection is active, market data flows to WebSocket clients:

### Subscribe to Market Data
```bash
# Connect to WebSocket
wscat -c ws://localhost:3000/ws

# Subscribe to specific symbol
{"action": "subscribe", "channel": "ticker", "symbol": "XAUUSD"}

# Subscribe to all symbols
{"action": "subscribe", "channel": "ticker", "symbol": "*"}
```

### Received Market Data
```json
{
  "type": "ticker",
  "symbol": "XAUUSD",
  "bid": "2650.50",
  "ask": "2651.00",
  "timestamp": "2025-12-29T10:30:45Z"
}
```

---

## Testing

### Quick Test Script
```bash
./test_datasource_api.sh
```

This script:
1. Tests health (before connection)
2. Tests status (before connection)
3. Starts FIX connection
4. Waits for connection to establish
5. Tests health (after connection - should be healthy)
6. Tests status (should show symbols + heartbeats)
7. Waits to accumulate heartbeats
8. Tests status again (heartbeat count should increase)
9. Stops connection
10. Tests health (should be unhealthy)

### Manual Testing Workflow
```bash
# Start server
cargo run

# Check initial status (disconnected)
curl http://localhost:3000/api/v1/datasource/status

# Start FIX connection
curl -X POST http://localhost:3000/api/v1/datasource/start \
  -H "Content-Type: application/json" \
  -d @fix_credentials.json

# Wait 5 seconds...

# Check status (should show connected + symbols)
curl http://localhost:3000/api/v1/datasource/status | jq '.total_symbols'

# Check health
curl http://localhost:3000/api/v1/health | jq '.status'

# Stop connection
curl -X POST http://localhost:3000/api/v1/datasource/stop
```

---

## Security Considerations

### Credential Management
- ⚠️ **Never commit credentials to git**
- Store credentials in environment variables or secure vault
- Use HTTPS in production to encrypt API requests
- Password field is never returned in status responses

### Recommended Practices
```bash
# Store credentials in a file (add to .gitignore!)
cat > fix_credentials.json <<EOF
{
  "host": "live-uk-eqx-01.p.c-trader.com",
  "port": 5201,
  "credentials": {
    "sender_comp_id": "live.fxpro.8244184",
    "target_comp_id": "cServer",
    "sender_sub_id": "QUOTE",
    "target_sub_id": "QUOTE",
    "username": "8244184",
    "password": "$FIX_PASSWORD"
  }
}
EOF

# Use environment variable for password
export FIX_PASSWORD="your_password_here"
envsubst < fix_credentials.json | curl -X POST http://localhost:3000/api/v1/datasource/start \
  -H "Content-Type: application/json" \
  -d @-
```

### Authentication (Future Enhancement)
Consider adding:
- API key authentication for datasource endpoints
- Role-based access control (RBAC)
- Rate limiting
- Audit logging

---

## Architecture

### Components

**DatasourceManager**
- Manages FIX connection lifecycle
- Tracks heartbeats and symbols
- Provides status/health information

**CTraderFixClient**
- Handles FIX protocol communication
- Callbacks for heartbeats and security list
- Streams market ticks to bridge

**FixToWebSocketBridge**
- Converts FIX ticks to WebSocket messages
- Broadcasts to all subscribed clients

**Broadcaster**
- WebSocket fanout infrastructure
- Per-channel subscriptions

### Data Flow
```
cTrader FIX Server
        ↓
CTraderFixClient (callbacks)
        ↓
DatasourceManager (state tracking)
        ↓
FixToWebSocketBridge
        ↓
Broadcaster
        ↓
WebSocket Clients
```

---

## Troubleshooting

### Connection Fails
- Check credentials are correct
- Verify cTrader server host/port
- Check firewall rules
- Review server logs for FIX errors

### No Heartbeats Received
- Check `last_heartbeat_seconds_ago` in status
- If >60 seconds, connection may be stale
- Try stopping and restarting connection
- Check network connectivity

### No Symbols Received
- Check `total_symbols` in status
- If 0, Security List Response may have failed
- Review logs for parsing errors
- Verify account has access to symbols

### Health Status Degraded
- Check `last_heartbeat_seconds_ago`
- Wait 30 seconds for next heartbeat
- If persists, stop and restart connection

---

## Future Enhancements

- [ ] Automatic reconnection on disconnect
- [ ] Exponential backoff for retries
- [ ] Symbol filtering (subscribe to specific symbols)
- [ ] Multiple FIX connections (different accounts)
- [ ] Metrics (tick rate, latency, errors)
- [ ] WebSocket events for datasource state changes
- [ ] Credential encryption at rest
- [ ] Support for other FIX protocol versions

---

## API Reference

All endpoints are documented in Swagger UI:
```
http://localhost:3000/swagger-ui
```

Select "v2.0" from the dropdown to see datasource endpoints.
