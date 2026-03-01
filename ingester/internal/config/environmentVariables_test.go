package config

import (
	"os"
	"testing"
)

func TestMapsTo(t *testing.T) {
	os.Setenv("TEST_VAR", "test-value")
	defer os.Unsetenv("TEST_VAR")

	getter := mapsTo("TEST_VAR")
	value := getter()

	if value != "test-value" {
		t.Errorf("Expected 'test-value', got '%s'", value)
	}
}

func TestMapsTo_LazyEvaluation(t *testing.T) {
	os.Setenv("TEST_VAR", "initial")
	defer os.Unsetenv("TEST_VAR")

	getter := mapsTo("TEST_VAR")

	value1 := getter()
	if value1 != "initial" {
		t.Errorf("Expected 'initial', got '%s'", value1)
	}

	os.Setenv("TEST_VAR", "updated")
	value2 := getter()
	if value2 != "updated" {
		t.Errorf("Expected 'updated' (lazy evaluation), got '%s'", value2)
	}
}

func TestGetEnvVariable_MissingVariable(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Expected panic for missing variable")
		}
	}()

	getEnvVariable("NONEXISTENT_VAR")
}

func TestGetEnvVariable_WhitespaceOnly(t *testing.T) {
	os.Setenv("TEST_VAR", "   ")
	defer os.Unsetenv("TEST_VAR")

	defer func() {
		if r := recover(); r == nil {
			t.Error("Expected panic for whitespace-only value")
		}
	}()

	getEnvVariable("TEST_VAR")
}

func TestGetEnvVariable_TrimsWhitespace(t *testing.T) {
	os.Setenv("TEST_VAR", "  value  ")
	defer os.Unsetenv("TEST_VAR")

	value := getEnvVariable("TEST_VAR")
	if value != "value" {
		t.Errorf("Expected 'value' (trimmed), got '%s'", value)
	}
}

func TestConfigGetters(t *testing.T) {
	testCases := []struct {
		envVar string
		getter func() string
	}{
		{"POLYGON_RPC_URL", GetPolygonRPCURL},
		{"PAIR_ADDRESS", GetPairAddress},
		{"DAPR_HOST", GetDaprHost},
		{"DAPR_HTTP_PORT", GetDaprHTTPPort},
		{"PUBSUB_NAME", GetPubSubName},
		{"TOPIC_TRADING_EVENTS", GetTopicTradingEvents},
		{"TOPIC_LIQUIDITY_EVENTS", GetTopicLiquidityEvents},
		{"APP_PORT", GetAppPort},
	}

	for _, tc := range testCases {
		t.Run(tc.envVar, func(t *testing.T) {
			os.Setenv(tc.envVar, "test-value")
			defer os.Unsetenv(tc.envVar)

			value := tc.getter()
			if value != "test-value" {
				t.Errorf("Expected 'test-value', got '%s'", value)
			}
		})
	}
}

func TestConfigGetters_Stateless(t *testing.T) {
	os.Setenv("POLYGON_RPC_URL", "initial-url")
	defer os.Unsetenv("POLYGON_RPC_URL")

	value1 := GetPolygonRPCURL()
	if value1 != "initial-url" {
		t.Errorf("Expected 'initial-url', got '%s'", value1)
	}

	os.Setenv("POLYGON_RPC_URL", "updated-url")
	value2 := GetPolygonRPCURL()
	if value2 != "updated-url" {
		t.Errorf("Expected 'updated-url' (stateless), got '%s'", value2)
	}
}
