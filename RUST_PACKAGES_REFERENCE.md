# Rust Packages Reference Guide for Exchange Development

**Complete reference of essential Rust packages for building high-performance trading systems**

---

## Table of Contents

1. [Web Framework Layer](#1-web-framework-layer)
   - [Axum](#axum)
   - [Tower](#tower)
   - [Tower-HTTP](#tower-http)
2. [Async Runtime & Concurrency](#2-async-runtime--concurrency)
   - [Tokio](#tokio)
   - [Tokio-util](#tokio-util)
   - [Rayon](#rayon)
   - [Crossbeam](#crossbeam)
3. [Serialization & Data](#3-serialization--data)
   - [Serde](#serde)
   - [Serde_json](#serde_json)
   - [Bytes](#bytes)
   - [Bincode](#bincode)
4. [Observability & Metrics](#4-observability--metrics)
   - [Prometheus](#prometheus)
   - [Tracing](#tracing)
   - [Tracing-subscriber](#tracing-subscriber)
   - [Metrics](#metrics)
5. [Database & Persistence](#5-database--persistence)
   - [SQLx](#sqlx)
   - [Redis](#redis)
   - [RocksDB](#rocksdb)
6. [Error Handling](#6-error-handling)
   - [Thiserror](#thiserror)
   - [Anyhow](#anyhow)
7. [Time & Decimals](#7-time--decimals)
   - [Chrono](#chrono)
   - [Rust_decimal](#rust_decimal)
8. [Network & Protocol](#8-network--protocol)
   - [Hyper](#hyper)
   - [Reqwest](#reqwest)
   - [Tungstenite](#tungstenite)
9. [Utilities](#9-utilities)
   - [Uuid](#uuid)
   - [DashMap](#dashmap)
   - [Once_cell](#once_cell)
   - [Parking_lot](#parking_lot)
10. [API Documentation](#10-api-documentation)
    - [Utoipa](#utoipa)

---

## 1. Web Framework Layer

### Axum

**Purpose**: Ergonomic, modular web framework built on Tokio and Tower with compile-time correctness.

**Core Concepts**:
- Handler functions with type-safe extractors
- State sharing via `Arc`
- Middleware via Tower layers
- Native WebSocket support

#### Popular Functions & Types

```rust
use axum::{
    Router,           // Main routing struct
    routing::{get, post, delete},  // HTTP method helpers
    extract::{Path, Query, State, Json, WebSocketUpgrade},
    response::{IntoResponse, Response},
    middleware,       // Middleware utilities
    serve,           // Server runner
};

// 1. Router::new() - Create new router
let app = Router::new();

// 2. route(path, method_router) - Add routes
let app = Router::new()
    .route("/api/orders", post(create_order))
    .route("/api/orders/:symbol/:id", get(get_order))
    .route("/api/orders/:symbol/:id", delete(cancel_order));

// 3. nest(prefix, router) - Mount sub-routers
let api_v1 = Router::new()
    .route("/orders", post(create_order));
let app = Router::new()
    .nest("/api/v1", api_v1);

// 4. with_state(state) - Share application state
#[derive(Clone)]
struct AppState {
    engine: Arc<OrderBookEngine>,
}
let app = Router::new()
    .route("/orders", post(create_order))
    .with_state(AppState { engine });

// 5. layer(layer) - Add middleware
use tower_http::cors::CorsLayer;
let app = Router::new()
    .route("/api/orders", post(create_order))
    .layer(CorsLayer::permissive());

// 6. merge(router) - Combine routers
let public_routes = Router::new().route("/health", get(health_check));
let api_routes = Router::new().route("/orders", post(create_order));
let app = public_routes.merge(api_routes);

// 7. fallback(handler) - Handle 404s
let app = Router::new()
    .route("/api/orders", post(create_order))
    .fallback(not_found_handler);
```

#### Extractors (Request Data)

```rust
use axum::extract::*;

// 1. Path<T> - Extract path parameters
async fn get_order(
    Path((symbol, order_id)): Path<(String, String)>
) -> Response {
    // symbol and order_id extracted from /orders/:symbol/:order_id
}

// 2. Query<T> - Extract query parameters
#[derive(Deserialize)]
struct Pagination { limit: Option<usize>, offset: Option<usize> }
async fn list_trades(Query(params): Query<Pagination>) -> Response {
    let limit = params.limit.unwrap_or(100);
}

// 3. Json<T> - Parse JSON body
#[derive(Deserialize)]
struct CreateOrderRequest { symbol: String, price: Decimal }
async fn create_order(Json(req): Json<CreateOrderRequest>) -> Response {
    // req is deserialized from JSON body
}

// 4. State<T> - Access shared state
async fn create_order(
    State(engine): State<Arc<OrderBookEngine>>,
    Json(req): Json<CreateOrderRequest>
) -> Response {
    engine.submit_order(...);
}

// 5. WebSocketUpgrade - Upgrade to WebSocket
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<Arc<Broadcaster>>
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

// 6. Extension<T> - Extract from request extensions
async fn handler(Extension(user_id): Extension<UserId>) -> Response {
    // user_id added by middleware
}

// 7. headers::HeaderMap - Access request headers
use axum::http::HeaderMap;
async fn handler(headers: HeaderMap) -> Response {
    if let Some(auth) = headers.get("authorization") {
        // Handle auth header
    }
}
```

#### Response Types

```rust
use axum::response::*;
use axum::http::StatusCode;

// 1. Json<T> - Return JSON response
async fn get_order() -> Json<OrderResponse> {
    Json(OrderResponse { id: "123".into(), ... })
}

// 2. (StatusCode, Json<T>) - Custom status with JSON
async fn create_order() -> (StatusCode, Json<OrderResponse>) {
    (StatusCode::CREATED, Json(order_response))
}

// 3. Result<Json<T>, (StatusCode, String)> - Error handling
async fn get_order() -> Result<Json<OrderResponse>, (StatusCode, String)> {
    let order = engine.get_order(id)
        .ok_or((StatusCode::NOT_FOUND, "Order not found".into()))?;
    Ok(Json(order))
}

// 4. impl IntoResponse - Custom response types
struct ApiError(String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

// 5. Response - Full control
use axum::http::Response as HttpResponse;
async fn custom() -> HttpResponse<Body> {
    HttpResponse::builder()
        .status(200)
        .header("X-Custom", "value")
        .body(Body::from("data"))
        .unwrap()
}
```

#### WebSocket Support

```rust
use axum::extract::ws::{WebSocket, Message};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<Arc<Broadcaster>>
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

async fn handle_socket(mut socket: WebSocket, broadcaster: Arc<Broadcaster>) {
    // 1. send() - Send message
    socket.send(Message::Text("Hello".into())).await.ok();

    // 2. recv() - Receive message
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                // Handle text message
            }
            Message::Binary(data) => {
                // Handle binary message
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // 3. close() - Close connection
    socket.close().await.ok();
}
```

#### Example: Complete Exchange API

```rust
use axum::{Router, routing::{get, post, delete}, extract::{Path, Query, State, Json}};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    engine: Arc<OrderBookEngine>,
    broadcaster: Arc<Broadcaster>,
}

async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>
) -> Result<Json<OrderResponse>, (StatusCode, String)> {
    let order = state.engine.submit_order(req.into())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(order.into()))
}

async fn get_order_book(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(params): Query<DepthParams>
) -> Json<OrderBookSnapshot> {
    let book = state.engine.get_order_book(&symbol, params.depth);
    Json(book)
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/orders", post(create_order))
        .route("/api/v1/orders/:symbol/:id", get(get_order).delete(cancel_order))
        .route("/api/v1/orderbook/:symbol", get(get_order_book))
        .route("/ws", get(ws_handler))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() {
    let state = AppState {
        engine: Arc::new(OrderBookEngine::new()),
        broadcaster: Arc::new(Broadcaster::new()),
    };

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Best Practices**:
- Use `Arc` for shared state (cheap clones)
- Prefer extractors over manual request parsing
- Implement `IntoResponse` for custom error types
- Use middleware for cross-cutting concerns (auth, logging)
- Keep handlers thin - business logic in services

---

### Tower

**Purpose**: Modular components for building robust networking clients and servers. Foundation for Axum middleware.

**Core Concepts**:
- `Service` trait: Transform requests into responses
- `Layer` trait: Wrap services with middleware
- Composable middleware stack
- Backpressure and load management

#### Popular Functions & Types

```rust
use tower::{
    Service,           // Core service trait
    ServiceBuilder,    // Compose middleware layers
    ServiceExt,        // Service extension methods
    Layer,            // Middleware layer trait
    timeout::Timeout, // Timeout middleware
    limit::RateLimit, // Rate limiting
    buffer::Buffer,   // Request buffering
    load_shed::LoadShed, // Load shedding
};

// 1. Service trait - Core abstraction
pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    fn call(&mut self, req: Request) -> Self::Future;
}

// 2. ServiceBuilder - Compose layers
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .rate_limit(100, Duration::from_secs(1))
    .load_shed()
    .buffer(1024)
    .service(my_service);

// 3. ServiceExt::map_response() - Transform responses
use tower::ServiceExt;
let service = my_service.map_response(|resp| {
    // Transform response
    resp
});

// 4. ServiceExt::map_err() - Transform errors
let service = my_service.map_err(|err| {
    eprintln!("Error: {}", err);
    err
});

// 5. ServiceExt::and_then() - Chain async operations
let service = my_service.and_then(|resp| async move {
    // Async transformation
    Ok(resp)
});

// 6. ServiceExt::retry() - Add retry logic
use tower::retry::{Policy, RetryLayer};
let service = ServiceBuilder::new()
    .retry(MyRetryPolicy)
    .service(my_service);
```

#### Middleware Layers

```rust
use tower::{timeout::TimeoutLayer, limit::RateLimitLayer};
use std::time::Duration;

// 1. TimeoutLayer - Request timeout
let layer = TimeoutLayer::new(Duration::from_secs(30));

// 2. RateLimitLayer - Rate limiting
let layer = RateLimitLayer::new(
    100,  // 100 requests
    Duration::from_secs(1)  // per second
);

// 3. BufferLayer - Request buffering
use tower::buffer::BufferLayer;
let layer = BufferLayer::new(1024);  // Buffer up to 1024 requests

// 4. LoadShedLayer - Drop requests under load
use tower::load_shed::LoadShedLayer;
let layer = LoadShedLayer::new();

// 5. ConcurrencyLimitLayer - Limit concurrent requests
use tower::limit::ConcurrencyLimitLayer;
let layer = ConcurrencyLimitLayer::new(100);

// 6. Custom Layer - Implement your own
use tower::Layer;
struct MyLayer;
impl<S> Layer<S> for MyLayer {
    type Service = MyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyService { inner }
    }
}
```

#### Example: Building Middleware Stack for Exchange

```rust
use tower::{ServiceBuilder, timeout::TimeoutLayer, limit::RateLimitLayer};
use std::time::Duration;

// Custom metrics middleware
struct MetricsLayer;
impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner, counter: Arc::new(AtomicU64::new(0)) }
    }
}

struct MetricsService<S> {
    inner: S,
    counter: Arc<AtomicU64>,
}

impl<S, Request> Service<Request> for MetricsService<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.counter.fetch_add(1, Ordering::Relaxed);
        self.inner.call(req)
    }
}

// Build complete middleware stack
fn build_service<S>(inner: S) -> impl Service<Request, Response = Response, Error = BoxError>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send,
{
    ServiceBuilder::new()
        // Outer layers execute first
        .layer(MetricsLayer)                          // Track request count
        .timeout(Duration::from_secs(30))              // 30s timeout
        .rate_limit(1000, Duration::from_secs(1))      // 1000 req/s
        .concurrency_limit(500)                         // Max 500 concurrent
        .load_shed()                                    // Drop under pressure
        .buffer(2048)                                   // Buffer 2048 requests
        .service(inner)
        // Inner service executes last
}
```

**Best Practices**:
- Order matters: outer layers wrap inner layers
- Use `ServiceBuilder` for clean composition
- Implement backpressure via `poll_ready`
- Keep middleware stateless when possible
- Use `Buffer` to smooth traffic spikes

---

### Tower-HTTP

**Purpose**: HTTP-specific middleware for Tower and Axum applications.

**Core Concepts**:
- CORS handling
- Compression (gzip, brotli)
- Tracing and metrics
- Request ID propagation

#### Popular Middleware

```rust
use tower_http::{
    cors::{CorsLayer, Any},
    compression::CompressionLayer,
    trace::TraceLayer,
    request_id::{MakeRequestId, RequestId, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
    validate_request::ValidateRequestHeaderLayer,
};

// 1. CorsLayer - CORS configuration
use tower_http::cors::{CorsLayer, AllowOrigin};
use axum::http::{Method, HeaderValue};

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact("https://example.com".parse().unwrap()))
    .allow_methods([Method::GET, Method::POST, Method::DELETE])
    .allow_headers(Any)
    .max_age(Duration::from_secs(3600));

// Permissive CORS (development)
let cors = CorsLayer::permissive();

// 2. CompressionLayer - Response compression
let compression = CompressionLayer::new()
    .gzip(true)
    .br(true)      // Brotli
    .deflate(true);

// 3. TraceLayer - HTTP tracing
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};

let trace = TraceLayer::new_for_http()
    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
    .on_response(DefaultOnResponse::new().level(Level::INFO));

// 4. RequestId - Request tracking
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

struct MyMakeRequestId;
impl MakeRequestId for MyMakeRequestId {
    fn make_request_id<B>(&mut self, request: &axum::http::Request<B>) -> Option<RequestId> {
        let id = uuid::Uuid::new_v4().to_string().parse().ok()?;
        Some(RequestId::new(id))
    }
}

let request_id_layer = SetRequestIdLayer::new(
    HeaderName::from_static("x-request-id"),
    MyMakeRequestId
);
let propagate_layer = PropagateRequestIdLayer::new(
    HeaderName::from_static("x-request-id")
);

// 5. TimeoutLayer - Request timeout
let timeout = TimeoutLayer::new(Duration::from_secs(30));

// 6. RequestBodyLimitLayer - Limit body size
let body_limit = RequestBodyLimitLayer::new(1024 * 1024 * 5); // 5MB

// 7. ValidateRequestHeaderLayer - Require headers
use tower_http::validate_request::ValidateRequestHeaderLayer;
let auth_layer = ValidateRequestHeaderLayer::bearer("secret-token");
```

#### Example: Production Middleware Stack

```rust
use axum::Router;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
    request_id::{SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestUuid},
    limit::RequestBodyLimitLayer,
};
use tower::ServiceBuilder;

fn create_production_router() -> Router {
    let middleware = ServiceBuilder::new()
        // Request ID generation and propagation
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        // Tracing with request IDs
        .layer(TraceLayer::new_for_http())
        // Security
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB limit
        // Performance
        .layer(CompressionLayer::new())
        // CORS for browser clients
        .layer(
            CorsLayer::new()
                .allow_origin("https://trading-ui.example.com".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        );

    Router::new()
        .route("/api/v1/orders", post(create_order))
        .route("/api/v1/orderbook/:symbol", get(get_orderbook))
        .layer(middleware)
}
```

**Best Practices**:
- Always set request body limits to prevent DoS
- Use request IDs for distributed tracing
- Enable compression for JSON responses
- Configure CORS restrictively in production
- Place tracing layer early to capture all requests

---

## 2. Async Runtime & Concurrency

### Tokio

**Purpose**: Async runtime for writing reliable network applications. Powers Axum, Hyper, and most async Rust.

**Core Concepts**:
- Runtime executor for async tasks
- Green threads (tasks) with work-stealing scheduler
- Async I/O (TCP, UDP, files)
- Synchronization primitives
- Timers and intervals

#### Popular Functions & Types

```rust
use tokio::{
    runtime::{Runtime, Builder},
    task::{spawn, spawn_blocking, JoinHandle},
    sync::{mpsc, broadcast, watch, oneshot, Mutex, RwLock, Semaphore},
    time::{sleep, interval, timeout, Duration, Instant},
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    join,
};

// 1. #[tokio::main] - Main async runtime
#[tokio::main]
async fn main() {
    // Entry point for async application
}

// Equivalent to:
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // async code
    });
}

// 2. Runtime configuration
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)           // 8 worker threads
    .thread_name("exchange-worker")
    .enable_all()                 // Enable I/O and time
    .build()
    .unwrap();

// Single-threaded runtime (testing/simple apps)
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();

// 3. spawn() - Spawn new task
let handle: JoinHandle<String> = tokio::spawn(async {
    // Task runs concurrently
    "result".to_string()
});
let result = handle.await.unwrap();

// 4. spawn_blocking() - Run blocking code
let result = tokio::task::spawn_blocking(|| {
    // CPU-intensive or blocking operation
    std::thread::sleep(Duration::from_secs(1));
    "done"
}).await.unwrap();

// 5. JoinHandle - Wait for task completion
let handle = tokio::spawn(async { 42 });
let value = handle.await.unwrap();  // Wait and unwrap result
```

#### Channels (Inter-task Communication)

```rust
use tokio::sync::{mpsc, broadcast, watch, oneshot};

// 1. mpsc - Multi-producer, single-consumer (most common)
let (tx, mut rx) = mpsc::channel::<String>(100);  // Buffer 100 messages

// Send from multiple tasks
let tx1 = tx.clone();
tokio::spawn(async move {
    tx1.send("message".to_string()).await.ok();
});

// Receive from one task
while let Some(msg) = rx.recv().await {
    println!("Got: {}", msg);
}

// Unbounded channel (use with caution)
let (tx, mut rx) = mpsc::unbounded_channel::<String>();
tx.send("message".to_string()).ok();

// 2. broadcast - Multi-producer, multi-consumer
let (tx, mut rx1) = broadcast::channel::<String>(100);
let mut rx2 = tx.subscribe();  // Additional receivers

tokio::spawn(async move {
    while let Ok(msg) = rx1.recv().await {
        println!("Receiver 1: {}", msg);
    }
});

tokio::spawn(async move {
    while let Ok(msg) = rx2.recv().await {
        println!("Receiver 2: {}", msg);
    }
});

tx.send("broadcast message".to_string()).ok();

// 3. watch - Single value with multiple subscribers (state watching)
let (tx, mut rx) = watch::channel("initial");

tokio::spawn(async move {
    while rx.changed().await.is_ok() {
        let value = rx.borrow();
        println!("Value changed to: {}", *value);
    }
});

tx.send("updated").ok();

// 4. oneshot - One-time send (request/response pattern)
let (tx, rx) = oneshot::channel::<String>();

tokio::spawn(async move {
    let result = perform_calculation().await;
    tx.send(result).ok();
});

let result = rx.await.unwrap();
```

#### Synchronization Primitives

```rust
use tokio::sync::{Mutex, RwLock, Semaphore, Barrier, Notify};
use std::sync::Arc;

// 1. Mutex - Mutual exclusion (async)
let mutex = Arc::new(Mutex::new(0));

let mutex_clone = mutex.clone();
tokio::spawn(async move {
    let mut guard = mutex_clone.lock().await;
    *guard += 1;
    // Lock released when guard drops
});

// 2. RwLock - Read-write lock (many readers, one writer)
let rwlock = Arc::new(RwLock::new(HashMap::new()));

// Multiple readers
let read_guard = rwlock.read().await;
let value = read_guard.get("key");

// Single writer
let mut write_guard = rwlock.write().await;
write_guard.insert("key", "value");

// 3. Semaphore - Limit concurrent access
let semaphore = Arc::new(Semaphore::new(5)); // Allow 5 concurrent

let permit = semaphore.acquire().await.unwrap();
// Do work while holding permit
drop(permit); // Release permit

// 4. Barrier - Wait for N tasks
let barrier = Arc::new(Barrier::new(3));

for i in 0..3 {
    let barrier_clone = barrier.clone();
    tokio::spawn(async move {
        println!("Task {} before barrier", i);
        barrier_clone.wait().await;
        println!("Task {} after barrier", i);
    });
}

// 5. Notify - Wake up waiting tasks
let notify = Arc::new(Notify::new());

let notify_clone = notify.clone();
tokio::spawn(async move {
    notify_clone.notified().await;
    println!("Notified!");
});

notify.notify_one();  // Wake one waiter
// notify.notify_waiters();  // Wake all waiters
```

#### Time Operations

```rust
use tokio::time::{sleep, interval, timeout, Duration, Instant};

// 1. sleep() - Async delay
tokio::time::sleep(Duration::from_secs(1)).await;

// 2. interval() - Periodic ticks
let mut interval = tokio::time::interval(Duration::from_millis(100));
loop {
    interval.tick().await;
    println!("Tick!");
}

// 3. timeout() - Add timeout to future
use tokio::time::timeout;

let result = timeout(Duration::from_secs(5), slow_operation()).await;
match result {
    Ok(value) => println!("Completed: {:?}", value),
    Err(_) => println!("Timeout!"),
}

// 4. Instant - Timing measurements
let start = Instant::now();
perform_operation().await;
let elapsed = start.elapsed();
println!("Took: {:?}", elapsed);

// 5. sleep_until() - Sleep until specific time
let deadline = Instant::now() + Duration::from_secs(10);
tokio::time::sleep_until(deadline).await;
```

#### Control Flow Macros

```rust
use tokio::{select, join, try_join};

// 1. select! - Wait for first completion (race)
let mut rx1 = mpsc::channel(10).1;
let mut rx2 = mpsc::channel(10).1;

tokio::select! {
    msg = rx1.recv() => {
        println!("Got from rx1: {:?}", msg);
    }
    msg = rx2.recv() => {
        println!("Got from rx2: {:?}", msg);
    }
    _ = tokio::time::sleep(Duration::from_secs(1)) => {
        println!("Timeout");
    }
}

// 2. join! - Wait for all to complete (parallel)
let (result1, result2, result3) = tokio::join!(
    fetch_data_1(),
    fetch_data_2(),
    fetch_data_3()
);

// 3. try_join! - Wait for all, short-circuit on error
let result = tokio::try_join!(
    fallible_operation_1(),
    fallible_operation_2(),
    fallible_operation_3()
);
match result {
    Ok((r1, r2, r3)) => println!("All succeeded"),
    Err(e) => println!("One failed: {}", e),
}

// 4. pin! - Pin value to stack
use tokio::pin;
let fut = async { /* ... */ };
pin!(fut);
// Now fut can be polled safely
```

#### Async I/O

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::fs::File;

// 1. TcpListener - Accept connections
let listener = TcpListener::bind("127.0.0.1:8080").await?;

loop {
    let (socket, addr) = listener.accept().await?;
    tokio::spawn(async move {
        handle_connection(socket).await;
    });
}

// 2. TcpStream - Read/write TCP
async fn handle_connection(mut socket: TcpStream) {
    let mut buf = [0u8; 1024];

    // Read
    let n = socket.read(&mut buf).await.unwrap();

    // Write
    socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();

    // Shutdown
    socket.shutdown().await.unwrap();
}

// 3. File I/O
use tokio::fs::File;

let mut file = File::create("output.txt").await?;
file.write_all(b"Hello, world!").await?;

let mut file = File::open("input.txt").await?;
let mut contents = String::new();
file.read_to_string(&mut contents).await?;

// 4. split() - Split read/write
let (mut reader, mut writer) = socket.split();

tokio::spawn(async move {
    writer.write_all(b"data").await.ok();
});

tokio::spawn(async move {
    let mut buf = vec![0u8; 1024];
    reader.read(&mut buf).await.ok();
});
```

#### Example: Exchange Order Processing Pipeline

```rust
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

struct OrderProcessor {
    order_rx: mpsc::UnboundedReceiver<Order>,
    engine: Arc<RwLock<OrderBookEngine>>,
    broadcast_tx: broadcast::Sender<TradeUpdate>,
}

impl OrderProcessor {
    async fn run(mut self) {
        // Process orders concurrently with backpressure
        let semaphore = Arc::new(Semaphore::new(100)); // Max 100 concurrent

        while let Some(order) = self.order_rx.recv().await {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let engine = self.engine.clone();
            let broadcast_tx = self.broadcast_tx.clone();

            tokio::spawn(async move {
                let _permit = permit; // Hold permit until done

                // Acquire write lock
                let mut engine_guard = engine.write().await;

                // Match order (CPU-intensive, spawn_blocking)
                let trades = tokio::task::spawn_blocking(move || {
                    // Perform matching logic
                    vec![]
                }).await.unwrap();

                // Broadcast trades
                for trade in trades {
                    broadcast_tx.send(TradeUpdate { trade }).ok();
                }
            });
        }
    }
}

// Spawn with timeout and cancellation
async fn process_with_timeout(order: Order) -> Result<Vec<Trade>, String> {
    let timeout_duration = Duration::from_millis(100);

    tokio::select! {
        result = process_order(order) => result,
        _ = tokio::time::sleep(timeout_duration) => {
            Err("Order processing timeout".into())
        }
    }
}

// Heartbeat sender
async fn send_heartbeats(mut writer: OwnedWriteHalf) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;
        if writer.write_all(b"HEARTBEAT\n").await.is_err() {
            break;
        }
    }
}
```

**Best Practices**:
- Use `spawn_blocking` for CPU-intensive or blocking operations
- Prefer bounded channels over unbounded (backpressure)
- Use `Arc` for shared state across tasks
- Always handle errors in spawned tasks (they don't propagate)
- Use `select!` for cancellation and timeouts
- Tune worker threads based on workload (CPU-bound vs I/O-bound)

---

### Tokio-util

**Purpose**: Utility crates for common async patterns with Tokio.

**Core Concepts**:
- Codec for framing byte streams
- Async trait compatibility
- Stream and sink utilities

#### Popular Functions

```rust
use tokio_util::{
    codec::{Decoder, Encoder, Framed, LinesCodec},
    sync::CancellationToken,
    task::TaskTracker,
};

// 1. LinesCodec - Line-delimited text protocol
use tokio_util::codec::{Framed, LinesCodec};

let stream = TcpStream::connect("127.0.0.1:8080").await?;
let mut framed = Framed::new(stream, LinesCodec::new());

// Send line
framed.send("SUBSCRIBE XAUUSD".to_string()).await?;

// Receive lines
while let Some(line) = framed.try_next().await? {
    println!("Received: {}", line);
}

// 2. Custom Codec - FIX protocol example
use bytes::{Buf, BufMut, BytesMut};

struct FixCodec;

impl Decoder for FixCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Find SOH (0x01) delimiter
        if let Some(pos) = src.iter().position(|&b| b == 0x01) {
            let message = src.split_to(pos);
            src.advance(1); // Skip SOH
            Ok(Some(String::from_utf8_lossy(&message).to_string()))
        } else {
            Ok(None) // Need more data
        }
    }
}

impl Encoder<String> for FixCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put(item.as_bytes());
        dst.put_u8(0x01); // SOH delimiter
        Ok(())
    }
}

// 3. CancellationToken - Graceful shutdown
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();

let token_clone = token.clone();
tokio::spawn(async move {
    tokio::select! {
        _ = do_work() => {},
        _ = token_clone.cancelled() => {
            println!("Shutting down gracefully");
        }
    }
});

// Signal cancellation
token.cancel();

// 4. TaskTracker - Track spawned tasks
use tokio_util::task::TaskTracker;

let tracker = TaskTracker::new();

for i in 0..10 {
    tracker.spawn(async move {
        println!("Task {}", i);
    });
}

// Wait for all tasks
tracker.close();
tracker.wait().await;
```

**Best Practices**:
- Use codecs for protocol parsing (cleaner than manual buffering)
- Implement `CancellationToken` for graceful shutdown
- Use `TaskTracker` to wait for background tasks

---

### Rayon

**Purpose**: Data parallelism library for CPU-bound work. Splits work across thread pool.

**Core Concepts**:
- Parallel iterators
- Work-stealing thread pool
- Automatic work distribution

#### Popular Functions

```rust
use rayon::prelude::*;

// 1. par_iter() - Parallel iteration
let numbers: Vec<i32> = (0..1_000_000).collect();
let sum: i32 = numbers.par_iter().sum();

// 2. par_iter_mut() - Mutable parallel iteration
let mut data = vec![1, 2, 3, 4, 5];
data.par_iter_mut().for_each(|x| *x *= 2);

// 3. map/filter/reduce - Parallel operations
let result: Vec<i32> = numbers
    .par_iter()
    .filter(|&&x| x % 2 == 0)
    .map(|&x| x * x)
    .collect();

// 4. par_chunks() - Process chunks in parallel
let data: Vec<u8> = vec![0; 1_000_000];
let checksums: Vec<u32> = data
    .par_chunks(1000)
    .map(|chunk| chunk.iter().map(|&x| x as u32).sum())
    .collect();

// 5. par_sort() - Parallel sorting
let mut data = vec![5, 2, 8, 1, 9];
data.par_sort();

// 6. join() - Fork-join parallelism
use rayon::join;
let (result1, result2) = rayon::join(
    || expensive_computation_1(),
    || expensive_computation_2()
);

// 7. scope() - Scoped parallelism (borrow data)
use rayon::scope;
let mut data = vec![1, 2, 3];

rayon::scope(|s| {
    s.spawn(|_| {
        // Can safely borrow data
        data[0] += 1;
    });
});

// 8. ThreadPool - Custom pool configuration
use rayon::ThreadPoolBuilder;

let pool = ThreadPoolBuilder::new()
    .num_threads(8)
    .build()
    .unwrap();

pool.install(|| {
    // Work runs on this pool
    let result: i32 = (0..1000).into_par_iter().sum();
});
```

#### Example: Parallel Order Book Calculations

```rust
use rayon::prelude::*;

// Calculate VWAP for all symbols in parallel
fn calculate_all_vwaps(order_books: &HashMap<String, OrderBook>) -> HashMap<String, Decimal> {
    order_books
        .par_iter()
        .map(|(symbol, book)| {
            let vwap = calculate_vwap(book);
            (symbol.clone(), vwap)
        })
        .collect()
}

// Process order validations in parallel
fn validate_orders(orders: Vec<Order>) -> Vec<Result<Order, ValidationError>> {
    orders
        .into_par_iter()
        .map(|order| validate_order(order))
        .collect()
}

// Parallel fee calculations
fn calculate_fees_parallel(trades: &[Trade]) -> Decimal {
    trades
        .par_iter()
        .map(|trade| calculate_trade_fee(trade))
        .sum()
}
```

**Best Practices**:
- Use Rayon for CPU-bound work, Tokio for I/O
- Don't nest par_iter (causes overhead)
- Prefer Rayon over manual threading
- Works great with `spawn_blocking` from Tokio

---

### Crossbeam

**Purpose**: Concurrent data structures and utilities for lock-free programming.

**Core Concepts**:
- Lock-free channels (faster than std)
- Concurrent data structures
- Scoped threads

#### Popular Functions

```rust
use crossbeam::{
    channel::{unbounded, bounded, Sender, Receiver},
    deque::{Worker, Stealer},
    queue::ArrayQueue,
};

// 1. unbounded() - MPMC unbounded channel
let (tx, rx) = crossbeam::channel::unbounded::<String>();

tx.send("message".into()).unwrap();
let msg = rx.recv().unwrap();

// 2. bounded() - MPMC bounded channel
let (tx, rx) = crossbeam::channel::bounded::<String>(100);

// Non-blocking send
match tx.try_send("msg".into()) {
    Ok(()) => {},
    Err(_) => println!("Channel full"),
}

// 3. select! - Wait on multiple channels
use crossbeam::channel::select;

let (tx1, rx1) = unbounded();
let (tx2, rx2) = unbounded();

select! {
    recv(rx1) -> msg => println!("rx1: {:?}", msg),
    recv(rx2) -> msg => println!("rx2: {:?}", msg),
}

// 4. ArrayQueue - Lock-free bounded queue
use crossbeam::queue::ArrayQueue;

let queue = ArrayQueue::new(100);
queue.push(42).ok();
let value = queue.pop();

// 5. Scoped threads - Borrow data safely
use crossbeam::thread;

let data = vec![1, 2, 3, 4, 5];

crossbeam::thread::scope(|s| {
    s.spawn(|_| {
        // Can safely borrow data
        println!("Sum: {}", data.iter().sum::<i32>());
    });
}).unwrap();

// 6. Work-stealing deque
use crossbeam::deque::{Worker, Stealer};

let worker = Worker::new_fifo();
let stealer = worker.stealer();

worker.push(42);
let task = stealer.steal();
```

**Best Practices**:
- Use Crossbeam channels for multi-producer multi-consumer
- Prefer `ArrayQueue` for lock-free producer-consumer
- Use scoped threads to avoid `Arc` overhead

---

## 3. Serialization & Data

### Serde

**Purpose**: Generic serialization/deserialization framework. Foundation for JSON, MessagePack, Bincode, etc.

**Core Concepts**:
- `Serialize` and `Deserialize` traits
- Derive macros for automatic implementation
- Format-agnostic (works with any format)

#### Popular Attributes & Functions

```rust
use serde::{Serialize, Deserialize, Serializer, Deserializer};

// 1. Basic derive
#[derive(Serialize, Deserialize, Debug)]
struct Order {
    id: String,
    symbol: String,
    price: f64,
    quantity: f64,
}

// 2. Rename fields
#[derive(Serialize, Deserialize)]
struct User {
    #[serde(rename = "userId")]
    user_id: String,

    #[serde(rename = "firstName")]
    first_name: String,
}

// 3. Skip fields
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(skip)]
    runtime_cache: HashMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    optional_field: Option<String>,
}

// 4. Default values
#[derive(Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    enabled: bool,  // Defaults to false

    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    8080
}

// 5. Flatten nested structures
#[derive(Serialize, Deserialize)]
struct OrderWithMetadata {
    #[serde(flatten)]
    order: Order,

    #[serde(flatten)]
    metadata: Metadata,
}
// Serializes as: { "id": "...", "symbol": "...", "timestamp": "..." }

// 6. Custom serialization
#[derive(Serialize)]
struct Trade {
    id: String,

    #[serde(serialize_with = "serialize_decimal")]
    price: Decimal,
}

fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

// 7. Custom deserialization
#[derive(Deserialize)]
struct Order {
    #[serde(deserialize_with = "deserialize_decimal")]
    price: Decimal,
}

fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Decimal::from_str(&s).map_err(serde::de::Error::custom)
}

// 8. Enums with variants
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum OrderType {
    Market,
    Limit { price: Decimal },
    Stop { stop_price: Decimal },
}
// Tagged: { "type": "Limit", "price": "100.50" }

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Response {
    Success(SuccessData),
    Error(ErrorData),
}
// Untagged: { "status": "ok", "data": ... } or { "error": "..." }

// 9. Rename all fields (snake_case, camelCase, etc.)
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderRequest {
    order_id: String,      // Serializes as "orderId"
    user_id: String,       // Serializes as "userId"
}

// 10. With for common patterns
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Serialize, Deserialize)]
struct Trade {
    #[serde_as(as = "DisplayFromStr")]
    price: Decimal,  // Serialize as string using Display/FromStr
}
```

#### Example: Exchange Order Serialization

```rust
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderDto {
    pub order_id: String,
    pub symbol: String,

    #[serde(serialize_with = "serialize_decimal")]
    #[serde(deserialize_with = "deserialize_decimal")]
    pub price: Decimal,

    #[serde(serialize_with = "serialize_decimal")]
    #[serde(deserialize_with = "deserialize_decimal")]
    pub quantity: Decimal,

    pub side: OrderSide,
    pub order_type: OrderType,
    pub status: OrderStatus,

    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_quantity: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Market,
    Limit,
}
```

**Best Practices**:
- Use `#[serde(rename_all = "camelCase")]` for JSON APIs
- Skip internal fields with `#[serde(skip)]`
- Use custom serializers for `Decimal` (serialize as string)
- Flatten nested structs for cleaner JSON
- Use `serde_with` crate for common patterns

---

### Serde_json

**Purpose**: Fast JSON serialization/deserialization using Serde.

#### Popular Functions

```rust
use serde_json::{json, Value, from_str, to_string, to_value};

// 1. to_string() - Serialize to JSON string
let order = Order { id: "123".into(), price: 100.0 };
let json = serde_json::to_string(&order)?;

// 2. to_string_pretty() - Pretty-print JSON
let json = serde_json::to_string_pretty(&order)?;

// 3. from_str() - Deserialize from JSON string
let order: Order = serde_json::from_str(r#"{"id":"123","price":100.0}"#)?;

// 4. to_value() - Convert to serde_json::Value
let value = serde_json::to_value(&order)?;

// 5. json! macro - Build JSON dynamically
let response = json!({
    "status": "success",
    "data": {
        "orderId": "123",
        "price": 100.50
    }
});

// 6. Value manipulation
let mut value = json!({ "name": "Alice" });
value["age"] = json!(30);
value["hobbies"] = json!(["trading", "coding"]);

// Access nested values
if let Some(name) = value["name"].as_str() {
    println!("Name: {}", name);
}

// 7. from_value() - Convert from Value to type
let order: Order = serde_json::from_value(value)?;

// 8. Streaming parser for large JSON
use serde_json::Deserializer;
let json = r#"{"a":1}{"b":2}{"c":3}"#;
for value in Deserializer::from_str(json).into_iter::<Value>() {
    println!("{:?}", value?);
}
```

**Best Practices**:
- Use typed deserialization over `Value` when schema is known
- Use `json!` macro for dynamic responses
- For large JSON, use streaming parser

---

### Bytes

**Purpose**: Zero-copy byte buffer for network programming.

**Core Concepts**:
- `Bytes`: Cheaply cloneable, immutable byte buffer
- `BytesMut`: Mutable byte buffer
- Reference counting (zero-copy clones)

#### Popular Functions

```rust
use bytes::{Bytes, BytesMut, Buf, BufMut};

// 1. Bytes - Immutable buffer
let bytes = Bytes::from("hello world");
let slice = bytes.slice(0..5);  // Zero-copy slice

// Clone is cheap (reference counting)
let clone = bytes.clone();

// 2. BytesMut - Mutable buffer
let mut buf = BytesMut::with_capacity(1024);

// Write data
buf.put_u8(1);
buf.put_u16(256);
buf.put_slice(b"hello");
buf.put(&b"world"[..]);

// Convert to immutable
let bytes = buf.freeze();

// 3. split_to() - Split buffer
let mut buf = BytesMut::from(&b"hello world"[..]);
let hello = buf.split_to(5);  // buf now contains " world"

// 4. Buf trait - Reading
let mut buf = &b"hello"[..];
let byte = buf.get_u8();
let word = buf.get_u16();

// 5. BufMut trait - Writing
let mut buf = vec![];
buf.put_u32(42);
buf.put_slice(b"data");

// 6. remaining() - Check bytes left
let buf = &b"hello"[..];
println!("Remaining: {}", buf.remaining());

// 7. Reserve capacity
let mut buf = BytesMut::with_capacity(64);
buf.reserve(128);  // Ensure 128 bytes available
```

#### Example: FIX Message Parsing with Bytes

```rust
use bytes::{Bytes, BytesMut, Buf};

fn parse_fix_message(mut data: Bytes) -> Option<String> {
    // Find SOH delimiter (0x01)
    let pos = data.iter().position(|&b| b == 0x01)?;

    // Extract message (zero-copy)
    let message = data.slice(0..pos);

    // Advance past SOH
    data.advance(pos + 1);

    Some(String::from_utf8_lossy(&message).to_string())
}

// Efficient buffer management
struct MessageBuffer {
    buf: BytesMut,
}

impl MessageBuffer {
    fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(8192),
        }
    }

    fn append(&mut self, data: &[u8]) {
        self.buf.put_slice(data);
    }

    fn extract_message(&mut self) -> Option<Bytes> {
        // Find delimiter
        let pos = self.buf.iter().position(|&b| b == 0x01)?;

        // Extract (zero-copy)
        let message = self.buf.split_to(pos).freeze();
        self.buf.advance(1); // Skip SOH

        Some(message)
    }
}
```

**Best Practices**:
- Use `Bytes` for zero-copy message passing
- Use `BytesMut` for building messages
- Prefer `slice()` over copying when possible
- Pre-allocate with `with_capacity()` for known sizes

---

### Bincode

**Purpose**: Fast binary serialization using Serde. Much smaller and faster than JSON.

#### Popular Functions

```rust
use bincode::{serialize, deserialize, config};

// 1. serialize() - Encode to binary
let order = Order { id: "123".into(), price: 100.0 };
let bytes = bincode::serialize(&order)?;

// 2. deserialize() - Decode from binary
let order: Order = bincode::deserialize(&bytes)?;

// 3. Custom configuration
use bincode::config::standard;

let config = standard()
    .with_little_endian()
    .with_fixed_int_encoding();

let bytes = bincode::encode_to_vec(&order, config)?;
let order: Order = bincode::decode_from_slice(&bytes, config)?.0;

// 4. Serialize to writer
use std::fs::File;
let file = File::create("orders.bin")?;
bincode::serialize_into(file, &orders)?;

// 5. Deserialize from reader
let file = File::open("orders.bin")?;
let orders: Vec<Order> = bincode::deserialize_from(file)?;
```

**Best Practices**:
- Use bincode for internal services (not public APIs)
- 5-10x faster than JSON for large datasets
- Much smaller size (no field names)
- Not human-readable (use JSON for debugging)

---

## 4. Observability & Metrics

### Prometheus

**Purpose**: Metrics collection and exposition for Prometheus monitoring system.

**Core Concepts**:
- Metric types: Counter, Gauge, Histogram
- Labels for dimensions
- Registry for metric collection

#### Popular Functions & Types

```rust
use prometheus::{
    Registry, Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge,
    register_counter, register_gauge, register_histogram,
    Encoder, TextEncoder,
};

// 1. Counter - Monotonically increasing value
use prometheus::{Counter, register_counter};

let orders_total = register_counter!(
    "orders_total",
    "Total number of orders processed"
).unwrap();

orders_total.inc();        // Increment by 1
orders_total.inc_by(5.0);  // Increment by 5

// 2. Gauge - Value that can go up or down
let active_connections = register_gauge!(
    "active_connections",
    "Number of active WebSocket connections"
).unwrap();

active_connections.inc();
active_connections.dec();
active_connections.set(42.0);

// 3. Histogram - Distribution of values (latency, sizes)
let order_latency = register_histogram!(
    "order_latency_seconds",
    "Order processing latency in seconds",
    vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]  // Buckets
).unwrap();

let start = Instant::now();
process_order();
order_latency.observe(start.elapsed().as_secs_f64());

// 4. Labels - Add dimensions to metrics
use prometheus::{register_int_counter_vec, register_histogram_vec};

let orders_by_symbol = register_int_counter_vec!(
    "orders_by_symbol_total",
    "Orders by symbol",
    &["symbol", "side"]  // Label names
).unwrap();

orders_by_symbol
    .with_label_values(&["XAUUSD", "BUY"])
    .inc();

// Histogram with labels
let latency_by_operation = register_histogram_vec!(
    "operation_latency_seconds",
    "Operation latency by type",
    &["operation"],
    vec![0.001, 0.01, 0.1, 1.0]
).unwrap();

latency_by_operation
    .with_label_values(&["match_order"])
    .observe(0.005);

// 5. Registry - Collect all metrics
let registry = Registry::new();

let counter = IntCounter::new("my_counter", "help")?;
registry.register(Box::new(counter.clone()))?;

// 6. TextEncoder - Export metrics for Prometheus
use prometheus::{Encoder, TextEncoder};

let encoder = TextEncoder::new();
let metric_families = registry.gather();
let mut buffer = vec![];
encoder.encode(&metric_families, &mut buffer)?;

let output = String::from_utf8(buffer)?;
// Returns Prometheus text format:
// # HELP orders_total Total number of orders processed
// # TYPE orders_total counter
// orders_total 42

// 7. Custom registry
let custom_registry = Registry::new_custom(Some("exchange".into()), None)?;
```

#### Example: Complete Exchange Metrics Setup

```rust
use prometheus::{
    Registry, IntCounter, IntGauge, Histogram, HistogramVec, HistogramOpts,
    register_int_counter_with_registry, register_int_gauge_with_registry,
    register_histogram_vec_with_registry, Encoder, TextEncoder,
};
use std::sync::Arc;

pub struct ExchangeMetrics {
    // Counters
    pub orders_total: IntCounter,
    pub trades_total: IntCounter,
    pub orders_cancelled: IntCounter,

    // Gauges
    pub active_orders: IntGauge,
    pub websocket_connections: IntGauge,

    // Histograms
    pub order_latency: HistogramVec,
    pub trade_volume: HistogramVec,

    registry: Registry,
}

impl ExchangeMetrics {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Registry::new_custom(Some("exchange".into()), None)?;

        let orders_total = register_int_counter_with_registry!(
            "orders_total",
            "Total orders processed",
            registry
        )?;

        let trades_total = register_int_counter_with_registry!(
            "trades_total",
            "Total trades executed",
            registry
        )?;

        let orders_cancelled = register_int_counter_with_registry!(
            "orders_cancelled_total",
            "Total orders cancelled",
            registry
        )?;

        let active_orders = register_int_gauge_with_registry!(
            "active_orders",
            "Currently active orders in book",
            registry
        )?;

        let websocket_connections = register_int_gauge_with_registry!(
            "websocket_connections",
            "Active WebSocket connections",
            registry
        )?;

        let order_latency = register_histogram_vec_with_registry!(
            "order_latency_seconds",
            "Order processing latency",
            &["operation"],  // Labels: submit, cancel, match
            vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1],
            registry
        )?;

        let trade_volume = register_histogram_vec_with_registry!(
            "trade_volume",
            "Trade volume distribution",
            &["symbol"],
            vec![10.0, 100.0, 1000.0, 10000.0],
            registry
        )?;

        Ok(Self {
            orders_total,
            trades_total,
            orders_cancelled,
            active_orders,
            websocket_connections,
            order_latency,
            trade_volume,
            registry,
        })
    }

    pub fn gather(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

// Usage in handlers
async fn metrics_handler(
    State(metrics): State<Arc<ExchangeMetrics>>
) -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(Body::from(metrics.gather()))
        .unwrap()
}

// Instrument order processing
async fn submit_order(
    State(metrics): State<Arc<ExchangeMetrics>>,
    Json(req): Json<OrderRequest>
) -> Result<Json<OrderResponse>, StatusCode> {
    let timer = metrics.order_latency
        .with_label_values(&["submit"])
        .start_timer();

    let result = process_order(req).await;

    timer.observe_duration();
    metrics.orders_total.inc();
    metrics.active_orders.inc();

    result.map(Json)
}
```

**Connecting to Prometheus**:

1. **Expose metrics endpoint**:
```rust
let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .with_state(metrics);
```

2. **Configure Prometheus** (`prometheus.yml`):
```yaml
scrape_configs:
  - job_name: 'exchange'
    scrape_interval: 5s
    static_configs:
      - targets: ['localhost:3000']
        labels:
          service: 'order-book-exchange'
```

3. **Run Prometheus**:
```bash
docker run -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus
```

**Best Practices**:
- Use IntCounter/IntGauge when values are integers (more efficient)
- Add labels for dimensions, but keep cardinality low (< 10 values per label)
- Use histograms for latencies (not gauges)
- Choose bucket sizes carefully (P50, P90, P99 coverage)
- Use custom registry to avoid global state conflicts

---

### Tracing

**Purpose**: Application-level tracing with structured logging. Like logging++, supports spans and events.

**Core Concepts**:
- Spans: Represent units of work (with duration)
- Events: Single point-in-time logs
- Subscribers: Output destinations (console, file, Loki)
- Instruments: Auto-trace functions

#### Popular Functions & Macros

```rust
use tracing::{info, warn, error, debug, trace, span, Level, instrument};

// 1. Event macros - Structured logging
tracing::info!("Server started on port {}", 3000);
tracing::warn!(user_id = "123", "Rate limit exceeded");
tracing::error!(error = ?err, "Failed to process order");

// With structured fields
tracing::info!(
    order_id = %order.id,
    symbol = %order.symbol,
    price = %order.price,
    "Order submitted"
);

// 2. Spans - Trace operations with duration
let span = tracing::info_span!("process_order", order_id = %order.id);
let _enter = span.enter();

// ... work happens here ...

drop(_enter);  // Span ends

// 3. instrument macro - Auto-instrument functions
#[tracing::instrument]
async fn process_order(order: Order) -> Result<Trade, Error> {
    // Automatically creates span with function name and args
    tracing::info!("Processing order");
    Ok(Trade { })
}

// With custom fields
#[tracing::instrument(skip(engine), fields(symbol = %order.symbol))]
async fn submit_order(
    engine: &OrderBookEngine,
    order: Order
) -> Result<OrderResponse, Error> {
    // 'engine' skipped from span (avoid large Debug output)
    // 'symbol' added as field
}

// 4. Span context
use tracing::Span;

let span = tracing::info_span!("parent_operation");
span.in_scope(|| {
    // Events logged here are children of span
    tracing::info!("Inside span");
});

// 5. Dynamic fields
let span = tracing::info_span!("operation");
span.record("result", &"success");  // Add field later

// 6. Level control
tracing::trace!("Detailed debug info");  // Most verbose
tracing::debug!("Debug information");
tracing::info!("Informational message");
tracing::warn!("Warning message");
tracing::error!("Error occurred");
```

#### Example: Tracing Exchange Operations

```rust
use tracing::{info, warn, error, instrument, Span};

#[instrument(skip(engine), fields(order_id = %order.id, symbol = %order.symbol))]
async fn submit_order(
    engine: Arc<OrderBookEngine>,
    order: Order
) -> Result<OrderResponse, OrderError> {
    info!("Validating order");

    validate_order(&order)?;

    let start = Instant::now();

    info!("Matching order against book");
    let trades = engine.match_order(order.clone()).await?;

    let latency = start.elapsed();
    Span::current().record("latency_ms", latency.as_millis());
    Span::current().record("trades_count", trades.len());

    if !trades.is_empty() {
        info!(
            trades_count = trades.len(),
            total_volume = ?trades.iter().map(|t| t.quantity).sum::<Decimal>(),
            "Order matched"
        );
    }

    Ok(OrderResponse { trades })
}

#[instrument]
async fn handle_websocket_connection(socket: WebSocket, user_id: String) {
    info!("WebSocket connected");

    let span = Span::current();
    span.record("user_id", &user_id);

    while let Some(Ok(msg)) = socket.recv().await {
        process_message(msg).await;
    }

    warn!("WebSocket disconnected");
}
```

**Best Practices**:
- Use `#[instrument]` on async functions
- Skip large arguments with `skip()`
- Use structured fields, not string concatenation
- Keep span names consistent (aids searching)
- Record important metrics as span fields

---

### Tracing-subscriber

**Purpose**: Subscriber implementations for tracing (output formatting, filtering).

**Core Concepts**:
- Layers: Composable subscribers
- Filters: Control which spans/events are recorded
- Formatting: Console, JSON, etc.

#### Popular Functions

```rust
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Layer,
};

// 1. Simple console logging
tracing_subscriber::fmt::init();

// 2. With custom format
tracing_subscriber::fmt()
    .with_target(false)           // Don't include target (module path)
    .with_thread_ids(true)         // Include thread IDs
    .with_level(true)              // Include level
    .with_line_number(true)        // Include line numbers
    .compact()                     // Compact format
    .init();

// 3. JSON formatting (for structured logging)
tracing_subscriber::fmt()
    .json()
    .with_current_span(false)
    .init();

// 4. EnvFilter - Control log levels
use tracing_subscriber::EnvFilter;

let filter = EnvFilter::new("info")
    .add_directive("order_book_exchange=debug".parse().unwrap())
    .add_directive("tokio=warn".parse().unwrap());

tracing_subscriber::fmt()
    .with_env_filter(filter)
    .init();

// Or from environment variable:
// RUST_LOG=info,order_book_exchange=debug cargo run

// 5. Layered subscribers - Multiple outputs
use tracing_subscriber::layer::SubscriberExt;

let fmt_layer = fmt::layer()
    .with_target(false)
    .with_writer(std::io::stdout);

let filter = EnvFilter::from_default_env()
    .add_directive(Level::INFO.into());

tracing_subscriber::registry()
    .with(filter)
    .with(fmt_layer)
    .init();

// 6. Multiple layers - Console + File
use std::fs::File;

let file = File::create("exchange.log").unwrap();
let file_layer = fmt::layer()
    .json()
    .with_writer(Arc::new(file));

let stdout_layer = fmt::layer()
    .compact()
    .with_writer(std::io::stdout);

tracing_subscriber::registry()
    .with(EnvFilter::from_default_env())
    .with(file_layer)
    .with(stdout_layer)
    .init();
```

#### Connecting to Loki (Grafana)

**Using tracing-loki crate**:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_loki::Layer as LokiLayer;

async fn setup_tracing() {
    // Loki endpoint
    let (loki_layer, task) = LokiLayer::new(
        tracing_loki::url::Url::parse("http://localhost:3100").unwrap(),
        vec![
            ("service".into(), "order-book-exchange".into()),
            ("environment".into(), "production".into()),
        ].into_iter().collect(),
    ).unwrap();

    // Spawn background task
    tokio::spawn(task);

    // Combine with console logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(loki_layer)
        .init();
}

#[tokio::main]
async fn main() {
    setup_tracing().await;

    tracing::info!("Application started");
    // Logs sent to both console and Loki
}
```

**Docker Compose for Loki + Grafana**:

```yaml
version: '3.8'

services:
  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    command: -config.file=/etc/loki/local-config.yaml

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    environment:
      - GF_AUTH_ANONYMOUS_ENABLED=true
      - GF_AUTH_ANONYMOUS_ORG_ROLE=Admin
    volumes:
      - grafana-storage:/var/lib/grafana
    depends_on:
      - loki

volumes:
  grafana-storage:
```

**Start services**:
```bash
docker-compose up -d
```

**Configure Grafana**:
1. Open http://localhost:3001
2. Go to Configuration → Data Sources
3. Add Loki data source: http://loki:3100
4. Create dashboard with log queries: `{service="order-book-exchange"}`

**Best Practices**:
- Use `EnvFilter` for runtime control (RUST_LOG env var)
- JSON format for production (machine-readable)
- Compact format for development (human-readable)
- Send logs to Loki for centralized viewing
- Add service/environment labels for filtering

---

### Metrics

**Purpose**: High-level metrics abstraction (frontend for Prometheus, StatsD, etc.).

#### Popular Functions

```rust
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, Unit};

// 1. counter! - Increment counter
metrics::counter!("orders.total").increment(1);
metrics::counter!("orders.by_symbol", "symbol" => "XAUUSD").increment(1);

// 2. gauge! - Set gauge value
metrics::gauge!("websocket.connections").set(42.0);
metrics::gauge!("orderbook.depth", "symbol" => "BTCUSD").set(100.0);

// 3. histogram! - Record distribution
metrics::histogram!("order.latency").record(0.005);
metrics::histogram!("trade.volume", "symbol" => "XAUUSD").record(1000.0);

// 4. describe_* - Add metric documentation
metrics::describe_counter!("orders.total", "Total orders processed");
metrics::describe_gauge!("websocket.connections", Unit::Count, "Active WS connections");
metrics::describe_histogram!("order.latency", Unit::Seconds, "Order processing latency");

// 5. Setup metrics exporter (Prometheus)
use metrics_exporter_prometheus::PrometheusBuilder;

let builder = PrometheusBuilder::new();
builder.install().expect("Failed to install Prometheus exporter");

// Now metrics! macros automatically export to Prometheus
```

**Best Practices**:
- Use `metrics` crate for abstraction (swap backends easily)
- Use `prometheus` directly for fine-grained control
- Describe all metrics for Grafana dashboards

---

## 5. Database & Persistence

### SQLx

**Purpose**: Async SQL toolkit with compile-time checked queries.

**Core Concepts**:
- Async database queries
- Compile-time SQL verification
- Connection pooling
- Migrations

#### Popular Functions

```rust
use sqlx::{PgPool, Pool, Postgres, FromRow, query, query_as};

// 1. Create connection pool
let pool = PgPool::connect("postgresql://user:pass@localhost/exchange").await?;

// 2. query!() macro - Compile-time checked
let orders = sqlx::query!(
    r#"
    SELECT id, symbol, price, quantity
    FROM orders
    WHERE symbol = $1
    "#,
    "XAUUSD"
)
.fetch_all(&pool)
.await?;

// Returns anonymous struct with typed fields
for order in orders {
    println!("{}: {}", order.id, order.price);
}

// 3. query_as!() - Map to struct
#[derive(FromRow)]
struct Order {
    id: String,
    symbol: String,
    price: Decimal,
}

let orders = sqlx::query_as!(
    Order,
    "SELECT id, symbol, price FROM orders WHERE symbol = $1",
    "XAUUSD"
)
.fetch_all(&pool)
.await?;

// 4. Execute queries
sqlx::query!(
    "INSERT INTO orders (id, symbol, price) VALUES ($1, $2, $3)",
    order_id,
    symbol,
    price
)
.execute(&pool)
.await?;

// 5. Transactions
let mut tx = pool.begin().await?;

sqlx::query!("INSERT INTO orders ...")
    .execute(&mut *tx)
    .await?;

sqlx::query!("INSERT INTO trades ...")
    .execute(&mut *tx)
    .await?;

tx.commit().await?;  // Or tx.rollback()

// 6. fetch_one() vs fetch_all() vs fetch_optional()
let order = sqlx::query_as!(Order, "SELECT * FROM orders WHERE id = $1", id)
    .fetch_one(&pool)    // Returns error if not found
    .await?;

let order_opt = sqlx::query_as!(Order, "SELECT * FROM orders WHERE id = $1", id)
    .fetch_optional(&pool)  // Returns None if not found
    .await?;

// 7. Migrations
// migrations/001_create_orders.sql
sqlx::migrate!("./migrations")
    .run(&pool)
    .await?;
```

**Best Practices**:
- Use query! macros for type safety
- Always use connection pools
- Use transactions for multi-step operations
- Enable `offline` mode for CI/CD (cached query metadata)

---

### Redis

**Purpose**: Async Redis client for caching and pub/sub.

#### Popular Functions

```rust
use redis::{AsyncCommands, Client};

// 1. Connect
let client = redis::Client::open("redis://127.0.0.1/")?;
let mut con = client.get_async_connection().await?;

// 2. SET/GET
con.set("order:123", "data").await?;
let value: String = con.get("order:123").await?;

// 3. SET with expiration
con.set_ex("session:abc", "data", 3600).await?;  // Expires in 1 hour

// 4. Pub/Sub
let mut pubsub = client.get_async_connection().await?.into_pubsub();
pubsub.subscribe("trades").await?;

let mut stream = pubsub.on_message();
while let Some(msg) = stream.next().await {
    let payload: String = msg.get_payload()?;
    println!("Received: {}", payload);
}

// Publish
con.publish("trades", "trade data").await?;

// 5. Hashes
con.hset("order:123", "symbol", "XAUUSD").await?;
con.hset("order:123", "price", "2000").await?;

let symbol: String = con.hget("order:123", "symbol").await?;
let all: HashMap<String, String> = con.hgetall("order:123").await?;
```

**Best Practices**:
- Use Redis for caching hot data (order books)
- Use pub/sub for real-time updates between services
- Set TTLs on cache entries

---

### RocksDB

**Purpose**: Embedded key-value store for WAL and persistence.

#### Popular Functions

```rust
use rocksdb::{DB, Options, WriteBatch};

// 1. Open database
let mut opts = Options::default();
opts.create_if_missing(true);
let db = DB::open(&opts, "path/to/db")?;

// 2. Put/Get
db.put(b"key", b"value")?;
let value = db.get(b"key")?;  // Returns Option<Vec<u8>>

// 3. Delete
db.delete(b"key")?;

// 4. Batch writes (atomic)
let mut batch = WriteBatch::default();
batch.put(b"key1", b"value1");
batch.put(b"key2", b"value2");
batch.delete(b"old_key");
db.write(batch)?;

// 5. Iteration
let iter = db.iterator(rocksdb::IteratorMode::Start);
for item in iter {
    let (key, value) = item?;
    println!("{:?}: {:?}", key, value);
}

// 6. Prefix iteration
let prefix = b"order:";
let iter = db.prefix_iterator(prefix);
for item in iter {
    let (key, value) = item?;
}
```

**Best Practices**:
- Use for Write-Ahead Log (WAL) persistence
- Use batch writes for atomicity
- Great for embedded storage (no external DB)

---

## 6. Error Handling

### Thiserror

**Purpose**: Derive macro for implementing `std::error::Error` trait.

#### Popular Patterns

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order not found: {0}")]
    NotFound(String),

    #[error("Invalid price: {price}, must be positive")]
    InvalidPrice { price: Decimal },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: Decimal, available: Decimal },

    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error")]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Usage
fn validate_order(order: &Order) -> Result<(), OrderError> {
    if order.price <= Decimal::ZERO {
        return Err(OrderError::InvalidPrice { price: order.price });
    }
    Ok(())
}
```

**Best Practices**:
- Use `#[from]` for automatic conversion with `?` operator
- Include context in error messages
- Use `#[error(transparent)]` for wrapping unknown errors

---

### Anyhow

**Purpose**: Flexible error handling for applications (not libraries).

#### Popular Functions

```rust
use anyhow::{Result, Context, anyhow, bail};

// 1. Result type alias
fn do_work() -> anyhow::Result<String> {
    Ok("done".into())
}

// 2. context() - Add error context
use anyhow::Context;

let config = std::fs::read_to_string("config.toml")
    .context("Failed to read config file")?;

// 3. with_context() - Lazy context
let value = get_value()
    .with_context(|| format!("Failed to get value for key: {}", key))?;

// 4. anyhow! macro - Create error
return Err(anyhow!("Invalid configuration"));

// 5. bail! macro - Early return with error
if price <= 0.0 {
    bail!("Price must be positive");
}

// 6. Error downcast
match err.downcast_ref::<OrderError>() {
    Some(OrderError::NotFound(id)) => {},
    _ => {}
}
```

**Best Practices**:
- Use in applications (not libraries)
- Add context with `.context()`
- Use thiserror for library errors, anyhow for app errors

---

## 7. Time & Decimals

### Chrono

**Purpose**: Date and time handling.

#### Popular Functions

```rust
use chrono::{DateTime, Utc, Duration, NaiveDate};

// 1. Current time
let now = Utc::now();

// 2. Parse from string
let dt = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")?;

// 3. Format
let formatted = now.format("%Y-%m-%d %H:%M:%S").to_string();

// 4. Duration arithmetic
let tomorrow = now + Duration::days(1);
let an_hour_ago = now - Duration::hours(1);

// 5. Unix timestamp
let timestamp = now.timestamp();  // Seconds since epoch
let timestamp_millis = now.timestamp_millis();

// 6. From timestamp
let dt = DateTime::from_timestamp(timestamp, 0).unwrap();
```

---

### Rust_decimal

**Purpose**: Fixed-point decimal arithmetic for finance (avoid floating-point errors).

#### Popular Functions

```rust
use rust_decimal::Decimal;
use std::str::FromStr;

// 1. Create decimals
let price = Decimal::new(10050, 2);  // 100.50
let price = Decimal::from_str("100.50")?;

// 2. Arithmetic
let total = price * quantity;
let avg = (price1 + price2) / Decimal::TWO;

// 3. Comparison
if price > Decimal::ZERO && price < max_price {
    // ...
}

// 4. Rounding
let rounded = price.round_dp(2);  // Round to 2 decimal places

// 5. Conversion
let float_val: f64 = price.to_f64().unwrap();
let string_val = price.to_string();
```

**Best Practices**:
- Always use Decimal for money/prices (never f64)
- Serialize as string in JSON to preserve precision
- Use round_dp() before display

---

## 8. Network & Protocol

### Hyper

**Purpose**: Low-level HTTP library (foundation for Axum and Reqwest).

#### Popular Types

```rust
use hyper::{Body, Request, Response, Client, Server, Method, StatusCode};

// Usually you use Axum instead, but for custom protocols:

// 1. Custom HTTP client
let client = Client::new();

let req = Request::builder()
    .method(Method::POST)
    .uri("http://api.example.com/orders")
    .header("content-type", "application/json")
    .body(Body::from(r#"{"symbol":"XAUUSD"}"#))?;

let resp = client.request(req).await?;
let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
```

---

### Reqwest

**Purpose**: High-level HTTP client for making requests.

#### Popular Functions

```rust
use reqwest::{Client, header};

// 1. GET request
let body = reqwest::get("https://api.example.com/data")
    .await?
    .text()
    .await?;

// 2. POST JSON
let client = Client::new();
let res = client.post("https://api.example.com/orders")
    .json(&order_request)
    .send()
    .await?;

let order: OrderResponse = res.json().await?;

// 3. Headers
let res = client.get("https://api.example.com/data")
    .header("Authorization", "Bearer token")
    .send()
    .await?;

// 4. Query parameters
let res = client.get("https://api.example.com/orders")
    .query(&[("symbol", "XAUUSD"), ("limit", "100")])
    .send()
    .await?;

// 5. Timeout
let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()?;
```

---

### Tungstenite

**Purpose**: WebSocket protocol implementation (used by Axum internally).

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Client
let (ws_stream, _) = connect_async("ws://localhost:3000/ws").await?;
let (mut write, mut read) = ws_stream.split();

write.send(Message::Text("SUBSCRIBE XAUUSD".into())).await?;

while let Some(msg) = read.next().await {
    let msg = msg?;
    println!("Received: {:?}", msg);
}
```

---

## 9. Utilities

### Uuid

**Purpose**: Generate unique identifiers.

#### Popular Functions

```rust
use uuid::Uuid;

// 1. v4 - Random UUID
let id = Uuid::new_v4();
let id_str = id.to_string();  // "550e8400-e29b-41d4-a716-446655440000"

// 2. v7 - Time-ordered UUID (new in uuid 1.0)
let id = Uuid::now_v7();  // Sortable by creation time

// 3. Parse
let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;

// 4. Nil UUID
let nil = Uuid::nil();  // 00000000-0000-0000-0000-000000000000
```

---

### DashMap

**Purpose**: Concurrent HashMap (faster than RwLock<HashMap>).

#### Popular Functions

```rust
use dashmap::DashMap;

let map = DashMap::new();

// 1. Insert
map.insert("key", "value");

// 2. Get
if let Some(entry) = map.get("key") {
    println!("Value: {}", *entry);
}

// 3. Modify
map.alter("key", |_, v| v + 1);

// 4. Entry API
map.entry("key").or_insert(0);

// 5. Iteration
for entry in map.iter() {
    println!("{}: {}", entry.key(), entry.value());
}
```

**Best Practices**:
- Use for concurrent shared state (WebSocket subscribers, caches)
- Faster than Arc<RwLock<HashMap>>

---

### Once_cell

**Purpose**: Lazy initialization and global state.

#### Popular Patterns

```rust
use once_cell::sync::{Lazy, OnceCell};

// 1. Lazy static
static CONFIG: Lazy<Config> = Lazy::new(|| {
    load_config_from_file()
});

fn use_config() {
    println!("{}", CONFIG.api_key);
}

// 2. OnceCell - Initialize once
static DB_POOL: OnceCell<PgPool> = OnceCell::new();

async fn init() {
    let pool = PgPool::connect("...").await.unwrap();
    DB_POOL.set(pool).ok();
}

fn get_pool() -> &'static PgPool {
    DB_POOL.get().expect("DB not initialized")
}
```

---

### Parking_lot

**Purpose**: Faster Mutex/RwLock than std.

```rust
use parking_lot::{Mutex, RwLock};

// Drop-in replacement for std::sync::Mutex
let mutex = Mutex::new(0);
let mut guard = mutex.lock();
*guard += 1;

// RwLock
let rwlock = RwLock::new(HashMap::new());
let read = rwlock.read();
let write = rwlock.write();
```

**Best Practices**:
- Drop-in replacement for std (no poisoning, smaller, faster)
- Use parking_lot by default unless you need std compatibility

---

## 10. API Documentation

### Utoipa

**Purpose**: Generate OpenAPI (Swagger) documentation from code.

#### Popular Macros

```rust
use utoipa::{OpenApi, ToSchema, IntoParams};

// 1. Define API schema
#[derive(OpenApi)]
#[openapi(
    paths(
        create_order,
        get_order,
        get_orderbook
    ),
    components(
        schemas(OrderRequest, OrderResponse, OrderSide)
    ),
    tags(
        (name = "orders", description = "Order management endpoints")
    ),
    info(
        title = "Order Book Exchange API",
        version = "1.0.0",
        description = "High-performance order matching engine"
    )
)]
struct ApiDoc;

// 2. Document request struct
#[derive(Serialize, Deserialize, ToSchema)]
struct OrderRequest {
    /// Symbol to trade (e.g. XAUUSD, BTCUSD)
    #[schema(example = "XAUUSD")]
    symbol: String,

    /// Order price
    #[schema(example = 2000.50)]
    price: f64,
}

// 3. Document handler
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    request_body = OrderRequest,
    responses(
        (status = 201, description = "Order created", body = OrderResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "orders"
)]
async fn create_order(
    Json(req): Json<OrderRequest>
) -> Result<Json<OrderResponse>, StatusCode> {
    // ...
}

// 4. Serve Swagger UI
use utoipa_swagger_ui::SwaggerUi;

let app = Router::new()
    .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    .route("/api/v1/orders", post(create_order));
```

**Best Practices**:
- Use `#[derive(ToSchema)]` on all DTOs
- Document all endpoints with `#[utoipa::path]`
- Add examples to schema fields
- Serve Swagger UI for interactive docs

---

## Summary Table

| Category | Package | Primary Use Case |
|----------|---------|------------------|
| **Web** | Axum | HTTP server, routing, WebSocket |
| | Tower | Middleware, service composition |
| | Tower-HTTP | CORS, compression, tracing |
| **Async** | Tokio | Async runtime, tasks, I/O |
| | Tokio-util | Codecs, cancellation tokens |
| | Rayon | CPU parallelism |
| | Crossbeam | Lock-free channels |
| **Data** | Serde | Serialization framework |
| | Serde_json | JSON support |
| | Bytes | Zero-copy buffers |
| | Bincode | Binary serialization |
| **Observability** | Prometheus | Metrics collection |
| | Tracing | Structured logging, spans |
| | Tracing-subscriber | Log formatting, Loki |
| **Database** | SQLx | Async SQL with type safety |
| | Redis | Caching, pub/sub |
| | RocksDB | Embedded key-value store |
| **Errors** | Thiserror | Library errors |
| | Anyhow | Application errors |
| **Time** | Chrono | Date/time handling |
| | Rust_decimal | Financial decimals |
| **Network** | Hyper | Low-level HTTP |
| | Reqwest | HTTP client |
| | Tungstenite | WebSocket |
| **Utils** | Uuid | Unique IDs |
| | DashMap | Concurrent HashMap |
| | Once_cell | Lazy initialization |
| | Parking_lot | Fast mutexes |
| **Docs** | Utoipa | OpenAPI/Swagger |

---

## Integration Example: Complete Exchange Stack

```rust
// main.rs - Bringing it all together

use axum::{Router, routing::{get, post}, extract::State};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer, compression::CompressionLayer};
use std::sync::Arc;
use tokio::sync::RwLock;

// Application state
#[derive(Clone)]
struct AppState {
    engine: Arc<RwLock<OrderBookEngine>>,
    metrics: Arc<ExchangeMetrics>,
    db_pool: PgPool,
    redis: redis::Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup tracing (console + Loki)
    setup_tracing().await;

    // 2. Initialize metrics
    let metrics = Arc::new(ExchangeMetrics::new()?);

    // 3. Setup database
    let db_pool = sqlx::postgres::PgPool::connect(&env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    // 4. Setup Redis
    let redis = redis::Client::open("redis://127.0.0.1/")?;

    // 5. Create application state
    let state = AppState {
        engine: Arc::new(RwLock::new(OrderBookEngine::new())),
        metrics,
        db_pool,
        redis,
    };

    // 6. Build middleware stack
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    // 7. Build router
    let app = Router::new()
        .route("/api/v1/orders", post(create_order))
        .route("/api/v1/orderbook/:symbol", get(get_orderbook))
        .route("/ws", get(ws_handler))
        .route("/metrics", get(metrics_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
        .layer(middleware);

    // 8. Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn setup_tracing() {
    let (loki_layer, task) = tracing_loki::Layer::new(
        url::Url::parse("http://localhost:3100").unwrap(),
        vec![("service".into(), "exchange".into())].into_iter().collect(),
    ).unwrap();

    tokio::spawn(task);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(loki_layer)
        .init();
}

#[tracing::instrument(skip(state), fields(order_id))]
async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<OrderRequest>
) -> Result<Json<OrderResponse>, (StatusCode, String)> {
    let timer = state.metrics.order_latency
        .with_label_values(&["submit"])
        .start_timer();

    // Process order
    let order = state.engine.write().await.submit_order(req.into())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Store in database
    sqlx::query!(
        "INSERT INTO orders (id, symbol, price) VALUES ($1, $2, $3)",
        order.id,
        order.symbol,
        order.price
    )
    .execute(&state.db_pool)
    .await
    .ok();

    // Cache in Redis
    let mut redis_conn = state.redis.get_async_connection().await.ok();
    if let Some(ref mut conn) = redis_conn {
        redis::AsyncCommands::set_ex(
            conn,
            format!("order:{}", order.id),
            serde_json::to_string(&order).unwrap(),
            3600
        ).await.ok();
    }

    // Update metrics
    timer.observe_duration();
    state.metrics.orders_total.inc();

    tracing::info!(
        order_id = %order.id,
        symbol = %order.symbol,
        "Order created"
    );

    Ok(Json(order.into()))
}
```

This reference covers all essential packages for building production-grade exchanges in Rust!
