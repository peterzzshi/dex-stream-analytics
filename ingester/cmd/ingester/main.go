package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/ethereum/go-ethereum/common"

	"ingester/internal/blockchain"
	"ingester/internal/config"
	ierrors "ingester/internal/errors"
	"ingester/internal/events"
	"ingester/internal/publisher"
)

const (
	eventChannelBuffer  = 100
	healthServerTimeout = 5 * time.Second
	shutdownGracePeriod = 5 * time.Second
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

func main() {
	if err := run(); err != nil {
		logger.Error("Ingester stopped", "error", err)
		os.Exit(1)
	}
}

func run() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	rpcURL := config.GetPolygonRPCURL()
	if err := validateRPCURL(rpcURL); err != nil {
		return err
	}

	pairAddress, err := parsePairAddress(config.GetPairAddress())
	if err != nil {
		return err
	}

	logger.Info("Configuration loaded", "pair", pairAddress.Hex())

	healthServer, err := startHealthServer(config.GetAppPort())
	if err != nil {
		return err
	}
	defer func() {
		if healthServer != nil {
			stopHealthServer(healthServer)
		}
	}()

	codecs, err := publisher.CreateCodecMap()
	if err != nil {
		return err
	}

	topicMapper := publisher.DefaultTopicMapper()
	urlBuilder := publisher.DefaultURLBuilder()
	httpDoer := func(req *http.Request) (*http.Response, error) {
		client := &http.Client{Timeout: 10 * time.Second}
		return client.Do(req)
	}

	listener, err := blockchain.NewListener(ctx, rpcURL, pairAddress)
	if err != nil {
		return err
	}
	defer listener.Close()

	logger.Info("Listener ready")

	eventChannel := make(chan events.Event, eventChannelBuffer)
	errorChannel := make(chan error, 1)

	go func() {
		errorChannel <- listener.Listen(ctx, eventChannel)
	}()

	signalChannel := make(chan os.Signal, 1)
	signal.Notify(signalChannel, os.Interrupt, syscall.SIGTERM)

	return consumeEvents(ctx, cancel, codecs, topicMapper, urlBuilder, httpDoer, eventChannel, errorChannel, signalChannel)
}

func consumeEvents(
	ctx context.Context,
	cancel context.CancelFunc,
	codecs publisher.CodecMap,
	topicMapper publisher.TopicMapper,
	urlBuilder publisher.URLBuilder,
	httpDoer publisher.HTTPDoer,
	eventChannel <-chan events.Event,
	errorChannel <-chan error,
	signalChannel <-chan os.Signal,
) error {
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-signalChannel:
			logger.Info("Shutdown signal received")
			cancel()
			return nil
		case err := <-errorChannel:
			if err == nil || errors.Is(err, context.Canceled) {
				return nil
			}
			cancel()
			return err
		case event := <-eventChannel:
			publisher.Publish(ctx, event, codecs, topicMapper, urlBuilder, httpDoer)
		}
	}
}

func startHealthServer(appPort string) (*http.Server, error) {
	if appPort == "" {
		return nil, ierrors.Config("APP_PORT", "port is required")
	}

	healthServer := &http.Server{
		Addr:              fmt.Sprintf(":%s", appPort),
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
				logger.Error("Health server stopped unexpectedly", "error", err)
			}
		}
	}()

	select {
	case err := <-errChan:
		return nil, ierrors.Connection("health-server", err)
	case <-time.After(100 * time.Millisecond):
		// Server started successfully
	}

	return healthServer, nil
}

func stopHealthServer(healthServer *http.Server) {
	shutdownCtx, cancel := context.WithTimeout(context.Background(), shutdownGracePeriod)
	defer cancel()
	err := healthServer.Shutdown(shutdownCtx)
	if err != nil {
		logger.Error("Failed to stop health server", "error", err)
	}
}

func healthHandler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(writer http.ResponseWriter, _ *http.Request) {
		writer.WriteHeader(http.StatusOK)
	})
	return mux
}

func parsePairAddress(text string) (common.Address, error) {
	if !common.IsHexAddress(text) {
		return common.Address{}, ierrors.Config("PAIR_ADDRESS", "must be valid hex address")
	}
	return common.HexToAddress(text), nil
}

func validateRPCURL(url string) error {
	if url == "" {
		return ierrors.Config("POLYGON_RPC_URL", "required (must be WebSocket: wss:// or ws://)")
	}
	if len(url) < 5 {
		return ierrors.Config("POLYGON_RPC_URL", "URL too short")
	}
	if strings.HasPrefix(url, "wss://") || strings.HasPrefix(url, "ws://") {
		return nil
	}
	return ierrors.Config("POLYGON_RPC_URL", "must be WebSocket URL (wss:// or ws://)")
}
