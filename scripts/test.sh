#!/bin/bash

set -e

echo "🧪 Running All Tests"
echo "==================="
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

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Test Ingester (Go)
echo "Testing Ingester (Go)..."
cd ingester
if go test -v ./...; then
    print_success "Ingester tests passed"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    print_error "Ingester tests failed"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
cd ..

echo ""

# Test Aggregator (Java/Flink)
echo "Testing Aggregator (Java/Flink)..."
cd aggregator
if mvn test; then
    print_success "Aggregator tests passed"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    print_error "Aggregator tests failed"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
cd ..

echo ""

# Test API (Go)
echo "Testing API (Go)..."
cd api
if go test -v ./...; then
    print_success "API tests passed"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    print_error "API tests failed"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
cd ..

echo ""

# Test Contracts (if exists)
if [ -f "contracts/package.json" ]; then
    echo "Testing Contracts (Hardhat)..."
    cd contracts
    if npm test; then
        print_success "Contract tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        print_error "Contract tests failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    cd ..
    echo ""
fi

# Summary
echo "=================="
echo "Test Summary"
echo "=================="
echo -e "${GREEN}Passed:${NC} $TESTS_PASSED"
echo -e "${RED}Failed:${NC} $TESTS_FAILED"

if [ $TESTS_FAILED -eq 0 ]; then
    print_success "All tests passed!"
    exit 0
else
    print_error "Some tests failed"
    exit 1
fi
