package config

import (
	"fmt"
	"os"
	"strings"
)

type EnvReader func() string

func mapsTo(name string) EnvReader {
	return func() string {
		return getEnvVariable(name)
	}
}

func getEnvVariable(name string) string {
	value := strings.TrimSpace(os.Getenv(name))
	if value == "" {
		panic(fmt.Sprintf("required environment variable %s is not set", name))
	}
	return value
}

var (
	GetPolygonRPCURL        = mapsTo("POLYGON_RPC_URL")
	GetPairAddress          = mapsTo("PAIR_ADDRESS")
	GetDaprHost             = mapsTo("DAPR_HOST")
	GetDaprHTTPPort         = mapsTo("DAPR_HTTP_PORT")
	GetPubSubName           = mapsTo("PUBSUB_NAME")
	GetTopicTradingEvents   = mapsTo("TOPIC_TRADING_EVENTS")
	GetTopicLiquidityEvents = mapsTo("TOPIC_LIQUIDITY_EVENTS")
	GetAppPort              = mapsTo("APP_PORT")
)
