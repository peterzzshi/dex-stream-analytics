package contract

import (
	"context"
	_ "embed"
	"fmt"
	"strings"
	"sync"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
)

// Embedded ABI JSON files
//
//go:embed erc20_decimals.abi.json
var erc20DecimalsABIJSON string

//go:embed erc20_symbol.abi.json
var erc20SymbolABIJSON string

//go:embed chainlink_aggregator.abi.json
var chainlinkAggregatorABIJSON string

//go:embed uniswap_v2_pair.abi.json
var uniswapV2PairABIJSON string

// ABIName represents a known contract ABI identifier
type ABIName string

const (
	ERC20Decimals       ABIName = "erc20_decimals"
	ERC20Symbol         ABIName = "erc20_symbol"
	ChainlinkAggregator ABIName = "chainlink_aggregator"
	UniswapV2Pair       ABIName = "uniswap_v2_pair"
)

// Lazy-loaded ABI cache using sync.Once for each ABI (no locks needed after init)
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

// GetABI returns a parsed ABI by name (type-safe)
// Uses sync.Once per ABI - thread-safe, no locks after first call
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

// parseABI parses an ABI from the embedded JSON strings
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

// CallContract makes a read-only contract call with generic type support
func CallContract[T any](
	ctx context.Context,
	client *ethclient.Client,
	contractAddress common.Address,
	contractABI abi.ABI,
	methodName string,
	resultPtr *T,
) error {
	contract := bind.NewBoundContract(contractAddress, contractABI, client, client, client)
	results := []interface{}{resultPtr}
	return contract.Call(&bind.CallOpts{Context: ctx}, &results, methodName)
}
