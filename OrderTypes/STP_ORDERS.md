# Self-Trade Prevention (STP)

## 📋 Overview

**Self-Trade Prevention (STP)** is a critical feature that prevents a trader's buy order from matching against their own sell order.

> 🚨 **Critical Insight**: STP is **absolutely essential for market makers**. Without it, market making algorithms would constantly trade with themselves, paying fees to execute meaningless trades while generating fake volume. Many professional market makers won't even connect to an exchange that doesn't offer STP.

---

## 🤔 Why Self-Trading is a Problem

### The Market Maker Scenario

**Your Algorithm Places**:
- **Sell order**: 100 shares of Apple at $150.50 (ask side)
- **Buy order**: 100 shares of Apple at $150.40 (bid side)
- **Goal**: Earn the spread ($0.10 per share) when other traders cross it

**Then Your Algorithm Adjusts**:
- **New Buy order**: 100 shares at $150.50 (willing to pay more)

### Without STP - The Disaster:

**What Happens**:
- ❌ Your new buy order matches against your own sell order
- ❌ You pay yourself $150.50 per share (pointless transaction)
- ❌ You pay **taker fees** (~0.20% = $30.10)
- ❌ You receive **maker fees** (~0.10% = $15.05)
- ❌ **Net loss**: $15.05 in fees for a completely useless trade
- ❌ You generate **wash trading** (illegal in many jurisdictions)
- ❌ You create **fake volume** that misleads other market participants

### The Financial Impact:

**On $1 Million Daily Volume**:
- Self-trades: 10% of volume ($100,000)
- Maker fees paid: -$100 (0.10%)
- Taker fees paid: -$200 (0.20%)
- **Net loss from self-trading**: $300/day
- **Annual loss**: $109,500

> ⚖️ **Regulatory Risk**: Self-trading can be considered "wash trading" which is illegal under SEC rules and can result in fines, trading bans, or criminal charges. Even if accidental, regulators may view it as market manipulation because it artificially inflates trading volume.

---

## 🛡️ The Six STP Modes

### 1. None (No Protection)

**What It Means**: Allow self-trades to happen

**Behavior**:
- No checks performed
- Orders from same user can match
- Trades execute normally

**Use Cases**:
- ⚠️ Very rare - only if you have multiple independent trading desks
- Different strategies that should be allowed to trade with each other
- Institutional firms with Chinese walls between desks

**Risks**:
- Fees on meaningless trades
- Regulatory compliance issues
- Misleading market data
- Potential wash trading violations

**When to Use**:
- Multi-desk institutional setup
- Different legal entities using same account
- Explicit business requirement for internal crossing

---

### 2. Cancel Resting ⭐ (Most Common)

**What It Means**: Cancel the order already sitting in the book, allow new order to continue

**Behavior**:
- Old order in book is cancelled
- New incoming order continues matching
- Can still match with other traders

**Use Cases**:
- ✅ Your latest order reflects current market view
- ✅ Want to keep the most recent order active
- ✅ Automatic cleanup of stale orders

**Example**:
```
Initial State:
  - You have SELL 100 @ $150.50 in the book

You Submit:
  - BUY 50 @ $150.50 (would self-trade)

Result:
  - Old SELL order CANCELLED
  - New BUY 50 @ $150.50 stays active
  - Can match with other traders' sell orders
```

**Why Most Popular**:
- ✅ Latest order is most relevant (reflects current algo state)
- ✅ Automatic order management
- ✅ Prevents stale orders from lingering
- ✅ Keeps strategy current

> 📊 **Industry Standard**: Cancel Resting is preferred by ~70% of market makers because it ensures your most recent orders (reflecting current market conditions) remain active while stale orders are cleaned up automatically.

---

### 3. Cancel Incoming

**What It Means**: Cancel the new incoming order, keep the resting order

**Behavior**:
- Incoming order is rejected/cancelled
- Resting order remains in book
- Conservative approach

**Use Cases**:
- ✅ Don't disrupt existing book state
- ✅ Conservative risk management
- ✅ Preserve order priority (time priority matters)

**Example**:
```
Initial State:
  - You have SELL 100 @ $150.50 in the book

You Submit:
  - BUY 50 @ $150.50 (would self-trade)

Result:
  - New BUY order CANCELLED immediately
  - Old SELL 100 @ $150.50 remains in book
  - Your sell order keeps its time priority
```

