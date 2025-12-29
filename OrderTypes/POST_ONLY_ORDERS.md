# Post-Only Orders

## 📋 Overview

**Post-Only orders** are a special instruction that guarantees your order will **only add liquidity** to the order book, never remove it.

> 💰 **Key Insight**: Post-Only is essentially a "maker fee guarantee" - it ensures you NEVER pay taker fees, only receive maker rebates. For market makers running on thin margins, this difference can mean the entire profit margin of their business.

---

## 🎯 The Core Concept

### What "Post-Only" Means:

**The Guarantee**:
If your order would **match immediately** (cross the spread), the exchange **rejects it entirely** instead of executing it.

**The Alternative**:
Without post-only, if you accidentally cross the spread, your order becomes a taker and you pay higher fees.

### The Philosophy:

**Post-Only Says**:
- "I want to be a liquidity provider (maker) ONLY"
- "If I can't add to the book, reject my order"
- "Never let me accidentally become a taker"
- "Guarantee my maker fees/rebates"

---

## 💵 Understanding Maker vs. Taker

### Maker (Liquidity Provider):

**Characteristics**:
- Order sits in the order book waiting
- Other traders execute against your order
- You provide liquidity to the market
- Rewarded with lower fees or rebates

**Fee Structure**:
- Typical: 0.10% fee
- Advanced: -0.01% (you get paid!)
- Some exchanges: -0.03% rebate

### Taker (Liquidity Remover):

**Characteristics**:
- Order matches immediately against existing orders
- You remove liquidity from the book
- You demand immediate execution
- Penalized with higher fees

**Fee Structure**:
- Typical: 0.20% fee
- Can be as high as 0.30%
- Always positive (you pay)

### The Financial Impact:

**On $1 Million in Trading Volume**:

| Role | Fee Rate | Cost/Rebate | Annual (250 days) |
|------|----------|-------------|-------------------|
| **Taker** | 0.20% | -$2,000 | -$500,000 |
| **Maker** | 0.10% | -$1,000 | -$250,000 |
| **Maker (rebate)** | -0.01% | +$100 | +$25,000 |
| **Difference** | — | **$2,100** | **$525,000** |

> 🚀 **For High-Frequency Market Makers**: A firm trading $1 billion/day could pay $2M in fees as a taker, but earn $100K in rebates as a maker - a **$2.1M daily difference**! Over a year, this is $525M in fee differential.

---

## 🛡️ The Problem Post-Only Solves

### Scenario: The Market Maker Accident

**Current Order Book**:
```
Asks (Sells):
  $150.50 - 100 shares
  $150.55 - 200 shares

Bids (Buys):
  $150.45 - 100 shares
  $150.40 - 150 shares

Spread: $0.05
```

**Your Algorithm's Intent**:
Place a buy order at **$150.49** to add liquidity inside the spread

**Oops - Programming Error**:
Your algorithm has a bug and places: **Buy at $150.50** instead

### Without Post-Only - The Disaster:

**What Happens**:
1. Your order matches immediately with the $150.50 sell
2. You pay **taker fees** (0.20% = $30.10 on 100 shares @ $150.50)
3. You just lost $30.10 on a mistake
4. Your algorithm executed when you didn't want it to
5. You overpaid for the stock (wanted $150.49, paid $150.50)

**Total Loss**: $0.01 per share price + $30.10 fees = **$31.10 wasted**

### With Post-Only - Protection Activated:

**What Happens**:
1. Exchange sees your buy at $150.50 would match the ask at $150.50
2. Exchange **rejects the order** immediately
3. Error message: "Post-only order would match immediately"
4. You pay **zero fees**
5. Your algorithm detects the error and can correct itself

**Result**: **$0 loss, protected from mistake**

---

## 🎯 Real-World Use Cases

### Use Case 1: Market Making Algorithm Protection

**Normal Operation**:

```
Your Algorithm Places:
  - BUY @ $150.45 (below best ask of $150.50) ✅ Posted
  - SELL @ $150.50 (above best bid of $150.45) ✅ Posted

Goal: Earn $0.05 spread when someone crosses
```

**Market Moves Suddenly**:

```
Breaking News: Apple beats earnings
Best ask drops to $150.43 in 10ms

Your buy at $150.45 would now cross the spread

Post-Only Response:
  ❌ Order REJECTED - would match immediately
  ✅ You avoided buying at stale price
  ✅ Your algorithm adjusts and requotes

Regular Order Response:
  ✅ Executes immediately at $150.43
  ❌ You bought when you meant to provide liquidity
  ❌ You paid taker fees instead of earning maker rebates
```

---

### Use Case 2: Spread Capture Strategy

**Your Strategy**:
- Always place orders $0.01 inside the current spread
- Wait for the spread to tighten
- Your order becomes best bid/ask
- Earn maker rebates when others trade against you

**Example Execution**:

