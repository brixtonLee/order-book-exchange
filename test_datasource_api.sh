#!/bin/bash

# Test script for Datasource API endpoints

echo "╔════════════════════════════════════════════════════════════╗"
echo "║         Datasource API Testing Script                     ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

BASE_URL="http://localhost:3000"

echo "1️⃣  Testing Health Endpoint (before connection)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/health" | jq '.'
echo ""

echo "2️⃣  Testing Datasource Status (before connection)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/datasource/status" | jq '.'
echo ""

echo "3️⃣  Starting FIX Connection"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "NOTE: Update credentials below with your cTrader account details"
echo ""

# Example request body - UPDATE WITH YOUR CREDENTIALS
curl -X POST "${BASE_URL}/api/v1/datasource/start" \
  -H "Content-Type: application/json" \
  -d '{
    "host": "live-uk-eqx-01.p.c-trader.com",
    "port": 5201,
    "credentials": {
      "sender_comp_id": "live.fxpro.8244184",
      "target_comp_id": "cServer",
      "sender_sub_id": "QUOTE",
      "target_sub_id": "QUOTE",
      "username": "8244184",
      "password": "fixapibrixton"
    }
  }' | jq '.'
echo ""

echo "⏳ Waiting 5 seconds for connection to establish..."
sleep 5
echo ""

echo "4️⃣  Testing Health Endpoint (after connection)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/health" | jq '.'
echo ""

echo "5️⃣  Testing Datasource Status (after connection)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/datasource/status" | jq '.'
echo ""

echo "⏳ Waiting 10 more seconds to accumulate heartbeats..."
sleep 10
echo ""

echo "6️⃣  Testing Datasource Status (with heartbeat count)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/datasource/status" | jq '.'
echo ""

echo "7️⃣  Stopping FIX Connection"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -X POST "${BASE_URL}/api/v1/datasource/stop" | jq '.'
echo ""

echo "8️⃣  Testing Health Endpoint (after stop)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "${BASE_URL}/api/v1/health" | jq '.'
echo ""

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                    Testing Complete                        ║"
echo "╚════════════════════════════════════════════════════════════╝"
