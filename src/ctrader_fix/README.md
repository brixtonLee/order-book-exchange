# cTrader FIX API Client

A minimal FIX 4.4 protocol implementation for connecting to cTrader's market data feed.

## 🚀 Quick Start

### Run the Test Client

```bash
cargo run --bin ctrader_fix_test
```

When prompted, enter your cTrader account password (account 8244184).

## 📁 Project Structure

```
src/ctrader_fix/
├── mod.rs          # Module exports
├── messages.rs     # FIX message builder and parser
├── client.rs       # TCP connection and FIX session management
└── README.md       # This file

src/bin/
└── ctrader_fix_test.rs  # Standalone test binary
```

## 🔧 Your Configuration

```
Host: live-uk-eqx-01.p.c-trader.com
Port: 5201 (Plain text)
SenderCompID: live.fxpro.8244184
TargetCompID: cServer
SenderSubID: QUOTE
Username: 8244184
Password: [Your account password]
```

## 📚 How FIX Protocol Works

### 1. **Connection Flow**

```
Client                          cTrader Server
  |                                    |
  |-- TCP Connect -------------------->|
  |                                    |
  |-- Logon (35=A) ------------------->|
  |<-- Logon Response (35=A) ----------|
  |                                    |
  |-- Market Data Request (35=V) ----->|
  |<-- Market Data Snapshot (35=W) ----|
  |<-- Market Data Updates (35=X) -----|
  |                                    |
  |<-- Heartbeat (35=0) ---------------|
  |-- Heartbeat (35=0) --------------->|
```

### 2. **FIX Message Format**

Every FIX message follows this structure:

```
8=FIX.4.4|9=123|35=A|49=SENDER|56=TARGET|...|10=123|
│         │      │     │         │             │
│         │      │     │         │             └─ Checksum (tag 10)
│         │      │     │         └─ TargetCompID (tag 56)
│         │      │     └─ SenderCompID (tag 49)
│         │      └─ MsgType (tag 35)
│         └─ BodyLength (tag 9)
└─ BeginString (tag 8)
```

**Note**: `|` represents SOH (Start of Header, ASCII 0x01)

### 3. **Key Message Types**

| MsgType | Name | Direction | Purpose |
|---------|------|-----------|---------|
| A | Logon | Both | Authenticate and establish session |
| 0 | Heartbeat | Both | Keep connection alive |
| 1 | Test Request | Both | Check connection health |
| 5 | Logout | Both | Gracefully close session |
| V | Market Data Request | Client → Server | Subscribe to market data |
| W | Market Data Snapshot | Server → Client | Full order book snapshot |
| X | Market Data Incremental | Server → Client | Price updates |
| Y | Market Data Reject | Server → Client | Request rejected |

### 4. **Important FIX Tags**

| Tag | Name | Description |
|-----|------|-------------|
| 8 | BeginString | FIX version (FIX.4.4) |
| 9 | BodyLength | Message body length |
| 10 | CheckSum | Message checksum (mod 256) |
| 35 | MsgType | Message type identifier |
| 49 | SenderCompID | Sender's identifier |
| 56 | TargetCompID | Target's identifier |
| 50 | SenderSubID | Sender's sub-identifier (QUOTE/TRADE) |
| 34 | MsgSeqNum | Message sequence number |
| 52 | SendingTime | Message timestamp |
| 55 | Symbol | FIX Symbol ID (cTrader-specific) |
| 108 | HeartBtInt | Heartbeat interval (seconds) |
| 141 | ResetSeqNumFlag | Reset sequence numbers |
| 262 | MDReqID | Market data request ID |
| 263 | SubscriptionRequestType | 1=Subscribe, 2=Unsubscribe |
| 268 | NoMDEntries | Number of market data entries |
| 269 | MDEntryType | 0=Bid, 1=Ask, 2=Trade |
| 270 | MDEntryPx | Price |
| 271 | MDEntrySize | Size/Volume |
| 553 | Username | Login username |
| 554 | Password | Login password |

## 🔍 Finding Symbol IDs

**IMPORTANT**: cTrader uses internal FIX Symbol IDs (integers), not standard symbols like "EURUSD".

### How to Find Symbol IDs:

