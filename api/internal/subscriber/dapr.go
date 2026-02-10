package subscriber

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	"api/internal/storage"
)

// Subscriber consumes analytics events from Dapr and updates storage.
type Subscriber struct {
	store  storage.Writer
	client *http.Client
}

func New(store storage.Writer) *Subscriber {
	return &Subscriber{store: store, client: &http.Client{}}
}

func (s *Subscriber) Start(ctx context.Context) error {
	// Minimal stub: real implementation would bind HTTP handler; keep side effects minimal
	<-ctx.Done()
	return ctx.Err()
}

func (s *Subscriber) Handle(event []byte) error {
	var payload struct {
		Pair   string  `json:"pairAddress"`
		TWAP   float64 `json:"twap"`
		Volume float64 `json:"volumeUSD"`
	}
	if err := json.Unmarshal(event, &payload); err != nil {
		return fmt.Errorf("decode: %w", err)
	}
	s.store.Upsert(payload.Pair, payload.TWAP, payload.Volume)
	return nil
}

func (s *Subscriber) Close() {}
