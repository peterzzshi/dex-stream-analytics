package blockchain

import (
	"errors"
	"math/big"
	"strings"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

const uniswapV2PairABIJSON = `[{"anonymous":false,"inputs":[{"indexed":true,"internalType":"address","name":"sender","type":"address"},{"indexed":false,"internalType":"uint256","name":"amount0In","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"amount1In","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"amount0Out","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"amount1Out","type":"uint256"},{"indexed":true,"internalType":"address","name":"to","type":"address"}],"name":"Swap","type":"event"},{"inputs":[],"name":"token0","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"token1","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"}]`

type PairMetadata struct {
	PairAddress   common.Address
	Token0Address common.Address
	Token1Address common.Address
}

type SwapLogData struct {
	SenderAddress    common.Address
	RecipientAddress common.Address
	Amount0In        *big.Int
	Amount1In        *big.Int
	Amount0Out       *big.Int
	Amount1Out       *big.Int
}

func loadUniswapV2PairABI() (abi.ABI, error) {
	return abi.JSON(strings.NewReader(uniswapV2PairABIJSON))
}

func swapEventTopicFromABI(pairContractABI abi.ABI) (common.Hash, error) {
	swapEvent, found := pairContractABI.Events["Swap"]
	if !found {
		return common.Hash{}, errors.New("swap event not found in ABI")
	}
	return swapEvent.ID, nil
}

func parseSwapLog(pairContractABI abi.ABI, logEntry types.Log) (SwapLogData, error) {
	if len(logEntry.Topics) < 3 {
		return SwapLogData{}, errors.New("swap log is missing indexed topics")
	}

	decodedValues := map[string]interface{}{}
	errorValue := pairContractABI.UnpackIntoMap(decodedValues, "Swap", logEntry.Data)
	if errorValue != nil {
		return SwapLogData{}, errorValue
	}

	amount0In, errorValue := readBigInt(decodedValues, "amount0In")
	if errorValue != nil {
		return SwapLogData{}, errorValue
	}

	amount1In, errorValue := readBigInt(decodedValues, "amount1In")
	if errorValue != nil {
		return SwapLogData{}, errorValue
	}

	amount0Out, errorValue := readBigInt(decodedValues, "amount0Out")
	if errorValue != nil {
		return SwapLogData{}, errorValue
	}

	amount1Out, errorValue := readBigInt(decodedValues, "amount1Out")
	if errorValue != nil {
		return SwapLogData{}, errorValue
	}

	senderAddress := common.BytesToAddress(logEntry.Topics[1].Bytes())
	recipientAddress := common.BytesToAddress(logEntry.Topics[2].Bytes())

	return SwapLogData{
		SenderAddress:    senderAddress,
		RecipientAddress: recipientAddress,
		Amount0In:        amount0In,
		Amount1In:        amount1In,
		Amount0Out:       amount0Out,
		Amount1Out:       amount1Out,
	}, nil
}

func readBigInt(values map[string]interface{}, key string) (*big.Int, error) {
	value, found := values[key]
	if !found {
		return nil, errors.New("swap log is missing numeric value")
	}
	bigIntValue, valueIsBigInt := value.(*big.Int)
	if !valueIsBigInt {
		return nil, errors.New("swap log has invalid numeric value")
	}
	return bigIntValue, nil
}
