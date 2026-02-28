package errors

import "fmt"

// ConfigError represents startup/configuration errors (fatal - app cannot start)
type ConfigError struct {
	Field   string
	Message string
}

func (e *ConfigError) Error() string {
	return fmt.Sprintf("config: %s: %s", e.Field, e.Message)
}

// ConnectionError represents network/RPC/service connectivity errors (transient - retry)
type ConnectionError struct {
	Service string
	Cause   error
}

func (e *ConnectionError) Error() string {
	return fmt.Sprintf("connection: %s: %v", e.Service, e.Cause)
}

func (e *ConnectionError) Unwrap() error {
	return e.Cause
}

// DataError represents data processing errors (skippable - log and continue)
type DataError struct {
	Operation string
	Context   string
	Cause     error
}

func (e *DataError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("data: %s [%s]: %v", e.Operation, e.Context, e.Cause)
	}
	return fmt.Sprintf("data: %s [%s]", e.Operation, e.Context)
}

func (e *DataError) Unwrap() error {
	return e.Cause
}

// PublishError represents Kafka publishing errors (transient - retry)
type PublishError struct {
	Topic string
	Cause error
}

func (e *PublishError) Error() string {
	return fmt.Sprintf("publish: topic=%s: %v", e.Topic, e.Cause)
}

func (e *PublishError) Unwrap() error {
	return e.Cause
}

// Constructors

func Config(field, message string) *ConfigError {
	return &ConfigError{Field: field, Message: message}
}

func Connection(service string, cause error) *ConnectionError {
	return &ConnectionError{Service: service, Cause: cause}
}

func Data(operation, context string, cause error) *DataError {
	return &DataError{Operation: operation, Context: context, Cause: cause}
}

func Publish(topic string, cause error) *PublishError {
	return &PublishError{Topic: topic, Cause: cause}
}
