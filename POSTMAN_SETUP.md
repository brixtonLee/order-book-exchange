# Postman Collection Setup Guide

## 📦 **Import Instructions**

### **1. Import the Collection**
1. Open Postman
2. Click **Import** button (top left)
3. Select **File** tab
4. Choose `postman_collection.json` from this directory
5. Click **Import**

### **2. Environment Setup (Optional)**
The collection includes a default environment variable:
- `base_url`: `http://127.0.0.1:3000`

If you want to test against a different server:
1. Click **Environments** (left sidebar)
2. Create **New Environment**
3. Add variable:
   - Key: `base_url`
   - Value: `http://your-server:port`
4. Select the environment (top right dropdown)

---

## 🧪 **Test Collection Structure**

### **Folders:**

1. **Setup & Health Check** (2 tests)
   - Health check
   - Exchange metrics

2. **Phase 1 - Baseline Tests** (2 tests)
   - Basic limit order (sell)
   - Matching buy order

3. **Phase 2 - Post-Only Orders** (3 tests)
   - Setup sell order
   - Post-only rejection (would match)
   - Post-only acceptance (adds liquidity)

4. **Phase 2 - Self-Trade Prevention** (2 tests)
   - Setup sell order from MM1
   - Buy from same user (STP prevents match)

5. **Phase 2 - IOC Orders** (2 tests)
   - Setup partial liquidity
   - IOC partial fill test

6. **Phase 2 - FOK Orders** (4 tests)
   - Setup insufficient liquidity
   - FOK rejection
   - Setup sufficient liquidity
   - FOK acceptance

7. **Phase 2 - Market Orders & VWAP** (2 tests)
   - Setup multiple price levels
   - Market order multi-level match

8. **Phase 2 - Get Order with New Fields** (2 tests)
   - Create order with Phase 2 fields
   - Verify fields in response

9. **Phase 2 - Order Book & Trades** (2 tests)
   - Get order book
   - Get recent trades

---

## 🚀 **Running the Tests**

### **Option 1: Run Entire Collection**
1. Click on collection name
2. Click **Run** button
3. Click **Run Order Book API - Phase 2 Features**
4. Watch tests execute automatically

### **Option 2: Run Individual Folders**
1. Right-click on folder (e.g., "Phase 2 - Post-Only Orders")
2. Click **Run folder**

### **Option 3: Run Individual Requests**
1. Click on any request
2. Click **Send**
3. View **Test Results** tab at bottom

---

## ✅ **What Each Test Validates**

### **Post-Only Tests:**
```javascript
// Test 1: Rejection when would match
pm.test('Status code is 400', ...);
pm.test('Error message indicates post-only rejection', ...);

// Test 2: Acceptance when adds liquidity
pm.test('Status code is 201', ...);
pm.test('Post-only order accepted', ...);
```

### **Self-Trade Prevention:**
```javascript
pm.test('No self-trade occurred', function () {
    const jsonData = pm.response.json();
    pm.expect(jsonData.trades).to.be.empty;
    pm.expect(jsonData.filled_quantity).to.eql('0');
});
```

### **IOC Orders:**
```javascript
pm.test('IOC order partially filled', function () {
    const jsonData = pm.response.json();
    pm.expect(jsonData.status).to.eql('partially_filled');
    pm.expect(jsonData.filled_quantity).to.eql('50');
});
```

### **FOK Orders:**
```javascript
// Rejection test
pm.test('FOK rejected due to insufficient liquidity', ...);

// Acceptance test
pm.test('FOK order completely filled', function () {
    const jsonData = pm.response.json();
    pm.expect(jsonData.status).to.eql('filled');
    pm.expect(jsonData.filled_quantity).to.eql('100');
});
```

### **VWAP Calculation:**
```javascript
pm.test('Calculate VWAP', function () {
    const jsonData = pm.response.json();
    let totalValue = 0;
    let totalQuantity = 0;

    jsonData.trades.forEach(trade => {
        totalValue += parseFloat(trade.price) * parseFloat(trade.quantity);
        totalQuantity += parseFloat(trade.quantity);
    });

    const vwap = totalValue / totalQuantity;
    pm.expect(vwap).to.be.closeTo(300.30, 0.01);
});
```

---

## 📊 **Expected Results**

When you run the full collection with the server running, you should see:

```
✓ Status code is 200
✓ Response contains status
✓ Status code is 200
✓ Response has required metrics
✓ Status code is 201
✓ Order created successfully
✓ Status code is 201
✓ Order filled with trade
✓ Trade executed at correct price
✓ Fees calculated correctly
✓ Setup order created
✓ Status code is 400 (Bad Request)
✓ Error message indicates post-only rejection
✓ Status code is 201
✓ Post-only order accepted (adds liquidity)
... (40+ more tests)

Tests:    45/45 passed
Duration: ~2-3 seconds
```

---

## 🔍 **Understanding Request Bodies**

### **Basic Order (Phase 1):**
```json
{
  "symbol": "AAPL",
  "side": "buy",
  "order_type": "limit",
  "price": 150.50,
  "quantity": 100,
  "user_id": "buyer1"
}
```