**When to Use**:
- Order queue position is valuable (first in line)
- Resting order has priority you don't want to lose
- Conservative approach to book management

---

### 4. Cancel Both

**What It Means**: Cancel BOTH orders - the one in the book AND the incoming one

**Behavior**:
- Both orders cancelled
- No orders remain from this user at this price
- Maximum caution

**Use Cases**:
- ✅ Any conflict = remove everything
- ✅ Ultra-conservative risk management
- ✅ Prevent any possibility of self-trade

**Example**:
```
Initial State:
  - You have SELL 100 @ $150.50 in the book

You Submit:
  - BUY 50 @ $150.50 (would self-trade)

Result:
  - Old SELL order CANCELLED
  - New BUY order CANCELLED
  - You're completely out of the market at $150.50
```

**When to Use**:
- Conflicting orders indicate algo error
- Want to halt trading at that price
- Need to investigate before continuing

---

### 5. Cancel Smallest

**What It Means**: Cancel whichever order has the smaller quantity

**Behavior**:
- Compare order sizes
- Cancel the smaller one
- Keep the larger one

**Use Cases**:
- ✅ Optimize for maximum liquidity
- ✅ Keep larger orders in market
- ✅ Better capital efficiency

**Example**:
```
Initial State:
  - You have SELL 1,000 @ $150.50 in the book

You Submit:
  - BUY 100 @ $150.50 (would self-trade)

Result:
  - New BUY 100 CANCELLED (it's smaller)
  - SELL 1,000 remains in book
  - Maintains larger market presence
```

**Why It Matters**:
- Larger orders provide more liquidity
- Better for market impact
- Optimizes capital deployment

---

### 6. Decrement Both 🔬 (Advanced)

**What It Means**: Reduce both orders by the matching quantity WITHOUT creating a trade

**Behavior**:
- Calculate overlapping quantity
- Reduce both orders by that amount
- No trade created, no fees paid
- Remaining quantities stay in book

**Use Cases**:
- ✅ Advanced market making
- ✅ Reduce exposure on both sides
- ✅ Avoid round-trip fees
- ✅ Inventory management

**Example**:
```
Initial State:
  - You have SELL 500 @ $150.50 in the book

You Submit:
  - BUY 300 @ $150.50 (would self-trade)

Result:
  - SELL 500 becomes SELL 200 (reduced by 300)
  - BUY 300 becomes BUY 0 (fully decremented)
  - NO TRADE CREATED
  - NO FEES PAID
  - Net effect: Reduced sell-side exposure by 300
```

> 💎 **Most Sophisticated**: Decrement Both is the most advanced mode - it's essentially saying "I want to reduce my exposure on both sides without paying round-trip fees." This is common when market makers need to quickly reduce their inventory without generating meaningless trades.

**Real-World Application**:
- End-of-day inventory reduction
- Risk limit approaching
- Quick position flattening
- Avoid unnecessary trade tape entries

---

## 🎯 Real-World Use Cases

### Use Case 1: High-Frequency Market Maker

**Scenario**: Your algorithm updates quotes 1,000 times per second

**Timeline**:
- **9:30:00.000** - Place SELL @ $150.50 (100 shares)
- **9:30:00.100** - Place SELL @ $150.45 (100 shares) - market moved
- **9:30:00.200** - Place BUY @ $150.45 (50 shares) - algorithm adjustment

**Without STP**:
- Your buy at $150.45 matches your own sell at $150.45
- Pay $15 in fees for nothing
- Generate fake volume

**With STP (Cancel Resting)**:
- Your old sell at $150.45 gets cancelled
- Buy order continues normally
- Clean order management

---

### Use Case 2: Multi-Strategy Fund

**Setup**: You run 5 different trading algorithms:
- Algorithm A: Long-term trend following
- Algorithm B: Short-term mean reversion
- Algorithm C: Pairs trading
- Algorithm D: Market making
- Algorithm E: News-based trading

**Problem**:
- Algorithm B wants to sell Apple
- Algorithm D wants to buy Apple
- They might match each other

**Solution**:
- Set STP mode to **None** with different user IDs per algorithm
- Within same algorithm: **Cancel Resting** to prevent self-trades
- Allows intentional internal crossing when appropriate

---

### Use Case 3: Institutional Execution Desk

**Scenario**: VWAP execution over several hours