```
Current Spread: $150.45 bid / $150.50 ask

Your Post-Only Orders:
  - BUY @ $150.46 (inside spread)
  - SELL @ $150.49 (inside spread)

If Accepted:
  New Spread: $150.46 bid / $150.49 ask
  You're at the top of the queue
  Earn maker fees on fills

If Rejected (someone else just posted same price):
  Try $150.47 / $150.48 instead
  Continue adjusting until accepted
```

**Why Post-Only is Essential**:
- Guarantees you either add liquidity or get rejected
- No accidental taker execution
- Maintains your maker-only profitability model

---

### Use Case 3: Penny-Jumping (Queue Position)

**Objective**: Get ahead in the order queue

```
Current Best Bid: $150.45 with 5,000 shares ahead of you

Your Strategy:
  - Bid $150.46 to jump ahead (penny-jumping)
  - Get priority over the 5,000 shares

Risk Without Post-Only:
  - Someone else just bid $150.46 milliseconds before you
  - Your order crosses the spread and matches an ask at $150.46
  - You become a taker, paying 0.20% fees
  - Your strategy fails

Protection With Post-Only:
  - If $150.46 would cross spread → REJECTED
  - You remain a maker-only participant
  - Try $150.47 instead
  - Your profitability model stays intact
```

---

## 🌐 Network Latency Protection

### The Latency Problem:

**In High-Frequency Trading**:
- Your view of the order book may be **1-10 milliseconds stale**
- Network latency means prices change while your order is in flight
- What looked like a maker order becomes a taker

**Example Timeline**:

```
T+0ms: You see best ask at $150.50
       You submit buy @ $150.49 (maker)

T+3ms: Order arrives at exchange
       Best ask is now $150.48 (someone sold aggressively)
       Your $150.49 would cross the spread!

Without Post-Only:
  ✅ Executes at $150.48
  ❌ You're a taker, pay 0.20% fees
  ❌ You overpaid vs. your intent

With Post-Only:
  ❌ REJECTED - would match
  ✅ You pay $0 fees
  ✅ You requote at current price
```

> 🔌 **Latency Reality**: Even with co-located servers in the same data center, network latency is typically 50-500 microseconds. Over the internet, it's 1-50ms. Post-only protects you from these timing gaps.

---

## 📊 When Post-Only Gets Rejected

### Buy Order Examples:

| Current Best Ask | Your Post-Only Buy | Result | Reason |
|------------------|-------------------|--------|--------|
| $150.50 | $150.50 | ❌ REJECTED | Would match immediately |
| $150.50 | $150.51 | ❌ REJECTED | Would cross spread |
| $150.50 | $150.49 | ✅ ACCEPTED | Adds liquidity to bid side |
| $150.50 | $150.45 | ✅ ACCEPTED | Adds liquidity to bid side |

### Sell Order Examples:

| Current Best Bid | Your Post-Only Sell | Result | Reason |
|------------------|---------------------|--------|--------|
| $150.45 | $150.45 | ❌ REJECTED | Would match immediately |
| $150.45 | $150.44 | ❌ REJECTED | Would cross spread |
| $150.45 | $150.46 | ✅ ACCEPTED | Adds liquidity to ask side |
| $150.45 | $150.50 | ✅ ACCEPTED | Adds liquidity to ask side |

---

## 💼 Market Maker Economics

### Without Post-Only (Risky):

**Daily Trading Stats**:
- 1,000 orders placed
- 950 are makers (0.10% fee) = **-$950**
- 50 are accidental takers (0.20% fee) = **-$2,000**

**Financial Result**:
- Total fees: **-$2,950**
- Spread revenue: **+$5,000**
- **Net profit**: **$2,050**

### With Post-Only (Protected):

**Daily Trading Stats**:
- 1,000 orders attempted
- 950 accepted as makers (0.10% fee) = **-$950**
- 50 rejected by post-only = **$0**

**Financial Result**:
- Total fees: **-$950**
- Spread revenue: **$4,750** (slightly less due to rejections)
- **Net profit**: **$3,800**

**Improvement**: **85% higher profit!** ($3,800 vs $2,050)

---

## 🔗 Combining Post-Only with Other Features

### Post-Only + STP (Self-Trade Prevention):

**Scenario**:
```
You have: SELL 100 @ $150.49 in book
You submit: Post-Only BUY 50 @ $150.49
```

**Process**:
1. Post-only check: Would match? YES
2. STP check: Same user? YES
3. STP mode: Cancel Resting
4. **Result**: Old sell cancelled, new buy posted
5. Both features work together harmoniously

**Benefit**: Clean order management with maker guarantee

---

### Post-Only + IOC (Contradictory!):

**Conflict**:
- Post-Only: "Only add to book, never take liquidity"
- IOC: "Execute immediately or cancel"

**Result**: **Most exchanges reject this combination**

**Why**:
- These are mutually exclusive instructions
- IOC wants immediate execution (taker)
- Post-Only forbids taking liquidity
- Logical contradiction

---

### Post-Only + FOK (Contradictory!):

**Conflict**:
- Post-Only: "Only add to book"
- FOK: "Fill completely now or kill"

**Result**: **Not allowed on most exchanges**

