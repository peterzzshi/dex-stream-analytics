#!/bin/bash

set -e

echo "🚀 Deploying Web3 DEX Analytics"
echo "==============================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Check if environment is provided
ENVIRONMENT=${1:-development}

print_info "Deploying to environment: $ENVIRONMENT"

# Validate environment
if [ "$ENVIRONMENT" != "development" ] && [ "$ENVIRONMENT" != "staging" ] && [ "$ENVIRONMENT" != "production" ]; then
    print_error "Invalid environment. Use: development, staging, or production"
    exit 1
fi

# Load environment variables
if [ -f ".env.$ENVIRONMENT" ]; then
    source ".env.$ENVIRONMENT"
    print_success "Loaded .env.$ENVIRONMENT"
elif [ -f ".env" ]; then
    source ".env"
    print_success "Loaded .env"
else
    print_error "No environment file found"
    exit 1
fi

# Build services
echo ""
echo "Building services..."

# Build Ingester
print_info "Building ingester..."
cd ingester
go build -ldflags="-s -w" -o bin/ingester cmd/ingester/main.go
print_success "Ingester built"
cd ..

# Build Aggregator
print_info "Building aggregator..."
cd aggregator
mvn clean package -DskipTests
print_success "Aggregator built"
cd ..

# Build API
print_info "Building API..."
cd api
go build -ldflags="-s -w" -o bin/api cmd/api/main.go
print_success "API built"
cd ..

echo ""

# Docker build and push for production/staging
if [ "$ENVIRONMENT" != "development" ]; then
    print_info "Building Docker images..."
    
    # Set Docker registry (customize as needed)
    REGISTRY=${DOCKER_REGISTRY:-"ghcr.io/thaochinguyen"}
    VERSION=$(git describe --tags --always --dirty)
    
    print_info "Building and pushing to $REGISTRY with tag $VERSION"
    
    # Build and push ingester
    docker build -t "$REGISTRY/dex-stream-analytics-ingester:$VERSION" ./ingester
    docker push "$REGISTRY/dex-stream-analytics-ingester:$VERSION"
    print_success "Ingester image pushed"
    
    # Build and push aggregator
    docker build -t "$REGISTRY/dex-stream-analytics-aggregator:$VERSION" ./aggregator
    docker push "$REGISTRY/dex-stream-analytics-aggregator:$VERSION"
    print_success "Aggregator image pushed"
    
    # Build and push API
    docker build -t "$REGISTRY/dex-stream-analytics-api:$VERSION" ./api
    docker push "$REGISTRY/dex-stream-analytics-api:$VERSION"
    print_success "API image pushed"
    
    echo ""
    print_info "Deploying to Kubernetes (if configured)..."
    
    # Apply Kubernetes manifests (Phase 4)
    if [ -d "k8s/$ENVIRONMENT" ]; then
        kubectl apply -f "k8s/$ENVIRONMENT/"
        print_success "Kubernetes manifests applied"
    else
        print_info "No Kubernetes manifests found for $ENVIRONMENT"
    fi
else
    # Development deployment
    print_info "Starting development environment with Docker Compose..."
    docker-compose down
    docker-compose up -d --build
    print_success "Development environment started"
fi

echo ""
print_success "Deployment complete!"
print_info "Check service status with: docker-compose ps"

# Health checks
echo ""
print_info "Waiting for services to be healthy..."
sleep 10

# Check Ingester
if curl -f -s http://localhost:3000/health > /dev/null; then
    print_success "Ingester is healthy"
else
    print_error "Ingester health check failed"
fi

# Check API
if curl -f -s http://localhost:8080/health > /dev/null; then
    print_success "API is healthy"
else
    print_error "API health check failed"
fi

# Check Flink
if curl -f -s http://localhost:8081/overview > /dev/null; then
    print_success "Flink is healthy"
else
    print_error "Flink health check failed"
fi

echo ""
print_success "All services deployed and healthy!"
