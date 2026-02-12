package blockchain

import (
	"context"
	"fmt"
	"math/big"
	"time"

	"ingester/logger"
	"ingester/pkg/events"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/ethclient"
)

type Listener struct {
	pairAddress       common.Address
	applicationLogger *logger.Logger
	client            *ethclient.Client
	pairContractABI   abi.ABI
	swapEventTopic    common.Hash
	pairMetadata      PairMetadata
}

func NewListener(executionContext context.Context, remoteProcedureCallURL string, pairAddress common.Address, applicationLogger *logger.Logger) (*Listener, error) {
	client, errorValue := ethclient.Dial(remoteProcedureCallURL)
	if errorValue != nil {
		return nil, errorValue
	}

	pairContractABI, errorValue := loadUniswapV2PairABI()
	if errorValue != nil {
		client.Close()
		return nil, errorValue
	}

	swapEventTopic, errorValue := swapEventTopicFromABI(pairContractABI)
	if errorValue != nil {
		client.Close()
		return nil, errorValue
	}

	pairMetadata, errorValue := fetchPairMetadata(executionContext, client, pairAddress, pairContractABI)
	if errorValue != nil {
		client.Close()
		return nil, errorValue
	}

	return &Listener{
		pairAddress:       pairAddress,
		applicationLogger: applicationLogger,
		client:            client,
		pairContractABI:   pairContractABI,
		swapEventTopic:    swapEventTopic,
		pairMetadata:      pairMetadata,
	}, nil
}

func (listener *Listener) PairMetadata() PairMetadata {
	return listener.pairMetadata
}

func (listener *Listener) Listen(executionContext context.Context, outputChannel chan<- events.SwapEvent) error {
	filterQuery := ethereum.FilterQuery{
		Addresses: []common.Address{listener.pairAddress},
		Topics:    [][]common.Hash{{listener.swapEventTopic}},
	}

	logChannel := make(chan types.Log)
	subscription, errorValue := listener.client.SubscribeFilterLogs(executionContext, filterQuery, logChannel)
	if errorValue != nil {
		return errorValue
	}

	for {
		select {
		case <-executionContext.Done():
			return executionContext.Err()
		case subscriptionError := <-subscription.Err():
			return subscriptionError
		case logEntry := <-logChannel:
			swapEvent, errorValue := listener.swapEventFromLog(executionContext, logEntry)
			if errorValue != nil {
				listener.applicationLogger.Error(executionContext, "Failed to parse swap event", "error", errorValue)
				continue
			}
			outputChannel <- swapEvent
		}
	}
}

func (listener *Listener) Close() {
	listener.client.Close()
}

func (listener *Listener) swapEventFromLog(executionContext context.Context, logEntry types.Log) (events.SwapEvent, error) {
	swapLogData, errorValue := parseSwapLog(listener.pairContractABI, logEntry)
	if errorValue != nil {
		return events.SwapEvent{}, errorValue
	}

	blockTimestamp, errorValue := fetchBlockTimestamp(executionContext, listener.client, logEntry.BlockNumber)
	if errorValue != nil {
		return events.SwapEvent{}, errorValue
	}

	gasUsed, gasPrice, errorValue := fetchGasDetails(executionContext, listener.client, logEntry.TxHash)
	if errorValue != nil {
		return events.SwapEvent{}, errorValue
	}

	eventTimestamp := time.Now().Unix()

	swapEventEnvelope := SwapEventEnvelope{
		SwapLogData:     swapLogData,
		PairMetadata:    listener.pairMetadata,
		BlockNumber:     logEntry.BlockNumber,
		BlockTimestamp:  blockTimestamp,
		TransactionHash: logEntry.TxHash,
		LogIndex:        logEntry.Index,
		GasUsed:         gasUsed,
		GasPrice:        gasPrice,
		EventTimestamp:  eventTimestamp,
	}

	return buildSwapEvent(swapEventEnvelope), nil
}

type SwapEventEnvelope struct {
	SwapLogData     SwapLogData
	PairMetadata    PairMetadata
	BlockNumber     uint64
	BlockTimestamp  int64
	TransactionHash common.Hash
	LogIndex        uint
	GasUsed         int64
	GasPrice        string
	EventTimestamp  int64
}

