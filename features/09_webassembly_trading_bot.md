# WebAssembly Trading Bot

## Purpose

**WebAssembly (WASM)** enables running Rust code in web browsers, serverless environments, and embedded systems at near-native speed. For trading applications, this opens up unique possibilities:

1. **Browser-Based Trading Tools**: Run backtests and analytics entirely client-side
2. **Serverless Edge Computing**: Deploy strategies to Cloudflare Workers, AWS Lambda@Edge
3. **Sandboxed Strategy Execution**: Run untrusted user strategies safely
4. **Cross-Platform Deployment**: Same code runs in browser, Node.js, and native
5. **Plugin Systems**: Let users write custom indicators in Rust, run in WASM sandbox

### Why WASM for Trading?

- **Security**: Perfect sandbox for running user-submitted strategies
- **Performance**: 50-80% of native speed (much faster than JavaScript)
- **Portability**: Deploy anywhere WASM runtime exists
- **Size**: Optimized WASM binaries are tiny (~50KB gzipped)

---

## Technology Stack

### Core Libraries

```toml
[dependencies]
# WASM bindings
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"  # Async support

# JavaScript interop
js-sys = "0.3"               # JavaScript standard library
web-sys = "0.3"              # Web APIs (DOM, fetch, WebSocket)

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde-wasm-bindgen = "0.6"   # Efficient JS ↔ Rust serialization

# Math and stats
rust_decimal = { version = "1.33", default-features = false }
chrono = { version = "0.4", default-features = false, features = ["wasmbind"] }

# Charting (optional)
plotters = "0.3"
plotters-canvas = "0.3"      # Render charts to HTML canvas

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Async runtime for WASM
wasm-bindgen-futures = "0.4"

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
panic = "abort"        # Smaller binary
strip = true           # Remove debug symbols
```

### Build Tools

```bash
# Install wasm-pack (build tool)
cargo install wasm-pack

# Install wasm-opt (optimizer)
cargo install wasm-opt

# Or use binaryen tools
npm install -g wasm-opt
```

---

## Implementation Guide

### Phase 1: Basic WASM Module

#### Step 1: Simple Trading Strategy Interface

```rust
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

#[wasm_bindgen]
pub struct TradingBot {
    strategy: Box<dyn Strategy>,
    balance: f64,
    position: f64,
}

#[wasm_bindgen]
impl TradingBot {
    #[wasm_bindgen(constructor)]
    pub fn new(initial_balance: f64) -> Self {
        // Set panic hook for better error messages in browser
        console_error_panic_hook::set_once();

        Self {
            strategy: Box::new(SimpleMovingAverageCrossover::new(10, 20)),
            balance: initial_balance,
            position: 0.0,
        }
    }

    /// Process new market data tick
    #[wasm_bindgen]
    pub fn on_tick(&mut self, price: f64, timestamp: f64) -> JsValue {
        let signal = self.strategy.calculate_signal(price);

        let action = match signal {
            Signal::Buy if self.position <= 0.0 => {
                self.buy(price);
                "BUY"
            }
            Signal::Sell if self.position >= 0.0 => {
                self.sell(price);
                "SELL"
            }
            _ => "HOLD",
        };

        // Return result as JavaScript object
        serde_wasm_bindgen::to_value(&TradeEvent {
            action: action.to_string(),
            price,
            balance: self.balance,
            position: self.position,
            timestamp,
        })
        .unwrap()
    }

    /// Get current portfolio value
    #[wasm_bindgen]
    pub fn get_portfolio_value(&self, current_price: f64) -> f64 {
        self.balance + (self.position * current_price)
    }

    fn buy(&mut self, price: f64) {
        let quantity = self.balance / price;
        self.position += quantity;
        self.balance = 0.0;
    }

    fn sell(&mut self, price: f64) {
        self.balance += self.position * price;
        self.position = 0.0;
    }
}

#[derive(Serialize, Deserialize)]
struct TradeEvent {
    action: String,
    price: f64,
    balance: f64,
    position: f64,
    timestamp: f64,
}

enum Signal {
    Buy,
    Sell,
    Hold,
}

trait Strategy {
    fn calculate_signal(&mut self, price: f64) -> Signal;
}

struct SimpleMovingAverageCrossover {
    short_window: Vec<f64>,
    long_window: Vec<f64>,
    short_period: usize,
    long_period: usize,
}

impl SimpleMovingAverageCrossover {
    fn new(short_period: usize, long_period: usize) -> Self {
        Self {
            short_window: Vec::new(),
            long_window: Vec::new(),
            short_period,
            long_period,
        }
    }

    fn sma(window: &[f64]) -> f64 {
        window.iter().sum::<f64>() / window.len() as f64
    }
}

impl Strategy for SimpleMovingAverageCrossover {
    fn calculate_signal(&mut self, price: f64) -> Signal {
        self.short_window.push(price);
        self.long_window.push(price);

        if self.short_window.len() > self.short_period {
            self.short_window.remove(0);
        }
        if self.long_window.len() > self.long_period {
            self.long_window.remove(0);
        }

        if self.short_window.len() < self.short_period
            || self.long_window.len() < self.long_period
        {
            return Signal::Hold;
        }

        let short_ma = Self::sma(&self.short_window);
        let long_ma = Self::sma(&self.long_window);

        if short_ma > long_ma {
            Signal::Buy
        } else if short_ma < long_ma {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}

// Utility function to log to browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Macro for easier console logging
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
```

