package oracle

import (
	"context"
	"math/big"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"

	"ingester/internal/cache"
	"ingester/internal/contract"
)

type PriceFetcher func(ctx context.Context, tokenAddr common.Address) (float64, bool)

var stablecoinSet = map[common.Address]bool{
	common.HexToAddress("0x2791bca1f2de4661ed88a30c99a7a9449aa84174"): true, // USDC
	common.HexToAddress("0xc2132d05d31c914a87c6611c10748aeb04b58e8f"): true, // USDT
	common.HexToAddress("0x8f3cf7ad23cd3cadbD9735aff958023239c6a063"): true, // DAI
}

var chainlinkPriceFeeds = map[common.Address]common.Address{
	common.HexToAddress("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"): common.HexToAddress("0xab594600376ec9fd91f8e885dadf0ce036862de0"), // WMATIC/USD
	common.HexToAddress("0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"): common.HexToAddress("0xf9680d99d6c9589e2a93a78a04a279e509205945"), // WETH/USD
	common.HexToAddress("0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6"): common.HexToAddress("0xc907e116054ad103354f2d350fd2514433d57f6f"), // WBTC/USD
	common.HexToAddress("0x2791bca1f2de4661ed88a30c99a7a9449aa84174"): common.HexToAddress("0xfe4a8cc5b5b2366c1b58bea3858e81843581b2f7"), // USDC/USD
	common.HexToAddress("0xc2132d05d31c914a87c6611c10748aeb04b58e8f"): common.HexToAddress("0x0a6513e40db6eb1b165753ad52e80663aea50545"), // USDT/USD
	common.HexToAddress("0x8f3cf7ad23cd3cadbD9735aff958023239c6a063"): common.HexToAddress("0x4746dec9e833a82ec7c2c1356372ccf2cfcd2f3d"), // DAI/USD
}

func GetTokenUSDPrice(
	ctx context.Context,
	tokenAddress common.Address,
	priceCache *cache.Cache[common.Address, float64],
	priceFetcher PriceFetcher,
) (float64, bool) {
	if stablecoinSet[tokenAddress] {
		return 1.0, true
	}

	return priceCache.GetOrFetch(ctx, tokenAddress, func(ctx context.Context, addr common.Address) (float64, bool) {
		return priceFetcher(ctx, addr)
	})
}

func FetchPriceFromChainlink(
	ctx context.Context,
	tokenAddress common.Address,
	callContract contract.ContractCaller,
) (float64, bool) {
	priceFeedAddress, exists := chainlinkPriceFeeds[tokenAddress]
	if !exists {
		return 0, false
	}

	aggregatorABI, err := contract.GetABI(contract.ChainlinkAggregator)
	if err != nil {
		return 0, false
	}

	// Fetch decimals using injected ContractCaller
	var decimals uint8
	if err := contract.CallContract(ctx, callContract, priceFeedAddress, aggregatorABI, "decimals", &decimals); err != nil {
		return 0, false
	}

	// Fetch latest round data using injected ContractCaller
	var result struct {
		RoundId         *big.Int
		Answer          *big.Int
		StartedAt       *big.Int
		UpdatedAt       *big.Int
		AnsweredInRound *big.Int
	}

	// Pack the latestRoundData call
	data, err := aggregatorABI.Pack("latestRoundData")
	if err != nil {
		return 0, false
	}

	// Make the call using injected function
	responseData, err := callContract(ctx, ethereum.CallMsg{
		To:   &priceFeedAddress,
		Data: data,
	}, nil)
	if err != nil {
		return 0, false
	}

	// Unpack the response
	values, err := aggregatorABI.Unpack("latestRoundData", responseData)
	if err != nil {
		return 0, false
	}

	if len(values) != 5 {
		return 0, false
	}

	result.RoundId = values[0].(*big.Int)
	result.Answer = values[1].(*big.Int)
	result.StartedAt = values[2].(*big.Int)
	result.UpdatedAt = values[3].(*big.Int)
	result.AnsweredInRound = values[4].(*big.Int)

	if time.Now().Unix()-result.UpdatedAt.Int64() > 24*60*60 {
		return 0, false
	}

	return priceWithDecimals(result.Answer, decimals), true
}

func priceWithDecimals(answer *big.Int, decimals uint8) float64 {
	divisor := new(big.Float).SetInt(new(big.Int).Exp(big.NewInt(10), big.NewInt(int64(decimals)), nil))
	price := new(big.Float).SetInt(answer)
	price.Quo(price, divisor)
	priceFloat, _ := price.Float64()
	return priceFloat
}

func CreatePriceFetcherWithChainlink(callContract contract.ContractCaller) PriceFetcher {
	return func(ctx context.Context, tokenAddr common.Address) (float64, bool) {
		return FetchPriceFromChainlink(ctx, tokenAddr, callContract)
	}
}
