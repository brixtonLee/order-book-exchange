# Risk Management Engine

## Purpose

A **Risk Management Engine** is a critical component of any trading system that monitors and controls financial risk in real-time. It ensures traders and algorithms don't exceed exposure limits, preventing catastrophic losses.

### Core Responsibilities

1. **Position Tracking**: Monitor current holdings across all assets
2. **Limit Enforcement**: Block trades that would violate risk limits
3. **Margin Calculation**: Ensure sufficient collateral for leveraged positions
4. **Portfolio Risk Metrics**: Calculate VaR (Value at Risk), Greeks, correlation exposure
5. **Pre-Trade Checks**: Validate orders before execution
6. **Real-Time Alerts**: Notify when approaching limits

This is where **Rust's type system shines** - we can encode risk rules at compile time, making it impossible to submit invalid orders.

---

## Technology Stack

### Core Libraries

```toml
[dependencies]
# Type-safe financial calculations
rust_decimal = "1.33"
rust_decimal_macros = "1.33"

# Date/time handling
chrono = "0.4"

# Advanced type patterns
derive_more = "0.99"     # Derive custom traits
typed-builder = "0.18"   # Builder pattern with compile-time validation

# Concurrent state management
dashmap = "6.0"          # Concurrent HashMap
parking_lot = "0.12"     # RwLock

# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Statistics for VaR calculation
statrs = "0.16"          # Statistical distributions
nalgebra = "0.32"        # Linear algebra for portfolio math
```

---

## Implementation Guide

### Phase 1: Type-Safe Position Management

#### Step 1: Phantom Types for Currency and Asset Classes

Phantom types prevent mixing incompatible units (e.g., adding USD to BTC).

```rust
use rust_decimal::Decimal;
use std::marker::PhantomData;

// Phantom type markers
pub struct USD;
pub struct BTC;
pub struct ETH;

// Type-safe quantity
#[derive(Debug, Clone, Copy)]
pub struct Quantity<T> {
    amount: Decimal,
    _phantom: PhantomData<T>,
}

impl<T> Quantity<T> {
    pub fn new(amount: Decimal) -> Self {
        Self {
            amount,
            _phantom: PhantomData,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }
}

// Can only add quantities of the same type
impl<T> std::ops::Add for Quantity<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.amount + rhs.amount)
    }
}

// Example usage:
fn example() {
    let btc_qty = Quantity::<BTC>::new(Decimal::from(10));
    let more_btc = Quantity::<BTC>::new(Decimal::from(5));

    let total = btc_qty + more_btc;  // ✅ Compiles

    let usd_qty = Quantity::<USD>::new(Decimal::from(1000));
    // let invalid = btc_qty + usd_qty;  // ❌ Compile error!
}
```

**Why phantom types?**
- Zero runtime cost (PhantomData is zero-sized)
- Prevents mixing currencies at compile time
- Self-documenting code

---

#### Step 2: Type-State Pattern for Order Validation

Encode order lifecycle in types - prevent sending unvalidated orders.

```rust
// States
pub struct Draft;
pub struct Validated;
pub struct RiskChecked;

// Order with state
pub struct Order<State> {
    symbol: String,
    quantity: Decimal,
    price: Decimal,
    _state: PhantomData<State>,
}

impl Order<Draft> {
    pub fn new(symbol: String, quantity: Decimal, price: Decimal) -> Self {
        Self {
            symbol,
            quantity,
            price,
            _state: PhantomData,
        }
    }

    // Transition to Validated state
    pub fn validate(self) -> Result<Order<Validated>, ValidationError> {
        if self.quantity <= Decimal::ZERO {
            return Err(ValidationError::InvalidQuantity);
        }
        if self.price <= Decimal::ZERO {
            return Err(ValidationError::InvalidPrice);
        }

        Ok(Order {
            symbol: self.symbol,
            quantity: self.quantity,
            price: self.price,
            _state: PhantomData,
        })
    }
}

impl Order<Validated> {
    // Only validated orders can be risk-checked
    pub fn check_risk(
        self,
        risk_engine: &RiskEngine,
    ) -> Result<Order<RiskChecked>, RiskError> {
        risk_engine.check_order(&self)?;

        Ok(Order {
            symbol: self.symbol,
            quantity: self.quantity,
            price: self.price,
            _state: PhantomData,
        })
    }
}

impl Order<RiskChecked> {
    // Only risk-checked orders can be submitted
    pub fn submit(self) -> OrderId {
        // Submit to exchange
        submit_to_exchange(&self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Quantity must be positive")]
    InvalidQuantity,
    #[error("Price must be positive")]
    InvalidPrice,
}

// Usage:
fn place_order(risk_engine: &RiskEngine) -> Result<OrderId, Box<dyn std::error::Error>> {
    let order = Order::new("BTCUSD".to_string(), dec!(0.5), dec!(50000));

    let validated = order.validate()?;
    let risk_checked = validated.check_risk(risk_engine)?;
    let order_id = risk_checked.submit();  // Type-safe!

    Ok(order_id)
}
```

