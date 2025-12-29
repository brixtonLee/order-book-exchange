# FIX API Update - TargetSubID Fix

## Issue Found

cTrader was rejecting the FIX connection with error:
```
TargetSubID is assigned with the unexpected value '', expected 'QUOTE'
```

## Root Cause

The FIX messages were **missing tag 57 (TargetSubID)**, which cTrader requires for proper message routing.

## Fix Applied

Added **TargetSubID (tag 57)** to all FIX messages:

### Updated Functions:
1. `create_logon_message()` - Added `target_sub_id` parameter
2. `create_market_data_request()` - Added `target_sub_id` parameter
3. `create_heartbeat()` - Added `target_sub_id` parameter
4. `CTraderFixClient::new()` - Added `target_sub_id` field
5. `get_field_name()` - Added mapping for tag 57

### Configuration:
- **SenderSubID (tag 50)**: `"QUOTE"`
- **TargetSubID (tag 57)**: `"QUOTE"` ← **Now included**

## Test Again

Run the updated client:

```bash
cargo run --bin ctrader_fix_test
```

Enter your password when prompted.

## Expected Flow

1. ✅ TCP connection established
2. ✅ Logon message sent **with TargetSubID=QUOTE**
3. ✅ Logon response received (authentication successful)
4. ✅ Market Data Request sent
5. ✅ Market data received!

## FIX Message Format (Before vs After)

### ❌ Before (Missing tag 57):
```
8=FIX.4.4|9=123|35=A|49=live.fxpro.8244184|50=QUOTE|56=cServer|...
```

### ✅ After (With tag 57):
```
8=FIX.4.4|9=130|35=A|49=live.fxpro.8244184|50=QUOTE|56=cServer|57=QUOTE|...
                                                                 ^^^^^^^^
                                                            TargetSubID added!
```

## Why This Matters

cTrader uses **two separate FIX sessions**:
- **QUOTE** session - For market data (read-only)
- **TRADE** session - For order execution

The `TargetSubID` field tells cTrader which session handler should process the message. Without it, cTrader doesn't know where to route the message and rejects it.

---

**You should now be able to connect successfully!** 🚀