1. Open cTrader desktop application
2. Right-click any symbol in the Market Watch
3. Select "Symbol Info"
4. Look for **FIX Symbol ID** field
5. Copy the numeric ID

**Example Symbol IDs** (broker-specific, may vary):
- `1` → EURUSD
- `2` → GBPUSD
- `3` → USDJPY
- `4` → AUDUSD

**⚠️ These IDs vary by broker!** You must check your specific broker's symbol IDs.

## 📊 Understanding Market Data

### Market Data Snapshot (MsgType=W)

A full snapshot of the order book:

```
35=W|55=1|268=2|
269=0|270=1.10500|271=1000000|    ← Bid
269=1|270=1.10502|271=1000000|    ← Ask
```

**Fields:**
- `55=1` → Symbol ID (e.g., EURUSD)
- `268=2` → 2 entries (bid + ask)
- `269=0` → Bid price
- `270=1.10500` → Price value
- `271=1000000` → Size (1,000,000 units)

### Market Data Incremental Refresh (MsgType=X)

Real-time price updates:

```
35=X|55=1|268=1|
269=0|270=1.10505|271=1500000|    ← Bid update
```

## 🛠️ Extending This Code

### 1. **Add More Symbols**

In `client.rs`, modify the symbol list:

```rust
let md_request = create_market_data_request(
    &self.sender_comp_id,
    &self.target_comp_id,
    &self.sender_sub_id,
    seq,
    &["1", "2", "3"], // Multiple symbol IDs
);
```

### 2. **Store Price Data**

Add a price cache:

```rust
use std::collections::HashMap;

pub struct PriceCache {
    prices: HashMap<String, (f64, f64)>, // symbol -> (bid, ask)
}
```

### 3. **Integrate with WebSocket**

Broadcast FIX data to WebSocket clients:

```rust
// In client.rs
self.broadcaster.send(WsMessage::QuoteUpdate {
    symbol: "EURUSD",
    bid: 1.10500,
    ask: 1.10502,
    timestamp: Utc::now(),
});
```

### 4. **Use SSL Connection (Port 5211)**

Replace `TcpStream` with `tokio_native_tls::TlsStream`:

```rust
use tokio_native_tls::{TlsConnector, TlsStream};

let connector = TlsConnector::from(
    native_tls::TlsConnector::new()?
);
let stream = TcpStream::connect(format!("{}:{}", host, 5211)).await?;
let stream = connector.connect(host, stream).await?;
```

## ⚠️ Current Limitations

1. **Simplified Message Parsing**: Doesn't handle repeating groups properly
2. **No Sequence Gap Handling**: Doesn't request retransmission
3. **Basic Heartbeat**: Heartbeat is logged but not sent (needs shared writer)
4. **No Session Recovery**: Doesn't handle reconnections automatically
5. **Hardcoded Symbol Request**: Only requests symbol ID "1"

## 🔐 Security Notes

- **Never hardcode passwords** in production code
- Use environment variables: `std::env::var("CTRADER_PASSWORD")`
- Consider using SSL port 5211 for production
- Implement proper key management for live accounts

## 📖 Resources

- [cTrader FIX API Documentation](https://help.ctrader.com/fix/)
- [FIX Protocol Specification](https://www.fixtrading.org/standards/)
- [cTrader Community Forum](https://community.ctrader.com/forum/fix-api/)

## 🐛 Troubleshooting

### "Connection refused"
- Check if port 5201 is correct (use 5211 for SSL)
- Verify your IP is not blocked by cTrader

### "Logon rejected"
- Verify SenderCompID: `live.fxpro.8244184`
- Check password is correct
- Ensure account has FIX API enabled

### "No market data received"
- Check symbol ID is correct for your broker
- Verify SenderSubID is "QUOTE" (not "TRADE")
- Some brokers restrict market data access

### "Connection drops after 30 seconds"
- Heartbeat implementation needs improvement
- The current code logs heartbeats but doesn't send them properly

## 🎯 Next Steps

1. **Test the connection** with your credentials
2. **Find your broker's symbol IDs** in cTrader
3. **Parse market data** and print bid/ask prices
4. **Integrate with your order book engine**
5. **Add WebSocket broadcasting** for frontend clients

---

**Happy Trading!** 🚀📈
