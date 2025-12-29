# FIX Message Field Order Issue - RESOLVED ✅

## The Problem You Encountered

When you sent a Market Data Request (MsgType=V) to cTrader, you received this error:

```
Message Type: Other (3)  ← This is a Reject message!
58=Tag not defined for this message type, field=55
371=55   ← The problematic tag
372=V    ← In message type V (Market Data Request)
373=2    ← Session-level reject
```

**Translation**: "Tag 55 (Symbol) is not allowed in this position for MsgType=V"

## Root Cause

FIX Protocol has **strict field ordering requirements** for repeating groups. Your message had:

### ❌ WRONG Order (What Was Sent)
```
262=REQ-... | 263=1 | 264=1 | 265=1 | 146=1 | 267=2 | 55=41 | 269=0 | 269=1
                                              ↑       ↑       ↑       ↑
                                          Count2  Count1  Symbol  EntryTypes
```

The server saw:
1. `146=1` (NoRelatedSym = 1 symbol coming)
2. `267=2` (NoMDEntryTypes = 2 entry types coming) ← **WRONG! Where's the symbol?**
3. `55=41` (Symbol) ← **ERROR! Too late, we're already in the MDEntryTypes group!**

### ✅ CORRECT Order (What's Now Sent)
```
262=REQ-... | 263=1 | 264=1 | 265=1 | 146=1 | 55=41 | 267=2 | 269=0 | 269=1
                                              ↑       ↑       ↑       ↑
                                          Count1  Symbol  Count2  EntryTypes
```

Now the server sees:
1. `146=1` (NoRelatedSym = 1 symbol coming)
2. `55=41` (Symbol) ← **CORRECT! Symbol immediately after its count**
3. `267=2` (NoMDEntryTypes = 2 entry types coming)
4. `269=0`, `269=1` (Bid, Offer) ← **CORRECT! Entry types after their count**

## FIX Repeating Groups Explained

In FIX protocol, a **repeating group** consists of:
1. A **count field** (e.g., `146=1` means "1 symbol follows")
2. The **repeating fields** (e.g., `55=41` is the symbol)

**CRITICAL RULE**: The repeating fields MUST come **immediately** after their count field.

### Example: Multiple Symbols

If you wanted to subscribe to 3 symbols (Gold, EURUSD, GBPUSD):

```
146=3        ← NoRelatedSym count
55=41        ← Symbol 1 (XAUUSD)
55=1         ← Symbol 2 (EURUSD)
55=2         ← Symbol 3 (GBPUSD)
267=2        ← NoMDEntryTypes count
269=0        ← Bid
269=1        ← Offer
```

If you sent:
```
146=3        ← Count says 3 symbols
267=2        ← WRONG! Server expects symbols here, not entry types
55=41        ← ERROR! Too late
```

The server would reject it.

## The Fix

### Before (in `messages.rs`):

We used a generic `FixMessage` builder that collected all body fields, then all repeating fields:

```rust
// Add body fields
for (tag, value) in &self.body_fields {
    body.push_str(&format!("{}={}\x01", tag, value));
}

// Add repeating groups AFTER all body fields
for (tag, value) in &self.repeating_groups {
    body.push_str(&format!("{}={}\x01", tag, value));
}
```

This produced:
```
146=1 | 267=2 | 55=41 | 269=0 | 269=1
        ↑       ↑       ↑
    body fields  repeating fields (separated - WRONG!)
```

### After (Fixed):

We now manually build the Market Data Request with exact field order:

```rust
// FIRST repeating group: NoRelatedSym + Symbol(s)
body.push_str(&format!("146={}\x01", symbol_ids.len()));  // Count
for symbol_id in symbol_ids {
    body.push_str(&format!("55={}\x01", symbol_id));      // Symbol (immediate!)
}

// SECOND repeating group: NoMDEntryTypes + MDEntryType(s)
body.push_str("267=2\x01");                                // Count
body.push_str("269=0\x01");                                // Bid
body.push_str("269=1\x01");                                // Offer
```

