package dlq

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"go.uber.org/zap"

	"evm-ingester/internal/events"
)

const (
	defaultDLQDir      = "./dlq"
	defaultFlushSize   = 100
	defaultFlushPeriod = 5 * time.Second
)

// FileSink writes DLQ events to disk as newline-delimited JSON.
// Events are batched and flushed periodically or when batch size is reached.
type FileSink struct {
	dir         string
	file        *os.File
	buffer      []events.Event
	flushSize   int
	flushPeriod time.Duration
	logger      *zap.Logger
	mu          sync.Mutex
	ctx         context.Context
	cancel      context.CancelFunc
	wg          sync.WaitGroup
}

// NewFileSink creates a DLQ file sink that writes to the specified directory.
// Events are written to a timestamped file in newline-delimited JSON format.
func NewFileSink(dir string, logger *zap.Logger) (*FileSink, error) {
	if dir == "" {
		dir = defaultDLQDir
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("create DLQ directory: %w", err)
	}

	filename := fmt.Sprintf("dlq_%s.jsonl", time.Now().Format("20060102_150405"))
	filepath := filepath.Join(dir, filename)

	file, err := os.OpenFile(filepath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return nil, fmt.Errorf("open DLQ file: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	sink := &FileSink{
		dir:         dir,
		file:        file,
		buffer:      make([]events.Event, 0, defaultFlushSize),
		flushSize:   defaultFlushSize,
		flushPeriod: defaultFlushPeriod,
		logger:      logger,
		ctx:         ctx,
		cancel:      cancel,
	}

	// Start background flusher
	sink.wg.Add(1)
	go sink.periodicFlush()

	return sink, nil
}

// Write adds an event to the buffer. Returns immediately; actual disk write is async.
func (s *FileSink) Write(event events.Event) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.buffer = append(s.buffer, event)

	if len(s.buffer) >= s.flushSize {
		return s.flushLocked()
	}

	return nil
}

// Consume drains events from a channel and writes them to disk.
// Blocks until the channel is closed or context is cancelled.
func (s *FileSink) Consume(ctx context.Context, ch <-chan events.Event) {
	for {
		select {
		case <-ctx.Done():
			return
		case event, ok := <-ch:
			if !ok {
				return
			}
			if err := s.Write(event); err != nil {
				s.logger.Error("Failed to write DLQ event",
					zap.String("event_id", event.GetEventID()),
					zap.Error(err),
				)
			}
		}
	}
}

// Flush writes all buffered events to disk.
func (s *FileSink) Flush() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.flushLocked()
}

func (s *FileSink) flushLocked() error {
	if len(s.buffer) == 0 {
		return nil
	}

	for _, event := range s.buffer {
		data, err := json.Marshal(event.ToMap())
		if err != nil {
			s.logger.Error("Failed to serialize DLQ event",
				zap.String("event_id", event.GetEventID()),
				zap.Error(err),
			)
			continue
		}

		if _, err := s.file.Write(append(data, '\n')); err != nil {
			return fmt.Errorf("write DLQ event: %w", err)
		}
	}

	if err := s.file.Sync(); err != nil {
		return fmt.Errorf("sync DLQ file: %w", err)
	}

	s.logger.Info("Flushed DLQ events to disk",
		zap.Int("count", len(s.buffer)),
		zap.String("file", s.file.Name()),
	)

	s.buffer = s.buffer[:0]
	return nil
}

func (s *FileSink) periodicFlush() {
	defer s.wg.Done()

	ticker := time.NewTicker(s.flushPeriod)
	defer ticker.Stop()

	for {
		select {
		case <-s.ctx.Done():
			return
		case <-ticker.C:
			if err := s.Flush(); err != nil {
				s.logger.Error("Periodic DLQ flush failed", zap.Error(err))
			}
		}
	}
}

// Close flushes remaining events and closes the file.
func (s *FileSink) Close() error {
	s.cancel()
	s.wg.Wait()

	if err := s.Flush(); err != nil {
		return err
	}

	return s.file.Close()
}
