package blockchain

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"math/big"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/ethclient"

	"ingester/internal/cache"
	"ingester/internal/contract"
	ierrors "ingester/internal/errors"
	"ingester/internal/events"
	"ingester/internal/oracle"
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

// symbolCache caches ERC20 token symbols (never expires since symbols are immutable)
var symbolCache = cache.NewCache[common.Address, string](0)

type Listener struct {
	pairAddress  common.Address
	client       *ethclient.Client
	pairMetadata PairMetadata
	priceCache   *cache.Cache[common.Address, float64]
	priceFetcher oracle.PriceFetcher
}

func NewListener(ctx context.Context, rpcURL string, pairAddress common.Address) (*Listener, error) {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return nil, ierrors.Connection("rpc", err)
	}

	pairMetadata, err := fetchPairMetadata(ctx, client, pairAddress, UniswapV2PairABI)
	if err != nil {
		client.Close()
		return nil, err
	}

	// Create ContractCaller function for FP-style oracle
	contractCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		return client.CallContract(ctx, call, blockNumber)
	}

	// Create price fetcher using FP style
	priceFetcher := oracle.CreatePriceFetcherWithChainlink(contractCaller)

	// Create price cache (5 minutes TTL)
	priceCache := cache.NewCache[common.Address, float64](5 * time.Minute)

	return &Listener{
		pairAddress:  pairAddress,
		client:       client,
		pairMetadata: pairMetadata,
		priceCache:   priceCache,
		priceFetcher: priceFetcher,
	}, nil
}

func (l *Listener) PairMetadata() PairMetadata {
	return l.pairMetadata
}

func (l *Listener) Listen(ctx context.Context, outputChannel chan<- events.Event) error {
	filterQuery := ethereum.FilterQuery{
		Addresses: []common.Address{l.pairAddress},
		Topics:    [][]common.Hash{{SwapEventTopic, MintEventTopic, BurnEventTopic}},
	}

	logChannel := make(chan types.Log)
	subscription, err := l.client.SubscribeFilterLogs(ctx, filterQuery, logChannel)
	if err != nil {
		return ierrors.Connection("event-subscription", err)
	}

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case subscriptionErr := <-subscription.Err():
			return ierrors.Connection("event-stream", subscriptionErr)
		case logEntry := <-logChannel:
			event, err := l.eventFromLog(ctx, logEntry)
			if err != nil {
				var dataErr *ierrors.DataError
				if errors.As(err, &dataErr) {
					logger.Warn("Skipping bad event data", "error", err)
					continue
				}
				return err
			}
			select {
			case outputChannel <- event:
			case <-ctx.Done():
				return ctx.Err()
			}
		}
	}
}

func (l *Listener) Close() {
	l.client.Close()
}

func (l *Listener) eventFromLog(ctx context.Context, logEntry types.Log) (events.Event, error) {
	if len(logEntry.Topics) == 0 {
		return nil, ierrors.Data("parse", fmt.Sprintf("block=%d tx=%s", logEntry.BlockNumber, logEntry.TxHash.Hex()),
			fmt.Errorf("no topics"))
	}

	topic := logEntry.Topics[0]

	switch topic {
	case SwapEventTopic:
		return l.parseSwapEvent(ctx, logEntry)
	case MintEventTopic:
		return l.parseMintEvent(ctx, logEntry)
	case BurnEventTopic:
		return l.parseBurnEvent(ctx, logEntry)
	default:
		return nil, ierrors.Data("parse", fmt.Sprintf("block=%d tx=%s", logEntry.BlockNumber, logEntry.TxHash.Hex()),
			fmt.Errorf("unknown topic: %s", topic.Hex()))
	}
}

func (l *Listener) parseSwapEvent(ctx context.Context, logEntry types.Log) (events.SwapEvent, error) {
	sender, recipient, amount0In, amount1In, amount0Out, amount1Out, err := parseSwapLog(logEntry)
	if err != nil {
		return events.SwapEvent{}, ierrors.Data("parse", fmt.Sprintf("swap block=%d tx=%s", logEntry.BlockNumber, logEntry.TxHash.Hex()), err)
	}

	blockTimestamp, err := fetchBlockTimestamp(ctx, l.client, logEntry.BlockNumber)
	if err != nil {
		return events.SwapEvent{}, err
	}

	gasUsed, gasPrice, err := fetchGasDetails(ctx, l.client, logEntry.TxHash)
	if err != nil {
		return events.SwapEvent{}, err
	}

	base := buildBaseEvent(ctx, l.client, logEntry, events.EventTypeSwap, blockTimestamp, l.pairMetadata)
	price := priceFromSwapAmounts(amount0In, amount1In, amount0Out, amount1Out, l.pairMetadata)

	return events.SwapEvent{
		BaseEvent:  base,
		Sender:     sender.Hex(),
		Recipient:  recipient.Hex(),
		Amount0In:  amount0In.String(),
		Amount1In:  amount1In.String(),
		Amount0Out: amount0Out.String(),
		Amount1Out: amount1Out.String(),
		Price:      price,
		VolumeUSD:  volumeUSDFromSwap(ctx, l.priceCache, l.priceFetcher, amount0In, amount1In, amount0Out, amount1Out, l.pairMetadata, price),
		GasUsed:    gasUsed,
		GasPrice:   gasPrice,
	}, nil
}

