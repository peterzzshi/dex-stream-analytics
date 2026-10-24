// Package config loads the EVM ingester's declarative YAML configuration.
//
// Structural facts (chains, pools, topics, Dapr coordinates) live in the YAML, so they
// are versioned and reviewable. Secrets never appear in it: values written as ${VAR}
// are expanded from the environment at load time and loading fails if the variable is
// unset. Expansion covers the whole file, comments included.
package config

import (
	"fmt"
	"os"
	"strings"

	"github.com/ethereum/go-ethereum/common"
	"gopkg.in/yaml.v3"

	apperr "evm-ingester/internal/errors"
)

const EnvConfigPath = "CONFIG_PATH"

// Config is the root of evm-ingester.yaml.
type Config struct {
	Dapr   DaprConfig   `yaml:"dapr"`
	Topics TopicsConfig `yaml:"topics"`
	App    AppConfig    `yaml:"app"`
	Chains []Chain      `yaml:"chains"`
}

type DaprConfig struct {
	Host       string `yaml:"host"`
	HTTPPort   int    `yaml:"httpPort"`
	PubSubName string `yaml:"pubsubName"`
}

type TopicsConfig struct {
	Trading   string `yaml:"trading"`
	Liquidity string `yaml:"liquidity"`
}

type AppConfig struct {
	Port int `yaml:"port"`
}

type Chain struct {
	Chain         string `yaml:"chain"`
	Name          string `yaml:"name"`
	RPC           string `yaml:"rpc"`
	Venue         string `yaml:"venue"`
	Confirmations uint64 `yaml:"confirmations"`
	Pools         []Pool `yaml:"pools"`
}

type Pool struct {
	PairAddress string `yaml:"pairAddress"`
	Name        string `yaml:"name"`
}

func (c Chain) VenueOr() string {
	if c.Venue != "" {
		return c.Venue
	}
	return "uniswap-v2"
}

// Load reads, expands, and decodes the config. Validates only runtime-variable concerns:
// environment expansion, address formats, type support.
func Load(path string) (*Config, error) {
	if path == "" {
		return nil, &apperr.ConfigError{Message: "CONFIG_PATH is required"}
	}

	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, &apperr.ConfigError{Message: fmt.Sprintf("cannot read %s", path), Cause: err}
	}

	expanded, err := expandEnv(string(raw))
	if err != nil {
		return nil, err
	}

	var cfg Config
	decoder := yaml.NewDecoder(strings.NewReader(expanded))
	decoder.KnownFields(true) // Catch typos
	if err := decoder.Decode(&cfg); err != nil {
		return nil, &apperr.ConfigError{Message: "invalid YAML", Cause: err}
	}

	if err := cfg.validate(); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func LoadFromEnv() (*Config, error) {
	return Load(os.Getenv(EnvConfigPath))
}

// validate checks only runtime-variable concerns: address formats.
func (c *Config) validate() error {
	for _, chain := range c.Chains {
		// Validate EVM address format for all pools
		for _, pool := range chain.Pools {
			if !common.IsHexAddress(pool.PairAddress) {
				return &apperr.ConfigError{Message: fmt.Sprintf(
					"chain %q pool %q has invalid EVM address: %s",
					chain.Chain, pool.Name, pool.PairAddress)}
			}
		}
	}

	return nil
}

// expandEnv substitutes ${VAR}, failing if unset. Secrets can be misconfigured.
func expandEnv(text string) (string, error) {
	var missing []string

	expanded := os.Expand(text, func(name string) string {
		value := strings.TrimSpace(os.Getenv(name))
		if value == "" {
			missing = append(missing, name)
		}
		return value
	})

	if len(missing) > 0 {
		return "", &apperr.ConfigError{Message: fmt.Sprintf(
			"config references unset environment variables: %s", strings.Join(missing, ", "))}
	}

	return expanded, nil
}
