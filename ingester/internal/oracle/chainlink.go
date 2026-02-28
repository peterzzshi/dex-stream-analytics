package oracle

import (
	"context"
	"math/big"
	"strings"
	"time"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"

	"ingester/internal/cache"
	"ingester/internal/contract"
)

// ChainlinkOracle fetches token prices from Chainlink Data Feeds
type ChainlinkOracle struct {
	client     *ethclient.Client
	priceCache *cache.Cache[common.Address, float64]
}

// NewChainlinkOracle creates a new Chainlink price oracle
func NewChainlinkOracle(client *ethclient.Client) *ChainlinkOracle {
	// Cache prices for 5 minutes (reasonable for DEX analytics)
	// Chainlink updates vary: 0.5% deviation or 1-24 hours
	return &ChainlinkOracle{
		client:     client,
		priceCache: cache.NewCache[common.Address, float64](5 * time.Minute),
	}
}

// GetTokenUSDPrice fetches the USD price of a token with caching
// Returns (price, found) where found indicates if a price feed exists
func (o *ChainlinkOracle) GetTokenUSDPrice(ctx context.Context, tokenAddress common.Address) (float64, bool) {
	// Check if token is a stablecoin first (optimization)
	if isStablecoin(tokenAddress) {
		return 1.0, true
	}

	// Try cache first
	price, found := o.priceCache.GetOrFetch(ctx, tokenAddress, func(ctx context.Context, addr common.Address) (float64, bool) {
		return o.fetchPriceFromChainlink(ctx, addr)
	})

	return price, found
}

// fetchPriceFromChainlink queries Chainlink price feed for a token
func (o *ChainlinkOracle) fetchPriceFromChainlink(ctx context.Context, tokenAddress common.Address) (float64, bool) {
	priceFeedAddress, exists := getChainlinkPriceFeed(tokenAddress)
	if !exists {
		return 0, false
	}

	aggregatorABI, err := contract.GetABI(contract.ChainlinkAggregator)
	if err != nil {
		return 0, false
	}

	contractBinding := bind.NewBoundContract(priceFeedAddress, aggregatorABI, o.client, o.client, o.client)

	// Fetch decimals
	var decimals uint8
	if err := contract.CallContract(ctx, o.client, priceFeedAddress, aggregatorABI, "decimals", &decimals); err != nil {
		return 0, false
	}

	// Fetch latest round data
	priceResults := []interface{}{
		new(*big.Int), // roundId (unused)
		new(*big.Int), // answer (price)
		new(*big.Int), // startedAt (unused)
		new(*big.Int), // updatedAt
		new(*big.Int), // answeredInRound (unused)
	}
	if err := contractBinding.Call(&bind.CallOpts{Context: ctx}, &priceResults, "latestRoundData"); err != nil {
		return 0, false
	}

	answer := *priceResults[1].(**big.Int)
	updatedAt := *priceResults[3].(**big.Int)

	// Reject stale prices (>24 hours old)
	if time.Now().Unix()-updatedAt.Int64() > 24*60*60 {
		return 0, false
	}

	// Convert to float64 with proper decimal adjustment
	return priceWithDecimals(answer, decimals), true
}

// priceWithDecimals converts a Chainlink price answer to float64 with decimals
func priceWithDecimals(answer *big.Int, decimals uint8) float64 {
	divisor := new(big.Float).SetInt(new(big.Int).Exp(big.NewInt(10), big.NewInt(int64(decimals)), nil))
	price := new(big.Float).SetInt(answer)
	price.Quo(price, divisor)
	priceFloat, _ := price.Float64()
	return priceFloat
}

// ClearCache clears the price cache (useful for testing)
func (o *ChainlinkOracle) ClearCache() {
	o.priceCache.Clear()
}

// GetCacheStats returns cache statistics
func (o *ChainlinkOracle) GetCacheStats() int {
	return o.priceCache.Size()
}

// Polygon Mainnet Chainlink Price Feed addresses
// Source: https://docs.chain.link/data-feeds/price-feeds/addresses?network=polygon
var polygonPriceFeeds = map[common.Address]common.Address{
	// Token Address -> Price Feed Address
	common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"): common.HexToAddress("0xAB594600376Ec9fD91F8e885dADF0CE036862dE0"), // WMATIC/USD
	common.HexToAddress("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"): common.HexToAddress("0xF9680D99D6C9589e2a93a78A04A279e509205945"), // WETH/USD
	common.HexToAddress("0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6"): common.HexToAddress("0xc907E116054Ad103354f2D350FD2514433D57F6f"), // WBTC/USD
	common.HexToAddress("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"): common.HexToAddress("0xfE4A8cc5b5B2366C1B58Bea3858e81843581b2F7"), // USDC/USD
	common.HexToAddress("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"): common.HexToAddress("0x0A6513e40db6EB1b165753AD52E80663aeA50545"), // USDT/USD
	common.HexToAddress("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"): common.HexToAddress("0x4746DeC9e833A82EC7C2C1356372CcF2cfcD2F3D"), // DAI/USD
}

// stablecoinSet for O(1) lookup instead of O(n) iteration
var stablecoinSet = map[common.Address]bool{
	common.HexToAddress("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"): true, // USDC
	common.HexToAddress("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"): true, // USDT
	common.HexToAddress("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"): true, // DAI
}

// getChainlinkPriceFeed returns the Chainlink price feed address for a token
func getChainlinkPriceFeed(tokenAddress common.Address) (common.Address, bool) {
	normalizedAddr := common.HexToAddress(strings.ToLower(tokenAddress.Hex()))
	feed, exists := polygonPriceFeeds[normalizedAddr]
	return feed, exists
}

// isStablecoin checks if a token is a known stablecoin (price = $1)
// Uses map lookup for O(1) performance instead of slice iteration
func isStablecoin(tokenAddress common.Address) bool {
	normalizedAddr := common.HexToAddress(strings.ToLower(tokenAddress.Hex()))
	return stablecoinSet[normalizedAddr]
}