func (l *Listener) parseMintEvent(ctx context.Context, logEntry types.Log) (events.MintEvent, error) {
	sender, amount0, amount1, err := parseMintLog(logEntry)
	if err != nil {
		return events.MintEvent{}, ierrors.Data("parse", fmt.Sprintf("mint block=%d tx=%s", logEntry.BlockNumber, logEntry.TxHash.Hex()), err)
	}

	blockTimestamp, err := fetchBlockTimestamp(ctx, l.client, logEntry.BlockNumber)
	if err != nil {
		return events.MintEvent{}, err
	}

	base := buildBaseEvent(ctx, l.client, logEntry, events.EventTypeMint, blockTimestamp, l.pairMetadata)

	return events.MintEvent{
		BaseEvent: base,
		Sender:    sender.Hex(),
		Amount0:   amount0.String(),
		Amount1:   amount1.String(),
	}, nil
}

func (l *Listener) parseBurnEvent(ctx context.Context, logEntry types.Log) (events.BurnEvent, error) {
	sender, recipient, amount0, amount1, err := parseBurnLog(logEntry)
	if err != nil {
		return events.BurnEvent{}, ierrors.Data("parse", fmt.Sprintf("burn block=%d tx=%s", logEntry.BlockNumber, logEntry.TxHash.Hex()), err)
	}

	blockTimestamp, err := fetchBlockTimestamp(ctx, l.client, logEntry.BlockNumber)
	if err != nil {
		return events.BurnEvent{}, err
	}

	base := buildBaseEvent(ctx, l.client, logEntry, events.EventTypeBurn, blockTimestamp, l.pairMetadata)

	return events.BurnEvent{
		BaseEvent: base,
		Sender:    sender.Hex(),
		Recipient: recipient.Hex(),
		Amount0:   amount0.String(),
		Amount1:   amount1.String(),
	}, nil
}

func buildBaseEvent(ctx context.Context, client *ethclient.Client, logEntry types.Log, eventType events.EventType, blockTimestamp int64, pairMetadata PairMetadata) events.BaseEvent {
	// Build event identifier inline (txHash:logIndex)
	var eventIDBuilder strings.Builder
	eventIDBuilder.Grow(66 + 1 + 10) // 0x + 64 hex chars + : + max uint digits
	eventIDBuilder.WriteString(logEntry.TxHash.Hex())
	eventIDBuilder.WriteByte(':')
	eventIDBuilder.WriteString(strconv.FormatUint(uint64(logEntry.Index), 10))

	// Fetch token symbols (with caching to minimize RPC calls)
	// First event from a pair will make 2 RPC calls, subsequent events hit cache
	token0Symbol := fetchTokenSymbol(ctx, client, pairMetadata.Token0Address)
	token1Symbol := fetchTokenSymbol(ctx, client, pairMetadata.Token1Address)

	// Convert to *string (nullable - empty means symbol not available)
	var token0SymbolPtr, token1SymbolPtr *string
	if token0Symbol != "" {
		token0SymbolPtr = &token0Symbol
	}
	if token1Symbol != "" {
		token1SymbolPtr = &token1Symbol
	}

	return events.BaseEvent{
		EventType:       string(eventType),
		EventID:         eventIDBuilder.String(),
		BlockNumber:     int64(logEntry.BlockNumber),
		BlockTimestamp:  blockTimestamp,
		TransactionHash: logEntry.TxHash.Hex(),
		LogIndex:        int32(logEntry.Index),
		PairAddress:     pairMetadata.PairAddress.Hex(),
		Token0:          pairMetadata.Token0Address.Hex(),
		Token1:          pairMetadata.Token1Address.Hex(),
		Token0Symbol:    token0SymbolPtr,
		Token1Symbol:    token1SymbolPtr,
		EventTimestamp:  time.Now().Unix(),
	}
}

