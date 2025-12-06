#!/bin/bash

# Test script for Order Book API
# Make sure the server is running: cargo run

BASE_URL="http://127.0.0.1:3000"

echo "========================================"
echo "Order Book API Test Script"
echo "========================================"
echo ""

echo "1. Health Check"
echo "----------------------------------------"
curl -s "$BASE_URL/health" | jq .
echo ""
echo ""

echo "2. Submit Sell Order (AAPL @ 150.50, qty: 100)"
echo "----------------------------------------"
SELL_ORDER=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "sell",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 100,
    "user_id": "seller1"
  }')
echo "$SELL_ORDER" | jq .
SELL_ORDER_ID=$(echo "$SELL_ORDER" | jq -r '.order_id')
echo ""
echo ""

echo "3. Submit Another Sell Order (AAPL @ 150.55, qty: 50)"
echo "----------------------------------------"
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "sell",
    "order_type": "limit",
    "price": 150.55,
    "quantity": 50,
    "user_id": "seller2"
  }' | jq .
echo ""
echo ""

echo "4. Submit Buy Order (AAPL @ 150.45, qty: 75)"
echo "----------------------------------------"
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "buy",
    "order_type": "limit",
    "price": 150.45,
    "quantity": 75,
    "user_id": "buyer1"
  }' | jq .
echo ""
echo ""

echo "5. View Order Book"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/orderbook/AAPL?depth=10" | jq .
echo ""
echo ""

echo "6. Get Spread Metrics"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/orderbook/AAPL/spread" | jq .
echo ""
echo ""

echo "7. Submit Matching Buy Order (AAPL @ 150.50, qty: 30)"
echo "----------------------------------------"
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "buy",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 30,
    "user_id": "buyer2"
  }' | jq .
echo ""
echo ""

echo "8. View Recent Trades"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/trades/AAPL?limit=10" | jq .
echo ""
echo ""

echo "9. View Updated Order Book"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/orderbook/AAPL?depth=10" | jq .
echo ""
echo ""

echo "10. Get Exchange Metrics"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/metrics/exchange" | jq .
echo ""
echo ""

echo "11. Get Order Status"
echo "----------------------------------------"
curl -s "$BASE_URL/api/v1/orders/AAPL/$SELL_ORDER_ID" | jq .
echo ""
echo ""

echo "12. Cancel Remaining Order"
echo "----------------------------------------"
curl -s -X DELETE "$BASE_URL/api/v1/orders/AAPL/$SELL_ORDER_ID" | jq .
echo ""
echo ""

echo "========================================"
echo "Test Complete!"
echo "========================================"
