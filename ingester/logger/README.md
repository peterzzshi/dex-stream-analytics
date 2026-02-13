# Context-aware slog Logger

This package wraps Go's `log/slog` with context-scoped attributes for structured JSON logging.

## Why slog for this project

- Standard library support with stable semantics and fewer moving parts.
- Built-in JSON handler and level filtering.
- Easy to attach structured attributes without bespoke serialisation code.

## Usage

```go
import "ingester/logger"

executionContext := context.Background()
applicationLogger := logger.New("info")

logContext := logger.NewLogContext(logger.LogContextData{}).
    WithSessionID("request-123").
    WithTags("api").
    WithMetadata(map[string]string{"userId": "456"})

_, _ = logger.WithLogContext(executionContext, logContext, func(scopedContext context.Context) (struct{}, error) {
    applicationLogger.Info(scopedContext, "Processing request", "path", "/status")
    return struct{}{}, nil
})
```

## Log context

```go
logContext := logger.NewLogContext(logger.LogContextData{}).
    WithCategory("api").
    WithSessionID("request-123").
    WithTags("tag1", "tag2").
    WithMetadata(map[string]string{"key": "value"}).
    WithoutTags("old-tag").
    WithoutMetadata("old-key")
```

## Levels

```go
errorValue := errors.New("failure")

applicationLogger.Debug(executionContext, "Debug message")
applicationLogger.Info(executionContext, "Info message")
applicationLogger.Warn(executionContext, "Warning message")
applicationLogger.Error(executionContext, "Error message", "error", errorValue)
```

## Testing

```bash
go test ./logger -v
```
