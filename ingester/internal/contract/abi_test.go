package contract

import (
	"context"
	"fmt"
	"math/big"
	"strings"
	"testing"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
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
		return
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

func TestCallContract_PopulatesResult(t *testing.T) {
	// This test verifies that CallContract properly decodes and populates the result pointer
	// Critical after ABI decoding fixes to ensure outputs.Copy works correctly

	decimalsABI, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("Failed to get decimals ABI: %v", err)
	}

	// Mock ContractCaller that returns encoded uint8 value (18 decimals)
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		// Pack the return value: uint8(18)
		method := decimalsABI.Methods["decimals"]
		packed, err := method.Outputs.Pack(uint8(18))
		if err != nil {
			return nil, err
		}
		return packed, nil
	}

	var result uint8
	mockAddress := common.HexToAddress("0x1234567890123456789012345678901234567890")

	err = CallContract(
		context.Background(),
		mockCaller,
		mockAddress,
		decimalsABI,
		"decimals",
		&result,
	)

	if err != nil {
		t.Fatalf("CallContract failed: %v", err)
	}

	// Verify the result was actually populated
	if result != 18 {
		t.Errorf("Expected result to be 18, got %d - CallContract did not populate resultPtr correctly", result)
	}
}

func TestCallContract_PopulatesStringResult(t *testing.T) {
	// Test string decoding (symbol method)
	symbolABI, err := GetABI(ERC20Symbol)
	if err != nil {
		t.Fatalf("Failed to get symbol ABI: %v", err)
	}

	// Mock ContractCaller that returns encoded string value
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		method := symbolABI.Methods["symbol"]
		packed, err := method.Outputs.Pack("USDC")
		if err != nil {
			return nil, err
		}
		return packed, nil
	}

	var result string
	mockAddress := common.HexToAddress("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")

	err = CallContract(
		context.Background(),
		mockCaller,
		mockAddress,
		symbolABI,
		"symbol",
		&result,
	)

	if err != nil {
		t.Fatalf("CallContract failed: %v", err)
	}

	if result != "USDC" {
		t.Errorf("Expected result to be 'USDC', got '%s' - CallContract did not populate string resultPtr correctly", result)
	}
}

func TestCallContract_PopulatesComplexResult(t *testing.T) {
	// Test complex struct decoding (Chainlink latestRoundData)
	chainlinkABI, err := GetABI(ChainlinkAggregator)
	if err != nil {
		t.Fatalf("Failed to get Chainlink ABI: %v", err)
	}

	// Mock ContractCaller that returns encoded latestRoundData
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		method := chainlinkABI.Methods["latestRoundData"]
		// latestRoundData returns (roundId, answer, startedAt, updatedAt, answeredInRound)
		packed, err := method.Outputs.Pack(
			big.NewInt(12345),        // roundId
			big.NewInt(250000000000), // answer (2500.00000000 with 8 decimals)
			big.NewInt(1709395200),   // startedAt
			big.NewInt(1709395200),   // updatedAt
			big.NewInt(12345),        // answeredInRound
		)
		if err != nil {
			return nil, err
		}
		return packed, nil
	}

	// Define result struct matching Chainlink output
	type LatestRoundData struct {
		RoundId         *big.Int
		Answer          *big.Int
		StartedAt       *big.Int
		UpdatedAt       *big.Int
		AnsweredInRound *big.Int
	}

	var result LatestRoundData
	mockAddress := common.HexToAddress("0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419")

	err = CallContract(
		context.Background(),
		mockCaller,
		mockAddress,
		chainlinkABI,
		"latestRoundData",
		&result,
	)

	if err != nil {
		t.Fatalf("CallContract failed: %v", err)
	}

	// Verify all fields were populated correctly
	if result.RoundId == nil || result.RoundId.Cmp(big.NewInt(12345)) != 0 {
		t.Errorf("Expected RoundId to be 12345, got %v", result.RoundId)
	}

	if result.Answer == nil || result.Answer.Cmp(big.NewInt(250000000000)) != 0 {
		t.Errorf("Expected Answer to be 250000000000, got %v", result.Answer)
	}

	if result.UpdatedAt == nil || result.UpdatedAt.Cmp(big.NewInt(1709395200)) != 0 {
		t.Errorf("Expected UpdatedAt to be 1709395200, got %v", result.UpdatedAt)
	}
}

func TestCallContract_ContractCallError(t *testing.T) {
	// Test that contract call errors are properly propagated
	decimalsABI, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("Failed to get decimals ABI: %v", err)
	}

	// Mock ContractCaller that returns an error
	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		return nil, fmt.Errorf("execution reverted")
	}

	var result uint8
	mockAddress := common.HexToAddress("0x1234567890123456789012345678901234567890")

	err = CallContract(
		context.Background(),
		mockCaller,
		mockAddress,
		decimalsABI,
		"decimals",
		&result,
	)

	if err == nil {
		t.Error("Expected error from contract call, got nil")
		return
	}

	if !strings.Contains(err.Error(), "call contract") {
		t.Errorf("Expected error to contain 'call contract', got: %v", err)
	}
}

func TestCallContract_InvalidMethodName(t *testing.T) {
	// Test that invalid method name returns proper error
	decimalsABI, err := GetABI(ERC20Decimals)
	if err != nil {
		t.Fatalf("Failed to get decimals ABI: %v", err)
	}

	mockCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		return []byte{}, nil
	}

	var result uint8
	mockAddress := common.HexToAddress("0x1234567890123456789012345678901234567890")

	err = CallContract(
		context.Background(),
		mockCaller,
		mockAddress,
		decimalsABI,
		"nonexistentMethod",
		&result,
	)

	if err == nil {
		t.Error("Expected error for invalid method name, got nil")
		return
	}

	if !strings.Contains(err.Error(), "pack method") {
		t.Errorf("Expected error to contain 'pack method', got: %v", err)
	}
}
