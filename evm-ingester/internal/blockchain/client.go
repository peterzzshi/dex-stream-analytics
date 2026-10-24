package blockchain

import (
	"context"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/ethclient"
)

// Client abstracts JSON-RPC operations used by Listener.
// *ethclient.Client satisfies this without adaptation.
type Client interface {
	BlockNumber(ctx context.Context) (uint64, error)
	SubscribeFilterLogs(ctx context.Context, q ethereum.FilterQuery, ch chan<- types.Log) (ethereum.Subscription, error)
	HeaderByNumber(ctx context.Context, number *big.Int) (*types.Header, error)
	TransactionReceipt(ctx context.Context, txHash common.Hash) (*types.Receipt, error)
	CallContract(ctx context.Context, msg ethereum.CallMsg, blockNumber *big.Int) ([]byte, error)
	Close()
}

var _ Client = (*ethclient.Client)(nil)

// CallContract calls a contract method and unpacks the result into resultPtr.
func CallContract[T any](
	ctx context.Context,
	client Client,
	contractAddress common.Address,
	contractABI abi.ABI,
	methodName string,
	resultPtr *T,
) error {
	data, err := contractABI.Pack(methodName)
	if err != nil {
		return fmt.Errorf("pack method %s: %w", methodName, err)
	}

	result, err := client.CallContract(ctx, ethereum.CallMsg{
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
