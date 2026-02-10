#!/bin/bash

set -e

echo "🚀 Web3 DEX Analytics - Project Setup"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed. Please install Docker first."
    exit 1
fi
print_success "Docker is installed"

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    print_error "Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi
print_success "Docker Compose is installed"

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    cp .env.example .env
    print_success "Created .env file from .env.example"
    print_info "Please edit .env with your configuration"
else
    print_info ".env file already exists"
fi

# Create necessary directories
echo ""
echo "Creating project directories..."
mkdir -p ingester/bin
mkdir -p ingester/internal/{blockchain,config,publisher}
mkdir -p ingester/pkg/{logger,events}
mkdir -p aggregator/target
mkdir -p aggregator/src/{main,test}/java/com/web3analytics
mkdir -p aggregator/src/main/resources
mkdir -p api/bin
mkdir -p api/internal/{handlers,config,storage,subscriber}
mkdir -p api/pkg/logger
mkdir -p docs/diagrams
mkdir -p dapr/components
mkdir -p dapr/config
mkdir -p scripts
print_success "Project directories created"

# Initialize Go modules if not already done
echo ""
echo "Initialising Go modules..."

if [ ! -f ingester/go.sum ]; then
    cd ingester
    go mod tidy
    cd ..
    print_success "Ingester Go modules initialised"
else
    print_info "Ingester Go modules already initialised"
fi

if [ ! -f api/go.sum ]; then
    cd api
    go mod tidy
    cd ..
    print_success "API Go modules initialised"
else
    print_info "API Go modules already initialised"
fi

# Check if Java/Maven is installed for aggregator
echo ""
if command -v mvn &> /dev/null; then
    print_success "Maven is installed"
    echo "Downloading Maven dependencies (this may take a while)..."
    cd aggregator
    mvn dependency:go-offline -q
    cd ..
    print_success "Maven dependencies downloaded"
else
    print_info "Maven not found locally - will use Docker for builds"
fi

# Pull required Docker images
echo ""
echo "Pulling Docker images (this may take a while)..."
docker-compose pull
print_success "Docker images pulled"

# Create example topic in Kafka (optional)
echo ""
print_info "To create Kafka topics, run: make kafka-create-topic"

# Final instructions
echo ""
echo "======================================"
print_success "Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Edit .env with your Polygon RPC URL (optional)"
echo "  2. Run 'make start' to start all services"
echo "  3. Run 'make status' to check service health"
echo "  4. Access APIs at:"
echo "     - API: http://localhost:8080"
echo "     - Flink UI: http://localhost:8081"
echo "     - Schema Registry: http://localhost:8082"
echo ""
echo "For more commands, run: make help"
echo ""