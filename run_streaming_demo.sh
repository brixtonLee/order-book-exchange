#!/bin/bash

# cTrader FIX Streaming Demo Launcher
# This script helps you quickly run the streaming demo with your credentials

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║     cTrader FIX → WebSocket Streaming Demo                    ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Check if credentials are already set
if [ -z "$CTRADER_SENDER_COMP_ID" ]; then
    echo "📝 Please configure your cTrader credentials:"
    echo ""

    # Example configuration
    echo "For cTrader demo/live accounts, use this format:"
    echo "  SENDER_COMP_ID: demo.fxpro.ACCOUNT_ID or live.fxpro.ACCOUNT_ID"
    echo "  USERNAME: ACCOUNT_ID (just the number)"
    echo ""

    # Read credentials
    read -p "Enter SENDER_COMP_ID (e.g., live.fxpro.8244184): " SENDER_COMP_ID
    read -p "Enter USERNAME (e.g., 8244184): " USERNAME
    read -sp "Enter PASSWORD: " PASSWORD
    echo ""
    echo ""

    export CTRADER_SENDER_COMP_ID="$SENDER_COMP_ID"
    export CTRADER_USERNAME="$USERNAME"
    export CTRADER_PASSWORD="$PASSWORD"
fi

# Set defaults for other fields
export CTRADER_HOST="${CTRADER_HOST:-live-uk-eqx-01.p.c-trader.com}"
export CTRADER_PORT="${CTRADER_PORT:-5201}"
export CTRADER_TARGET_COMP_ID="${CTRADER_TARGET_COMP_ID:-cServer}"
export CTRADER_SENDER_SUB_ID="${CTRADER_SENDER_SUB_ID:-QUOTE}"
export CTRADER_TARGET_SUB_ID="${CTRADER_TARGET_SUB_ID:-QUOTE}"

echo "✅ Configuration loaded:"
echo "   Host: $CTRADER_HOST:$CTRADER_PORT"
echo "   Sender: $CTRADER_SENDER_COMP_ID"
echo "   Username: $CTRADER_USERNAME"
echo ""

echo "🚀 Building and running streaming demo..."
echo ""

# Build and run
cargo run --bin ctrader_streaming_demo

echo ""
echo "✅ Demo finished"
