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
    exit 1
fi

echo "🚀 Starting API..."
echo "🌐 Port: ${APP_PORT:-8080}"
echo ""

cd api
go run ./cmd/api
