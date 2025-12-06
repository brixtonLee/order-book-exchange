#!/bin/bash

# Phase 2 Features Test Script
# Tests all new Phase 2 capabilities

BASE_URL="http://127.0.0.1:3000"

echo "🧪 Testing Phase 2 Features"
echo "============================"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to print test results
test_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

echo -e "${BLUE}Test 1: Post-Only Order (should reject if would match)${NC}"
echo "------------------------------------------------------"

# Add a sell order first
echo "Adding sell order at 150.50..."
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "sell",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 100,
    "user_id": "seller1"
  }' | jq '.'

sleep 1

# Try post-only buy at same price (should be rejected)
echo ""
echo "Trying post-only buy at 150.50 (should be rejected)..."
RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "side": "buy",
    "order_type": "limit",
    "price": 150.50,
    "quantity": 50,
    "user_id": "buyer1",
    "post_only": true
  }')

echo "$RESPONSE" | jq '.'

if echo "$RESPONSE" | grep -q "Post-only"; then
    test_result 0 "Post-only order correctly rejected"
else
    test_result 1 "Post-only order should have been rejected"
fi

echo ""
echo "========================================================"
echo ""

echo -e "${BLUE}Test 2: Self-Trade Prevention${NC}"
echo "------------------------------------------------------"

# Same user places both sides with STP
echo "Placing sell order from mm1..."
SELL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "GOOGL",
    "side": "sell",
    "order_type": "limit",
    "price": 140.00,
    "quantity": 100,
    "user_id": "mm1",
    "stp_mode": "CANCEL_RESTING"
  }')

echo "$SELL_RESPONSE" | jq '.'

sleep 1

echo ""
echo "Placing buy order from same user mm1 (should trigger STP)..."
BUY_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "GOOGL",
    "side": "buy",
    "order_type": "limit",
    "price": 140.00,
    "quantity": 50,
    "user_id": "mm1",
    "stp_mode": "CANCEL_RESTING"
  }')

echo "$BUY_RESPONSE" | jq '.'

# Check that no trade occurred (self-trade prevented)
TRADES=$(echo "$BUY_RESPONSE" | jq '.trades | length')
if [ "$TRADES" == "0" ]; then
    test_result 0 "Self-trade successfully prevented"
else
    test_result 1 "Self-trade should have been prevented"
fi

echo ""
echo "========================================================"
echo ""

echo -e "${BLUE}Test 3: IOC (Immediate-Or-Cancel) Order${NC}"
echo "------------------------------------------------------"

# Add partial liquidity
echo "Adding 50 shares of MSFT at 370.00..."
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "MSFT",
    "side": "sell",
    "order_type": "limit",
    "price": 370.00,
    "quantity": 50,
    "user_id": "seller1"
  }' | jq '.'

sleep 1

echo ""
echo "Placing IOC buy order for 100 shares (only 50 available)..."
IOC_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "MSFT",
    "side": "buy",
    "order_type": "limit",
    "price": 370.00,
    "quantity": 100,
    "user_id": "buyer1",
    "time_in_force": "IOC"
  }')

echo "$IOC_RESPONSE" | jq '.'

FILLED=$(echo "$IOC_RESPONSE" | jq -r '.filled_quantity')
STATUS=$(echo "$IOC_RESPONSE" | jq -r '.status')

if [ "$FILLED" == "50" ] && [ "$STATUS" == "partially_filled" ]; then
    test_result 0 "IOC order correctly filled 50 and cancelled remainder"
else
    test_result 1 "IOC order should have filled 50 and cancelled remainder"
fi

echo ""
echo "========================================================"
echo ""

echo -e "${BLUE}Test 4: FOK (Fill-Or-Kill) Order${NC}"
echo "------------------------------------------------------"

# Add partial liquidity
echo "Adding 30 shares of NVDA at 500.00..."
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "NVDA",
    "side": "sell",
    "order_type": "limit",
    "price": 500.00,
    "quantity": 30,
    "user_id": "seller1"
  }' | jq '.'

sleep 1

echo ""
echo "Placing FOK buy order for 100 shares (only 30 available - should reject)..."
FOK_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "NVDA",
    "side": "buy",
    "order_type": "limit",
    "price": 500.00,
    "quantity": 100,
    "user_id": "buyer1",
    "time_in_force": "FOK"
  }')

echo "$FOK_RESPONSE" | jq '.'

if echo "$FOK_RESPONSE" | grep -q "Fill-or-kill"; then
    test_result 0 "FOK order correctly rejected (insufficient liquidity)"
else
    test_result 1 "FOK order should have been rejected"
fi

echo ""
echo "========================================================"
echo ""

echo -e "${BLUE}Test 5: Market Order with VWAP${NC}"
echo "------------------------------------------------------"

# Add multiple price levels
echo "Adding liquidity at multiple price levels..."
curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "TSLA",
    "side": "sell",
    "order_type": "limit",
    "price": 200.00,
    "quantity": 100,
    "user_id": "seller1"
  }' > /dev/null

curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "TSLA",
    "side": "sell",
    "order_type": "limit",
    "price": 200.50,
    "quantity": 100,
    "user_id": "seller2"
  }' > /dev/null

curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "TSLA",
    "side": "sell",
    "order_type": "limit",
    "price": 201.00,
    "quantity": 100,
    "user_id": "seller3"
  }' > /dev/null

sleep 1

echo ""
echo "Placing market order for 250 shares..."
MARKET_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "TSLA",
    "side": "buy",
    "order_type": "market",
    "quantity": 250,
    "user_id": "buyer1"
  }')

echo "$MARKET_RESPONSE" | jq '.'

TRADES_COUNT=$(echo "$MARKET_RESPONSE" | jq '.trades | length')
if [ "$TRADES_COUNT" -ge "2" ]; then
    test_result 0 "Market order executed across multiple price levels"
else
    test_result 1 "Market order should have executed across multiple levels"
fi

echo ""
echo "========================================================"
echo ""

echo -e "${BLUE}Test 6: Get Order with New Fields${NC}"
echo "------------------------------------------------------"

# Submit order with all Phase 2 fields
echo "Submitting order with all Phase 2 fields..."
ORDER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "META",
    "side": "buy",
    "order_type": "limit",
    "price": 300.00,
    "quantity": 100,
    "user_id": "trader1",
    "time_in_force": "GTC",
    "stp_mode": "CANCEL_RESTING",
    "post_only": false
  }')

ORDER_ID=$(echo "$ORDER_RESPONSE" | jq -r '.order_id')
echo "Order ID: $ORDER_ID"

sleep 1

echo ""
echo "Retrieving order to verify new fields..."
GET_RESPONSE=$(curl -s "$BASE_URL/api/v1/orders/META/$ORDER_ID")
echo "$GET_RESPONSE" | jq '.'

if echo "$GET_RESPONSE" | jq -e '.time_in_force' > /dev/null && \
   echo "$GET_RESPONSE" | jq -e '.stp_mode' > /dev/null && \
   echo "$GET_RESPONSE" | jq -e '.post_only' > /dev/null; then
    test_result 0 "Order response includes all Phase 2 fields"
else
    test_result 1 "Order response missing Phase 2 fields"
fi

echo ""
echo "========================================================"
echo ""

# Summary
echo -e "${YELLOW}Test Summary${NC}"
echo "============"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All Phase 2 features working correctly!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Check output above.${NC}"
    exit 1
fi
