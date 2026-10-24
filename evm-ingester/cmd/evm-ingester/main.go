package main

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"

	"evm-ingester/internal/blockchain"
	"evm-ingester/internal/config"
	"evm-ingester/internal/dlq"
	apperr "evm-ingester/internal/errors"
	"evm-ingester/internal/events"
	"evm-ingester/internal/finality"
	"evm-ingester/internal/oracle"
	"evm-ingester/internal/publisher"
)

const (
	eventChannelBuffer  = 100
	dlqChannelBuffer    = 100
	healthServerTimeout = 5 * time.Second
	shutdownGracePeriod = 5 * time.Second
	publishTimeout      = 10 * time.Second
)

func main() {
	logger := newLogger()
	defer logger.Sync()

	if err := run(logger); err != nil {
		logger.Error("Ingester stopped", zap.Error(err))
		os.Exit(1)
	}
}

func newLogger() *zap.Logger {
	config := zap.NewProductionConfig()
	config.EncoderConfig.EncodeTime = zapcore.ISO8601TimeEncoder
	logger, err := config.Build()
	if err != nil {
		panic(fmt.Sprintf("failed to build logger: %v", err))
	}
	return logger
}

func run(logger *zap.Logger) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	cfg, err := config.LoadFromEnv()
	if err != nil {
		return err
	}

	chains := cfg.Chains

	httpDoer := newHTTPDoer()

	// Create DLQ filesystem sink
	dlqSink, err := dlq.NewFileSink("./dlq", logger.With(zap.String("component", "dlq")))
	if err != nil {
		return fmt.Errorf("failed to create DLQ sink: %w", err)
	}
	defer dlqSink.Close()

	healthServer, err := startHealthServer(cfg.App.Port, logger)
	if err != nil {
		return err
	}
	defer func() {
		if healthServer != nil {
			stopHealthServer(healthServer, logger)
		}
	}()

	errorChannel := make(chan error, len(chains))

	// One listener and one finality buffer per pool. Buffers are not shared across
	// chains: release is decided by comparing an event's block number to a single known
	// tip, so two chains at different heights in one buffer would release events from
	// one chain based on the other's tip.
	var listeners []*blockchain.Listener
	var processors sync.WaitGroup

	for _, chain := range chains {
		// Chain-level logger
		chainLogger := logger.With(
			zap.String("chain", chain.Chain),
			zap.String("chainName", chain.Name),
		)

		pub, err := publisher.New(cfg, chain, httpDoer, chainLogger)
		if err != nil {
			closeListeners(listeners)
			return err
		}

		// One RPC client and price oracle per chain, shared across all pools
		rpcClient, err := ethclient.Dial(chain.RPC)
		if err != nil {
			closeListeners(listeners)
			return &apperr.ConnectionError{Message: "rpc dial failed", Cause: err}
		}
		defer rpcClient.Close()

		priceOracle := oracle.NewChainlinkOracle(rpcClient)

		for _, pool := range chain.Pools {
			pairAddress := common.HexToAddress(pool.PairAddress)

			// Pool-level logger
			poolLogger := chainLogger.With(
				zap.String("pair", pool.PairAddress),
				zap.String("poolName", pool.Name),
			)

			listener, err := blockchain.NewListener(ctx, chain.RPC, pairAddress, chain.Chain, chain.Name, priceOracle, poolLogger)
			if err != nil {
				closeListeners(listeners)
				return err
			}
			listeners = append(listeners, listener)

			buffer := finality.NewBuffer(chain.Confirmations, poolLogger)
			eventChannel := make(chan events.Event, eventChannelBuffer)
			dlqChannel := make(chan events.Event, dlqChannelBuffer)

			logger.Info("Pool configured",
				zap.String("chain", chain.Chain),
				zap.String("chainName", chain.Name),
				zap.String("pool", pool.Name),
				zap.String("pair", pool.PairAddress),
				zap.Uint64("finalityConfirmations", buffer.RequiredConfirmations()),
			)

			processors.Add(1)
			go func(poolLogger *zap.Logger) {
				defer processors.Done()
				consumeEvents(ctx, pub, buffer, eventChannel, dlqChannel, poolLogger)
			}(poolLogger)

			// Start DLQ consumer for this pool
			processors.Add(1)
			go func(dlqCh <-chan events.Event) {
				defer processors.Done()
				dlqSink.Consume(ctx, dlqCh)
			}(dlqChannel)

			go func() {
				if err := listener.Listen(ctx, eventChannel); err != nil {
					select {
					case errorChannel <- err:
					case <-ctx.Done():
					}
				}
			}()
		}
	}
	defer closeListeners(listeners)

	signalChannel := make(chan os.Signal, 1)
	signal.Notify(signalChannel, os.Interrupt, syscall.SIGTERM)

	select {
	case <-signalChannel:
		logger.Info("Shutdown signal received, flushing finality buffers")
		cancel()
		// Buffers release their unconfirmed events on context cancellation, so the
		// graceful shutdown path drains everything already observed.
		processors.Wait()
		return nil
	case err := <-errorChannel:
		if err == nil || errors.Is(err, context.Canceled) {
			return nil
		}
		cancel()
		return err
	}
}