func buildSwapEvent(envelope SwapEventEnvelope) events.SwapEvent {
	return events.SwapEvent{
		EventID:         buildEventIdentifier(envelope.TransactionHash, envelope.LogIndex),
		BlockNumber:     int64(envelope.BlockNumber),
		BlockTimestamp:  envelope.BlockTimestamp,
		TransactionHash: envelope.TransactionHash.Hex(),
		LogIndex:        int32(envelope.LogIndex),
		PairAddress:     envelope.PairMetadata.PairAddress.Hex(),
		Token0:          envelope.PairMetadata.Token0Address.Hex(),
		Token1:          envelope.PairMetadata.Token1Address.Hex(),
		Token0Symbol:    nil,
		Token1Symbol:    nil,
		Sender:          envelope.SwapLogData.SenderAddress.Hex(),
		Recipient:       envelope.SwapLogData.RecipientAddress.Hex(),
		Amount0In:       envelope.SwapLogData.Amount0In.String(),
		Amount1In:       envelope.SwapLogData.Amount1In.String(),
		Amount0Out:      envelope.SwapLogData.Amount0Out.String(),
		Amount1Out:      envelope.SwapLogData.Amount1Out.String(),
		Price:           priceFromSwapAmounts(envelope.SwapLogData),
		VolumeUSD:       nil,
		GasUsed:         envelope.GasUsed,
		GasPrice:        envelope.GasPrice,
		EventTimestamp:  envelope.EventTimestamp,
	}
}

func buildEventIdentifier(transactionHash common.Hash, logIndex uint) string {
	return fmt.Sprintf("%s:%d", transactionHash.Hex(), logIndex)
}

func priceFromSwapAmounts(swapLogData SwapLogData) float64 {
	if swapLogData.Amount0In.Sign() > 0 && swapLogData.Amount1Out.Sign() > 0 {
		return ratioToFloat64(swapLogData.Amount1Out, swapLogData.Amount0In)
	}
	if swapLogData.Amount1In.Sign() > 0 && swapLogData.Amount0Out.Sign() > 0 {
		return ratioToFloat64(swapLogData.Amount1In, swapLogData.Amount0Out)
	}
	return 0
}

func ratioToFloat64(numerator *big.Int, denominator *big.Int) float64 {
	if denominator.Sign() == 0 {
		return 0
	}
	ratio := new(big.Rat).SetFrac(numerator, denominator)
	value, _ := ratio.Float64()
	return value
}

func fetchBlockTimestamp(executionContext context.Context, client *ethclient.Client, blockNumber uint64) (int64, error) {
	blockNumberValue := new(big.Int).SetUint64(blockNumber)
	header, errorValue := client.HeaderByNumber(executionContext, blockNumberValue)
	if errorValue != nil {
		return 0, errorValue
	}
	return int64(header.Time), nil
}

func fetchGasDetails(executionContext context.Context, client *ethclient.Client, transactionHash common.Hash) (int64, string, error) {
	receipt, errorValue := client.TransactionReceipt(executionContext, transactionHash)
	if errorValue != nil {
		return 0, "", errorValue
	}

	gasPrice := receipt.EffectiveGasPrice
	if gasPrice == nil {
		gasPrice = big.NewInt(0)
	}

	return int64(receipt.GasUsed), gasPrice.String(), nil
}

func fetchPairMetadata(executionContext context.Context, client *ethclient.Client, pairAddress common.Address, pairContractABI abi.ABI) (PairMetadata, error) {
	contract := bind.NewBoundContract(pairAddress, pairContractABI, client, client, client)

	token0Results := []interface{}{new(common.Address)}
	errorValue := contract.Call(&bind.CallOpts{Context: executionContext}, &token0Results, "token0")
	if errorValue != nil {
		return PairMetadata{}, errorValue
	}
	token0Address := *token0Results[0].(*common.Address)

	token1Results := []interface{}{new(common.Address)}
	errorValue = contract.Call(&bind.CallOpts{Context: executionContext}, &token1Results, "token1")
	if errorValue != nil {
		return PairMetadata{}, errorValue
	}
	token1Address := *token1Results[0].(*common.Address)

	return PairMetadata{
		PairAddress:   pairAddress,
		Token0Address: token0Address,
		Token1Address: token1Address,
	}, nil
}
