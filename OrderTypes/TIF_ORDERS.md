# Time-In-Force (TIF) Orders

## 📋 Overview

**Time-In-Force (TIF)** is an instruction that tells the exchange how long an order should remain active before it expires or gets cancelled.

> 💡 **Key Insight**: TIF is like setting a "shelf life" for your order. Just as you wouldn't want milk to sit in your fridge forever, traders don't want orders sitting in the order book indefinitely at prices that may no longer make sense.

---

## 🎯 The Five TIF Types

### 1. GTC (Good-Till-Cancelled)

**Definition**: Order stays active until you manually cancel it or it gets filled

**Characteristics**:
- Remains in order book indefinitely
- Survives market close and overnight
- Must be explicitly cancelled by trader
- Most common for long-term positions

**Use Cases**:
- Patient investors waiting for target price
- Long-term limit orders
- Value investing strategies
- Setting profit targets weeks in advance

**Example Scenario**:
You want to buy Apple stock at $150, and you're happy to wait until it drops to that price, even if it takes a month.

**Risks**:
- ⚠️ Forgotten orders executing at stale prices
- ⚠️ Market conditions change, order becomes inappropriate
- ⚠️ Tied-up capital (pending orders may affect margin)

---

### 2. IOC (Immediate-Or-Cancel)

**Definition**: Execute immediately whatever you can, cancel the rest

**Characteristics**:
- Executes against available liquidity instantly
- Unfilled portion is cancelled (never added to book)
- No residual order remains
- Most common in high-frequency trading

**Use Cases**:
- High-frequency trading strategies
- Taking available liquidity without revealing size
- Avoiding information leakage
- Smart order routing

**Example Scenario**:
You want 1,000 shares, but only 300 are available at your price. IOC fills 300 immediately and cancels the remaining 700.

**Why Traders Love It**:
- ✅ No information leakage - your large order size isn't revealed to the market
- ✅ No adverse selection risk from sitting in book
- ✅ Immediate execution feedback
- ✅ No cleanup required (no partial orders to manage)

> 📊 **Industry Stat**: IOC is used in ~95% of high-frequency trading orders because leaving orders in the book reveals your strategy to competitors who can then front-run you or adjust their prices.

---

### 3. FOK (Fill-Or-Kill)

**Definition**: Either fill my ENTIRE order right now, or cancel all of it

**Characteristics**:
- Atomic execution (all-or-nothing)
- No partial fills allowed
- Rejected if insufficient liquidity
- Never added to order book

**Use Cases**:
- Arbitrage strategies requiring exact quantities
- Multi-leg option strategies
- Basket trades requiring all components
- Hedging strategies where partial hedges are ineffective

**Example Scenario**:
You're executing an arbitrage trade that requires exactly 1,000 shares to work. If you only get 600 shares, the arbitrage breaks, so FOK ensures you get all 1,000 or nothing.

**Critical For**:
- Multi-leg strategies (spreads, straddles, butterflies)
- Basket trades (ETF creation/redemption)
- Hedging strategies (must match exposure exactly)
- Algorithmic trading requiring precise execution

**Trade-Off**:
- Higher rejection rate vs. IOC
- Guarantees strategy integrity
- May miss execution opportunities

---

### 4. GTD (Good-Till-Date)

**Definition**: Order expires at a specific date/time you choose

**Characteristics**:
- Custom expiration timestamp
- Automatically cancelled at expiry
- Flexible duration (hours to months)
- Event-driven timing

**Use Cases**:
- Event-driven trading (earnings, FOMC, elections)
- Option expiration strategies
- Avoiding weekend risk
- Corporate action timing (dividends, splits)

**Example Scenario**:
You expect a company to announce earnings on Friday at 4pm. You set a GTD order to expire at 4:05pm - if it doesn't fill by then, you no longer want the position.

**Practical Applications**:
- **Earnings Trading**: Expire orders after announcement
- **Economic Data**: FOMC minutes, jobs report, GDP
- **Option Expiry**: Close positions before expiration
- **Weekend Risk**: Expire Friday 4pm to avoid gap risk

**Advantages**:
- ✅ Automatic risk management
- ✅ No manual monitoring required
- ✅ Precise timing control
- ✅ Prevents stale execution after event

---

### 5. DAY

**Definition**: Order expires at the end of the trading day (typically 4pm ET for US markets)

**Characteristics**:
- Expires at market close
- Most common retail default
- No overnight exposure
- Automatic daily cleanup

**Use Cases**:
- Day trading strategies
- Avoiding overnight news risk
- Preventing gap risk
- Managing margin requirements

**Example Scenario**:
You're day trading Tesla and don't want to hold positions overnight due to Elon's late-night tweets. DAY orders ensure you don't accidentally hold a position into the next day.

**Industry Standard**:
- Most retail brokers default to DAY orders
- Protects inexperienced traders from overnight risk
- Reduces customer service calls about forgotten orders
- Aligns with typical day-trading workflow

