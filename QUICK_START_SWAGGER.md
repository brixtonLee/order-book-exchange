# Quick Start: Swagger UI

## 🚀 Step 1: Start the Server

```bash
cargo run
```

You should see output like:
```
Order Book API server running on http://127.0.0.1:3000
Health check: http://127.0.0.1:3000/health
Swagger UI: http://127.0.0.1:3000/swagger-ui
API Docs (v1): http://127.0.0.1:3000/api-docs/v1/openapi.json
API Docs (v2): http://127.0.0.1:3000/api-docs/v2/openapi.json
```

## 📖 Step 2: Open Swagger UI

Open your browser and navigate to:
```
http://127.0.0.1:3000/swagger-ui
```

## 🔄 Step 3: Select API Version

In the top-right corner of the Swagger UI, you'll see a dropdown menu:
- Click on it
- Select either **v1.0** or **v2.0**

## 🎯 Step 4: Try an Endpoint

### Example: Submit a Sell Order

1. **Find the endpoint:**
   - Look for the "Orders" tag
   - Click on `POST /api/v1/orders`

2. **Click "Try it out"** button on the right

3. **Enter request body:**
   ```json
   {
     "symbol": "AAPL",
     "side": "sell",
     "order_type": "limit",
     "price": 150.50,
     "quantity": 100,
     "user_id": "seller1"
   }
   ```

4. **Click "Execute"** button

5. **View the response:**
   - Scroll down to see the response
   - Status code: `201 Created`
   - Response body with order details

## 📊 Step 5: View the Order Book

1. Find `GET /api/v1/orderbook/{symbol}`
2. Click "Try it out"
3. Enter symbol: `AAPL`
4. Enter depth (optional): `10`
5. Click "Execute"
6. See the current order book state!

## 🔍 Step 6: Explore Schemas

Scroll to the bottom of the Swagger UI to see all data models:
- **SubmitOrderRequest** - How to format order submissions
- **OrderBookResponse** - Structure of order book data
- **TradeResponse** - Trade execution details
- And many more!

## 💡 Tips

### Quick Tests
- **Health Check**: `GET /health` - No parameters needed
- **Exchange Metrics**: `GET /api/v1/metrics/exchange` - See system stats
- **Order Book**: `GET /api/v1/orderbook/AAPL` - View current market

### Common Workflow
1. Submit sell order (creates liquidity)
2. Submit buy order at same/better price (matches!)
3. View trades to see execution
4. Check order book to see remaining orders
5. View exchange metrics to see totals

### Understanding Responses

**201 Created** = Order successfully submitted
**200 OK** = Request successful
**400 Bad Request** = Invalid input (check error message)
**404 Not Found** = Order/symbol doesn't exist
**409 Conflict** = Duplicate order ID

## 🎨 Features to Try

### 1. Matching Orders
```json
// First, create a sell order
{
  "symbol": "AAPL",
  "side": "sell",
  "order_type": "limit",
  "price": 150.00,
  "quantity": 100,
  "user_id": "seller1"
}

// Then, create a matching buy order
{
  "symbol": "AAPL",
  "side": "buy",
  "order_type": "limit",
  "price": 150.00,
  "quantity": 50,
  "user_id": "buyer1"
}
```

You'll see the trade execution in the response!

### 2. Spread Analysis
After creating bid and ask orders:
```
GET /api/v1/orderbook/AAPL/spread
```

See:
- Best bid/ask prices
- Spread in absolute, percentage, and basis points
- Market depth on each side

### 3. Trade History
```
GET /api/v1/trades/AAPL?limit=10
```

View recent executions with:
- Trade price and quantity
- Buyer and seller IDs
- Maker and taker fees
- Timestamps

## 🔧 Troubleshooting

**Swagger UI not loading?**
- Check server is running: `cargo run`
- Try: http://127.0.0.1:3000/swagger-ui/

**Can't execute requests?**
- Ensure no firewall blocking port 3000
- Check server logs for errors

**Version dropdown not showing?**
- Refresh the page
- Clear browser cache

## 🎓 Learning the API

The best way to learn the API is through Swagger UI:

1. **Start simple** - Try health check
2. **Submit orders** - Create buy/sell orders
3. **View market data** - Check order book
4. **Explore schemas** - Understand data structures
5. **Test error cases** - See how validation works

## 📚 Next Steps

- Read [SWAGGER_DOCS.md](SWAGGER_DOCS.md) for detailed documentation
- Check [README.md](README.md) for curl examples
- Run `./test_api.sh` for automated testing
- Import OpenAPI JSON into Postman

Happy trading! 📈
