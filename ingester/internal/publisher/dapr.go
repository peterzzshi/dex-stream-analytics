package publisher

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"time"

	"github.com/linkedin/goavro/v2"

	"ingester/internal/avro"
	"ingester/internal/config"
	"ingester/internal/events"
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

type HTTPDoer func(req *http.Request) (*http.Response, error)
type CodecMap map[events.EventType]*goavro.Codec
type TopicMapper func(eventType events.EventType) (string, error)
type URLBuilder func(topic string) string

func CreateCodecMap() (CodecMap, error) {
	codecs := make(CodecMap)

	for _, eventType := range events.AllEventTypes {
		codec, err := avro.NewCodec(eventType)
		if err != nil {
			return nil, err
		}
		codecs[eventType] = codec
	}

	return codecs, nil
}

func DefaultTopicMapper() TopicMapper {
	return func(eventType events.EventType) (string, error) {
		switch eventType {
		case events.EventTypeSwap:
			return config.GetTopicTradingEvents(), nil
		case events.EventTypeMint, events.EventTypeBurn, events.EventTypeTransfer:
			return config.GetTopicLiquidityEvents(), nil
		default:
			return "", fmt.Errorf("no mapping for event type: %s", eventType)
		}
	}
}

func DefaultURLBuilder() URLBuilder {
	return func(topic string) string {
		return fmt.Sprintf("http://%s:%s/v1.0/publish/%s/%s",
			config.GetDaprHost(),
			config.GetDaprHTTPPort(),
			config.GetPubSubName(),
			topic)
	}
}

func Publish(
	ctx context.Context,
	event events.Event,
	codecs CodecMap,
	topicMapper TopicMapper,
	urlBuilder URLBuilder,
	httpDoer HTTPDoer,
) {
	eventID := event.GetEventID()
	eventType := event.GetEventType()
	pair := event.GetPairAddress()

	cloudEventJSON, topic, err := createCloudEvent(event, codecs, topicMapper)
	if err != nil {
		logger.Error("Failed to prepare payload", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
		return
	}

	url := urlBuilder(topic)
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(cloudEventJSON))
	if err != nil {
		logger.Error("Failed to create HTTP request", "event_id", eventID, "event_type", eventType, "pair", pair, "url", url, "error", err)
		return
	}

	request.Header.Set("Content-Type", "application/cloudevents+json")
	request.Header.Set("partitionKey", event.GetPairAddress())

	response, err := httpDoer(request)
	if err != nil {
		logger.Warn("Publish failed (retryable)", "event_id", eventID, "event_type", eventType, "pair", pair, "error", err)
		return
	}
	defer response.Body.Close()

	if response.StatusCode >= 300 {
		logger.Warn("Publish failed (retryable)", "event_id", eventID, "event_type", eventType, "pair", pair, "status", response.StatusCode)
		return
	}

	logger.Info("Event published", "event_id", eventID, "event_type", eventType, "pair", pair)
}

func createCloudEvent(event events.Event, codecs CodecMap, topicMapper TopicMapper) ([]byte, string, error) {
	eventType := event.GetEventType()

	codec, ok := codecs[eventType]
	if !ok {
		return nil, "", fmt.Errorf("codec not found for event type: %s", eventType)
	}

	topic, err := topicMapper(eventType)
	if err != nil {
		return nil, "", err
	}

	avroPayload, err := codec.BinaryFromNative(nil, event.ToMap())
	if err != nil {
		return nil, "", fmt.Errorf("encode failed for %s: %w", eventType, err)
	}

	cloudEvent := map[string]interface{}{
		"specversion":     "1.0",
		"id":              event.GetEventID(),
		"source":          "ingester/uniswap-v2",
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
