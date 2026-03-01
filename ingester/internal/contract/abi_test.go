package contract

import (
	"strings"
	"testing"
)

func TestGetABI_AllKnownABIs(t *testing.T) {
	tests := []struct {
		name    string
		abiName ABIName
	}{
		{"ERC20 Decimals", ERC20Decimals},
		{"ERC20 Symbol", ERC20Symbol},
		{"Chainlink Aggregator", ChainlinkAggregator},
		{"Uniswap V2 Pair", UniswapV2Pair},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			abi, err := GetABI(tt.abiName)
			if err != nil {
				t.Fatalf("GetABI(%s) failed: %v", tt.abiName, err)
			}

			if abi.Methods == nil || len(abi.Methods) == 0 {
				t.Errorf("GetABI(%s) returned ABI with no methods", tt.abiName)
			}
		})
	}
}

func TestGetABI_LazyLoading(t *testing.T) {
	// Call same ABI twice - should use cached version
	abi1, err1 := GetABI(ERC20Decimals)
	if err1 != nil {
		t.Fatalf("First GetABI failed: %v", err1)
	}

	abi2, err2 := GetABI(ERC20Decimals)
	if err2 != nil {
		t.Fatalf("Second GetABI failed: %v", err2)
	}

	// Should return same instance (lazy loaded once)
	if len(abi1.Methods) != len(abi2.Methods) {
		t.Error("GetABI returned different ABIs on subsequent calls")
	}
}

func TestGetABI_UnknownABI(t *testing.T) {
	unknownABI := ABIName("unknown_contract")
	_, err := GetABI(unknownABI)

	if err == nil {
		t.Error("Expected error for unknown ABI, got nil")
	}

	if !strings.Contains(err.Error(), "unknown ABI") {
		t.Errorf("Expected 'unknown ABI' in error, got: %v", err)
	}
}

func TestGetABI_HasExpectedMethods(t *testing.T) {
	tests := []struct {
		abiName        ABIName
		expectedMethod string
	}{
		{ERC20Decimals, "decimals"},
		{ERC20Symbol, "symbol"},
		{ChainlinkAggregator, "latestRoundData"},
		{ChainlinkAggregator, "decimals"},
		{UniswapV2Pair, "Swap"},
		{UniswapV2Pair, "Mint"},
		{UniswapV2Pair, "Burn"},
	}

	for _, tt := range tests {
		t.Run(string(tt.abiName)+"_"+tt.expectedMethod, func(t *testing.T) {
			abi, err := GetABI(tt.abiName)
			if err != nil {
				t.Fatalf("GetABI failed: %v", err)
			}

			// Check for method in Methods map
			if _, exists := abi.Methods[tt.expectedMethod]; !exists {
				// For events, check Events map
				if _, existsEvent := abi.Events[tt.expectedMethod]; !existsEvent {
					t.Errorf("ABI %s missing expected method/event: %s", tt.abiName, tt.expectedMethod)
				}
			}
		})
	}
}

func TestParseABI_InvalidJSON(t *testing.T) {
	allABIs := []ABIName{
		ERC20Decimals,
		ERC20Symbol,
		ChainlinkAggregator,
		UniswapV2Pair,
	}

	for _, abiName := range allABIs {
		t.Run(string(abiName), func(t *testing.T) {
			_, err := GetABI(abiName)
			if err != nil {
				t.Errorf("Valid ABI %s failed to parse: %v", abiName, err)
			}
		})
	}
}

func TestGetABI_ConcurrentAccess(t *testing.T) {
	// Test that concurrent access to GetABI is safe
	done := make(chan bool)

	for i := 0; i < 10; i++ {
		go func() {
			_, err := GetABI(ERC20Decimals)
			if err != nil {
				t.Errorf("Concurrent GetABI failed: %v", err)
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}
}
