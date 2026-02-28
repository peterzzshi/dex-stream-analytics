package blockchain

import (
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"

	"ingester/internal/contract"
)

// UniswapV2PairABI is the parsed ABI for Uniswap V2 Pair contract
var UniswapV2PairABI = mustLoadUniswapV2ABI()

// Uniswap V2 standard event topic hashes
// Computed from event signatures: Keccak256("EventName(types...)")
var (
	SwapEventTopic = crypto.Keccak256Hash([]byte("Swap(address,uint256,uint256,uint256,uint256,address)"))
	MintEventTopic = crypto.Keccak256Hash([]byte("Mint(address,uint256,uint256)"))
	BurnEventTopic = crypto.Keccak256Hash([]byte("Burn(address,uint256,uint256,address)"))
)

func mustLoadUniswapV2ABI() abi.ABI {
	result, err := contract.GetABI(contract.UniswapV2Pair)
	if err != nil {
		panic(fmt.Sprintf("failed to load Uniswap V2 Pair ABI: %v", err))
	}
	return result
}

type PairMetadata struct {
	PairAddress    common.Address
	Token0Address  common.Address
	Token1Address  common.Address
	Token0Decimals uint8
	Token1Decimals uint8
}

// parseLog is the unified log parsing function — validates topics, unpacks data, delegates to parser
// parseSwapLog extracts swap event data directly from log entry
// Returns: sender, recipient, amount0In, amount1In, amount0Out, amount1Out
func parseSwapLog(logEntry types.Log) (sender, recipient common.Address, amount0In, amount1In, amount0Out, amount1Out *big.Int, err error) {
	if len(logEntry.Topics) < 3 {
		return common.Address{}, common.Address{}, nil, nil, nil, nil,
			fmt.Errorf("swap log missing indexed topics: expected 3, got %d", len(logEntry.Topics))
	}

	decodedValues := map[string]any{}
	if err := UniswapV2PairABI.UnpackIntoMap(decodedValues, "Swap", logEntry.Data); err != nil {
		return common.Address{}, common.Address{}, nil, nil, nil, nil,
			fmt.Errorf("unpack Swap data: %w", err)
	}

	amount0In, err = readBigInt(decodedValues, "amount0In")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, nil, nil, err
	}
	amount1In, err = readBigInt(decodedValues, "amount1In")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, nil, nil, err
	}
	amount0Out, err = readBigInt(decodedValues, "amount0Out")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, nil, nil, err
	}
	amount1Out, err = readBigInt(decodedValues, "amount1Out")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, nil, nil, err
	}

	if amount0In.Sign() < 0 || amount1In.Sign() < 0 || amount0Out.Sign() < 0 || amount1Out.Sign() < 0 {
		return common.Address{}, common.Address{}, nil, nil, nil, nil,
			fmt.Errorf("swap amounts cannot be negative")
	}

	sender = common.BytesToAddress(logEntry.Topics[1].Bytes())
	recipient = common.BytesToAddress(logEntry.Topics[2].Bytes())

	if sender == (common.Address{}) || recipient == (common.Address{}) {
		return common.Address{}, common.Address{}, nil, nil, nil, nil,
			fmt.Errorf("swap addresses cannot be zero")
	}

	return sender, recipient, amount0In, amount1In, amount0Out, amount1Out, nil
}

// parseMintLog extracts mint event data directly from log entry
// Returns: sender, amount0, amount1
func parseMintLog(logEntry types.Log) (sender common.Address, amount0, amount1 *big.Int, err error) {
	if len(logEntry.Topics) < 2 {
		return common.Address{}, nil, nil,
			fmt.Errorf("mint log missing indexed topics: expected 2, got %d", len(logEntry.Topics))
	}

	decodedValues := map[string]any{}
	if err := UniswapV2PairABI.UnpackIntoMap(decodedValues, "Mint", logEntry.Data); err != nil {
		return common.Address{}, nil, nil,
			fmt.Errorf("unpack Mint data: %w", err)
	}

	amount0, err = readBigInt(decodedValues, "amount0")
	if err != nil {
		return common.Address{}, nil, nil, err
	}
	amount1, err = readBigInt(decodedValues, "amount1")
	if err != nil {
		return common.Address{}, nil, nil, err
	}

	if amount0.Sign() < 0 || amount1.Sign() < 0 {
		return common.Address{}, nil, nil,
			fmt.Errorf("mint amounts cannot be negative")
	}

	sender = common.BytesToAddress(logEntry.Topics[1].Bytes())

	if sender == (common.Address{}) {
		return common.Address{}, nil, nil,
			fmt.Errorf("mint sender address cannot be zero")
	}

	return sender, amount0, amount1, nil
}

// parseBurnLog extracts burn event data directly from log entry
// Returns: sender, recipient, amount0, amount1
func parseBurnLog(logEntry types.Log) (sender, recipient common.Address, amount0, amount1 *big.Int, err error) {
	if len(logEntry.Topics) < 3 {
		return common.Address{}, common.Address{}, nil, nil,
			fmt.Errorf("burn log missing indexed topics: expected 3, got %d", len(logEntry.Topics))
	}

	decodedValues := map[string]any{}
	if err := UniswapV2PairABI.UnpackIntoMap(decodedValues, "Burn", logEntry.Data); err != nil {
		return common.Address{}, common.Address{}, nil, nil,
			fmt.Errorf("unpack Burn data: %w", err)
	}

	amount0, err = readBigInt(decodedValues, "amount0")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, err
	}
	amount1, err = readBigInt(decodedValues, "amount1")
	if err != nil {
		return common.Address{}, common.Address{}, nil, nil, err
	}

	if amount0.Sign() < 0 || amount1.Sign() < 0 {
		return common.Address{}, common.Address{}, nil, nil,
			fmt.Errorf("burn amounts cannot be negative")
	}

	sender = common.BytesToAddress(logEntry.Topics[1].Bytes())
	recipient = common.BytesToAddress(logEntry.Topics[2].Bytes())

	if sender == (common.Address{}) || recipient == (common.Address{}) {
		return common.Address{}, common.Address{}, nil, nil,
			fmt.Errorf("burn addresses cannot be zero")
	}

	return sender, recipient, amount0, amount1, nil
}

func readBigInt(values map[string]any, key string) (*big.Int, error) {
	value, found := values[key]
	if !found {
		return nil, fmt.Errorf("field %q not found in decoded values", key)
	}
	bigIntValue, ok := value.(*big.Int)
	if !ok {
		return nil, fmt.Errorf("field %q is not *big.Int, got %T", key, value)
	}
	return bigIntValue, nil
}
