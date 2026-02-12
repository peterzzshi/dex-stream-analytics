package logger

import (
	"context"
	"log/slog"
	"os"
	"sort"
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

func (logger *Logger) Debug(executionContext context.Context, message string, keyValues ...any) {
	logger.log(executionContext, slog.LevelDebug, message, keyValues...)
}

func (logger *Logger) Info(executionContext context.Context, message string, keyValues ...any) {
	logger.log(executionContext, slog.LevelInfo, message, keyValues...)
}

func (logger *Logger) Warn(executionContext context.Context, message string, keyValues ...any) {
	logger.log(executionContext, slog.LevelWarn, message, keyValues...)
}

func (logger *Logger) Error(executionContext context.Context, message string, keyValues ...any) {
	logger.log(executionContext, slog.LevelError, message, keyValues...)
}

func (logger *Logger) Fatal(executionContext context.Context, message string, keyValues ...any) {
	logger.log(executionContext, slog.LevelError, message, keyValues...)
	os.Exit(1)
}

func (logger *Logger) log(executionContext context.Context, level slog.Level, message string, keyValues ...any) {
	if executionContext == nil {
		executionContext = context.Background()
	}
	if !logger.baseLogger.Enabled(executionContext, level) {
		return
	}

	logContext := GetLogContext(executionContext)
	contextKeyValues := logContextKeyValues(logContext)
	combinedKeyValues := mergeKeyValues(contextKeyValues, keyValues)

	logger.baseLogger.Log(executionContext, level, message, combinedKeyValues...)
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

func logContextKeyValues(logContext *LogContext) []any {
	if logContext == nil {
		return nil
	}

	attributes := make([]any, 0, 8)
	data := logContext.data

	if data.SessionID != "" {
		attributes = append(attributes, "session_id", data.SessionID)
	}

	if data.Category != "" {
		attributes = append(attributes, "category", data.Category)
	}

	if len(data.Tags) > 0 {
		tags := make([]string, 0, len(data.Tags))
		for tag := range data.Tags {
			tags = append(tags, tag)
		}
		sort.Strings(tags)
		attributes = append(attributes, "tags", tags)
	}

	if len(data.Metadata) > 0 {
		metadata := make(map[string]string, len(data.Metadata))
		for key, value := range data.Metadata {
			metadata[key] = value
		}
		attributes = append(attributes, "metadata", metadata)
	}

	return attributes
}

func mergeKeyValues(primaryKeyValues []any, secondaryKeyValues []any) []any {
	if len(primaryKeyValues) == 0 {
		return secondaryKeyValues
	}
	if len(secondaryKeyValues) == 0 {
		return primaryKeyValues
	}
	combinedKeyValues := make([]any, 0, len(primaryKeyValues)+len(secondaryKeyValues))
	combinedKeyValues = append(combinedKeyValues, primaryKeyValues...)
	combinedKeyValues = append(combinedKeyValues, secondaryKeyValues...)
	return combinedKeyValues
}
