# cTrader FIX API - Quick Start Guide

## 🚀 Run the Test Client

```bash
cd /Users/brixton/Desktop/rust-order-book
cargo run --bin ctrader_fix_test
```

Enter your account password when prompted: `[Your cTrader account 8244184 password]`

## 📂 Files Created

```
src/
├── ctrader_fix/
│   ├── mod.rs           # Module entry point
│   ├── messages.rs      # FIX message builder/parser
│   ├── client.rs        # Connection and session logic
│   └── README.md        # Detailed documentation
└── bin/
    └── ctrader_fix_test.rs  # Standalone test binary
```

## 🔍 What the Code Does

1. **Connects** to cTrader FIX server via TCP
2. **Sends Logon** message with your credentials
3. **Subscribes** to market data for symbol ID "1"
4. **Receives** and **displays** FIX messages in console
5. **Parses** market data (bid/ask prices)

## 📊 Expected Output

```
╔════════════════════════════════════════════════════════════╗
║         cTrader FIX API Connection Test                   ║
╚════════════════════════════════════════════════════════════╝

🔌 Connecting to cTrader FIX API...
   Host: live-uk-eqx-01.p.c-trader.com:5201
   SenderCompID: live.fxpro.8244184
   TargetCompID: cServer

✅ TCP connection established!

📤 Sending Logon message...
✅ Logon message sent!

📥 Waiting for responses from cTrader...

╔══════════════════════════════════════════════════════════════╗
║ 📨 RECEIVED FIX MESSAGE                                      ║
╠══════════════════════════════════════════════════════════════╣
║ Message Type: Logon (A)                                      ║
╠══════════════════════════════════════════════════════════════╣
║ Parsed Fields:                                               ║
║ [ 35] MsgType             = A                                ║
║ [ 49] SenderCompID        = cServer                          ║
║ [ 56] TargetCompID        = live.fxpro.8244184              ║
║ ...                                                           ║
╚══════════════════════════════════════════════════════════════╝

✅ Logon successful! Sending Market Data Request...

╔══════════════════════════════════════════════════════════════╗
║ 📨 RECEIVED FIX MESSAGE                                      ║
╠══════════════════════════════════════════════════════════════╣
║ Message Type: Market Data Snapshot (W)                      ║
╠══════════════════════════════════════════════════════════════╣
║ Parsed Fields:                                               ║
║ [ 55] Symbol             = 1                                 ║
║ [268] NoMDEntries        = 2                                 ║
║ [269] MDEntryType        = 0                                 ║
║ [270] MDEntryPx          = 1.10500                          ║
║ [271] MDEntrySize        = 1000000                          ║
╚══════════════════════════════════════════════════════════════╝

💰 Market data received!
🔍 Market Data Details:
   Symbol ID: 1
   Number of entries: 2
   Entry Type: Bid
   📈 Price: 1.10500
   📊 Size: 1000000
```

## ⚠️ Important: Find Your Symbol IDs

The code requests symbol ID `"1"` by default. To find the correct symbol IDs for your broker:

1. Open **cTrader Desktop** app
2. Right-click any symbol → **"Symbol Info"**
3. Look for **"FIX Symbol ID"** field
4. Note the numeric ID

**Symbol IDs are broker-specific!** ID "1" might be EURUSD for one broker, but GBPUSD for another.

## 🛠️ Modify Symbol Subscriptions

Edit `src/ctrader_fix/client.rs` around line 212:

```rust
let md_request = create_market_data_request(
    &self.sender_comp_id,
    &self.target_comp_id,
    &self.sender_sub_id,
    seq,
    &["1"],  // ← Change this! Add multiple: &["1", "2", "3"]
);
```

## 🔐 Security Note

**Never commit passwords!** The current code prompts for password input. For production:

```rust
let password = std::env::var("CTRADER_PASSWORD")
    .expect("Set CTRADER_PASSWORD environment variable");
```

Then run:
```bash
export CTRADER_PASSWORD="your_password"
cargo run --bin ctrader_fix_test
```

## 📚 Learn More

See detailed documentation: `src/ctrader_fix/README.md`

- FIX protocol explained
- Message types reference
- Extending the code
- Troubleshooting guide

## 🎯 Next Steps

After testing the connection:

1. ✅ **Verify you receive market data**
2. 📝 **Find your broker's symbol IDs**
3. 🔧 **Modify symbol subscriptions**
4. 💾 **Store prices in a data structure**
5. 🌐 **Integrate with WebSocket broadcaster**
6. 📊 **Feed data into your order book engine**

---

**Ready to test?** Run:

```bash
cargo run --bin ctrader_fix_test
```

Good luck! 🚀
