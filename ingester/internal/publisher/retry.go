package publisher

import (
	"context"
	"fmt"
	"time"

	"go.uber.org/zap"

	"ingester/internal/events"
)

const (
	maxRetries        = 3
	initialRetryDelay = 100 * time.Millisecond
	maxRetryDelay     = 2 * time.Second
)

// PublishWithRetry attempts to publish with exponential backoff, returning error
// if exhausted (route to DLQ).
func (p *Publisher) PublishWithRetry(ctx context.Context, event events.Event) error {
	eventID := event.GetEventID()
	eventType := event.GetEventType()
	pair := event.GetPairAddress()

	cloudEventJSON, topic, err := createCloudEvent(event, p.Codecs, p.Topics, p.Source)
	if err != nil {
		p.logger.Error("Failed to prepare payload",
			zap.String("event_id", eventID),
			zap.String("event_type", string(eventType)),
			zap.String("pair", pair),
			zap.Error(err),
		)
		return err
	}

	url := p.URL(topic)
	delay := initialRetryDelay

	for attempt := 0; attempt <= maxRetries; attempt++ {
		if attempt > 0 {
			select {
			case <-time.After(delay):
				delay = minDuration(delay*2, maxRetryDelay)
			case <-ctx.Done():
				return ctx.Err()
			}
		}

		err := p.doPublish(ctx, url, cloudEventJSON, event)
		if err == nil {
			p.logger.Info("Event published",
				zap.String("event_id", eventID),
				zap.String("event_type", string(eventType)),
				zap.String("pair", pair),
				zap.Int("attempt", attempt+1),
			)
			return nil
		}

		if attempt < maxRetries {
			p.logger.Warn("Publish failed, retrying",
				zap.String("event_id", eventID),
				zap.String("event_type", string(eventType)),
				zap.String("pair", pair),
				zap.Int("attempt", attempt+1),
				zap.Duration("delay", delay),
				zap.Error(err),
			)
		}
	}

	p.logger.Error("Publish exhausted retries, routing to DLQ",
		zap.String("event_id", eventID),
		zap.String("event_type", string(eventType)),
		zap.String("pair", pair),
		zap.Int("attempts", maxRetries+1),
	)

	return fmt.Errorf("exhausted %d retries", maxRetries+1)
}

func minDuration(a, b time.Duration) time.Duration {
	if a < b {
		return a
	}
	return b
}