---

#### Step 2: Build Configuration

Create `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]  # Dynamic library for WASM
```

Build the WASM module:

```bash
# For web (includes JS bindings)
wasm-pack build --target web --release

# For Node.js
wasm-pack build --target nodejs --release

# For bundlers (webpack, vite)
wasm-pack build --target bundler --release
```

This generates:
- `pkg/trading_bot_bg.wasm` (WASM binary)
- `pkg/trading_bot.js` (JavaScript bindings)
- `pkg/trading_bot.d.ts` (TypeScript definitions)

---

### Phase 3: JavaScript Integration

#### Step 3: HTML/JavaScript Frontend

```html
<!DOCTYPE html>
<html>
<head>
    <title>WASM Trading Bot</title>
    <script type="module">
        import init, { TradingBot } from './pkg/trading_bot.js';

        async function run() {
            // Initialize WASM module
            await init();

            // Create bot instance
            const bot = new TradingBot(10000.0);

            // Simulate live data stream
            let prices = [
                100, 102, 101, 103, 105, 104, 106, 108, 107, 109,
                111, 110, 112, 114, 113, 115, 117, 116, 118, 120
            ];

            prices.forEach((price, i) => {
                const event = bot.on_tick(price, Date.now() + i * 1000);
                console.log('Trade Event:', event);

                const portfolio_value = bot.get_portfolio_value(price);
                console.log('Portfolio Value:', portfolio_value);
            });
        }

        run();
    </script>
</head>
<body>
    <h1>WASM Trading Bot</h1>
    <div id="chart"></div>
</body>
</html>
```

---

### Phase 4: Advanced Features

#### Step 4: WebSocket Real-Time Data

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent};

#[wasm_bindgen]
pub struct LiveTradingBot {
    bot: TradingBot,
    ws: Option<WebSocket>,
}

#[wasm_bindgen]
impl LiveTradingBot {
    #[wasm_bindgen(constructor)]
    pub fn new(initial_balance: f64, ws_url: &str) -> Result<LiveTradingBot, JsValue> {
        let bot = TradingBot::new(initial_balance);

        let ws = WebSocket::new(ws_url)?;

        // Clone for closures
        let ws_clone = ws.clone();

        // OnMessage handler
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let json: String = txt.into();
                console_log!("Received: {}", json);

                // Parse and process tick data
                // (In real code, deserialize and call bot.on_tick)
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();  // Keep closure alive

        // OnError handler
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("WebSocket error: {:?}", e);
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        Ok(LiveTradingBot {
            bot,
            ws: Some(ws),
        })
    }

    #[wasm_bindgen]
    pub fn disconnect(&mut self) {
        if let Some(ws) = self.ws.take() {
            let _ = ws.close();
        }
    }
}
```

---

#### Step 5: Backtesting with Plotters

```rust
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub fn run_backtest(
    canvas_id: &str,
    prices: Vec<f64>,
    initial_balance: f64,
) -> Result<JsValue, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;

    let backend = CanvasBackend::new(&canvas).unwrap();
    let root = backend.into_drawing_area();
    root.fill(&WHITE)?;

    let mut bot = TradingBot::new(initial_balance);
    let mut portfolio_values = Vec::new();

    // Run backtest
    for (i, &price) in prices.iter().enumerate() {
        bot.on_tick(price, i as f64);
        let value = bot.get_portfolio_value(price);
        portfolio_values.push(value);
    }

    // Plot results
    let max_value = portfolio_values.iter().cloned().fold(f64::MIN, f64::max);
    let min_value = portfolio_values.iter().cloned().fold(f64::MAX, f64::min);

    let mut chart = ChartBuilder::on(&root)
        .caption("Backtest Results", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(0..prices.len(), min_value..max_value)?;

    chart.configure_mesh().draw()?;

    // Draw portfolio value line
    chart.draw_series(LineSeries::new(
        portfolio_values.iter().enumerate().map(|(i, &v)| (i, v)),
        &BLUE,
    ))?;

    root.present()?;

    Ok(serde_wasm_bindgen::to_value(&BacktestResult {
        final_value: *portfolio_values.last().unwrap(),
        max_value,
        min_value,
        returns: (portfolio_values.last().unwrap() - initial_balance) / initial_balance * 100.0,
    })?)
}

