package logger

import (
    "log"
    "os"
)

// Logger is a minimal leveled logger keeping side effects at the edge.
type Logger struct{ *log.Logger }

func New(level string) *Logger {
    return &Logger{Logger: log.New(os.Stdout, "", log.LstdFlags|log.Lmicroseconds)}
}

func (l *Logger) Info(msg string, kv ...interface{})  { l.print("INFO", msg, kv...) }
func (l *Logger) Error(msg string, kv ...interface{}) { l.print("ERROR", msg, kv...) }
func (l *Logger) Debug(msg string, kv ...interface{}) { l.print("DEBUG", msg, kv...) }
func (l *Logger) Fatal(msg string, kv ...interface{}) { l.print("FATAL", msg, kv...); os.Exit(1) }

func (l *Logger) print(level, msg string, kv ...interface{}) {
    l.Printf("[%s] %s %v", level, msg, kv)
}