// consumeEvents buffers events until final, publishing as they confirm. Returns when the
// context is cancelled, flushing whatever remains buffered.
func consumeEvents(
	ctx context.Context,
	pub *publisher.Publisher,
	buffer *finality.Buffer,
	eventChannel <-chan events.Event,
	dlqChannel chan<- events.Event,
	logger *zap.Logger,
) {
	for {
		select {
		case <-ctx.Done():
			flush(ctx, pub, buffer, dlqChannel, logger)
			return
		case event := <-eventChannel:
			result := buffer.Add(event)

			// Route evicted events to DLQ
			for _, evt := range result.Evicted {
				select {
				case dlqChannel <- evt:
					logger.Info("Event routed to DLQ (buffer eviction)",
						zap.String("event_id", evt.GetEventID()),
						zap.String("pair", evt.GetPairAddress()),
					)
				case <-ctx.Done():
					return
				default:
					logger.Error("DLQ channel full, dropping evicted event",
						zap.String("event_id", evt.GetEventID()),
					)
				}
			}

			// Publish confirmed events with retry
			for _, evt := range result.Confirmed {
				publishWithRetryOrDLQ(ctx, pub, evt, dlqChannel, logger)
			}
		}
	}
}

func publishWithRetryOrDLQ(
	ctx context.Context,
	pub *publisher.Publisher,
	event events.Event,
	dlqChannel chan<- events.Event,
	logger *zap.Logger,
) {
	publishCtx, cancel := context.WithTimeout(ctx, publishTimeout)
	defer cancel()

	err := pub.PublishWithRetry(publishCtx, event)
	if err != nil {
		select {
		case dlqChannel <- event:
			logger.Info("Event routed to DLQ (publish failure)",
				zap.String("event_id", event.GetEventID()),
				zap.String("pair", event.GetPairAddress()),
				zap.Error(err),
			)
		case <-ctx.Done():
		default:
			logger.Error("DLQ channel full, dropping failed event",
				zap.String("event_id", event.GetEventID()),
			)
		}
	}
}

func flush(ctx context.Context, pub *publisher.Publisher, buffer *finality.Buffer, dlqChannel chan<- events.Event, logger *zap.Logger) {
	pending := buffer.Flush()
	if len(pending) == 0 {
		return
	}
	logger.Info("Flushing unconfirmed events", zap.Int("count", len(pending)))

	// Shutdown cancels the parent context, so publish under a fresh one to let the
	// in-flight flush finish rather than dropping exactly the events it exists to save.
	flushCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), shutdownGracePeriod)
	defer cancel()

	for _, evt := range pending {
		publishWithRetryOrDLQ(flushCtx, pub, evt, dlqChannel, logger)
	}
}

func closeListeners(listeners []*blockchain.Listener) {
	for _, listener := range listeners {
		listener.Close()
	}
}

func newHTTPDoer() publisher.HTTPDoer {
	client := &http.Client{Timeout: publishTimeout}
	return func(req *http.Request) (*http.Response, error) {
		return client.Do(req)
	}
}

func startHealthServer(appPort int, logger *zap.Logger) (*http.Server, error) {
	if appPort <= 0 {
		return nil, &apperr.ConfigError{Message: "app.port is required"}
	}

	healthServer := &http.Server{
		Addr:              fmt.Sprintf(":%d", appPort),
		Handler:           healthHandler(),
		ReadHeaderTimeout: healthServerTimeout,
	}

	errChan := make(chan error, 1)
	go func() {
		err := healthServer.ListenAndServe()
		if err != nil && !errors.Is(err, http.ErrServerClosed) {
			select {
			case errChan <- err:
			default:
				logger.Error("Health server stopped unexpectedly", zap.Error(err))
			}
		}
	}()

	select {
	case err := <-errChan:
		return nil, &apperr.ConnectionError{Message: "health server failed to start", Cause: err}
	case <-time.After(100 * time.Millisecond):
		// Server started successfully
	}

	return healthServer, nil
}

func stopHealthServer(healthServer *http.Server, logger *zap.Logger) {
	shutdownCtx, cancel := context.WithTimeout(context.Background(), shutdownGracePeriod)
	defer cancel()
	err := healthServer.Shutdown(shutdownCtx)
	if err != nil {
		logger.Error("Failed to stop health server", zap.Error(err))
	}
}

func healthHandler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(writer http.ResponseWriter, _ *http.Request) {
		writer.WriteHeader(http.StatusOK)
	})
	return mux
}
