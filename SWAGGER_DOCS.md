# Swagger/OpenAPI Documentation

The Order Book API includes comprehensive OpenAPI documentation with an interactive Swagger UI interface and support for multiple API versions.

## Accessing the Documentation

Once the server is running (`cargo run`), you can access:

### Interactive Swagger UI
**URL:** http://127.0.0.1:3000/swagger-ui

The Swagger UI provides an interactive interface where you can:
- Browse all available endpoints
- See detailed request/response schemas
- Try out API calls directly from the browser
- **Switch between API versions** using the dropdown in the top-right corner

![Swagger UI](https://via.placeholder.com/800x400?text=Swagger+UI+with+Version+Selector)

### Version Selection

The API documentation supports multiple versions:
- **v1.0** - Current stable version
- **v2.0** - Future version with enhanced features

To switch versions:
1. Open the Swagger UI at http://127.0.0.1:3000/swagger-ui
2. Look for the dropdown menu in the top-right corner
3. Select the version you want to explore

## OpenAPI JSON Specifications

You can also access the raw OpenAPI JSON specifications:

- **v1.0 Spec:** http://127.0.0.1:3000/api-docs/v1/openapi.json
- **v2.0 Spec:** http://127.0.0.1:3000/api-docs/v2/openapi.json

These JSON files can be imported into tools like:
- Postman
- Insomnia
- OpenAPI Generator
- Swagger Editor

## API Endpoints Documented

### Health
- `GET /health` - Health check endpoint

### Orders
- `POST /api/v1/orders` - Submit a new order
- `GET /api/v1/orders/{symbol}/{order_id}` - Get order status
- `DELETE /api/v1/orders/{symbol}/{order_id}` - Cancel an order

### Order Book
- `GET /api/v1/orderbook/{symbol}` - Get order book with depth
- `GET /api/v1/orderbook/{symbol}/spread` - Get spread metrics

### Trades
- `GET /api/v1/trades/{symbol}` - Get recent trades

### Metrics
- `GET /api/v1/metrics/exchange` - Get exchange-wide metrics

## Using the Swagger UI

### 1. Try Out an Endpoint

1. Navigate to http://127.0.0.1:3000/swagger-ui
2. Click on any endpoint (e.g., "POST /api/v1/orders")
3. Click the "Try it out" button
4. Fill in the request body:
   ```json
   {
     "symbol": "AAPL",
     "side": "buy",
     "order_type": "limit",
     "price": 150.50,
     "quantity": 100,
     "user_id": "user123"
   }
   ```
5. Click "Execute"
6. View the response below

### 2. View Schemas

- Scroll down to the "Schemas" section at the bottom
- Click on any schema (e.g., "SubmitOrderRequest") to see its structure
- All fields include descriptions and data types

### 3. Explore Response Examples

Each endpoint shows:
- Possible HTTP status codes (200, 201, 400, 404, etc.)
- Response schema for each status
- Example responses

## Example: Submit Order via Swagger UI

```
1. Open Swagger UI
   → http://127.0.0.1:3000/swagger-ui

2. Expand "Orders" tag
   → Click on "POST /api/v1/orders"

3. Click "Try it out"

4. Enter request body:
   {
     "symbol": "AAPL",
     "side": "sell",
     "order_type": "limit",
     "price": 150.50,
     "quantity": 100,
     "user_id": "seller1"
   }

5. Click "Execute"

6. See response:
   Status: 201 Created
   Response Body:
   {
     "order_id": "uuid-here",
     "status": "new",
     "filled_quantity": "0",
     "trades": [],
     "timestamp": "2025-11-12T10:30:00Z"
   }
```

## Importing to Postman

1. Copy the OpenAPI JSON URL:
   ```
   http://127.0.0.1:3000/api-docs/v1/openapi.json
   ```

2. In Postman:
   - Click "Import"
   - Select "Link"
   - Paste the URL
   - Click "Continue"

3. All endpoints will be imported as a collection

## Generating Client Code

Use OpenAPI Generator to create client libraries:

```bash
# Generate Python client
openapi-generator-cli generate \
  -i http://127.0.0.1:3000/api-docs/v1/openapi.json \
  -g python \
  -o ./generated-clients/python

# Generate TypeScript client
openapi-generator-cli generate \
  -i http://127.0.0.1:3000/api-docs/v1/openapi.json \
  -g typescript-axios \
  -o ./generated-clients/typescript
```

## API Tags

The documentation is organized by tags:

- **Health** - Service health and status
- **Orders** - Order submission, retrieval, and cancellation
- **Order Book** - Market data and order book snapshots
- **Trades** - Trade history and execution records
- **Metrics** - Exchange-wide statistics and metrics

## Schema Definitions

All data models are fully documented with field descriptions:

### Key Schemas

- **SubmitOrderRequest** - Request to create a new order
  - symbol: Trading symbol (e.g., "AAPL")
  - side: "buy" or "sell"
  - order_type: "limit" or "market"
  - price: Optional price (required for limit orders)
  - quantity: Order quantity
  - user_id: User identifier

- **SubmitOrderResponse** - Response after submitting an order
  - order_id: Unique order identifier
  - status: Order status ("new", "partially_filled", "filled", "cancelled")
  - filled_quantity: Amount already filled
  - trades: List of executed trades
  - timestamp: Order creation time

- **OrderBookResponse** - Order book snapshot
  - symbol: Trading symbol
  - bids: Buy orders (price, quantity, order count)
  - asks: Sell orders (price, quantity, order count)
  - best_bid, best_ask: Best prices
  - spread: Bid-ask spread
  - mid_price: Mid-market price

## Version Differences

### v1.0 (Current)
- All Phase 1 features
- Limit orders
- Order book management
- Trade execution
- Fee calculations
- Market metrics

### v2.0 (Future)
- Everything from v1.0
- Market orders (planned)
- Advanced analytics (planned)
- WebSocket support (planned)
- Enhanced metrics (planned)

## Technical Details

### Technology Stack
- **utoipa** - OpenAPI code generation
- **utoipa-swagger-ui** - Interactive Swagger UI
- **Rust derives** - Automatic schema generation from Rust types

### Automatic Documentation
All API documentation is generated automatically from:
- Rust struct definitions (`#[derive(ToSchema)]`)
- Handler function annotations (`#[utoipa::path(...)]`)
- Inline code comments

This ensures documentation is always in sync with the implementation!

## Troubleshooting

### Swagger UI not loading
- Ensure the server is running: `cargo run`
- Check the correct URL: http://127.0.0.1:3000/swagger-ui
- Try with trailing slash: http://127.0.0.1:3000/swagger-ui/

### API calls failing from Swagger UI
- Make sure the server is running on port 3000
- Check that you're using the correct request format
- Look at the response to see error details

### Version selector not showing
- Refresh the page
- Clear browser cache
- Check that both version endpoints are accessible:
  - http://127.0.0.1:3000/api-docs/v1/openapi.json
  - http://127.0.0.1:3000/api-docs/v2/openapi.json

## Benefits of OpenAPI Documentation

✅ **Interactive** - Test endpoints directly from browser
✅ **Version Control** - Switch between API versions
✅ **Always Up-to-Date** - Generated from source code
✅ **Client Generation** - Auto-generate client libraries
✅ **Standards Compliant** - OpenAPI 3.0 specification
✅ **Tool Integration** - Works with Postman, Insomnia, etc.

## Quick Links

When server is running:

- **Swagger UI:** http://127.0.0.1:3000/swagger-ui
- **Health Check:** http://127.0.0.1:3000/health
- **OpenAPI v1:** http://127.0.0.1:3000/api-docs/v1/openapi.json
- **OpenAPI v2:** http://127.0.0.1:3000/api-docs/v2/openapi.json
