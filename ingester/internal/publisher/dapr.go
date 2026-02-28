package publisher

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"time"

	"ingester/internal/avro"
	"ingester/internal/config"
	ierrors "ingester/internal/errors"
	"ingester/internal/events"

	"github.com/linkedin/goavro/v2"
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

type Publisher struct {
	httpClient *http.Client
	codecs     map[string]*goavro.Codec
}

func New() (*Publisher, error) {
	codecs := make(map[string]*goavro.Codec)

	for _, eventType := range events.AllEventTypes {
		codec, err := avro.NewCodec(eventType)
		if err != nil {
			return nil, err
		}
		codecs[string(eventType)] = codec
	}

	return &Publisher{
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
		codecs: codecs,
	}, nil
}

func (p *Publisher) Publish(ctx context.Context, event events.Event) {
	codec, topic, err := p.prepare(event)
	if err != nil {
		p.logError(event, err)
		return
	}

	encodedBody, err := p.encode(codec, event)
	if err != nil {
		p.logError(event, err)
		return
	}

	err = p.publish(ctx, encodedBody, topic, event)
	if err != nil {
		p.logError(event, err)
		return
	}

	logger.Info("Event published",
		"event_id", event.GetEventID(),
		"event_type", event.GetEventType(),
		"pair", event.GetPairAddress())
}

func (p *Publisher) logError(event events.Event, err error) {
	eventID := event.GetEventID()
	eventType := event.GetEventType()
	pair := event.GetPairAddress()

	var (
		connErr    *ierrors.ConnectionError
		publishErr *ierrors.PublishError
		configErr  *ierrors.ConfigError
		dataErr    *ierrors.DataError
	)

	switch {
	case errors.As(err, &connErr) || errors.As(err, &publishErr):
		logger.Warn("Publish will retry", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
	case errors.As(err, &configErr):
		logger.Error("Fatal configuration error", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
	case errors.As(err, &dataErr):
		logger.Warn("Skipping bad data", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
	default:
		logger.Error("Publish failed", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
	}
}

func (p *Publisher) prepare(event events.Event) (*goavro.Codec, string, error) {
	eventType := event.GetEventType()

	codec, ok := p.codecs[eventType]
	if !ok {
		return nil, "", ierrors.Config("codec", "not found for event type: "+eventType)
	}

	var topic string
	switch events.EventType(eventType) {
	case events.EventTypeSwap:
		topic = config.GetTopicTradingEvents()
	case events.EventTypeMint, events.EventTypeBurn:
		topic = config.GetTopicLiquidityEvents()
	default:
		return nil, "", ierrors.Config("topic", "no mapping for event type: "+eventType)
	}

	return codec, topic, nil
}

func (p *Publisher) encode(codec *goavro.Codec, event events.Event) ([]byte, error) {
	encodedBody, err := codec.BinaryFromNative(nil, event.ToMap())
	if err != nil {
		return nil, ierrors.Data("encode", event.GetEventType(), err)
	}
	return encodedBody, nil
}

func (p *Publisher) publish(ctx context.Context, body []byte, topic string, event events.Event) error {
	url := p.buildURL(topic)

	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		panic(fmt.Sprintf("BUG: http.NewRequestWithContext with url=%s: %v", url, err))
	}

	request.Header.Set("Content-Type", "application/avro-binary")
	request.Header.Set("partitionKey", event.GetPairAddress())

	response, err := p.httpClient.Do(request)
	if err != nil {
		return ierrors.Connection("dapr", err)
	}
	defer response.Body.Close()

	if response.StatusCode >= 300 {
		return ierrors.Publish(topic, fmt.Errorf("HTTP %d: %s", response.StatusCode, response.Status))
	}

	return nil
}

func (p *Publisher) buildURL(topic string) string {
	return fmt.Sprintf("http://%s:%s/v1.0/publish/%s/%s",
		config.GetDaprHost(),
		config.GetDaprHTTPPort(),
		config.GetPubSubName(),
		topic)
}

func (p *Publisher) Close() {}
