package config

import "os"

type Config struct {
    AppPort string
    Env     string
    LogLevel string
}

func Load() (*Config, error) {
    return &Config{
        AppPort: env("APP_PORT", "8080"),
        Env:     env("ENVIRONMENT", "development"),
        LogLevel: env("LOG_LEVEL", "info"),
    }, nil
}

func (c *Config) Validate() error { return nil }
func (c *Config) IsDevelopment() bool { return c.Env == "development" }

func env(key, def string) string {
    if v := os.Getenv(key); v != "" {
        return v
    }
    return def
}
