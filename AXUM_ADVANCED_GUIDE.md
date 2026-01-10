# Axum Advanced Guide: Ultra Deep Dive

**Master Axum for production-grade trading systems with advanced patterns and real-world examples**

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Advanced Routing Patterns](#2-advanced-routing-patterns)
3. [State Management Strategies](#3-state-management-strategies)
4. [Custom Extractors](#4-custom-extractors)
5. [Advanced Middleware](#5-advanced-middleware)
6. [Error Handling Mastery](#6-error-handling-mastery)
7. [WebSocket Patterns](#7-websocket-patterns)
8. [Server-Sent Events (SSE)](#8-server-sent-events-sse)
9. [Streaming Responses](#9-streaming-responses)
10. [Authentication & Authorization](#10-authentication--authorization)
11. [Rate Limiting & Throttling](#11-rate-limiting--throttling)
12. [Request/Response Interceptors](#12-requestresponse-interceptors)
13. [Testing Strategies](#13-testing-strategies)
14. [Performance Optimization](#14-performance-optimization)
15. [Production Patterns](#15-production-patterns)
16. [Trading System Examples](#16-trading-system-examples)

---

## 1. Architecture Overview

### How Axum Works

Axum is built on three foundational layers:

```
┌─────────────────────────────────────┐
│         Your Handlers               │
│  (async fn that return IntoResponse)│
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│      Axum Framework                 │
│  - Routing                          │
│  - Extractors (State, Json, etc.)   │
│  - IntoResponse trait               │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│      Tower Middleware               │
│  - Service trait                    │
│  - Layers (timeout, rate limit)     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│      Hyper HTTP Server              │
│  - TCP connection handling          │
│  - HTTP protocol implementation     │
└─────────────────────────────────────┘
```

### Request Flow

```rust
// Request flow through Axum
Client Request
    ↓
TCP Connection (Hyper)
    ↓
HTTP Parsing (Hyper)
    ↓
Tower Middleware Layers (outer → inner)
    ↓
Axum Router (matches path)
    ↓
Extractors (parse request into types)
    ↓
Handler Function (your code)
    ↓
IntoResponse (convert to HTTP response)
    ↓
Tower Middleware Layers (inner → outer)
    ↓
HTTP Response (Hyper)
    ↓
Client
```

---

## 2. Advanced Routing Patterns

### Nested Routers with State

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::State,
};
use std::sync::Arc;

// Different state types for different modules
#[derive(Clone)]
struct OrderBookState {
    engine: Arc<OrderBookEngine>,
}

#[derive(Clone)]
struct UserState {
    db: Arc<UserDatabase>,
}

#[derive(Clone)]
struct AppState {
    order_book: OrderBookState,
    user: UserState,
    config: Arc<Config>,
}

fn create_app() -> Router {
    let order_book_state = OrderBookState {
        engine: Arc::new(OrderBookEngine::new()),
    };

    let user_state = UserState {
        db: Arc::new(UserDatabase::new()),
    };

    let app_state = AppState {
        order_book: order_book_state.clone(),
        user: user_state.clone(),
        config: Arc::new(Config::load()),
    };

    // Nested routers with scoped state
    let orders_router = Router::new()
        .route("/", post(create_order).get(list_orders))
        .route("/:id", get(get_order).delete(cancel_order))
        .with_state(order_book_state);

    let users_router = Router::new()
        .route("/", post(create_user).get(list_users))
        .route("/:id", get(get_user))
        .with_state(user_state);

    Router::new()
        .nest("/api/v1/orders", orders_router)
        .nest("/api/v1/users", users_router)
        .with_state(app_state)
}

async fn create_order(
    State(state): State<OrderBookState>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderResponse>, ApiError> {
    let order = state.engine.submit_order(req.into()).await?;
    Ok(Json(order.into()))
}
```

### Route Grouping with Middleware

```rust
use tower_http::auth::RequireAuthorizationLayer;

fn create_app() -> Router {
    // Public routes (no auth)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/login", post(login))
        .route("/register", post(register));

    // Protected routes (requires auth)
    let protected_routes = Router::new()
        .route("/orders", post(create_order))
        .route("/orders/:id", get(get_order))
        .route("/profile", get(get_profile))
        .layer(RequireAuthorizationLayer::bearer("secret-token"));

    // Admin routes (requires admin role)
    let admin_routes = Router::new()
        .route("/users", get(list_all_users))
        .route("/system/stats", get(system_stats))
        .layer(AdminAuthLayer::new());

    Router::new()
        .merge(public_routes)
        .nest("/api", protected_routes)
        .nest("/admin", admin_routes)
}
```

### Fallback Handling

```rust
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

async fn not_found_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"))
}

async fn spa_fallback() -> impl IntoResponse {
    // Serve index.html for SPA routing
    Html(include_str!("../static/index.html"))
}

fn create_app() -> Router {
    Router::new()
        .route("/api/orders", get(list_orders))
        // Serve static files
        .nest_service("/assets", ServeDir::new("assets"))
        // Fallback for SPA (serve index.html)
        .fallback(spa_fallback)
}
```

### Dynamic Route Registration

```rust
use std::collections::HashMap;

struct PluginRegistry {
    routes: HashMap<String, Router>,
}

impl PluginRegistry {
    fn register(&mut self, name: String, router: Router) {
        self.routes.insert(name, router);
    }

    fn build(self) -> Router {
        let mut app = Router::new();

        for (path, router) in self.routes {
            app = app.nest(&format!("/plugins/{}", path), router);
        }

        app
    }
}

fn create_extensible_app() -> Router {
    let mut registry = PluginRegistry::default();

    // Plugin 1
    let plugin1 = Router::new()
        .route("/action", post(plugin1_action));
    registry.register("plugin1".into(), plugin1);

    // Plugin 2
    let plugin2 = Router::new()
        .route("/data", get(plugin2_data));
    registry.register("plugin2".into(), plugin2);

    registry.build()
}
```

### Method Routing with MethodRouter

```rust
use axum::routing::method_routing::{get, post, delete};

let app = Router::new()
    .route(
        "/orders/:id",
        get(get_order)
            .post(update_order)  // POST /orders/:id
            .delete(cancel_order)
            .patch(modify_order),
    )
    // Separate method routers can be merged
    .route(
        "/trades/:id",
        get(get_trade).delete(delete_trade),
    );
```

---

## 3. State Management Strategies

### Pattern 1: Single Global State

```rust
#[derive(Clone)]
struct AppState {
    db: Arc<PgPool>,
    redis: Arc<redis::Client>,
    engine: Arc<RwLock<OrderBookEngine>>,
    config: Arc<Config>,
}

async fn handler(State(state): State<AppState>) {
    let engine = state.engine.read().await;
    // Use engine...
}
```

**Pros**: Simple, everything in one place
**Cons**: Tight coupling, all handlers depend on full state

### Pattern 2: Layered State (Recommended)

```rust
// Core domain state
#[derive(Clone)]
struct DomainState {
    engine: Arc<OrderBookEngine>,
}

// Infrastructure state
#[derive(Clone)]
struct InfraState {
    db: PgPool,
    redis: redis::Client,
    metrics: Arc<MetricsRegistry>,
}

// Application state (composition)
#[derive(Clone)]
struct AppState {
    domain: DomainState,
    infra: InfraState,
}

// Handlers only extract what they need
async fn create_order(
    State(domain): State<DomainState>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderResponse>, ApiError> {
    let order = domain.engine.submit_order(req.into()).await?;
    Ok(Json(order.into()))
}

async fn save_audit_log(
    State(infra): State<InfraState>,
    Json(log): Json<AuditLog>,
) -> StatusCode {
    infra.db.save_audit(log).await;
    StatusCode::OK
}
```

### Pattern 3: Extension-Based State

```rust
use axum::Extension;

async fn handler(
    Extension(db): Extension<PgPool>,
    Extension(config): Extension<Arc<Config>>,
) {
    // Use extensions
}

let app = Router::new()
    .route("/orders", post(handler))
    .layer(Extension(db_pool))
    .layer(Extension(Arc::new(config)));
```

**Pros**: Flexible, easy to add new dependencies
**Cons**: No compile-time checking, runtime lookup

### Pattern 4: Request-Scoped State

```rust
use axum::middleware;

// Add request-scoped data in middleware
async fn add_request_id<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(request_id));
    next.run(req).await
}

// Extract in handler
async fn handler(Extension(request_id): Extension<RequestId>) {
    tracing::info!(request_id = %request_id.0, "Processing request");
}

let app = Router::new()
    .route("/orders", post(handler))
    .layer(middleware::from_fn(add_request_id));
```

### Pattern 5: Shared Mutable State (with Guards)

```rust
use tokio::sync::{RwLock, Mutex, Semaphore};
use dashmap::DashMap;

#[derive(Clone)]
struct SharedState {
    // Read-heavy: Use RwLock
    config: Arc<RwLock<Config>>,

    // Write-heavy: Use DashMap (lock-free)
    active_sessions: Arc<DashMap<String, Session>>,

    // Exclusive access needed: Use Mutex
    rate_limiter: Arc<Mutex<RateLimiter>>,

    // Limit concurrency: Use Semaphore
    db_semaphore: Arc<Semaphore>,
}

async fn update_config(
    State(state): State<SharedState>,
    Json(new_config): Json<Config>,
) -> StatusCode {
    let mut config = state.config.write().await;
    *config = new_config;
    StatusCode::OK
}

async fn get_session(
    State(state): State<SharedState>,
    Path(session_id): Path<String>,
) -> Option<Json<Session>> {
    state.active_sessions
        .get(&session_id)
        .map(|s| Json(s.clone()))
}

async fn query_database(
    State(state): State<SharedState>,
) -> Result<Json<Data>, ApiError> {
    // Limit concurrent DB queries
    let _permit = state.db_semaphore.acquire().await?;

    let data = fetch_from_db().await?;
    Ok(Json(data))
}
```

---

## 4. Custom Extractors

### Simple Custom Extractor

```rust
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

// Custom extractor for authenticated user
struct AuthenticatedUser {
    user_id: String,
    role: Role,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization header".into()))?;

        // Parse bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid authorization format".into()))?;

        // Validate token and extract user info
        let claims = validate_jwt(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".into()))?;

        Ok(AuthenticatedUser {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

// Use in handler
async fn protected_handler(user: AuthenticatedUser) -> String {
    format!("Hello, user {}!", user.user_id)
}
```

### Extractor with State Access

```rust
struct ValidatedOrder {
    order: Order,
}

#[async_trait]
impl FromRequestParts<AppState> for ValidatedOrder {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract JSON body
        let Json(order): Json<Order> = Json::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::InvalidRequest)?;

        // Validate against business rules using state
        let validator = &state.order_validator;
        validator.validate(&order)?;

        // Check user permissions
        let user: AuthenticatedUser = AuthenticatedUser::from_request_parts(parts, state).await?;
        if !user.can_submit_order(&order) {
            return Err(ApiError::Forbidden);
        }

        Ok(ValidatedOrder { order })
    }
}

async fn create_order(
    State(state): State<AppState>,
    validated: ValidatedOrder,
) -> Result<Json<OrderResponse>, ApiError> {
    // Order is already validated!
    let result = state.engine.submit_order(validated.order).await?;
    Ok(Json(result.into()))
}
```

### Pagination Extractor

```rust
#[derive(Debug, Clone)]
struct Pagination {
    page: u32,
    page_size: u32,
    offset: u32,
}

impl Pagination {
    const MAX_PAGE_SIZE: u32 = 1000;
    const DEFAULT_PAGE_SIZE: u32 = 50;
}

#[async_trait]
impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Parse query string
        let Query(params): Query<HashMap<String, String>> =
            Query::from_request_parts(parts, state)
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid query".into()))?;

        let page = params
            .get("page")
            .and_then(|p| p.parse().ok())
            .unwrap_or(1);

        let page_size = params
            .get("page_size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(Self::DEFAULT_PAGE_SIZE)
            .min(Self::MAX_PAGE_SIZE);

        let offset = (page.saturating_sub(1)) * page_size;

        Ok(Pagination {
            page,
            page_size,
            offset,
        })
    }
}

async fn list_orders(pagination: Pagination) -> Json<PaginatedResponse<Order>> {
    let orders = fetch_orders(pagination.offset, pagination.page_size).await;

    Json(PaginatedResponse {
        data: orders,
        page: pagination.page,
        page_size: pagination.page_size,
        total: get_total_count().await,
    })
}
```

### Header Extractor

```rust
struct ClientInfo {
    ip: String,
    user_agent: String,
    request_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for ClientInfo
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        let ip = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let user_agent = headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let request_id = headers
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(ClientInfo {
            ip,
            user_agent,
            request_id,
        })
    }
}

async fn handler(client: ClientInfo) {
    tracing::info!(
        ip = %client.ip,
        user_agent = %client.user_agent,
        request_id = %client.request_id,
        "Request received"
    );
}
```

### Optional Extractor

```rust
// Extractor that doesn't fail if extraction fails
struct OptionalAuth(Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;  // Never fails!

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract user, but don't fail if it doesn't work
        match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

async fn maybe_authenticated_handler(OptionalAuth(user): OptionalAuth) -> String {
    match user {
        Some(u) => format!("Hello, {}!", u.user_id),
        None => "Hello, guest!".to_string(),
    }
}
```

---

## 5. Advanced Middleware

### Custom Middleware with State

```rust
use tower::Layer;
use std::task::{Context, Poll};

#[derive(Clone)]
struct RequestTimingLayer {
    metrics: Arc<MetricsRegistry>,
}

impl<S> Layer<S> for RequestTimingLayer {
    type Service = RequestTimingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestTimingService {
            inner,
            metrics: self.metrics.clone(),
        }
    }
}

#[derive(Clone)]
struct RequestTimingService<S> {
    inner: S,
    metrics: Arc<MetricsRegistry>,
}

impl<S, B> Service<Request<B>> for RequestTimingService<S>
where
    S: Service<Request<B>> + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let start = Instant::now();
        let path = req.uri().path().to_string();
        let method = req.method().clone();
        let metrics = self.metrics.clone();

        let future = self.inner.call(req);

        Box::pin(async move {
            let response = future.await?;
            let duration = start.elapsed();

            metrics
                .request_duration
                .with_label_values(&[method.as_str(), &path])
                .observe(duration.as_secs_f64());

            Ok(response)
        })
    }
}
```

### Function-Based Middleware

```rust
async fn log_requests<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request processed"
    );

    response
}

let app = Router::new()
    .route("/orders", post(create_order))
    .layer(middleware::from_fn(log_requests));
```

### Middleware with State Access

```rust
async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate using state
    let user = state
        .auth_service
        .validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add user to request extensions
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

let app = Router::new()
    .route("/protected", get(handler))
    .layer(middleware::from_fn_with_state(
        app_state.clone(),
        auth_middleware,
    ))
    .with_state(app_state);
```

### Conditional Middleware

```rust
struct ConditionalLayer<L> {
    condition: bool,
    layer: L,
}

impl<L: Clone> Clone for ConditionalLayer<L> {
    fn clone(&self) -> Self {
        ConditionalLayer {
            condition: self.condition,
            layer: self.layer.clone(),
        }
    }
}

impl<S, L> Layer<S> for ConditionalLayer<L>
where
    L: Layer<S>,
{
    type Service = ConditionalService<S, L::Service>;

    fn layer(&self, inner: S) -> Self::Service {
        if self.condition {
            ConditionalService::WithLayer(self.layer.layer(inner))
        } else {
            ConditionalService::WithoutLayer(inner)
        }
    }
}

enum ConditionalService<S, T> {
    WithLayer(T),
    WithoutLayer(S),
}

// Usage
let config = Config::load();

let app = Router::new()
    .route("/api/orders", post(create_order))
    .layer(ConditionalLayer {
        condition: config.enable_rate_limiting,
        layer: RateLimitLayer::new(100, Duration::from_secs(60)),
    });
```

### Error Recovery Middleware

```rust
async fn error_recovery<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    match next.run(req).await.into_response() {
        response if response.status().is_server_error() => {
            tracing::error!("Server error occurred");

            // Log error details
            let body = response.into_body();
            let bytes = axum::body::to_bytes(body, usize::MAX).await.ok();

            if let Some(bytes) = bytes {
                tracing::error!("Error body: {}", String::from_utf8_lossy(&bytes));
            }

            // Return generic error to client
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "request_id": uuid::Uuid::new_v4(),
                })),
            )
                .into_response()
        }
        response => response,
    }
}
```

---

## 6. Error Handling Mastery

### Hierarchical Error Types

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;

// Domain errors
#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("Order not found: {0}")]
    NotFound(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: Decimal, available: Decimal },

    #[error("Invalid order: {0}")]
    ValidationError(String),

    #[error("Order already filled")]
    AlreadyFilled,
}

// Infrastructure errors
#[derive(Debug, thiserror::Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("External API error: {0}")]
    ExternalApi(String),
}

// API errors (combines all error types)
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Order error: {0}")]
    Order(#[from] OrderError),

    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfraError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            ApiError::Order(OrderError::NotFound(id)) => (
                StatusCode::NOT_FOUND,
                format!("Order {} not found", id),
                "ORDER_NOT_FOUND",
            ),
            ApiError::Order(OrderError::InsufficientBalance { required, available }) => (
                StatusCode::BAD_REQUEST,
                format!(
                    "Insufficient balance: required {}, available {}",
                    required, available
                ),
                "INSUFFICIENT_BALANCE",
            ),
            ApiError::Order(OrderError::ValidationError(msg)) => (
                StatusCode::BAD_REQUEST,
                msg,
                "VALIDATION_ERROR",
            ),
            ApiError::Order(OrderError::AlreadyFilled) => (
                StatusCode::CONFLICT,
                "Order already filled".to_string(),
                "ORDER_ALREADY_FILLED",
            ),
            ApiError::Infrastructure(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
                "INTERNAL_ERROR",
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized".to_string(),
                "UNAUTHORIZED",
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Forbidden".to_string(),
                "FORBIDDEN",
            ),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
                "RATE_LIMIT_EXCEEDED",
            ),
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                msg,
                "BAD_REQUEST",
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": error_message,
            }
        }));

        (status, body).into_response()
    }
}
```

### Error Middleware

```rust
async fn error_handler_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let request_id = uuid::Uuid::new_v4();
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    if response.status().is_server_error() {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %response.status(),
            "Server error occurred"
        );

        // Return user-friendly error with request ID for tracking
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "An internal error occurred. Please contact support.",
                    "request_id": request_id.to_string(),
                }
            })),
        )
            .into_response();
    }

    response
}
```

### Result Type Alias

```rust
// Create type alias for common Result
pub type ApiResult<T> = Result<Json<T>, ApiError>;

// Handlers become cleaner
async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> ApiResult<OrderResponse> {
    let order = state.engine.submit_order(req.into()).await?;
    Ok(Json(order.into()))
}

// Can also use tuple for custom status codes
pub type ApiResultWithStatus<T> = Result<(StatusCode, Json<T>), ApiError>;

async fn create_order_with_status(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> ApiResultWithStatus<OrderResponse> {
    let order = state.engine.submit_order(req.into()).await?;
    Ok((StatusCode::CREATED, Json(order.into())))
}
```

### Custom Error Response Format

```rust
#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<serde_json::Value>,
    request_id: String,
    timestamp: i64,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
}

impl ApiError {
    fn to_error_response(&self, request_id: String) -> ErrorResponse {
        let (code, message, context, field) = match self {
            ApiError::Order(OrderError::ValidationError(msg)) => (
                "VALIDATION_ERROR",
                msg.clone(),
                Some(json!({"fields": ["price", "quantity"]})),
                None,
            ),
            ApiError::Order(OrderError::InsufficientBalance { required, available }) => (
                "INSUFFICIENT_BALANCE",
                "Not enough balance".to_string(),
                Some(json!({
                    "required": required.to_string(),
                    "available": available.to_string(),
                })),
                Some("balance".to_string()),
            ),
            // ... other errors
        };

        ErrorResponse {
            error: ErrorDetail {
                code,
                message,
                field,
            },
            context,
            request_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = uuid::Uuid::new_v4().to_string();
        let status = self.status_code();
        let body = self.to_error_response(request_id);

        (status, Json(body)).into_response()
    }
}
```

---

## 7. WebSocket Patterns

### Basic WebSocket Handler

```rust
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task to handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    tracing::info!("Received text: {}", text);
                }
                Message::Binary(data) => {
                    tracing::info!("Received binary data: {} bytes", data.len());
                }
                Message::Close(_) => {
                    tracing::info!("Client closed connection");
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn a task to send messages
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            if sender
                .send(Message::Text("Ping".to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut recv_task => send_task.abort(),
        _ = &mut send_task => recv_task.abort(),
    }
}
```

### Pub/Sub WebSocket with Broadcaster

```rust
use dashmap::DashMap;
use tokio::sync::mpsc;

#[derive(Clone)]
struct Broadcaster {
    subscribers: Arc<DashMap<String, mpsc::UnboundedSender<Message>>>,
}

impl Broadcaster {
    fn new() -> Self {
        Broadcaster {
            subscribers: Arc::new(DashMap::new()),
        }
    }

    fn subscribe(&self, id: String) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.subscribers.insert(id, tx);
        rx
    }

    fn unsubscribe(&self, id: &str) {
        self.subscribers.remove(id);
    }

    fn broadcast(&self, message: Message) {
        self.subscribers.retain(|_, tx| {
            // Remove subscriber if send fails (disconnected)
            tx.send(message.clone()).is_ok()
        });
    }

    fn send_to(&self, id: &str, message: Message) -> bool {
        if let Some(tx) = self.subscribers.get(id) {
            tx.send(message).is_ok()
        } else {
            false
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<Broadcaster>,
) -> Response {
    ws.on_upgrade(move |socket| handle_pubsub_socket(socket, broadcaster))
}

async fn handle_pubsub_socket(socket: WebSocket, broadcaster: Broadcaster) {
    let subscriber_id = uuid::Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcaster
    let mut rx = broadcaster.subscribe(subscriber_id.clone());

    // Send messages from broadcaster to client
    let subscriber_id_clone = subscriber_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }

        tracing::info!("Send task ended for {}", subscriber_id_clone);
    });

    // Receive messages from client
    let broadcaster_clone = broadcaster.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse command
                    if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                        match cmd {
                            Command::Subscribe { channel } => {
                                tracing::info!("Subscribing to {}", channel);
                            }
                            Command::Broadcast { message } => {
                                broadcaster_clone.broadcast(Message::Text(message));
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        },
        _ = &mut recv_task => {
            send_task.abort();
        },
    }

    // Cleanup
    broadcaster.unsubscribe(&subscriber_id);
    tracing::info!("Connection closed for {}", subscriber_id);
}

#[derive(Deserialize)]
#[serde(tag = "action")]
enum Command {
    #[serde(rename = "subscribe")]
    Subscribe { channel: String },
    #[serde(rename = "broadcast")]
    Broadcast { message: String },
}
```

### WebSocket with Authentication

```rust
async fn authenticated_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    user: AuthenticatedUser,  // Custom extractor validates auth before upgrade
) -> Response {
    ws.on_upgrade(move |socket| handle_authenticated_socket(socket, state, user))
}

async fn handle_authenticated_socket(
    socket: WebSocket,
    state: AppState,
    user: AuthenticatedUser,
) {
    tracing::info!("Authenticated WebSocket for user: {}", user.user_id);

    let (mut sender, mut receiver) = socket.split();

    // User-specific subscription
    let mut user_events = state.event_bus.subscribe_user(&user.user_id);

    let mut send_task = tokio::spawn(async move {
        while let Some(event) = user_events.recv().await {
            let msg = Message::Text(serde_json::to_string(&event).unwrap());
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // Handle user commands
            if let Message::Text(text) = msg {
                handle_user_command(&user, &text, &state).await;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
```

### WebSocket Room Pattern

```rust
struct Room {
    name: String,
    subscribers: DashMap<String, mpsc::UnboundedSender<Message>>,
}

impl Room {
    fn new(name: String) -> Self {
        Room {
            name,
            subscribers: DashMap::new(),
        }
    }

    fn join(&self, user_id: String) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.subscribers.insert(user_id.clone(), tx);

        // Notify others
        self.broadcast_except(
            &user_id,
            Message::Text(format!("{} joined the room", user_id)),
        );

        rx
    }

    fn leave(&self, user_id: &str) {
        self.subscribers.remove(user_id);
        self.broadcast_except(
            user_id,
            Message::Text(format!("{} left the room", user_id)),
        );
    }

    fn broadcast(&self, message: Message) {
        self.subscribers.retain(|_, tx| tx.send(message.clone()).is_ok());
    }

    fn broadcast_except(&self, except_id: &str, message: Message) {
        self.subscribers.iter().for_each(|entry| {
            if entry.key() != except_id {
                let _ = entry.value().send(message.clone());
            }
        });
    }
}

struct RoomManager {
    rooms: DashMap<String, Arc<Room>>,
}

impl RoomManager {
    fn new() -> Self {
        RoomManager {
            rooms: DashMap::new(),
        }
    }

    fn get_or_create_room(&self, name: String) -> Arc<Room> {
        self.rooms
            .entry(name.clone())
            .or_insert_with(|| Arc::new(Room::new(name)))
            .clone()
    }
}
```

---

## 8. Server-Sent Events (SSE)

### Basic SSE Implementation

```rust
use axum::{
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;

async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        Event::default()
            .data(format!("Current time: {}", chrono::Utc::now()))
    })
    .map(Ok)
    .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

### SSE with Channel

```rust
use tokio::sync::broadcast;

#[derive(Clone)]
struct SseState {
    tx: broadcast::Sender<String>,
}

async fn sse_handler(
    State(state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.tx.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().data(msg));
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// Broadcast events
async fn send_event(State(state): State<SseState>, Json(data): Json<EventData>) {
    let _ = state.tx.send(serde_json::to_string(&data).unwrap());
}
```

### Real-Time Order Book Updates via SSE

```rust
use tokio::sync::broadcast;

#[derive(Clone, Serialize)]
struct OrderBookUpdate {
    symbol: String,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
    timestamp: i64,
}

async fn orderbook_sse(
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.orderbook_updates.subscribe();

    let stream = async_stream::stream! {
        while let Ok(update) = rx.recv().await {
            // Filter by symbol
            if update.symbol == symbol {
                let data = serde_json::to_string(&update).unwrap();
                yield Ok(Event::default()
                    .event("orderbook_update")
                    .data(data));
            }
        }
    };

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        )
}

// In your router
let app = Router::new()
    .route("/api/v1/orderbook/:symbol/stream", get(orderbook_sse))
    .with_state(state);
```

### SSE with Error Recovery

```rust
async fn resilient_sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.events.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    yield Ok(Event::default().data(data));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Client is too slow, missed n messages
                    tracing::warn!("SSE client lagged by {} messages", n);
                    yield Ok(Event::default()
                        .event("error")
                        .data(format!("Missed {} updates", n)));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

---

## 9. Streaming Responses

### Stream Large File

```rust
use axum::body::Body;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

async fn stream_file() -> Result<Response, (StatusCode, String)> {
    let file = File::open("large_file.dat")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .header("content-type", "application/octet-stream")
        .header("content-disposition", "attachment; filename=\"large_file.dat\"")
        .body(body)
        .unwrap();

    Ok(response)
}
```

### Stream Generated Data

```rust
use futures::stream;

async fn stream_trades() -> impl IntoResponse {
    let stream = stream::iter(0..1000)
        .then(|i| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let trade = Trade {
                id: i,
                price: 100.0 + (i as f64 * 0.1),
                quantity: 1.0,
                timestamp: chrono::Utc::now(),
            };
            serde_json::to_string(&trade).unwrap() + "\n"
        })
        .map(Ok::<_, Infallible>);

    (
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        Body::from_stream(stream),
    )
}
```

### Stream CSV Export

```rust
async fn stream_csv_export(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        // CSV header
        yield Ok::<_, Infallible>(Bytes::from("id,symbol,price,quantity,timestamp\n"));

        // Stream data from database in chunks
        let mut offset = 0;
        let limit = 1000;

        loop {
            let orders = state.db
                .fetch_orders(offset, limit)
                .await
                .unwrap_or_default();

            if orders.is_empty() {
                break;
            }

            for order in orders {
                let line = format!(
                    "{},{},{},{},{}\n",
                    order.id,
                    order.symbol,
                    order.price,
                    order.quantity,
                    order.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
                yield Ok(Bytes::from(line));
            }

            offset += limit;
        }
    };

    (
        [
            (header::CONTENT_TYPE, "text/csv"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"orders.csv\""),
        ],
        Body::from_stream(stream),
    )
}
```

### Chunked Transfer Encoding

```rust
use futures::stream::StreamExt;

async fn stream_large_response() -> impl IntoResponse {
    let chunks = vec![
        "chunk 1\n",
        "chunk 2\n",
        "chunk 3\n",
    ];

    let stream = futures::stream::iter(chunks)
        .map(|chunk| Ok::<_, Infallible>(chunk))
        .throttle(Duration::from_millis(100));

    Body::from_stream(stream)
}
```

---

## 10. Authentication & Authorization

### JWT Authentication

```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // Subject (user ID)
    exp: usize,   // Expiration time
    iat: usize,   // Issued at
    role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Role {
    Admin,
    Trader,
    Viewer,
}

struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtAuth {
    fn new(secret: &str) -> Self {
        JwtAuth {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    fn generate_token(&self, user_id: String, role: Role) -> Result<String, ApiError> {
        let now = chrono::Utc::now();
        let exp = (now + chrono::Duration::hours(24)).timestamp() as usize;

        let claims = Claims {
            sub: user_id,
            exp,
            iat: now.timestamp() as usize,
            role,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| ApiError::Internal("Token generation failed".into()))
    }

    fn validate_token(&self, token: &str) -> Result<Claims, ApiError> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|_| ApiError::Unauthorized)
    }
}

// Custom extractor for authenticated user
struct AuthenticatedUser {
    user_id: String,
    role: Role,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        // Get JWT auth from extensions (added by middleware)
        let jwt_auth = parts
            .extensions
            .get::<Arc<JwtAuth>>()
            .ok_or(ApiError::Internal("JWT auth not configured".into()))?;

        let claims = jwt_auth.validate_token(token)?;

        Ok(AuthenticatedUser {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

// Login endpoint
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let user = state.user_service
        .authenticate(&req.username, &req.password)
        .await?;

    let token = state.jwt_auth.generate_token(user.id, user.role)?;

    Ok(Json(LoginResponse { token }))
}

// Protected endpoint
async fn create_order(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderResponse>, ApiError> {
    // User is authenticated, proceed
    tracing::info!("User {} creating order", user.user_id);

    let order = state.engine.submit_order(req.into()).await?;
    Ok(Json(order.into()))
}
```

### Role-Based Access Control (RBAC)

```rust
struct RequireRole(Role);

#[async_trait]
impl<S> FromRequestParts<S> for RequireRole
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;

        // Check if user has required role
        if user.role == Role::Admin {
            Ok(RequireRole(user.role))
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

// Admin-only endpoint
async fn admin_endpoint(
    _role: RequireRole,  // Will fail if not admin
    State(state): State<AppState>,
) -> Json<SystemStats> {
    Json(state.get_system_stats())
}
```

### Permission-Based Authorization

```rust
#[derive(Debug, Clone, PartialEq)]
enum Permission {
    CreateOrder,
    CancelOrder,
    ViewOrders,
    ManageUsers,
    SystemAdmin,
}

struct RequirePermission(Permission);

#[async_trait]
impl<S> FromRequestParts<S> for RequirePermission
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;

        // Get required permission from request extensions
        let required_permission = parts
            .extensions
            .get::<Permission>()
            .ok_or(ApiError::Internal("Permission not set".into()))?;

        // Check if user has permission
        if has_permission(&user, required_permission) {
            Ok(RequirePermission(required_permission.clone()))
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

fn has_permission(user: &AuthenticatedUser, permission: &Permission) -> bool {
    match user.role {
        Role::Admin => true,  // Admin has all permissions
        Role::Trader => matches!(
            permission,
            Permission::CreateOrder | Permission::CancelOrder | Permission::ViewOrders
        ),
        Role::Viewer => matches!(permission, Permission::ViewOrders),
    }
}

// Use middleware to set required permission
async fn permission_middleware<B>(
    mut req: Request<B>,
    next: Next<B>,
    permission: Permission,
) -> Response {
    req.extensions_mut().insert(permission);
    next.run(req).await
}

// Router with permission-based authorization
let app = Router::new()
    .route("/orders", post(create_order))
    .layer(middleware::from_fn(move |req, next| {
        permission_middleware(req, next, Permission::CreateOrder)
    }));
```

---

## 11. Rate Limiting & Throttling

### Token Bucket Rate Limiter

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,  // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        TokenBucket {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }
}

#[derive(Clone)]
struct RateLimiter {
    buckets: Arc<DashMap<String, Arc<Mutex<TokenBucket>>>>,
    capacity: f64,
    refill_rate: f64,
}

impl RateLimiter {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        RateLimiter {
            buckets: Arc::new(DashMap::new()),
            capacity,
            refill_rate,
        }
    }

    async fn check(&self, key: String) -> bool {
        let bucket = self
            .buckets
            .entry(key)
            .or_insert_with(|| {
                Arc::new(Mutex::new(TokenBucket::new(self.capacity, self.refill_rate)))
            })
            .clone();

        let mut bucket = bucket.lock().await;
        bucket.try_acquire(1.0)
    }
}

// Middleware
async fn rate_limit_middleware<B>(
    State(limiter): State<RateLimiter>,
    client: ClientInfo,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, ApiError> {
    if !limiter.check(client.ip).await {
        return Err(ApiError::RateLimitExceeded);
    }

    Ok(next.run(req).await)
}

let app = Router::new()
    .route("/api/orders", post(create_order))
    .layer(middleware::from_fn_with_state(
        rate_limiter.clone(),
        rate_limit_middleware,
    ))
    .with_state(app_state);
```

### Sliding Window Rate Limiter

```rust
use std::collections::VecDeque;

struct SlidingWindow {
    requests: VecDeque<Instant>,
    capacity: usize,
    window: Duration,
}

impl SlidingWindow {
    fn new(capacity: usize, window: Duration) -> Self {
        SlidingWindow {
            requests: VecDeque::new(),
            capacity,
            window,
        }
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        // Remove old requests outside the window
        while let Some(&timestamp) = self.requests.front() {
            if timestamp < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }

        if self.requests.len() < self.capacity {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }
}
```

### Per-User Rate Limiting with Tiers

```rust
#[derive(Clone)]
enum RateLimitTier {
    Free { requests_per_minute: usize },
    Pro { requests_per_minute: usize },
    Enterprise { requests_per_minute: usize },
}

impl RateLimitTier {
    fn limit(&self) -> usize {
        match self {
            RateLimitTier::Free { requests_per_minute } => *requests_per_minute,
            RateLimitTier::Pro { requests_per_minute } => *requests_per_minute,
            RateLimitTier::Enterprise { requests_per_minute } => *requests_per_minute,
        }
    }
}

async fn tiered_rate_limit_middleware<B>(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, ApiError> {
    let tier = state.user_service.get_tier(&user.user_id).await?;
    let limit = tier.limit();

    let rate_limiter = RateLimiter::new(limit as f64, limit as f64 / 60.0);

    if !rate_limiter.check(user.user_id.clone()).await {
        return Err(ApiError::RateLimitExceeded);
    }

    Ok(next.run(req).await)
}
```

---

*(Continued in next part due to length...)*

## 12. Request/Response Interceptors

### Request Logger

```rust
async fn request_logger<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let start = Instant::now();

    tracing::debug!(
        method = %method,
        uri = %uri,
        headers = ?headers,
        "Incoming request"
    );

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}
```

### Response Transformer

```rust
async fn add_server_header<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "X-Server-Version",
        HeaderValue::from_static("1.0.0"),
    );

    response.headers_mut().insert(
        "X-Request-Id",
        HeaderValue::from_str(&uuid::Uuid::new_v4().to_string()).unwrap(),
    );

    response
}
```

### CORS Handler

```rust
async fn cors_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    // Handle preflight
    if req.method() == Method::OPTIONS {
        return Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
            .header("Access-Control-Max-Age", "86400")
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap();
    }

    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "Access-Control-Allow-Origin",
        HeaderValue::from_static("*"),
    );

    response
}
```

---

## 13. Testing Strategies

### Unit Testing Handlers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;  // For oneshot

    #[tokio::test]
    async fn test_create_order() {
        let state = AppState {
            engine: Arc::new(OrderBookEngine::new()),
        };

        let app = Router::new()
            .route("/orders", post(create_order))
            .with_state(state);

        let request = Request::builder()
            .method("POST")
            .uri("/orders")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"symbol":"XAUUSD","price":2000,"quantity":10}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum_test_helper::TestClient;

    #[tokio::test]
    async fn test_order_workflow() {
        let app = create_app();
        let client = TestClient::new(app);

        // Create order
        let response = client
            .post("/api/v1/orders")
            .json(&json!({
                "symbol": "XAUUSD",
                "price": 2000,
                "quantity": 10
            }))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::CREATED);

        let order: OrderResponse = response.json().await;

        // Get order
        let response = client
            .get(&format!("/api/v1/orders/{}", order.id))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Cancel order
        let response = client
            .delete(&format!("/api/v1/orders/{}", order.id))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

### Mocking Dependencies

```rust
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        OrderBookEngine {}

        impl OrderBookEngine {
            async fn submit_order(&self, order: Order) -> Result<Trade, OrderError>;
            async fn cancel_order(&self, id: &str) -> Result<Order, OrderError>;
        }
    }

    #[tokio::test]
    async fn test_with_mock() {
        let mut mock_engine = MockOrderBookEngine::new();

        mock_engine
            .expect_submit_order()
            .times(1)
            .returning(|_| {
                Ok(Trade {
                    id: "trade-1".into(),
                    price: dec!(2000),
                    quantity: dec!(10),
                })
            });

        let state = AppState {
            engine: Arc::new(mock_engine),
        };

        // Test handler with mocked engine
    }
}
```

---

## 14. Performance Optimization

### Connection Pooling

```rust
#[derive(Clone)]
struct AppState {
    // Database connection pool
    db: PgPool,

