package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"ingester/internal/blockchain"
	"ingester/internal/config"
	"ingester/internal/publisher"
	"ingester/pkg/logger"
)

func main() {
	// Initialize logger
	log := logger.New(os.Getenv("LOG_LEVEL"))
	log.Info("Starting Web3 DEX Analytics Ingester")

	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		log.Fatal("Failed to load configuration", "error", err)
	}

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		log.Fatal("Invalid configuration", "error", err)
	}

	log.Info("Configuration loaded",
		"rpc_url", cfg.PolygonRPCURL,
		"pair_address", cfg.PairAddress,
	)

	// Create context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Initialize publisher (DAPR pub/sub)
	pub, err := publisher.New(cfg, log)
	if err != nil {
		log.Fatal("Failed to create publisher", "error", err)
	}
	defer pub.Close()

	// Initialize blockchain listener
	listener, err := blockchain.NewListener(cfg, log)
	if err != nil {
		log.Fatal("Failed to create blockchain listener", "error", err)
	}
	defer listener.Close()

	// Channel for events
	eventChan := make(chan *blockchain.SwapEvent, 100)
	errorChan := make(chan error, 1)

	// Start listening to blockchain events
	go func() {
		log.Info("Starting blockchain event listener")
		if err := listener.Listen(ctx, eventChan); err != nil {
			errorChan <- fmt.Errorf("listener error: %w", err)
		}
	}()

	// Start publishing events
	go func() {
		log.Info("Starting event publisher")
		for {
			select {
			case <-ctx.Done():
				return
			case event := <-eventChan:
				if err := pub.Publish(ctx, event); err != nil {
					log.Error("Failed to publish event",
						"event_id", event.EventID,
						"error", err,
					)
					// Implement retry logic or dead letter queue here
					continue
				}
				log.Debug("Event published successfully",
					"event_id", event.EventID,
					"block", event.BlockNumber,
					"pair", event.PairAddress,
				)
			}
		}
	}()

	// Health check server
	go func() {
		if err := startHealthServer(cfg.AppPort, log); err != nil {
			log.Error("Health server error", "error", err)
		}
	}()

	// Wait for shutdown signal
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	select {
	case <-sigChan:
		log.Info("Shutdown signal received")
	case err := <-errorChan:
		log.Error("Fatal error", "error", err)
	}

	// Graceful shutdown
	log.Info("Shutting down gracefully...")
	cancel()
	time.Sleep(2 * time.Second)
	log.Info("Ingester stopped")
}

func startHealthServer(port string, log *logger.Logger) error {
	// Simple health check endpoint
	// Implementation in internal/server/health.go
	return nil
}