---

## 🎭 Real-World Example: Market Maker's Day

**Morning (9:30am) - Market Open**:
- Place **GTC** orders on both sides of the spread
- Goal: Earn rebates all day as passive maker
- Strategy: Patient liquidity provision

**Midday (12:00pm) - Breaking News**:
- iPhone sales miss expectations (breaking news)
- **Cancel all GTC orders immediately**
- Switch to **IOC** orders only
- Rationale: Avoid getting picked off at stale prices

**Afternoon (3:45pm) - Market Close Approaching**:
- Switch to **DAY** orders
- Goal: Ensure flat position by end of day
- Risk management: No overnight exposure

**Special Event (4:00pm) - Earnings Announcement**:
- Apple announces earnings at 4pm
- Place **GTD** order expiring at 4:05pm
- Capture initial volatility, avoid extended exposure

---

## 📊 TIF Comparison Table

| TIF Type | Duration | Partial Fills | Added to Book | Best For |
|----------|----------|---------------|---------------|----------|
| **GTC** | Until cancelled | ✅ Yes | ✅ Yes | Patient investors |
| **IOC** | Immediate | ✅ Yes | ❌ No | HFT, minimal leakage |
| **FOK** | Immediate | ❌ No | ❌ No | Atomic strategies |
| **GTD** | Custom timestamp | ✅ Yes | ✅ Yes | Event-driven trading |
| **DAY** | Until market close | ✅ Yes | ✅ Yes | Day traders |

---

## ⚠️ Why TIF Matters

### Without Proper TIF Usage:

**Risk 1: Stale Execution**
- Order placed at $100
- Stock crashes to $50 due to bad news
- GTC order sits and fills when stock rebounds to $100
- You bought something you no longer want

**Risk 2: Information Leakage**
- Large GTC limit order reveals your size
- Competitors adjust quotes
- You experience adverse selection
- Execution quality degrades

**Risk 3: Unmanaged Exposure**
- Forgot to cancel orders
- Accumulated unwanted overnight risk
- Margin calls due to unexpected fills
- Weekend gap risk

### With Proper TIF Usage:

**Benefit 1: Precise Risk Control**
- GTD expires after earnings (no stale exposure)
- DAY prevents overnight gaps
- FOK ensures strategy integrity

**Benefit 2: Minimal Information Leakage**
- IOC shows only executed quantity
- Competitors can't see full order size
- Better execution quality

**Benefit 3: Automated Risk Management**
- Orders expire automatically
- No manual monitoring required
- Prevents forgotten order fills

---

## 💼 Professional Trading Scenarios

### Scenario 1: Institutional Block Trade

**Challenge**: Execute 100,000 shares without moving the market

**Strategy**:
- Use **IOC** orders in small chunks (1,000-5,000 shares)
- Over 2-hour window via VWAP algorithm
- Never show full 100,000 size
- Minimize market impact

**Why not GTC?**
- Would reveal 100,000 share intention
- Other traders would front-run
- Price would move against you

---

### Scenario 2: Arbitrage Trading

**Challenge**: Execute multi-leg trade atomically

**Strategy**:
- Use **FOK** to buy ETF and sell components simultaneously
- All legs must fill or entire trade cancelled
- Arbitrage only works with complete execution

**Why not IOC?**
- Partial fills break the arbitrage
- You'd be left with unwanted inventory
- Risk the arbitrage disappearing

---

### Scenario 3: Earnings Trading

**Challenge**: Capture post-earnings volatility, avoid extended exposure

**Strategy**:
- Use **GTD** expiring 5 minutes after announcement
- Capture initial move
- Avoid holding through extended volatility

**Why not GTC or DAY?**
- GTC would stay active too long
- DAY might miss the specific timing window
- GTD provides precise control

---

## 🔑 Key Takeaways

1. **GTC** = Patient capital, willing to wait
2. **IOC** = Stealth execution, minimal leakage
3. **FOK** = All-or-nothing, strategy integrity
4. **GTD** = Event-driven, precise timing
5. **DAY** = Day trading, no overnight risk

> 💡 **Pro Tip**: Professional traders often use different TIF types throughout the day based on market conditions, news flow, and strategy requirements. The right TIF at the right time is a critical edge in trading.

---

## 📚 Additional Resources

**Order Book Implementation**:
- TIF controls whether unfilled portions remain in book (GTC, GTD, DAY)
- Or are immediately cancelled (IOC, FOK)
- Critical for trader flexibility and risk management

**Exchange Support**:
- All major exchanges support these 5 TIF types
- Required for institutional business
- Competitive necessity

**Regulatory Considerations**:
- TIF helps demonstrate best execution
- Audit trail shows intent and timing
- Risk management compliance

---

*This guide covers the essential Time-In-Force order types implemented in professional trading systems.*