### **Post-Only Order (Phase 2):**
```json
{
  "symbol": "GOOGL",
  "side": "buy",
  "order_type": "limit",
  "price": 154.50,
  "quantity": 50,
  "user_id": "buyer1",
  "post_only": true  // ← NEW
}
```

### **Self-Trade Prevention (Phase 2):**
```json
{
  "symbol": "MSFT",
  "side": "buy",
  "order_type": "limit",
  "price": 370.00,
  "quantity": 50,
  "user_id": "mm1",
  "stp_mode": "CANCEL_RESTING"  // ← NEW
}
```

### **IOC Order (Phase 2):**
```json
{
  "symbol": "TSLA",
  "side": "buy",
  "order_type": "limit",
  "price": 200.00,
  "quantity": 100,
  "user_id": "buyer1",
  "time_in_force": "IOC"  // ← NEW
}
```

### **FOK Order (Phase 2):**
```json
{
  "symbol": "NVDA",
  "side": "buy",
  "order_type": "limit",
  "price": 500.00,
  "quantity": 100,
  "user_id": "buyer1",
  "time_in_force": "FOK"  // ← NEW
}
```

### **Market Order (Phase 2 Enhanced):**
```json
{
  "symbol": "META",
  "side": "buy",
  "order_type": "market",  // No price needed
  "quantity": 250,
  "user_id": "buyer1"
}
```

### **Complete Phase 2 Order:**
```json
{
  "symbol": "AMZN",
  "side": "buy",
  "order_type": "limit",
  "price": 180.00,
  "quantity": 100,
  "user_id": "trader1",
  "time_in_force": "GTC",          // ← NEW
  "stp_mode": "CANCEL_RESTING",    // ← NEW
  "post_only": false,              // ← NEW
  "expire_time": null              // ← NEW (optional)
}
```

---

## 📋 **Expected Response Examples**

### **Successful Order Creation:**
```json
{
  "order_id": "123e4567-e89b-12d3-a456-426614174000",
  "status": "new",
  "filled_quantity": "0",
  "trades": [],
  "timestamp": "2025-11-19T10:30:00Z"
}
```

### **Order with Trade:**
```json
{
  "order_id": "223e4567-e89b-12d3-a456-426614174001",
  "status": "filled",
  "filled_quantity": "50",
  "trades": [
    {
      "trade_id": "323e4567-e89b-12d3-a456-426614174002",
      "price": "150.50",
      "quantity": "50",
      "maker_fee": "7.525",
      "taker_fee": "15.05",
      "timestamp": "2025-11-19T10:30:01Z"
    }
  ],
  "timestamp": "2025-11-19T10:30:01Z"
}
```

### **Post-Only Rejection:**
```json
{
  "error": "Bad Request",
  "message": "Post-only order would match immediately"
}
```

### **FOK Rejection:**
```json
{
  "error": "Bad Request",
  "message": "Fill-or-kill order cannot be completely filled"
}
```

### **Market Order with Multiple Fills:**
```json
{
  "order_id": "a23e4567-e89b-12d3-a456-426614174009",
  "status": "filled",
  "filled_quantity": "250",
  "trades": [
    {
      "trade_id": "b23e4567-e89b-12d3-a456-42661417400a",
      "price": "300.00",
      "quantity": "100",
      "maker_fee": "30.00",
      "taker_fee": "60.00",
      "timestamp": "2025-11-19T10:35:00Z"
    },
    {
      "trade_id": "c23e4567-e89b-12d3-a456-42661417400b",
      "price": "300.50",
      "quantity": "100",
      "maker_fee": "30.05",
      "taker_fee": "60.10",
      "timestamp": "2025-11-19T10:35:00Z"
    },
    {
      "trade_id": "d23e4567-e89b-12d3-a456-42661417400c",
      "price": "301.00",
      "quantity": "50",
      "maker_fee": "15.05",
      "taker_fee": "30.10",
      "timestamp": "2025-11-19T10:35:00Z"
    }
  ],
  "timestamp": "2025-11-19T10:35:00Z"
}
```

---

## 🐛 **Troubleshooting**

### **"Could not get any response" Error:**
- Ensure server is running: `cargo run --release`
- Check base_url is correct: `http://127.0.0.1:3000`
- Verify server is listening on correct port

### **Test Failures:**
- **FOK tests might fail** if you run collection multiple times (liquidity consumed)
- **Solution:** Restart server between full collection runs
- Or run tests individually/by folder

### **Variables Not Found:**
- Ensure you're running tests in order
- Some tests depend on previous tests (e.g., order IDs)
- Use **Run Collection** to execute in sequence

---

## 💡 **Tips**

1. **View Console Logs:**
   - Click **Console** button (bottom left)
   - See detailed request/response logs

2. **Save Responses:**
   - Click **Save Response** after sending request
   - Create example responses for documentation

3. **Export Results:**
   - After running collection, click **Export Results**
   - Share test results with team

4. **Custom Scripts:**
   - All tests use JavaScript
   - Modify in **Tests** tab of each request
   - Use `pm.test()` for assertions

---

## 📚 **Next Steps**

1. **Import Collection** → `postman_collection.json`
2. **Start Server** → `cargo run --release`
3. **Run Tests** → Click "Run" on collection
4. **Verify** → All 45+ tests should pass ✅

---

**Happy Testing! 🚀**
