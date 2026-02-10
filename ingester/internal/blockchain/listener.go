package blockchain

import (
	"context"
	"time"

	"ingester/pkg/logger"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
)

// Listener streams swap logs from a Uniswap V2 pair.
type Listener struct {
	rpcURL string
	pair   common.Address
	log    *logger.Logger
	client *ethclient.Client
}

func NewListener(rpcURL string, pair common.Address, log *logger.Logger) (*Listener, error) {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return nil, err
	}
	return &Listener{rpcURL: rpcURL, pair: pair, log: log, client: client}, nil
}

func (l *Listener) Listen(ctx context.Context, out chan<- SwapEvent) error {
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-ticker.C:
			// TODO: replace with real log subscription and parsing
			out <- SwapEvent{
				EventID:        time.Now().Format(time.RFC3339Nano),
				PairAddress:    l.pair.Hex(),
				BlockNumber:    0,
				BlockTimestamp: time.Now().Unix(),
			}
		}
	}
}

func (l *Listener) Close() { l.client.Close() }