func priceFromSwapAmounts(amount0In, amount1In, amount0Out, amount1Out *big.Int, pairMetadata PairMetadata) float64 {
	if amount0In.Sign() > 0 && amount1Out.Sign() > 0 {
		return ratioToFloat64(amount1Out, amount0In, pairMetadata.Token1Decimals, pairMetadata.Token0Decimals)
	}
	if amount1In.Sign() > 0 && amount0Out.Sign() > 0 {
		return ratioToFloat64(amount1In, amount0Out, pairMetadata.Token1Decimals, pairMetadata.Token0Decimals)
	}
	return 0
}

// volumeUSDFromSwap calculates USD volume using Chainlink price oracle (FP style)
// Pure function with injected price fetching behavior
func volumeUSDFromSwap(
	ctx context.Context,
	priceCache *cache.Cache[common.Address, float64],
	priceFetcher oracle.PriceFetcher,
	amount0In, amount1In, amount0Out, amount1Out *big.Int,
	pairMetadata PairMetadata,
	price float64,
) *float64 {
	// Calculate accurate USD volume using Chainlink price oracle
	// Four-tier strategy: stablecoin direct → oracle token0 → oracle token1 → price estimation

	// Strategy 1: If token1 is a stablecoin (most accurate, ~70% of pairs)
	// Oracle returns $1.00 immediately for stablecoins (USDC/USDT/DAI)
	token1Price, foundToken1 := oracle.GetTokenUSDPrice(ctx, pairMetadata.Token1Address, priceCache, priceFetcher)
	if foundToken1 && token1Price == 1.0 {
		// It's a stablecoin - use amount directly
		var amount1 *big.Int
		if amount1In.Sign() > 0 {
			amount1 = amount1In
		} else {
			amount1 = amount1Out
		}
		volumeUSD := adjustForDecimals(amount1, pairMetadata.Token1Decimals)
		return &volumeUSD
	}

	// Strategy 2: If token0 is a stablecoin
	token0Price, foundToken0 := oracle.GetTokenUSDPrice(ctx, pairMetadata.Token0Address, priceCache, priceFetcher)
	if foundToken0 && token0Price == 1.0 {
		// It's a stablecoin - use amount directly
		var amount0 *big.Int
		if amount0In.Sign() > 0 {
			amount0 = amount0In
		} else {
			amount0 = amount0Out
		}
		volumeUSD := adjustForDecimals(amount0, pairMetadata.Token0Decimals)
		return &volumeUSD
	}

	// Strategy 3: Use Chainlink oracle for non-stablecoin token0
	if foundToken0 && token0Price > 0 {
		var amount0 *big.Int
		if amount0In.Sign() > 0 {
			amount0 = amount0In
		} else {
			amount0 = amount0Out
		}
		token0Volume := adjustForDecimals(amount0, pairMetadata.Token0Decimals)
		volumeUSD := token0Volume * token0Price
		return &volumeUSD
	}

	// Strategy 4: Use Chainlink oracle for non-stablecoin token1
	if foundToken1 && token1Price > 0 {
		var amount1 *big.Int
		if amount1In.Sign() > 0 {
			amount1 = amount1In
		} else {
			amount1 = amount1Out
		}
		token1Volume := adjustForDecimals(amount1, pairMetadata.Token1Decimals)
		volumeUSD := token1Volume * token1Price
		return &volumeUSD
	}

	// Last resort: Use swap price (token1-denominated volume)
	// This happens when neither token has a Chainlink feed
	if price > 0 {
		var amount0 *big.Int
		if amount0In.Sign() > 0 {
			amount0 = amount0In
		} else {
			amount0 = amount0Out
		}
		token0Volume := adjustForDecimals(amount0, pairMetadata.Token0Decimals)
		volumeInToken1Terms := token0Volume * price
		return &volumeInToken1Terms
	}

	// No pricing available
	return nil
}

func adjustForDecimals(amount *big.Int, decimals uint8) float64 {
	divisor := new(big.Int).Exp(big.NewInt(10), big.NewInt(int64(decimals)), nil)
	ratio := new(big.Rat).SetFrac(amount, divisor)
	value, _ := ratio.Float64()
	return value
}

func ratioToFloat64(numerator *big.Int, denominator *big.Int, numeratorDecimals uint8, denominatorDecimals uint8) float64 {
	if denominator.Sign() == 0 {
		return 0
	}

	// Adjust for decimal differences: price = (numerator / 10^numDecimals) / (denominator / 10^denomDecimals)
	// Simplifies to: price = numerator * 10^(denomDecimals - numDecimals) / denominator
	decimalDiff := int(denominatorDecimals) - int(numeratorDecimals)

	adjustedNumerator := new(big.Int).Set(numerator)
	if decimalDiff > 0 {
		// Multiply numerator by 10^decimalDiff
		multiplier := new(big.Int).Exp(big.NewInt(10), big.NewInt(int64(decimalDiff)), nil)
		adjustedNumerator.Mul(adjustedNumerator, multiplier)
	} else if decimalDiff < 0 {
		// Multiply denominator by 10^(-decimalDiff)
		multiplier := new(big.Int).Exp(big.NewInt(10), big.NewInt(int64(-decimalDiff)), nil)
		denominator = new(big.Int).Mul(denominator, multiplier)
	}

	ratio := new(big.Rat).SetFrac(adjustedNumerator, denominator)
	value, _ := ratio.Float64()
	return value
}

