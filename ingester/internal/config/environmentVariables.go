package config

import (
	"fmt"
	"os"
	"strings"
)

// config holds all environment variables loaded at startup
var config struct {
	PolygonRPCURL        string
	PairAddress          string
	DaprHost             string
	DaprHTTPPort         string
	PubSubName           string
	TopicTradingEvents   string
	TopicLiquidityEvents string
	AppPort              string
}

// init loads all environment variables at startup (fail-fast)
func init() {
	config.PolygonRPCURL = mustGetEnv("POLYGON_RPC_URL")
	config.PairAddress = mustGetEnv("PAIR_ADDRESS")
	config.DaprHost = mustGetEnv("DAPR_HOST")
	config.DaprHTTPPort = mustGetEnv("DAPR_HTTP_PORT")
	config.PubSubName = mustGetEnv("PUBSUB_NAME")
	config.TopicTradingEvents = mustGetEnv("TOPIC_TRADING_EVENTS")
	config.TopicLiquidityEvents = mustGetEnv("TOPIC_LIQUIDITY_EVENTS")
	config.AppPort = mustGetEnv("APP_PORT")
}

// mustGetEnv returns the environment variable value or panics if not set
func mustGetEnv(name string) string {
	value := strings.TrimSpace(os.Getenv(name))
	if value == "" {
		panic(fmt.Sprintf("required environment variable %s is not set", name))
	}
	return value
}

// Getter functions - zero overhead, just return pre-loaded values
func GetPolygonRPCURL() string        { return config.PolygonRPCURL }
func GetPairAddress() string          { return config.PairAddress }
func GetDaprHost() string             { return config.DaprHost }
func GetDaprHTTPPort() string         { return config.DaprHTTPPort }
func GetPubSubName() string           { return config.PubSubName }
func GetTopicTradingEvents() string   { return config.TopicTradingEvents }
func GetTopicLiquidityEvents() string { return config.TopicLiquidityEvents }
func GetAppPort() string              { return config.AppPort }