#[derive(Serialize)]
struct BacktestResult {
    final_value: f64,
    max_value: f64,
    min_value: f64,
    returns: f64,
}
```

---

### Phase 5: Optimization

#### Step 6: Size Optimization

```bash
# Build with maximum optimization
wasm-pack build --target web --release

# Further optimize with wasm-opt
wasm-opt -Oz -o pkg/trading_bot_bg_opt.wasm pkg/trading_bot_bg.wasm

# Check size
ls -lh pkg/*.wasm
```

**Typical sizes:**
- Debug build: ~500KB
- Release build: ~150KB
- `wasm-opt -Oz`: ~50KB
- Gzipped: ~15KB

#### Size Reduction Tips

```toml
[profile.release]
opt-level = "z"           # Optimize for size
lto = true                # Link-time optimization
codegen-units = 1         # Better optimization
panic = "abort"           # Remove unwinding code
strip = true              # Strip symbols

[dependencies]
# Use no-std where possible
rust_decimal = { version = "1.33", default-features = false }

# Avoid heavy dependencies
# ❌ chrono (large)
# ✅ time (smaller alternative)
```

---

## Deployment Options

### 1. Cloudflare Workers

```javascript
// worker.js
import init, { TradingBot } from './trading_bot.js';

let bot;

export default {
  async fetch(request, env, ctx) {
    // Initialize WASM once
    if (!bot) {
      await init();
      bot = new TradingBot(10000.0);
    }

    // Handle request
    const { price, timestamp } = await request.json();
    const event = bot.on_tick(price, timestamp);

    return new Response(JSON.stringify(event), {
      headers: { 'Content-Type': 'application/json' },
    });
  },
};
```

### 2. Vercel Edge Functions

```typescript
import { TradingBot } from './pkg/trading_bot';

export const config = {
  runtime: 'edge',
};

export default async function handler(request: Request) {
  const bot = new TradingBot(10000.0);
  // ... handle request
}
```

---

## Advantages

1. **Sandboxed Execution**
   - Safe to run untrusted user code
   - Memory isolation
   - No file system access

2. **Performance**
   - 50-80% of native speed
   - Much faster than JavaScript
   - SIMD support (with `wasm-simd`)

3. **Portability**
   - Run in browser, Node.js, Deno, edge workers
   - Cross-platform without recompilation

4. **Small Size**
   - 15-50KB gzipped (vs MB for native)
   - Fast loading over network

5. **Type Safety**
   - TypeScript definitions auto-generated
   - Catch errors at compile time

---

## Disadvantages

1. **Limited APIs**
   - No threads (yet - `wasm-threads` coming)
   - No file I/O in browser
   - No native networking

2. **Debugging Harder**
   - Source maps help but not perfect
   - Stack traces less clear

3. **Startup Cost**
   - WASM module must be loaded and initialized
   - ~10-50ms overhead

4. **No Direct DOM Access**
   - Must use `web-sys` bindings (verbose)

---

## Limitations

1. **No Multi-Threading**
   - Single-threaded execution (Web Workers workaround)

2. **Garbage Collection**
   - Rust manages its memory, but JS objects stay in JS heap
   - Can cause memory leaks if not careful

3. **Binary Size**
   - Still larger than equivalent JS (but faster)

4. **Browser Support**
   - All modern browsers, but IE11 needs polyfill

---

## Alternatives

### 1. **JavaScript**
- **Pros**: Native to browser, smaller code
- **Cons**: Slower, no type safety
- **Use**: Simple UIs, not compute-heavy

### 2. **AssemblyScript (TypeScript → WASM)**
- **Pros**: Easier for JS developers
- **Cons**: Less mature, smaller ecosystem
- **Use**: Moderate performance needs

### 3. **C/C++ with Emscripten**
- **Pros**: Massive existing codebases
- **Cons**: Manual memory management, harder to bind
- **Use**: Porting legacy code

### 4. **Go TinyGo**
- **Pros**: Simple concurrency model
- **Cons**: Larger binaries, slower than Rust
- **Use**: Network-heavy apps

---

## Recommended Path

1. **Week 1**: Simple WASM module with `wasm-bindgen`
2. **Week 2**: Add JavaScript interop
3. **Week 3**: Implement backtesting engine
4. **Week 4**: Add WebSocket live data
5. **Week 5**: Optimize binary size
6. **Week 6**: Deploy to Cloudflare Workers

---

## Further Reading

- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [Rust and WebAssembly Book](https://rustwasm.github.io/book/)
- [MDN: WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly)
- [Cloudflare Workers WASM](https://developers.cloudflare.com/workers/runtime-apis/webassembly/)
