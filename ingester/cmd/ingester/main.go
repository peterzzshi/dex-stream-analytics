package main

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"ingester/internal/avro"
	"ingester/internal/blockchain"
	"ingester/internal/config"
	"ingester/internal/publisher"
	"ingester/logger"
	"ingester/pkg/events"
)

func main() {
	logLevel := os.Getenv("LOG_LEVEL")
	applicationLogger := logger.New(logLevel)

	errorValue := run(applicationLogger)
	if errorValue != nil {
		applicationLogger.Fatal(context.Background(), "Ingester stopped", "error", errorValue)
	}
}

func run(applicationLogger *logger.Logger) error {
	executionContext, cancel := context.WithCancel(context.Background())
	defer cancel()

	configuration, errorValue := config.Load()
	if errorValue != nil {
		return errorValue
	}

	errorValue = configuration.Validate()
	if errorValue != nil {
		return errorValue
	}

	applicationLogger.Info(
		executionContext,
		"Configuration loaded",
		"remote_procedure_call_url", configuration.PolygonRPCURL,
		"pair_address", configuration.PairAddress.Hex(),
	)

	healthServer, errorValue := startHealthServer(configuration.ApplicationPort, applicationLogger)
	if errorValue != nil {
		return errorValue
	}
	defer stopHealthServer(healthServer, applicationLogger)

	swapEventCodec, errorValue := avro.NewSwapEventCodec()
	if errorValue != nil {
		return errorValue
	}

	eventPublisher := publisher.New(configuration, swapEventCodec)
	defer eventPublisher.Close()

	applicationLogger.Info(executionContext, "Initialising blockchain listener")
	blockchainListener, errorValue := blockchain.NewListener(executionContext, configuration.PolygonRPCURL, configuration.PairAddress, applicationLogger)
	if errorValue != nil {
		return errorValue
	}
	defer blockchainListener.Close()

	pairMetadata := blockchainListener.PairMetadata()
	applicationLogger.Info(
		executionContext,
		"Pair metadata loaded",
		"token0_address", pairMetadata.Token0Address.Hex(),
		"token1_address", pairMetadata.Token1Address.Hex(),
	)

	swapEventChannel := make(chan events.SwapEvent, 100)
	errorChannel := make(chan error, 1)

	go func() {
		errorChannel <- blockchainListener.Listen(executionContext, swapEventChannel)
	}()

	signalChannel := buildSignalChannel()

	return consumeEvents(
		executionContext,
		cancel,
		applicationLogger,
		eventPublisher,
		swapEventChannel,
		errorChannel,
		signalChannel,
	)
}

func buildSignalChannel() chan os.Signal {
	signalChannel := make(chan os.Signal, 1)
	signal.Notify(signalChannel, os.Interrupt, syscall.SIGTERM)
	return signalChannel
}

func consumeEvents(
	executionContext context.Context,
	cancel context.CancelFunc,
	applicationLogger *logger.Logger,
	eventPublisher *publisher.Publisher,
	swapEventChannel <-chan events.SwapEvent,
	errorChannel <-chan error,
	signalChannel <-chan os.Signal,
) error {
	for {
		select {
		case <-executionContext.Done():
			return executionContext.Err()
		case <-signalChannel:
			applicationLogger.Info(executionContext, "Shutdown signal received")
			cancel()
			return nil
		case errorValue := <-errorChannel:
			if errorValue == nil || errors.Is(errorValue, context.Canceled) {
				return nil
			}
			cancel()
			return errorValue
		case swapEvent := <-swapEventChannel:
			publishSwapEvent(executionContext, applicationLogger, eventPublisher, swapEvent)
		}
	}
}

func publishSwapEvent(
	executionContext context.Context,
	applicationLogger *logger.Logger,
	eventPublisher *publisher.Publisher,
	swapEvent events.SwapEvent,
) {
	errorValue := eventPublisher.Publish(executionContext, swapEvent)
	if errorValue != nil {
		applicationLogger.Error(
			executionContext,
			"Failed to publish swap event",
			"event_id", swapEvent.EventID,
			"error", errorValue,
		)
		return
	}

	applicationLogger.Info(
		executionContext,
		"Swap event published",
		"event_id", swapEvent.EventID,
		"transaction_hash", swapEvent.TransactionHash,
	)
}

func startHealthServer(applicationPort string, applicationLogger *logger.Logger) (*http.Server, error) {
	if applicationPort == "" {
		return nil, errors.New("APP_PORT is required")
	}

	healthServer := &http.Server{
		Addr:              fmt.Sprintf(":%s", applicationPort),
		Handler:           healthHandler(),
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		errorValue := healthServer.ListenAndServe()
		if errorValue == nil || errors.Is(errorValue, http.ErrServerClosed) {
			return
		}
		applicationLogger.Error(context.Background(), "Health server stopped", "error", errorValue)
	}()

	return healthServer, nil
}

func stopHealthServer(healthServer *http.Server, applicationLogger *logger.Logger) {
	if healthServer == nil {
		return
	}
	shutdownContext, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	errorValue := healthServer.Shutdown(shutdownContext)
	if errorValue != nil {
		applicationLogger.Error(context.Background(), "Failed to stop health server", "error", errorValue)
	}
}

func healthHandler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(writer http.ResponseWriter, _ *http.Request) {
		writer.WriteHeader(http.StatusOK)
	})
	return mux
}