**Type-state benefits:**
- Impossible to submit unvalidated orders (compile-time guarantee)
- State transitions are explicit
- No runtime overhead

---

### Phase 2: Risk Limit Enforcement

#### Step 3: Position Limits with Builder Pattern

```rust
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RiskLimits {
    /// Maximum notional value per symbol
    #[builder(default = Decimal::from(1_000_000))]
    pub max_position_notional: Decimal,

    /// Maximum number of contracts/shares
    #[builder(default = Decimal::from(10_000))]
    pub max_position_size: Decimal,

    /// Maximum portfolio value
    #[builder(default = Decimal::from(10_000_000))]
    pub max_portfolio_value: Decimal,

    /// Maximum daily loss (stop trading if hit)
    #[builder(default = Decimal::from(100_000))]
    pub max_daily_loss: Decimal,

    /// Maximum leverage ratio (e.g., 10 = 10x)
    #[builder(default = Decimal::from(10))]
    pub max_leverage: Decimal,
}

// Usage with compile-time validation:
fn create_limits() -> RiskLimits {
    RiskLimits::builder()
        .max_position_notional(dec!(5_000_000))
        .max_leverage(dec!(5))
        .build()  // Other fields use defaults
}
```

---

#### Step 4: Position Tracker with Interior Mutability

```rust
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,  // Positive = long, Negative = short
    pub avg_entry_price: Decimal,
    pub realized_pnl: Decimal,
}

impl Position {
    pub fn notional_value(&self, current_price: Decimal) -> Decimal {
        self.quantity.abs() * current_price
    }

    pub fn unrealized_pnl(&self, current_price: Decimal) -> Decimal {
        (current_price - self.avg_entry_price) * self.quantity
    }
}

pub struct PositionTracker {
    // Concurrent map of symbol -> position
    positions: Arc<DashMap<String, Position>>,

    // Account-level state
    cash_balance: Arc<RwLock<Decimal>>,
    daily_pnl: Arc<RwLock<Decimal>>,
}

impl PositionTracker {
    pub fn new(initial_cash: Decimal) -> Self {
        Self {
            positions: Arc::new(DashMap::new()),
            cash_balance: Arc::new(RwLock::new(initial_cash)),
            daily_pnl: Arc::new(RwLock::new(Decimal::ZERO)),
        }
    }

    pub fn update_position(
        &self,
        symbol: &str,
        trade_quantity: Decimal,
        trade_price: Decimal,
    ) {
        self.positions
            .entry(symbol.to_string())
            .and_modify(|pos| {
                let old_qty = pos.quantity;
                let new_qty = old_qty + trade_quantity;

                if old_qty.signum() == new_qty.signum() {
                    // Adding to position - update average price
                    let old_value = old_qty * pos.avg_entry_price;
                    let trade_value = trade_quantity * trade_price;
                    pos.avg_entry_price = (old_value + trade_value) / new_qty;
                    pos.quantity = new_qty;
                } else if new_qty.abs() < old_qty.abs() {
                    // Reducing position - realize PnL
                    let pnl = (trade_price - pos.avg_entry_price) * trade_quantity.abs();
                    pos.realized_pnl += pnl;
                    pos.quantity = new_qty;

                    let mut daily_pnl = self.daily_pnl.write();
                    *daily_pnl += pnl;
                } else {
                    // Flipping position - realize full PnL and start new position
                    let close_pnl = (trade_price - pos.avg_entry_price) * old_qty;
                    pos.realized_pnl += close_pnl;
                    pos.quantity = new_qty;
                    pos.avg_entry_price = trade_price;

                    let mut daily_pnl = self.daily_pnl.write();
                    *daily_pnl += close_pnl;
                }
            })
            .or_insert_with(|| Position {
                symbol: symbol.to_string(),
                quantity: trade_quantity,
                avg_entry_price: trade_price,
                realized_pnl: Decimal::ZERO,
            });
    }

    pub fn get_position(&self, symbol: &str) -> Option<Position> {
        self.positions.get(symbol).map(|entry| entry.clone())
    }

    pub fn total_equity(&self, market_prices: &DashMap<String, Decimal>) -> Decimal {
        let cash = *self.cash_balance.read();
        let unrealized_pnl: Decimal = self.positions
            .iter()
            .filter_map(|entry| {
                let pos = entry.value();
                market_prices.get(&pos.symbol).map(|price| {
                    pos.unrealized_pnl(*price)
                })
            })
            .sum();

        cash + unrealized_pnl
    }
}
```

