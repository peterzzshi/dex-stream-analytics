package errors

import "fmt"

type ConfigError struct {
	Message string
	Cause   error
}

func (e *ConfigError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("config: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("config: %s", e.Message)
}

func (e *ConfigError) Unwrap() error {
	return e.Cause
}

type ConnectionError struct {
	Message string
	Cause   error
}

func (e *ConnectionError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("connection: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("connection: %s", e.Message)
}

func (e *ConnectionError) Unwrap() error {
	return e.Cause
}

type DataError struct {
	Message string
	Cause   error
}

func (e *DataError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("data: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("data: %s", e.Message)
}

func (e *DataError) Unwrap() error {
	return e.Cause
}

type PublishError struct {
	Message string
	Cause   error
}

func (e *PublishError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("publish: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("publish: %s", e.Message)
}

func (e *PublishError) Unwrap() error {
	return e.Cause
}