**Timeline**:
- **10:00am** - Place buy orders at multiple price levels
- **11:00am** - Market moves, need to cancel and replace orders
- **11:30am** - New orders might cross your own uncancelled orders

**With STP**:
- Automatic cleanup of conflicting orders
- No manual intervention required
- Clean execution without self-trades

---

## 📊 STP Mode Comparison Table

| STP Mode | Resting Order | Incoming Order | Trade Created | Fees Paid | Best For |
|----------|---------------|----------------|---------------|-----------|----------|
| **None** | Unchanged | Executes | ✅ Yes | ✅ Yes | Multi-desk firms |
| **Cancel Resting** | ❌ Cancelled | Continues | ❌ No | ❌ No | Most market makers |
| **Cancel Incoming** | ✅ Remains | ❌ Cancelled | ❌ No | ❌ No | Priority preservation |
| **Cancel Both** | ❌ Cancelled | ❌ Cancelled | ❌ No | ❌ No | Conservative risk |
| **Cancel Smallest** | Depends | Depends | ❌ No | ❌ No | Liquidity optimization |
| **Decrement Both** | ⬇️ Reduced | ⬇️ Reduced | ❌ No | ❌ No | Inventory management |

---

## 🏢 Why Exchanges Must Offer STP

### Without STP Support:

**Exchange Consequences**:
- ❌ Professional market makers won't participate
- ❌ Wide spreads due to poor liquidity
- ❌ Inflated (fake) trading volume
- ❌ Regulatory scrutiny
- ❌ Poor competitiveness vs. major exchanges
- ❌ Lower overall trading quality

### With STP Support:

**Exchange Benefits**:
- ✅ Attract sophisticated liquidity providers
- ✅ Tight spreads improve user experience
- ✅ Accurate volume metrics
- ✅ Regulatory compliance
- ✅ Competitive with Coinbase, Binance, Kraken
- ✅ Professional market infrastructure

> 📈 **Exchange Economics**: Major crypto exchanges like Coinbase Pro report that 60-80% of their order book depth comes from market makers. If you don't offer STP, those market makers simply won't participate, leaving you with a thin, illiquid market that attracts few traders.

---

## 🔍 STP vs. Other Protections

### What STP Is NOT:

**Position Limits**:
- Prevents you from getting too large overall
- Different purpose than STP

**Rate Limits**:
- Prevents too many orders per second
- Prevents system abuse

**Risk Checks**:
- Prevents excessive losses
- Margin and capital management

### What STP IS:

**Specific Protection**:
- Prevents trading with yourself
- Across multiple orders
- Same user, different sides
- Critical for algorithmic strategies operating on both sides simultaneously

---

## ⚙️ Implementation Details

### When STP Checks Occur:

**Order Lifecycle**:
1. Order arrives at exchange
2. Order enters matching engine
3. **STP check happens HERE** (before trade creation)
4. If same user_id detected:
   - Check STP mode
   - Take appropriate action
   - Continue or terminate matching
5. If allowed, create trade

### Performance Considerations:

**Fast Path**:
- User ID comparison is O(1)
- No performance impact on matching
- Critical for high-frequency systems

**Edge Cases**:
- Multiple orders at same price level
- Partial fills with STP
- Combining STP with other features (Post-Only, TIF)

---

## 🎓 Key Takeaways

1. **STP prevents self-trading** - Critical for market makers
2. **Six modes available** - Choose based on strategy
3. **Cancel Resting most common** - Keeps latest orders active
4. **Decrement Both most advanced** - Reduces exposure without fees
5. **Required for professional markets** - Competitive necessity

> 💡 **Pro Tip**: Most professional market makers use "Cancel Resting" as their default STP mode, with "Decrement Both" for end-of-day inventory management. Having both available gives maximum flexibility.

---

## 📚 Additional Resources

**Regulatory Context**:
- SEC Rules on Wash Trading
- MiFID II regulations in Europe
- Compliance requirements for exchanges

**Exchange Examples**:
- Coinbase Pro: Supports all 6 modes
- Binance: Supports Cancel Resting, Cancel Incoming, Cancel Both
- Kraken: Supports Cancel Newest, Cancel Oldest, Cancel Both

**Market Structure**:
- Understanding maker-taker economics
- Impact of self-trades on market data
- Professional market making strategies

---

*This guide covers Self-Trade Prevention, a critical feature for professional trading systems and exchange infrastructure.*