func fetchBlockTimestamp(ctx context.Context, client *ethclient.Client, blockNumber uint64) (int64, error) {
	blockNum := new(big.Int).SetUint64(blockNumber)
	header, err := client.HeaderByNumber(ctx, blockNum)
	if err != nil {
		return 0, ierrors.Connection("rpc", err)
	}
	return int64(header.Time), nil
}

func fetchGasDetails(ctx context.Context, client *ethclient.Client, txHash common.Hash) (int64, string, error) {
	receipt, err := client.TransactionReceipt(ctx, txHash)
	if err != nil {
		return 0, "", ierrors.Connection("rpc", err)
	}

	gasPrice := receipt.EffectiveGasPrice
	if gasPrice == nil {
		gasPrice = big.NewInt(0)
	}

	return int64(receipt.GasUsed), gasPrice.String(), nil
}

func fetchPairMetadata(ctx context.Context, client *ethclient.Client, pairAddress common.Address, pairABI abi.ABI) (PairMetadata, error) {
	pairContract := bind.NewBoundContract(pairAddress, pairABI, client, client, client)

	token0Results := []interface{}{new(common.Address)}
	if err := pairContract.Call(&bind.CallOpts{Context: ctx}, &token0Results, "token0"); err != nil {
		if ctx.Err() != nil {
			return PairMetadata{}, ctx.Err()
		}
		return PairMetadata{}, ierrors.Connection("rpc", err)
	}
	token0Address := *token0Results[0].(*common.Address)

	token1Results := []interface{}{new(common.Address)}
	if err := pairContract.Call(&bind.CallOpts{Context: ctx}, &token1Results, "token1"); err != nil {
		if ctx.Err() != nil {
			return PairMetadata{}, ctx.Err()
		}
		return PairMetadata{}, ierrors.Connection("rpc", err)
	}
	token1Address := *token1Results[0].(*common.Address)

	token0Decimals, err := fetchTokenDecimals(ctx, client, token0Address)
	if err != nil {
		return PairMetadata{}, err
	}

	token1Decimals, err := fetchTokenDecimals(ctx, client, token1Address)
	if err != nil {
		return PairMetadata{}, err
	}

	return PairMetadata{
		PairAddress:    pairAddress,
		Token0Address:  token0Address,
		Token1Address:  token1Address,
		Token0Decimals: token0Decimals,
		Token1Decimals: token1Decimals,
	}, nil
}

func fetchTokenDecimals(ctx context.Context, client *ethclient.Client, tokenAddress common.Address) (uint8, error) {
	decimalsABI, err := contract.GetABI(contract.ERC20Decimals)
	if err != nil {
		return 0, ierrors.Config("ABI", "failed to get ERC20 decimals ABI")
	}

	// Create ContractCaller from ethclient
	contractCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
		return client.CallContract(ctx, call, blockNumber)
	}

	var decimals uint8
	if err := contract.CallContract(ctx, contractCaller, tokenAddress, decimalsABI, "decimals", &decimals); err != nil {
		return 0, err
	}

	return decimals, nil
}

// fetchTokenSymbol retrieves the ERC20 symbol for a token address with caching
func fetchTokenSymbol(ctx context.Context, client *ethclient.Client, tokenAddress common.Address) string {
	symbol, found := symbolCache.GetOrFetch(ctx, tokenAddress, func(ctx context.Context, addr common.Address) (string, bool) {
		symbolABI, err := contract.GetABI(contract.ERC20Symbol)
		if err != nil {
			logger.Error("Failed to get symbol ABI", "error", err)
			return "", false
		}

		// Create ContractCaller from ethclient
		contractCaller := func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error) {
			return client.CallContract(ctx, call, blockNumber)
		}

		var symbol string
		if err := contract.CallContract(ctx, contractCaller, addr, symbolABI, "symbol", &symbol); err != nil {
			logger.Debug("Symbol fetch failed", "token", addr.Hex(), "error", err)
			return "", false
		}

		// Validate symbol
		if symbol == "" || len(symbol) > 20 {
			logger.Debug("Invalid symbol", "token", addr.Hex(), "symbol", symbol)
			return "", false
		}

		return symbol, true
	})

	if found {
		return symbol
	}
	return ""
}
