package oracle

import (
	"context"
	"fmt"
	"math/big"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"

	"ingester/internal/cache"
)

func TestGetTokenUSDPrice_StablecoinOptimization(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	fetcherCalled := false
	mockFetcher := func(ctx context.Context, addr common.Address) (float64, bool) {
		fetcherCalled = true
		return 0, false
	}

	stablecoins := []struct {
		name string
		addr string
	}{
		{"USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"},
		{"USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"},
		{"DAI", "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"},
	}

	for _, tt := range stablecoins {
		t.Run(tt.name, func(t *testing.T) {
			fetcherCalled = false
			addr := common.HexToAddress(tt.addr)
			price, found := GetTokenUSDPrice(context.Background(), addr, priceCache, mockFetcher)

			if !found {
				t.Errorf("Stablecoin %s should always be found", tt.name)
			}
			if price != 1.0 {
				t.Errorf("Stablecoin %s should be $1.0, got %f", tt.name, price)
			}
			if fetcherCalled {
				t.Errorf("Stablecoin %s should not call fetcher (optimization)", tt.name)
			}
		})
	}
}

func TestGetTokenUSDPrice_NonStablecoin(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	nonStablecoins := []string{
		"0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", // WMATIC
		"0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", // WETH
	}

	for _, addrStr := range nonStablecoins {
		addr := common.HexToAddress(addrStr)

		fetcherCalled := false
		mockFetcher := func(ctx context.Context, addr common.Address) (float64, bool) {
			fetcherCalled = true
			return 0.85, true
		}

		price, found := GetTokenUSDPrice(context.Background(), addr, priceCache, mockFetcher)

		if !fetcherCalled {
			t.Errorf("Non-stablecoin %s should call fetcher", addrStr)
		}
		if !found {
			t.Error("Expected to find price")
		}
		if price != 0.85 {
			t.Errorf("Expected $0.85, got $%f", price)
		}
	}
}

func TestGetTokenUSDPrice_WithMockFetcher(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	// Simple mock fetcher - just a function!
	mockFetcher := func(ctx context.Context, addr common.Address) (float64, bool) {
		return 0.85, true // Return $0.85 for all tokens
	}

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	price, found := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockFetcher)

	if !found {
		t.Error("Expected to find price")
	}
	if price != 0.85 {
		t.Errorf("Expected $0.85, got $%f", price)
	}
}

func TestGetTokenUSDPrice_Caching(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	// Mock fetcher that counts calls
	callCount := 0
	mockFetcher := func(ctx context.Context, addr common.Address) (float64, bool) {
		callCount++
		return 0.85, true
	}

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")

	// First call
	price1, _ := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockFetcher)
	if callCount != 1 {
		t.Errorf("Expected 1 fetcher call, got %d", callCount)
	}

	// Second call should hit cache
	price2, _ := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockFetcher)
	if callCount != 1 {
		t.Errorf("Cache not working: expected still 1 call, got %d", callCount)
	}

	if price1 != price2 {
		t.Errorf("Prices should match: %f vs %f", price1, price2)
	}
}

func TestGetTokenUSDPrice_UnknownToken(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	// Mock fetcher that returns not found
	mockFetcher := func(ctx context.Context, addr common.Address) (float64, bool) {
		return 0, false
	}

	unknown := common.HexToAddress("0x0000000000000000000000000000000000000001")
	price, found := GetTokenUSDPrice(context.Background(), unknown, priceCache, mockFetcher)

	if found {
		t.Error("Unknown token should not be found")
	}
	if price != 0 {
		t.Errorf("Price should be 0, got %f", price)
	}
}

func TestCreatePriceFetcherWithChainlink(t *testing.T) {
	// Simple mock ContractCaller - just a function!
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		// This would return properly encoded data in real scenario
		return nil, fmt.Errorf("not implemented in test")
	}

	// Create fetcher by partially applying the ContractCaller
	fetcher := CreatePriceFetcherWithChainlink(mockCaller)

	// Verify it's a function
	if fetcher == nil {
		t.Error("Expected non-nil fetcher")
	}

	// Verify it returns the right type
	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	_, found := fetcher(context.Background(), wmatic)

	// Should not find since mock doesn't implement proper encoding
	if found {
		t.Error("Mock fetcher should return not found")
	}
}

func TestPriceWithDecimals(t *testing.T) {
	tests := []struct {
		name     string
		answer   int64
		decimals uint8
		expected float64
	}{
		{"WMATIC with 8 decimals", 85000000, 8, 0.85},
		{"ETH with 8 decimals", 200000000000, 8, 2000.0},
		{"Token with 18 decimals", 1000000000000000000, 18, 1.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := priceWithDecimals(big.NewInt(tt.answer), tt.decimals)
			if result != tt.expected {
				t.Errorf("priceWithDecimals(%d, %d) = %f, want %f", tt.answer, tt.decimals, result, tt.expected)
			}
		})
	}
}

func TestFetchPriceFromChainlink_MockCaller(t *testing.T) {
	// Test that FetchPriceFromChainlink properly uses the injected ContractCaller
	callCount := 0
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		callCount++
		// Return error to simulate failure
		return nil, fmt.Errorf("mock error")
	}

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	price, found := FetchPriceFromChainlink(context.Background(), wmatic, mockCaller)

	// Should not find due to mock error
	if found {
		t.Error("Expected not found due to mock error")
	}
	if price != 0 {
		t.Errorf("Expected price 0, got %f", price)
	}

	// Verify the mock was called (at least once for decimals)
	if callCount == 0 {
		t.Error("Expected mock caller to be invoked")
	}
}

func TestGetTokenUSDPrice_BehaviorInjection(t *testing.T) {
	// Test with different behavior functions
	behaviors := []struct {
		name     string
		fetcher  PriceFetcher
		expected float64
		found    bool
	}{
		{
			name: "Always returns $1.50",
			fetcher: func(ctx context.Context, addr common.Address) (float64, bool) {
				return 1.50, true
			},
			expected: 1.50,
			found:    true,
		},
		{
			name: "Always returns not found",
			fetcher: func(ctx context.Context, addr common.Address) (float64, bool) {
				return 0, false
			},
			expected: 0,
			found:    false,
		},
		{
			name: "Returns price based on address",
			fetcher: func(ctx context.Context, addr common.Address) (float64, bool) {
				if addr.Hex() == "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270" {
					return 0.85, true
				}
				return 0, false
			},
			expected: 0.85,
			found:    true,
		},
	}

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")

	for _, tt := range behaviors {
		t.Run(tt.name, func(t *testing.T) {
			// Clear cache for each test
			priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

			price, found := GetTokenUSDPrice(context.Background(), wmatic, priceCache, tt.fetcher)

			if found != tt.found {
				t.Errorf("Expected found=%v, got %v", tt.found, found)
			}
			if price != tt.expected {
				t.Errorf("Expected price=%f, got %f", tt.expected, price)
			}
		})
	}
}