**Design choices:**
- **DashMap**: Lock-free reads for high concurrency
- **RwLock**: Multiple readers, single writer for account-level state
- **Interior mutability**: Shared ownership with mutation

---

### Phase 3: Pre-Trade Risk Checks

#### Step 5: Risk Engine

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiskError {
    #[error("Position limit exceeded: {symbol} would be {new_notional}, limit is {limit}")]
    PositionLimitExceeded {
        symbol: String,
        new_notional: Decimal,
        limit: Decimal,
    },

    #[error("Daily loss limit hit: {current_loss}, limit is {limit}")]
    DailyLossLimitHit {
        current_loss: Decimal,
        limit: Decimal,
    },

    #[error("Insufficient margin: need {required}, have {available}")]
    InsufficientMargin {
        required: Decimal,
        available: Decimal,
    },

    #[error("Leverage too high: {leverage}x, max is {max_leverage}x")]
    ExcessiveLeverage {
        leverage: Decimal,
        max_leverage: Decimal,
    },
}

pub struct RiskEngine {
    limits: RiskLimits,
    positions: Arc<PositionTracker>,
    market_prices: Arc<DashMap<String, Decimal>>,
}

impl RiskEngine {
    pub fn new(
        limits: RiskLimits,
        positions: Arc<PositionTracker>,
        market_prices: Arc<DashMap<String, Decimal>>,
    ) -> Self {
        Self {
            limits,
            positions,
            market_prices,
        }
    }

    pub fn check_order<State>(
        &self,
        order: &Order<State>,
    ) -> Result<(), RiskError> {
        // 1. Check daily loss limit
        self.check_daily_loss()?;

        // 2. Check position limit
        self.check_position_limit(order)?;

        // 3. Check leverage
        self.check_leverage(order)?;

        Ok(())
    }

    fn check_daily_loss(&self) -> Result<(), RiskError> {
        let daily_pnl = *self.positions.daily_pnl.read();

        if daily_pnl < -self.limits.max_daily_loss {
            return Err(RiskError::DailyLossLimitHit {
                current_loss: daily_pnl.abs(),
                limit: self.limits.max_daily_loss,
            });
        }

        Ok(())
    }

    fn check_position_limit<State>(
        &self,
        order: &Order<State>,
    ) -> Result<(), RiskError> {
        let current_pos = self.positions
            .get_position(&order.symbol)
            .map(|p| p.quantity)
            .unwrap_or(Decimal::ZERO);

        let new_qty = current_pos + order.quantity;
        let new_notional = new_qty.abs() * order.price;

        if new_notional > self.limits.max_position_notional {
            return Err(RiskError::PositionLimitExceeded {
                symbol: order.symbol.clone(),
                new_notional,
                limit: self.limits.max_position_notional,
            });
        }

        Ok(())
    }

    fn check_leverage<State>(
        &self,
        order: &Order<State>,
    ) -> Result<(), RiskError> {
        let total_equity = self.positions.total_equity(&self.market_prices);

        // Calculate what total notional would be after order
        let mut total_notional = Decimal::ZERO;
        for entry in self.positions.positions.iter() {
            if let Some(price) = self.market_prices.get(&entry.key().clone()) {
                total_notional += entry.value().quantity.abs() * *price;
            }
        }

        // Add this order's notional
        total_notional += order.quantity.abs() * order.price;

        let leverage = total_notional / total_equity;

        if leverage > self.limits.max_leverage {
            return Err(RiskError::ExcessiveLeverage {
                leverage,
                max_leverage: self.limits.max_leverage,
            });
        }

        Ok(())
    }
}
```

---

### Phase 4: Portfolio Risk Metrics (VaR)

#### Step 6: Value at Risk Calculation

```rust
use statrs::distribution::{Normal, ContinuousCDF};
use nalgebra::{DMatrix, DVector};

pub struct PortfolioRiskCalculator {
    confidence_level: f64,  // e.g., 0.95 for 95% VaR
}

impl PortfolioRiskCalculator {
    pub fn new(confidence_level: f64) -> Self {
        Self { confidence_level }
    }

