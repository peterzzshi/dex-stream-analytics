package publisher

import (
	"context"
	"fmt"
	"net/http"
	"strings"

	"ingester/internal/config"
	"ingester/pkg/events"
	"ingester/pkg/logger"

	"github.com/linkedin/goavro/v2"
)

// Publisher pushes Avro-encoded swap events to Dapr pub/sub.
type Publisher struct {
	httpClient *http.Client
	codec      *goavro.Codec
	cfg        *config.Config
	log        *logger.Logger
}

func New(cfg *config.Config, log *logger.Logger, codec *goavro.Codec) *Publisher {
	return &Publisher{
		httpClient: &http.Client{},
		codec:      codec,
		cfg:        cfg,
		log:        log,
	}
}

func (p *Publisher) Publish(ctx context.Context, event events.SwapEvent) error {
	body, err := p.codec.BinaryFromNative(nil, map[string]interface{}{})
	if err != nil {
		return err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost,
		fmt.Sprintf("http://localhost:%s/v1.0/publish/%s/%s", p.cfg.DaprHTTPPort, p.cfg.PubSubName, p.cfg.TopicName),
		strings.NewReader(string(body)))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/avro-binary")

	resp, err := p.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		return fmt.Errorf("publish failed: %s", resp.Status)
	}
	return nil
}

func (p *Publisher) Close() {}