**Why**:
- FOK demands immediate full execution
- Post-only forbids immediate execution
- Impossible to satisfy both

---

### Post-Only + GTC (Perfect Match):

**Combination**:
- Post-Only: Maker-only guarantee
- GTC: Stay in book until cancelled

**Result**: **Common professional setup**

**Use Case**:
- Patient market maker
- Want to provide liquidity long-term
- Never accidentally take liquidity
- Perfect for passive strategies

---

## 🏢 Why Exchanges Need Post-Only

### Without Post-Only - Thin Market:

**Order Book**:
```
Best Bid: $150.00
Best Ask: $151.00
Spread: $1.00 (terrible!)
```

**Characteristics**:
- Wide spreads
- Low volume
- High volatility
- Poor trader experience
- No professional market makers

---

### With Post-Only - Tight Market:

**Order Book**:
```
Best Bid: $150.45
Best Ask: $150.50
Spread: $0.05 (excellent!)
```

**Characteristics**:
- Tight spreads
- High volume
- Stable prices
- Great trader experience
- Professional market makers active

---

### Exchange Benefits:

**Trading Volume**:
- More market makers = more liquidity
- More liquidity = more traders
- More traders = more volume
- More volume = more fee revenue

**Competitiveness**:
- Professional infrastructure
- Attracts institutional clients
- Matches major exchanges (Coinbase, Binance, Kraken)
- Market legitimacy

**User Experience**:
- Tighter spreads benefit all traders
- Better price discovery
- Lower slippage
- Professional market

> 🏆 **Competitive Necessity**: Coinbase Pro, Binance, Kraken, and all major exchanges offer post-only orders. If an exchange doesn't support post-only, professional market makers simply won't provide liquidity there, resulting in wide spreads and poor trading experience for all users.

---

## ⚙️ Implementation Details

### The Check Process:

**Order Submission Flow**:
1. Order arrives at exchange
2. **Post-Only Check** (BEFORE matching):
   - Would this order match immediately?
   - Compare order price vs. best opposite side
3. **If would match** → **REJECT** with specific error
4. **If safe** → Proceed to add to order book
5. Order waits for someone else to match against it
6. When filled → **Maker fees apply** (guaranteed!)

### Regular Order Flow (For Comparison):

1. Order arrives
2. Match immediately if possible (might pay taker fees)
3. Whatever doesn't match goes to book

### The Critical Difference:

**Post-Only**: Check BEFORE matching, reject if would take
**Regular**: Match first, add remainder to book

---

## 💎 Maker Rebate Programs

### What Are Rebates?

**Negative Fees**:
- Instead of paying fees, you GET PAID
- Typical rebate: -0.01% to -0.03%
- Exchange pays you to provide liquidity

### The Economics:

**On $1 Million Traded**:
- -0.01% rebate = **+$100 earned**
- -0.02% rebate = **+$200 earned**
- -0.03% rebate = **+$300 earned**

**On $1 Billion Traded (Daily for Large MM)**:
- -0.01% rebate = **+$100,000 earned daily**
- **Annual**: **$25 million in rebates!**

> 💰 **Post-Only Guarantee**: With post-only orders, you NEVER miss these rebates by accidentally becoming a taker. This is critical when rebates are your primary revenue source.

---

## 🎓 Key Takeaways

1. **Post-Only guarantees maker status** - Never pay taker fees
2. **Protects against errors** - Rejects orders that would cross spread
3. **Essential for market makers** - Profitability depends on maker fees
4. **Compensates for latency** - Protects against stale market data
5. **Required by professionals** - Won't trade without it

### Best Practices:

✅ **Use Post-Only when**:
- Running market making algorithms
- Want guaranteed maker fees
- Operating on thin margins
- Providing passive liquidity

❌ **Don't use Post-Only when**:
- Need immediate execution (use IOC/FOK)
- Want to cross the spread intentionally
- Taking liquidity is acceptable
- Time-sensitive execution required

---

## 📊 Quick Reference Table

| Feature | Post-Only | Regular Order |
|---------|-----------|---------------|
| **Crosses Spread?** | ❌ Rejected | ✅ Executes |
| **Fee Type** | Maker only | Maker or Taker |
| **Typical Fee** | 0.10% or rebate | 0.10% to 0.20% |
| **Added to Book** | ✅ Always (if accepted) | Sometimes |
| **Execution Speed** | Slower (waits) | Faster (immediate) |
| **Use Case** | Market making | General trading |
| **Profitability** | Depends on rebates | Depends on price |

---

## 📚 Additional Resources

**Exchange Documentation**:
- Coinbase Pro: Post-Only orders
- Binance: Post-Only flag
- Kraken: Fill-or-post orders

**Market Microstructure**:
- Understanding maker-taker fee models
- The role of market makers in price discovery
- Spread dynamics and liquidity provision

**Professional Trading**:
- Market making strategies
- Statistical arbitrage with post-only
- High-frequency trading infrastructure

---

*This guide covers Post-Only orders, a critical feature for market makers and professional trading systems that operate on thin profit margins where maker vs. taker fee differences determine profitability.*