    /// Calculate parametric VaR using variance-covariance method
    pub fn calculate_var(
        &self,
        positions: &PositionTracker,
        returns_data: &DMatrix<f64>,  // Historical returns matrix
    ) -> Decimal {
        // 1. Get current portfolio weights
        let weights = self.calculate_weights(positions);

        // 2. Calculate covariance matrix
        let cov_matrix = self.covariance_matrix(returns_data);

        // 3. Portfolio variance = w^T * Σ * w
        let portfolio_variance = weights.transpose() * &cov_matrix * &weights;
        let portfolio_std = portfolio_variance[(0, 0)].sqrt();

        // 4. VaR = z-score * σ * portfolio_value
        let normal = Normal::new(0.0, 1.0).unwrap();
        let z_score = normal.inverse_cdf(1.0 - self.confidence_level);

        let total_value = self.portfolio_value(positions);
        let var = z_score.abs() * portfolio_std * total_value;

        Decimal::from_f64_retain(var).unwrap_or(Decimal::ZERO)
    }

    fn calculate_weights(&self, positions: &PositionTracker) -> DVector<f64> {
        // Convert positions to weight vector
        // Implementation omitted for brevity
        DVector::zeros(10)
    }

    fn covariance_matrix(&self, returns: &DMatrix<f64>) -> DMatrix<f64> {
        let n = returns.nrows() as f64;
        let mean = returns.column_mean();

        let centered = returns.clone() - DMatrix::from_rows(&vec![mean; returns.nrows()]);
        let cov = (centered.transpose() * centered) / n;

        cov
    }

    fn portfolio_value(&self, positions: &PositionTracker) -> f64 {
        // Implementation omitted
        1_000_000.0
    }
}
```

**VaR interpretation:**
- 95% VaR of $100,000 = "95% confident we won't lose more than $100k in one day"

---

## Advantages

1. **Compile-Time Safety**
   - Phantom types prevent currency mix-ups
   - Type-state pattern ensures proper order flow
   - Impossible to bypass risk checks

2. **Performance**
   - Lock-free position lookups with DashMap
   - Zero-cost abstractions (phantom types)
   - Efficient concurrent access

3. **Flexibility**
   - Easy to add custom risk checks
   - Configurable limits per account/strategy

4. **Auditability**
   - All risk violations logged with context
   - Type-safe errors make debugging easy

---

## Disadvantages

1. **Complexity**
   - Type-level programming has steep learning curve
   - More boilerplate than simple approach

2. **Rigidity**
   - Type-state pattern makes runtime flexibility harder
   - Changing states requires code changes

3. **Limited Dynamic Rules**
   - Compile-time checks can't encode all business rules
   - Some checks must remain runtime

---

## Limitations

1. **Single-Process Only**
   - No distributed risk management
   - All state in-memory

2. **Historical Data Required**
   - VaR calculation needs past returns
   - Cold start problem

3. **Market Risk Only**
   - Doesn't model counterparty risk, liquidity risk
   - No stress testing built-in

4. **Static Correlation Assumptions**
   - VaR assumes correlations don't change
   - Breaks down in crisis (correlations → 1)

---

## Alternatives

### 1. **Buy Commercial RMS**
- **Vendors**: Bloomberg AIM, Calypso, Murex
- **Pros**: Battle-tested, regulatory compliance built-in
- **Cons**: Very expensive ($100k-$1M+), slow

### 2. **Open Source: QuantLib**
- **Pros**: Free, extensive financial models
- **Cons**: C++, complex API, steep learning curve

### 3. **Python: PyRisk / Riskfolio-Lib**
- **Pros**: Easy prototyping, rich ecosystem
- **Cons**: Slower, no compile-time guarantees

### 4. **Event Sourcing Pattern**
- Store all trades as immutable events
- Rebuild positions from event log
- **Pros**: Perfect audit trail, time-travel debugging
- **Cons**: More complex infrastructure

---

## Recommended Implementation Order

1. **Week 1**: Type-safe positions with phantom types
2. **Week 2**: Position tracker with DashMap
3. **Week 3**: Basic risk checks (position limits, daily loss)
4. **Week 4**: Type-state pattern for order flow
5. **Week 5**: VaR calculation and portfolio metrics
6. **Week 6**: Stress testing and scenario analysis

---

## Further Reading

- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Type-Driven Development in Rust](https://willcrichton.net/rust-api-type-patterns/)
- [Financial Risk Management Basics](https://www.investopedia.com/terms/r/riskmanagement.asp)
- [Value at Risk (VaR)](https://www.investopedia.com/terms/v/var.asp)
