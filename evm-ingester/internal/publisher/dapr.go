package publisher

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/linkedin/goavro/v2"
	"go.uber.org/zap"

	"evm-ingester/internal/avro"
	"evm-ingester/internal/config"
	"evm-ingester/internal/events"
)

type HTTPDoer func(req *http.Request) (*http.Response, error)
type CodecMap map[events.EventType]*goavro.Codec
type TopicMapper func(eventType events.EventType) (string, error)
type URLBuilder func(topic string) string

// Publisher carries the per-run publishing configuration, so Publish needs only the
// event and the chain it came from.
type Publisher struct {
	Codecs CodecMap
	Topics TopicMapper
	URL    URLBuilder
	Doer   HTTPDoer
	Source string // CloudEvent source, e.g. evm-ingester/uniswap-v2
	logger *zap.Logger
}

// New builds a Publisher from loaded config for one chain.
func New(cfg *config.Config, chain config.Chain, doer HTTPDoer, logger *zap.Logger) (*Publisher, error) {
	codecs, err := CreateCodecMap()
	if err != nil {
		return nil, err
	}
	return &Publisher{
		Codecs: codecs,
		Topics: buildTopicMapper(cfg),
		URL:    PublishURL(cfg),
		Doer:   doer,
		Source: SourceFor(chain),
		logger: logger,
	}, nil
}

func CreateCodecMap() (CodecMap, error) {
	codecs := make(CodecMap)
	for _, eventType := range events.AllEventTypes() {
		codec, err := avro.NewCodec(eventType)
		if err != nil {
			return nil, err
		}
		codecs[eventType] = codec
	}
	return codecs, nil
}

// buildTopicMapper routes an event type to its output topic, from config.
func buildTopicMapper(cfg *config.Config) TopicMapper {
	return func(eventType events.EventType) (string, error) {
		switch eventType {
		case events.EventTypeSwap:
			return cfg.Topics.Trading, nil
		case events.EventTypeMint, events.EventTypeBurn, events.EventTypeTransfer:
			return cfg.Topics.Liquidity, nil
		default:
			return "", fmt.Errorf("no mapping for event type: %s", eventType)
		}
	}
}

// SourceFor names the producer in the CloudEvent envelope. EVM-only ingester.
func SourceFor(chain config.Chain) string {
	return fmt.Sprintf("evm-ingester/%s", chain.VenueOr())
}

// PublishURL builds the Dapr publish endpoint, from config.
func PublishURL(cfg *config.Config) URLBuilder {
	return func(topic string) string {
		return fmt.Sprintf("http://%s:%d/v1.0/publish/%s/%s",
			cfg.Dapr.Host,
			cfg.Dapr.HTTPPort,
			cfg.Dapr.PubSubName,
			topic)
	}
}

// doPublish performs the HTTP POST to Dapr. Extracted for retry logic.
func (p *Publisher) doPublish(ctx context.Context, url string, cloudEventJSON []byte, event events.Event) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(cloudEventJSON))
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}

	request.Header.Set("Content-Type", "application/cloudevents+json")
	request.Header.Set("partitionKey", event.GetPairAddress())

	response, err := p.Doer(request)
	if err != nil {
		return fmt.Errorf("http call: %w", err)
	}
	defer response.Body.Close()

	if response.StatusCode >= 300 {
		return fmt.Errorf("http %d", response.StatusCode)
	}

	return nil
}

func (p *Publisher) Publish(ctx context.Context, event events.Event) {
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
		return
	}

	url := p.URL(topic)
	if err := p.doPublish(ctx, url, cloudEventJSON, event); err != nil {
		p.logger.Warn("Publish failed",
			zap.String("event_id", eventID),
			zap.String("event_type", string(eventType)),
			zap.String("pair", pair),
			zap.Error(err),
		)
		return
	}

	p.logger.Info("Event published",
		zap.String("event_id", eventID),
		zap.String("event_type", string(eventType)),
		zap.String("pair", pair),
	)
}

// createCloudEvent is a pure function that encodes an event into CloudEvents format.
// It takes explicit dependencies rather than accessing Publisher state.
func createCloudEvent(event events.Event, codecs CodecMap, topics TopicMapper, source string) ([]byte, string, error) {
	eventType := event.GetEventType()

	codec, ok := codecs[eventType]
	if !ok {
		return nil, "", fmt.Errorf("codec not found for event type: %s", eventType)
	}

	topic, err := topics(eventType)
	if err != nil {
		return nil, "", err
	}

	avroPayload, err := codec.BinaryFromNative(nil, event.ToMap())
	if err != nil {
		return nil, "", fmt.Errorf("encode failed for %s: %w", eventType, err)
	}

	cloudEvent := map[string]any{
		"specversion":     "1.0",
		"id":              event.GetEventID(),
		"source":          source,
		"type":            eventType.CloudEventType(),
		"datacontenttype": "application/avro-binary",
		"subject":         event.GetPairAddress(),
		"time":            time.Unix(event.GetEventTimestamp(), 0).UTC().Format(time.RFC3339),
		"data_base64":     base64.StdEncoding.EncodeToString(avroPayload),
	}

	cloudEventJSON, err := json.Marshal(cloudEvent)
	if err != nil {
		return nil, "", fmt.Errorf("failed to serialize CloudEvent: %w", err)
	}

	return cloudEventJSON, topic, nil
}
