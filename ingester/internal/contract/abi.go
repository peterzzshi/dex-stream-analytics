package contract

import (
	"context"
	_ "embed"
	"fmt"
	"math/big"
	"strings"
	"sync"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

//go:embed erc20_decimals.abi.json
var erc20DecimalsABIJSON string

//go:embed erc20_symbol.abi.json
var erc20SymbolABIJSON string

//go:embed chainlink_aggregator.abi.json
var chainlinkAggregatorABIJSON string

//go:embed uniswap_v2_pair.abi.json
var uniswapV2PairABIJSON string

type ABIName string

const (
	ERC20Decimals       ABIName = "erc20_decimals"
	ERC20Symbol         ABIName = "erc20_symbol"
	ChainlinkAggregator ABIName = "chainlink_aggregator"
	UniswapV2Pair       ABIName = "uniswap_v2_pair"
)

type ContractCaller func(ctx context.Context, call ethereum.CallMsg, blockNumber *big.Int) ([]byte, error)

var (
	erc20DecimalsABI        abi.ABI
	erc20DecimalsOnce       sync.Once
	erc20SymbolABI          abi.ABI
	erc20SymbolOnce         sync.Once
	chainlinkAggregatorABI  abi.ABI
	chainlinkAggregatorOnce sync.Once
	uniswapV2PairABI        abi.ABI
	uniswapV2PairOnce       sync.Once
)

func GetABI(name ABIName) (abi.ABI, error) {
	switch name {
	case ERC20Decimals:
		var err error
		erc20DecimalsOnce.Do(func() {
			erc20DecimalsABI, err = parseABI(ERC20Decimals)
		})
		return erc20DecimalsABI, err

	case ERC20Symbol:
		var err error
		erc20SymbolOnce.Do(func() {
			erc20SymbolABI, err = parseABI(ERC20Symbol)
		})
		return erc20SymbolABI, err

	case ChainlinkAggregator:
		var err error
		chainlinkAggregatorOnce.Do(func() {
			chainlinkAggregatorABI, err = parseABI(ChainlinkAggregator)
		})
		return chainlinkAggregatorABI, err

	case UniswapV2Pair:
		var err error
		uniswapV2PairOnce.Do(func() {
			uniswapV2PairABI, err = parseABI(UniswapV2Pair)
		})
		return uniswapV2PairABI, err

	default:
		return abi.ABI{}, fmt.Errorf("unknown ABI: %s", name)
	}
}

func parseABI(name ABIName) (abi.ABI, error) {
	var jsonStr string

	switch name {
	case ERC20Decimals:
		jsonStr = erc20DecimalsABIJSON
	case ERC20Symbol:
		jsonStr = erc20SymbolABIJSON
	case ChainlinkAggregator:
		jsonStr = chainlinkAggregatorABIJSON
	case UniswapV2Pair:
		jsonStr = uniswapV2PairABIJSON
	default:
		return abi.ABI{}, fmt.Errorf("unknown ABI: %s", name)
	}

	parsed, err := abi.JSON(strings.NewReader(jsonStr))
	if err != nil {
		return abi.ABI{}, fmt.Errorf("parse %s ABI: %w", name, err)
	}

	return parsed, nil
}

func CallContract[T any](
	ctx context.Context,
	callContract ContractCaller,
	contractAddress common.Address,
	contractABI abi.ABI,
	methodName string,
	resultPtr *T,
) error {
	data, err := contractABI.Pack(methodName)
	if err != nil {
		return fmt.Errorf("pack method %s: %w", methodName, err)
	}

	result, err := callContract(ctx, ethereum.CallMsg{
		To:   &contractAddress,
		Data: data,
	}, nil)
	if err != nil {
		return fmt.Errorf("call contract: %w", err)
	}

	method, ok := contractABI.Methods[methodName]
	if !ok {
		return fmt.Errorf("method %s not found in ABI", methodName)
	}

	values, err := method.Outputs.Unpack(result)
	if err != nil {
		return fmt.Errorf("unpack result: %w", err)
	}

	if err := method.Outputs.Copy(resultPtr, values); err != nil {
		return fmt.Errorf("copy result: %w", err)
	}

	return nil
}