    // HTTP client with connection pooling
    http_client: reqwest::Client,

    // Redis connection pool
    redis: redis::aio::ConnectionManager,
}

async fn create_app_state() -> AppState {
    let db = PgPoolOptions::new()
        .max_connections(100)
        .min_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let redis_client = redis::Client::open(env::var("REDIS_URL").unwrap()).unwrap();
    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .unwrap();

    AppState {
        db,
        http_client,
        redis,
    }
}
```

### Response Compression

```rust
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .route("/api/orders", get(list_orders))
    .layer(CompressionLayer::new());  // Automatically compresses responses
```

### Caching Middleware

```rust
use dashmap::DashMap;

#[derive(Clone)]
struct CacheLayer {
    cache: Arc<DashMap<String, (Instant, Bytes)>>,
    ttl: Duration,
}

impl CacheLayer {
    fn new(ttl: Duration) -> Self {
        CacheLayer {
            cache: Arc::new(DashMap::new()),
            ttl,
        }
    }

    async fn get_or_compute<F, Fut>(
        &self,
        key: String,
        compute: F,
    ) -> Bytes
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Bytes>,
    {
        // Check cache
        if let Some(entry) = self.cache.get(&key) {
            let (timestamp, value) = entry.value();
            if timestamp.elapsed() < self.ttl {
                return value.clone();
            }
        }

        // Compute value
        let value = compute().await;

        // Store in cache
        self.cache.insert(key, (Instant::now(), value.clone()));

        value
    }
}
```

### Batching Requests

```rust
use tokio::sync::mpsc;

