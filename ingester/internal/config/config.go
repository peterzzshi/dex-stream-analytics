package config

import (
	"errors"
	"os"
	"strconv"
	"time"

	"github.com/ethereum/go-ethereum/common"
)

type Config struct {
	PolygonRPCURL string
	PairAddress   common.Address

	DaprHTTPPort string
	DaprGRPCPort string
	PubSubName   string
	TopicName    string

	BatchSize     int
	FlushInterval time.Duration
	RetryMaximum  int

	ApplicationPort string
	LogLevel        string
	Environment     string
}

func Load() (*Config, error) {
	configuration := &Config{
		PolygonRPCURL:   getEnvironmentValue("POLYGON_RPC_URL", ""),
		DaprHTTPPort:    getEnvironmentValue("DAPR_HTTP_PORT", "3500"),
		DaprGRPCPort:    getEnvironmentValue("DAPR_GRPC_PORT", "50001"),
		PubSubName:      getEnvironmentValue("PUBSUB_NAME", "kafka-pubsub"),
		TopicName:       getEnvironmentValue("TOPIC_DEX_EVENTS", "dex-events"),
		ApplicationPort: getEnvironmentValue("APP_PORT", "3000"),
		LogLevel:        getEnvironmentValue("LOG_LEVEL", "info"),
		Environment:     getEnvironmentValue("ENVIRONMENT", "development"),
		BatchSize:       getEnvironmentInt("PRODUCER_BATCH_SIZE", 100),
		RetryMaximum:    getEnvironmentInt("PRODUCER_RETRY_MAX", 3),
	}

	pairAddressText := getEnvironmentValue("PAIR_ADDRESS", "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827")
	if !common.IsHexAddress(pairAddressText) {
		return nil, errors.New("invalid PAIR_ADDRESS")
	}
	configuration.PairAddress = common.HexToAddress(pairAddressText)

	flushIntervalText := getEnvironmentValue("PRODUCER_FLUSH_INTERVAL", "5s")
	flushInterval, errorValue := time.ParseDuration(flushIntervalText)
	if errorValue != nil {
		return nil, errorValue
	}
	configuration.FlushInterval = flushInterval

	return configuration, nil
}

func (configuration *Config) Validate() error {
	if configuration.PolygonRPCURL == "" {
		return errors.New("POLYGON_RPC_URL is required - must be a WebSocket endpoint (wss://...)")
	}

	// Check if URL starts with wss:// for WebSocket support
	if len(configuration.PolygonRPCURL) >= 6 && configuration.PolygonRPCURL[:6] != "wss://" && configuration.PolygonRPCURL[:5] != "ws://" {
		return errors.New("POLYGON_RPC_URL must be a WebSocket endpoint starting with wss:// or ws:// (not http:// or https://). Event subscriptions require WebSocket support")
	}

	if configuration.PairAddress == (common.Address{}) {
		return errors.New("PAIR_ADDRESS is required")
	}

	if configuration.BatchSize <= 0 {
		return errors.New("PRODUCER_BATCH_SIZE must be positive")
	}

	if configuration.RetryMaximum < 0 {
		return errors.New("PRODUCER_RETRY_MAX cannot be negative")
	}

	return nil
}

func (configuration *Config) IsDevelopment() bool {
	return configuration.Environment == "development"
}

func (configuration *Config) IsProduction() bool {
	return configuration.Environment == "production"
}

func getEnvironmentValue(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getEnvironmentInt(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if intValue, errorValue := strconv.Atoi(value); errorValue == nil {
			return intValue
		}
	}
	return defaultValue
}
