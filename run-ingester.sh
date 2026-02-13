#!/bin/bash
set -e

# Load environment variables from .env file
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "❌ .env file not found!"
    echo ""
    echo "Create one by copying .env.example:"
    echo "  cp .env.example .env"
    echo ""
    echo "Then edit .env and set POLYGON_RPC_URL to your WebSocket endpoint"
    exit 1
fi

# Validate WebSocket URL
if [[ ! "$POLYGON_RPC_URL" =~ ^wss:// ]] && [[ ! "$POLYGON_RPC_URL" =~ ^ws:// ]]; then
    echo "⚠️  WARNING: POLYGON_RPC_URL should start with wss:// for WebSocket support"
    echo "Current value: $POLYGON_RPC_URL"
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "🚀 Starting ingester..."
echo "📡 RPC: $POLYGON_RPC_URL"
echo "📍 Pair: $PAIR_ADDRESS"
echo ""

cd ingester
go run ./cmd/ingester