struct BatchProcessor {
    tx: mpsc::UnboundedSender<(Order, oneshot::Sender<Result<Trade, Error>>)>,
}

impl BatchProcessor {
    fn new(engine: Arc<OrderBookEngine>) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut interval = tokio::time::interval(Duration::from_millis(10));

            loop {
                tokio::select! {
                    Some(item) = rx.recv() => {
                        batch.push(item);

                        if batch.len() >= 100 {
                            process_batch(&engine, &mut batch).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            process_batch(&engine, &mut batch).await;
                        }
                    }
                }
            }
        });

        BatchProcessor { tx }
    }

    async fn submit(&self, order: Order) -> Result<Trade, Error> {
        let (tx, rx) = oneshot::channel();
        self.tx.send((order, tx)).ok();
        rx.await.unwrap()
    }
}

async fn process_batch(
    engine: &OrderBookEngine,
    batch: &mut Vec<(Order, oneshot::Sender<Result<Trade, Error>>)>,
) {
    for (order, response_tx) in batch.drain(..) {
        let result = engine.submit_order(order).await;
        response_tx.send(result).ok();
    }
}
```

---

## 15. Production Patterns

### Graceful Shutdown

```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}

#[tokio::main]
async fn main() {
    let app = create_app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    tracing::info!("Server shutdown complete");
}
```

### Health Checks

```rust
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime: u64,
    checks: HashMap<String, CheckResult>,
}

