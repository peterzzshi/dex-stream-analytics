package config

import (
	"fmt"
	"os"
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

func mustEnv(name string) EnvReader {
	return func() string {
		value := strings.TrimSpace(os.Getenv(name))
		if value == "" {
			panic(fmt.Sprintf("required environment variable %s is not set", name))
		}
		return value
	}
}