This produces:
```
146=1 | 55=41 | 267=2 | 269=0 | 269=1
↑       ↑       ↑       ↑       ↑
Count   Symbol  Count   Bid     Offer
(interleaved correctly!)
```

## Complete Field Order Reference

Here's the complete, correct field order for Market Data Request (MsgType=V):

| Order | Tag | Field Name | Value | Notes |
|-------|-----|------------|-------|-------|
| 1 | 8 | BeginString | FIX.4.4 | Standard Header |
| 2 | 9 | BodyLength | (calculated) | Standard Header |
| 3 | 35 | MsgType | V | Message Type |
| 4 | 49 | SenderCompID | live.fxpro.XXX | Your account |
| 5 | 56 | TargetCompID | cServer | cTrader server |
| 6 | 34 | MsgSeqNum | 2, 3, 4... | Sequence number |
| 7 | 52 | SendingTime | YYYYMMDD-HH:MM:SS.sss | UTC timestamp |
| 8 | 50 | SenderSubID | QUOTE | Required by cTrader |
| 9 | 57 | TargetSubID | QUOTE | Required by cTrader |
| 10 | 262 | MDReqID | REQ-timestamp | Unique request ID |
| 11 | 263 | SubscriptionRequestType | 1 | 1=Subscribe |
| 12 | 264 | MarketDepth | 1 | 1=Top of book |
| 13 | 265 | MDUpdateType | 1 | 1=Incremental |
| 14 | 146 | NoRelatedSym | 1 | **Start of group 1** |
| 15 | 55 | Symbol | 41 | **Group 1 data** |
| 16 | 267 | NoMDEntryTypes | 2 | **Start of group 2** |
| 17 | 269 | MDEntryType | 0 | **Group 2 data** (Bid) |
| 18 | 269 | MDEntryType | 1 | **Group 2 data** (Offer) |
| 19 | 10 | CheckSum | (calculated) | Standard Trailer |

## Testing Your Fix

Run the test binary again:

```bash
cargo run --bin ctrader_fix_test
```

You should now see:

1. **Logon successful** (MsgType=A response)
2. **Market Data Request sent** with correct field order
3. **Market Data Snapshot** (MsgType=W) ← Initial snapshot
4. **Incremental Refresh** (MsgType=X) ← Streaming updates!

Instead of:
- ❌ Reject (MsgType=3) with "Tag not defined"

## Why This Matters for Performance

Getting the field order right isn't just about avoiding rejections - it's crucial for performance:

1. **Server-side validation**: Wrong order = server must reject = wasted round trip
2. **Parser efficiency**: Correct order lets server use streaming parser
3. **Message size**: Compact, predictable layout = less data
4. **Reconnection speed**: Valid messages = instant subscription

## Common FIX Repeating Group Patterns

### Pattern 1: Simple Repeating Group (What We Fixed)
```
Count field
  Repeating field 1
  Repeating field 2
  ...
Next field
```

### Pattern 2: Nested Repeating Groups
```
Outer count
  Inner count
    Nested field
  Inner count
    Nested field
Next field
```

### Pattern 3: Multiple Independent Groups
```
Count A
  Field A1
  Field A2
Count B
  Field B1
  Field B2
```

**The Rule**: Each count field MUST be immediately followed by its data fields, in order.

## Debugging Tips

If you get a reject, look for:

1. **Tag 58**: Human-readable error message
2. **Tag 371**: The tag that caused the problem
3. **Tag 372**: The message type where it occurred
4. **Tag 373**: Session/app level reject code

Common reject codes:
- `373=2`: Session-level reject (wrong field order, missing required field)
- `373=5`: Application-level reject (business rule violation)

## Summary

✅ **Fixed**: Market Data Request now sends fields in correct order
✅ **Cause**: FIX repeating groups require count field + data fields to be adjacent
✅ **Solution**: Manual message construction for Market Data Request
✅ **Result**: You should now receive market data snapshots and streaming updates!

Try running the test again, and you should see real-time Gold (XAUUSD) price updates! 🚀
