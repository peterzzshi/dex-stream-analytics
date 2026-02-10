package config

import (
	"errors"
	"os"
	"strconv"
	"time"

	"github.com/ethereum/go-ethereum/common"
)

// Config holds all application configuration
type Config struct {
	// Blockchain configuration
	PolygonRPCURL string
	PairAddress   common.Address

	// Kafka/DAPR configuration
	DaprHTTPPort      string
	DaprGRPCPort      string
	PubSubName        string
	TopicName         string
	SchemaRegistryURL string

	// Producer configuration
	BatchSize     int
	FlushInterval time.Duration
	RetryMax      int

	// Application configuration
	AppPort  string
	LogLevel string
	Env      string
}

// Load reads configuration from environment variables
func Load() (*Config, error) {
	cfg := &Config{
		PolygonRPCURL:     getEnv("POLYGON_RPC_URL", "https://polygon-rpc.com"),
		DaprHTTPPort:      getEnv("DAPR_HTTP_PORT", "3500"),
		DaprGRPCPort:      getEnv("DAPR_GRPC_PORT", "50001"),
		PubSubName:        getEnv("PUBSUB_NAME", "kafka-pubsub"),
		TopicName:         getEnv("TOPIC_DEX_EVENTS", "dex-events"),
		SchemaRegistryURL: getEnv("SCHEMA_REGISTRY_URL", "http://schema-registry:8081"),
		AppPort:           getEnv("APP_PORT", "3000"),
		LogLevel:          getEnv("LOG_LEVEL", "info"),
		Env:               getEnv("ENVIRONMENT", "development"),
		BatchSize:         getEnvInt("PRODUCER_BATCH_SIZE", 100),
		RetryMax:          getEnvInt("PRODUCER_RETRY_MAX", 3),
	}

	// Parse pair address
	pairAddrStr := getEnv("PAIR_ADDRESS", "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827")
	if !common.IsHexAddress(pairAddrStr) {
		return nil, errors.New("invalid PAIR_ADDRESS")
	}
	cfg.PairAddress = common.HexToAddress(pairAddrStr)

	// Parse flush interval
	flushIntervalStr := getEnv("PRODUCER_FLUSH_INTERVAL", "5s")
	interval, err := time.ParseDuration(flushIntervalStr)
	if err != nil {
		return nil, err
	}
	cfg.FlushInterval = interval

	return cfg, nil
}

// Validate checks if configuration is valid
func (c *Config) Validate() error {
	if c.PolygonRPCURL == "" {
		return errors.New("POLYGON_RPC_URL is required")
	}

	if c.PairAddress == (common.Address{}) {
		return errors.New("PAIR_ADDRESS is required")
	}

	if c.BatchSize <= 0 {
		return errors.New("PRODUCER_BATCH_SIZE must be positive")
	}

	if c.RetryMax < 0 {
		return errors.New("PRODUCER_RETRY_MAX cannot be negative")
	}

	return nil
}

// IsDevelopment returns true if running in development mode
func (c *Config) IsDevelopment() bool {
	return c.Env == "development"
}

// IsProduction returns true if running in production mode
func (c *Config) IsProduction() bool {
	return c.Env == "production"
}

// Helper functions
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getEnvInt(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if intVal, err := strconv.Atoi(value); err == nil {
			return intVal
		}
	}
	return defaultValue
}
