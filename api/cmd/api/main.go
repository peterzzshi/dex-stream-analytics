package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"api/internal/config"
	api "api/internal/handlers"
	"api/internal/storage"
	"api/internal/subscriber"
	"api/pkg/logger"

	"github.com/gin-gonic/gin"
)

//	@title			Web3 DEX Analytics API
//	@version		1.0
//	@description	REST API for DEX aggregated analytics
//	@termsOfService	http://swagger.io/terms/

//	@contact.name	API Support
//	@contact.url	http://www.swagger.io/support
//	@contact.email	support@swagger.io

//	@license.name	MIT
//	@license.url	https://opensource.org/licenses/MIT

//	@host		localhost:8080
//	@BasePath	/api/v1

func main() {
	// Initialize logger
	log := logger.New(os.Getenv("LOG_LEVEL"))
	log.Info("Starting Web3 DEX Analytics API")

	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		log.Fatal("Failed to load configuration", "error", err)
	}

	if err := cfg.Validate(); err != nil {
		log.Fatal("Invalid configuration", "error", err)
	}

	log.Info("Configuration loaded", "port", cfg.AppPort)

	// Create context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Initialize storage
	store := storage.NewMemory()

	// Initialize subscriber (DAPR pub/sub)
	sub := subscriber.New(store)
	defer sub.Close()

	// Start consuming analytics events
	go func() {
		log.Info("Starting analytics subscriber")
		if err := sub.Start(ctx); err != nil {
			log.Error("Subscriber error", "error", err)
		}
	}()

	// Setup Gin router
	if !cfg.IsDevelopment() {
		gin.SetMode(gin.ReleaseMode)
	}

	router := gin.New()
	router.Use(gin.Recovery())
	router.Use(gin.Logger())

	// Setup routes
	api.SetupRoutes(router, store)

	// Create HTTP server
	srv := &http.Server{
		Addr:         fmt.Sprintf(":%s", cfg.AppPort),
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Start server in goroutine
	go func() {
		log.Info("Starting HTTP server", "port", cfg.AppPort)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatal("Server failed to start", "error", err)
		}
	}()

	// Wait for shutdown signal
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	<-sigChan
	log.Info("Shutdown signal received")

	// Graceful shutdown
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer shutdownCancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Error("Server forced to shutdown", "error", err)
	}

	cancel()
	time.Sleep(2 * time.Second)
	log.Info("API stopped")
}
