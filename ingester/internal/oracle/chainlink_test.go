package oracle

import (
	"context"
	"testing"

	"github.com/ethereum/go-ethereum/common"
)

func TestIsStablecoin(t *testing.T) {
	tests := []struct {
		name     string
		address  string
		expected bool
	}{
		{"USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", true},
		{"USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F", true},
		{"DAI", "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", true},
		{"WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", false},
		{"WETH", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			addr := common.HexToAddress(tt.address)
			result := isStablecoin(addr)
			if result != tt.expected {
				t.Errorf("isStablecoin(%s) = %v, want %v", tt.name, result, tt.expected)
			}
		})
	}
}

func TestGetChainlinkPriceFeed(t *testing.T) {
	tests := []struct {
		name        string
		tokenAddr   string
		shouldExist bool
	}{
		{"WMATIC has feed", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", true},
		{"WETH has feed", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", true},
		{"WBTC has feed", "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6", true},
		{"USDC has feed", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", true},
		{"Random token no feed", "0x0000000000000000000000000000000000000001", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			addr := common.HexToAddress(tt.tokenAddr)
			_, exists := getChainlinkPriceFeed(addr)
			if exists != tt.shouldExist {
				t.Errorf("getChainlinkPriceFeed(%s) exists = %v, want %v", tt.name, exists, tt.shouldExist)
			}
		})
	}
}

func TestChainlinkOracleCaching(t *testing.T) {
	// Test cache behavior without actual RPC calls
	oracle := NewChainlinkOracle(nil)

	// Manually populate cache for testing
	testAddr := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	oracle.priceCache.Set(testAddr, 0.85, true)

	// Should hit cache
	price, found, exists := oracle.priceCache.Get(testAddr)
	if !exists {
		t.Error("Expected key to exist in cache")
	}
	if !found {
		t.Error("Expected to find cached price")
	}
	if price != 0.85 {
		t.Errorf("Expected price 0.85, got %f", price)
	}

	// Check cache stats
	stats := oracle.GetCacheStats()
	if stats != 1 {
		t.Errorf("Expected cache size 1, got %d", stats)
	}

	// Clear cache
	oracle.ClearCache()
	stats = oracle.GetCacheStats()
	if stats != 0 {
		t.Errorf("Expected cache size 0 after clear, got %d", stats)
	}
}

func TestStablecoinPriceOptimization(t *testing.T) {
	oracle := NewChainlinkOracle(nil)

	// Stablecoins should return $1 immediately without RPC call
	stablecoins := []string{
		"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", // USDC
		"0xc2132D05D31c914a87C6611C10748AEb04B58e8F", // USDT
		"0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", // DAI
	}

	for _, addrStr := range stablecoins {
		addr := common.HexToAddress(addrStr)
		price, found := oracle.GetTokenUSDPrice(context.Background(), addr)

		if !found {
			t.Errorf("Stablecoin %s should always be found", addrStr)
		}
		if price != 1.0 {
			t.Errorf("Stablecoin %s should be $1.0, got %f", addrStr, price)
		}
	}
}

// Note: Integration tests with real Polygon RPC would go in a separate file
// Example: TestChainlinkOracleIntegration (requires live RPC connection)
