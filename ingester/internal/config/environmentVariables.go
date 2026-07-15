package config

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

// EnvReader lazily reads a required environment variable.
// Panics on first access if unset (fail-fast startup, same as template.Must).
type EnvReader func() string

var (
	GetPolygonRPCURL        = mustEnv("POLYGON_RPC_URL")
	GetPairAddress          = mustEnv("PAIR_ADDRESS")
	GetDaprHost             = mustEnv("DAPR_HOST")
	GetDaprHTTPPort         = mustEnv("DAPR_HTTP_PORT")
	GetPubSubName           = mustEnv("PUBSUB_NAME")
	GetTopicTradingEvents   = mustEnv("TOPIC_TRADING_EVENTS")
	GetTopicLiquidityEvents = mustEnv("TOPIC_LIQUIDITY_EVENTS")
	GetAppPort              = mustEnv("APP_PORT")
)

// DefaultFinalityConfirmations is the default N-confirmation depth for Polygon.
const DefaultFinalityConfirmations uint64 = 64

// GetFinalityConfirmations returns the configured confirmation depth.
// Returns DefaultFinalityConfirmations if FINALITY_CONFIRMATIONS is unset.
// Set to 0 to disable finality buffering (pass-through mode).
func GetFinalityConfirmations() uint64 {
	raw := strings.TrimSpace(os.Getenv("FINALITY_CONFIRMATIONS"))
	if raw == "" {
		return DefaultFinalityConfirmations
	}
	n, err := strconv.ParseUint(raw, 10, 64)
	if err != nil {
		panic(fmt.Sprintf("FINALITY_CONFIRMATIONS must be a non-negative integer, got: %s", raw))
	}
	return n
}

func mustEnv(name string) EnvReader {
	return func() string {
		value := strings.TrimSpace(os.Getenv(name))
		if value == "" {
			panic(fmt.Sprintf("required environment variable %s is not set", name))
		}
		return value
	}
}
