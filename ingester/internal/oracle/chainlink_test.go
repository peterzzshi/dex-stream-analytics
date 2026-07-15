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
	"ingester/internal/contract"
	"ingester/internal/oracle/mocks"
)

func TestGetTokenUSDPrice_StablecoinOptimization(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)
	mockOracle := mocks.NewMockPriceOracle(t)

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
			addr := common.HexToAddress(tt.addr)
			price, found := GetTokenUSDPrice(context.Background(), addr, priceCache, mockOracle)

			if !found {
				t.Errorf("Stablecoin %s should always be found", tt.name)
			}
			if price != 1.0 {
				t.Errorf("Stablecoin %s should be $1.0, got %f", tt.name, price)
			}
		})
	}
	mockOracle.AssertNotCalled(t, "FetchPrice")
}

func TestGetTokenUSDPrice_NonStablecoin(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)
	mockOracle := mocks.NewMockPriceOracle(t)

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	mockOracle.EXPECT().FetchPrice(context.Background(), wmatic).Return(0.85, true).Once()

	price, found := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockOracle)

	if !found {
		t.Error("Expected to find price")
	}
	if price != 0.85 {
		t.Errorf("Expected $0.85, got $%f", price)
	}
}

func TestGetTokenUSDPrice_Caching(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)
	mockOracle := mocks.NewMockPriceOracle(t)

	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	mockOracle.EXPECT().FetchPrice(context.Background(), wmatic).Return(0.85, true).Once()

	price1, _ := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockOracle)
	price2, _ := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockOracle)

	if price1 != price2 {
		t.Errorf("Prices should match: %f vs %f", price1, price2)
	}
	mockOracle.AssertNumberOfCalls(t, "FetchPrice", 1)
}

func TestGetTokenUSDPrice_UnknownToken(t *testing.T) {
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)
	mockOracle := mocks.NewMockPriceOracle(t)

	unknown := common.HexToAddress("0x0000000000000000000000000000000000000001")
	mockOracle.EXPECT().FetchPrice(context.Background(), unknown).Return(0.0, false).Once()

	price, found := GetTokenUSDPrice(context.Background(), unknown, priceCache, mockOracle)

	if found {
		t.Error("Unknown token should not be found")
	}
	if price != 0 {
		t.Errorf("Price should be 0, got %f", price)
	}
}

func TestChainlinkOracle_FetchPrice_RpcError(t *testing.T) {
	caller := contract.ContractCallerFunc(func(ctx context.Context, msg ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		return nil, fmt.Errorf("rpc error")
	})

	o := NewChainlinkOracle(caller)
	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
	_, found := o.FetchPrice(context.Background(), wmatic)

	if found {
		t.Error("RPC error should cause not-found")
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

func TestGetTokenUSDPrice_MultipleBehaviors(t *testing.T) {
	wmatic := common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")

	tests := []struct {
		name          string
		price         float64
		found         bool
		expectedPrice float64
		expectedFound bool
	}{
		{"Returns $1.50", 1.50, true, 1.50, true},
		{"Returns not found", 0, false, 0, false},
		{"Returns $0.85", 0.85, true, 0.85, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)
			mockOracle := mocks.NewMockPriceOracle(t)
			mockOracle.EXPECT().FetchPrice(context.Background(), wmatic).Return(tt.price, tt.found).Maybe()

			price, found := GetTokenUSDPrice(context.Background(), wmatic, priceCache, mockOracle)

			if found != tt.expectedFound {
				t.Errorf("Expected found=%v, got %v", tt.expectedFound, found)
			}
			if price != tt.expectedPrice {
				t.Errorf("Expected price=%f, got %f", tt.expectedPrice, price)
			}
		})
	}
}

