package logger

import "context"

type contextKey string

const logContextKey contextKey = "logContext"

type LogContext struct {
	data LogContextData
}

func NewLogContext(data LogContextData) *LogContext {
	if data.Tags == nil {
		data.Tags = make(map[string]bool)
	}
	if data.Metadata == nil {
		data.Metadata = make(map[string]string)
	}
	return &LogContext{data: data}
}

func (logContext *LogContext) WithCategory(category string) *LogContext {
	updatedData := logContext.copyLogContextData()
	updatedData.Category = category
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) WithSessionID(sessionID string) *LogContext {
	updatedData := logContext.copyLogContextData()
	updatedData.SessionID = sessionID
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) WithTags(tags ...string) *LogContext {
	updatedData := logContext.copyLogContextData()
	for _, tag := range tags {
		updatedData.Tags[tag] = true
	}
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) WithoutTags(tags ...string) *LogContext {
	updatedData := logContext.copyLogContextData()
	for _, tag := range tags {
		delete(updatedData.Tags, tag)
	}
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) WithMetadata(metadata map[string]string) *LogContext {
	updatedData := logContext.copyLogContextData()
	for key, value := range metadata {
		updatedData.Metadata[key] = value
	}
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) WithoutMetadata(keys ...string) *LogContext {
	updatedData := logContext.copyLogContextData()
	for _, key := range keys {
		delete(updatedData.Metadata, key)
	}
	return &LogContext{data: updatedData}
}

func (logContext *LogContext) copyLogContextData() LogContextData {
	tags := make(map[string]bool, len(logContext.data.Tags))
	for key, value := range logContext.data.Tags {
		tags[key] = value
	}
	metadata := make(map[string]string, len(logContext.data.Metadata))
	for key, value := range logContext.data.Metadata {
		metadata[key] = value
	}
	return LogContextData{
		Tags:      tags,
		Category:  logContext.data.Category,
		Metadata:  metadata,
		SessionID: logContext.data.SessionID,
	}
}

func GetLogContext(executionContext context.Context) *LogContext {
	if logContext, ok := executionContext.Value(logContextKey).(*LogContext); ok {
		return logContext
	}
	return NewLogContext(LogContextData{})
}

func WithLogContext[T any](executionContext context.Context, logContext *LogContext, callback func(context.Context) (T, error)) (T, error) {
	enrichedContext := context.WithValue(executionContext, logContextKey, logContext)
	return callback(enrichedContext)
}
