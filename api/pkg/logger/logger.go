package logger

import (
	"log/slog"
	"os"
	"strings"
)

type Logger struct {
	baseLogger *slog.Logger
}

func New(levelText string) *Logger {
	level := parseLevel(levelText)
	handler := slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: level})
	return &Logger{baseLogger: slog.New(handler)}
}

func (logger *Logger) Debug(message string, keyValues ...any) {
	logger.baseLogger.Debug(message, keyValues...)
}

func (logger *Logger) Info(message string, keyValues ...any) {
	logger.baseLogger.Info(message, keyValues...)
}

func (logger *Logger) Warn(message string, keyValues ...any) {
	logger.baseLogger.Warn(message, keyValues...)
}

func (logger *Logger) Error(message string, keyValues ...any) {
	logger.baseLogger.Error(message, keyValues...)
}

func (logger *Logger) Fatal(message string, keyValues ...any) {
	logger.baseLogger.Error(message, keyValues...)
	os.Exit(1)
}

func parseLevel(levelText string) slog.Level {
	switch strings.ToLower(strings.TrimSpace(levelText)) {
	case "debug":
		return slog.LevelDebug
	case "warn", "warning":
		return slog.LevelWarn
	case "error":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}