#[derive(Serialize)]
struct CheckResult {
    status: String,
    message: Option<String>,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let mut checks = HashMap::new();

    // Check database
    let db_status = match state.db.acquire().await {
        Ok(_) => CheckResult {
            status: "healthy".into(),
            message: None,
        },
        Err(e) => CheckResult {
            status: "unhealthy".into(),
            message: Some(e.to_string()),
        },
    };
    checks.insert("database".into(), db_status);

    // Check Redis
    let redis_status = match redis::cmd("PING")
        .query_async::<_, String>(&mut state.redis.clone())
        .await
    {
        Ok(_) => CheckResult {
            status: "healthy".into(),
            message: None,
        },
        Err(e) => CheckResult {
            status: "unhealthy".into(),
            message: Some(e.to_string()),
        },
    };
    checks.insert("redis".into(), redis_status);

    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime: get_uptime_seconds(),
        checks,
    })
}
```

### Metrics Endpoint

```rust
async fn metrics_handler(
    State(metrics): State<Arc<MetricsRegistry>>,
) -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = metrics.registry.gather();
    let mut buffer = vec![];

    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .header("content-type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}
```

---

## 16. Trading System Examples

### Complete Order Management API

```rust
// See full implementation in repository

fn create_trading_api() -> Router {
    let state = AppState::new();

    Router::new()
        // Public routes
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))

        // Authentication
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))

        // Order management (authenticated)
        .route("/api/v1/orders", post(create_order).get(list_orders))
        .route(
            "/api/v1/orders/:id",
            get(get_order)
                .patch(modify_order)
                .delete(cancel_order),
        )

        // Order book
        .route("/api/v1/orderbook/:symbol", get(get_orderbook))
        .route("/api/v1/orderbook/:symbol/stream", get(orderbook_sse))

        // Trades
        .route("/api/v1/trades", get(list_trades))
        .route("/api/v1/trades/:symbol", get(get_symbol_trades))

        // WebSocket
        .route("/ws", get(websocket_handler))

        // Admin routes
        .nest("/admin", admin_routes())

        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive())
                .layer(TimeoutLayer::new(Duration::from_secs(30))),
        )
        .with_state(state)
}
```

This guide provides production-ready patterns for building high-performance trading systems with Axum!

