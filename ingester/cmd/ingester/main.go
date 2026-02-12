package main

import (
	"context"
	"errors"
	"os"
	"os/signal"
	"syscall"

	"ingester/internal/blockchain"
	"ingester/internal/config"
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

	return consumeEvents(executionContext, cancel, applicationLogger, swapEventChannel, errorChannel, signalChannel)
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
			logSwapEvent(executionContext, applicationLogger, swapEvent)
		}
	}
}

func logSwapEvent(executionContext context.Context, applicationLogger *logger.Logger, swapEvent events.SwapEvent) {
	applicationLogger.Info(executionContext, "Swap event captured", "event", swapEvent)
}
