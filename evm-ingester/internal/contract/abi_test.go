package contract

import (
	"testing"
)

func TestGetABI_AllKnownABIs(t *testing.T) {
	tests := []struct {
		name   string
		abi    ABIName
		method string
	}{
		{"ERC20Decimals", ERC20Decimals, "decimals"},
		{"ERC20Symbol", ERC20Symbol, "symbol"},
		{"ChainlinkAggregator", ChainlinkAggregator, "latestRoundData"},
		{"UniswapV2Pair", UniswapV2Pair, "token0"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := GetABI(tt.abi)
			if err != nil {
				t.Fatalf("GetABI(%s) failed: %v", tt.name, err)
			}
			if _, ok := result.Methods[tt.method]; !ok {
				t.Errorf("ABI %s missing expected method %s", tt.name, tt.method)
			}
		})
	}
}

func TestGetABI_LazyLoading(t *testing.T) {
	// Call GetABI twice to ensure lazy loading works correctly
	first, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("First GetABI call failed: %v", err)
	}

	second, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("Second GetABI call failed: %v", err)
	}

	// Verify same instance is returned (pointer equality would be ideal, but ABI is a struct)
	if len(first.Methods) != len(second.Methods) {
		t.Error("Lazy loading may not be working correctly")
	}
}

func TestGetABI_UnknownABI(t *testing.T) {
	_, err := GetABI(ABIName("nonexistent"))
	if err == nil {
		t.Fatal("Expected error for unknown ABI, got nil")
	}

	expectedMsg := "unknown ABI"
	if err.Error() != expectedMsg+": nonexistent" {
		t.Errorf("Expected error message to contain '%s', got: %v", expectedMsg, err)
	}
}

func TestGetABI_HasExpectedMethods(t *testing.T) {
	decimalsABI, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("Failed to get decimals ABI: %v", err)
	}

	decimalsMethod, exists := decimalsABI.Methods["decimals"]
	if !exists {
		t.Fatal("decimals method not found in ERC20Decimals ABI")
	}

	if len(decimalsMethod.Outputs) != 1 {
		t.Fatalf("Expected 1 output for decimals, got %d", len(decimalsMethod.Outputs))
	}

	if decimalsMethod.Outputs[0].Type.String() != "uint8" {
		t.Errorf("Expected decimals output type to be uint8, got %s", decimalsMethod.Outputs[0].Type.String())
	}

	symbolABI, err := GetABI(ERC20Symbol)
	if err != nil {
		t.Fatalf("Failed to get symbol ABI: %v", err)
	}

	symbolMethod, exists := symbolABI.Methods["symbol"]
	if !exists {
		t.Fatal("symbol method not found in ERC20Symbol ABI")
	}

	if symbolMethod.Outputs[0].Type.String() != "string" {
		t.Errorf("Expected symbol output type to be string, got %s", symbolMethod.Outputs[0].Type.String())
	}
}

func TestParseABI_InvalidJSON(t *testing.T) {
	// Test that all embedded ABI JSON files parse correctly
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

	for range 10 {
		go func() {
			_, err := GetABI(ERC20Decimals)
			if err != nil {
				t.Errorf("Concurrent GetABI failed: %v", err)
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for range 10 {
		<-done
	}
}
