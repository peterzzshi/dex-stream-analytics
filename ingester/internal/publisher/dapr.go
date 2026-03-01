package publisher

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"

	"github.com/linkedin/goavro/v2"

	"ingester/internal/avro"
	"ingester/internal/config"
	ierrors "ingester/internal/errors"
	"ingester/internal/events"
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

type HTTPDoer func(req *http.Request) (*http.Response, error)
type CodecMap map[string]*goavro.Codec
type TopicMapper func(eventType events.EventType) (string, error)
type URLBuilder func(topic string) string

func CreateCodecMap() (CodecMap, error) {
	codecs := make(CodecMap)

	for _, eventType := range events.AllEventTypes {
		codec, err := avro.NewCodec(eventType)
		if err != nil {
			return nil, err
		}
		codecs[string(eventType)] = codec
	}

	return codecs, nil
}

func DefaultTopicMapper() TopicMapper {
	return func(eventType events.EventType) (string, error) {
		switch eventType {
		case events.EventTypeSwap:
			return config.GetTopicTradingEvents(), nil
		case events.EventTypeMint, events.EventTypeBurn:
			return config.GetTopicLiquidityEvents(), nil
		default:
			return "", ierrors.Config("topic", "no mapping for event type: "+string(eventType))
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
	codec, topic, err := prepare(event, codecs, topicMapper)
	if err != nil {
		logError(event, err)
		return
	}

	encodedBody, err := encode(codec, event)
	if err != nil {
		logError(event, err)
		return
	}

	url := urlBuilder(topic)
	err = publish(ctx, encodedBody, url, event, httpDoer)
	if err != nil {
		logError(event, err)
		return
	}

	logger.Info("Event published",
		"event_id", event.GetEventID(),
		"event_type", event.GetEventType(),
		"pair", event.GetPairAddress())
}

func logError(event events.Event, err error) {
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

func prepare(event events.Event, codecs CodecMap, topicMapper TopicMapper) (*goavro.Codec, string, error) {
	eventType := event.GetEventType()

	codec, ok := codecs[eventType]
	if !ok {
		return nil, "", ierrors.Config("codec", "not found for event type: "+eventType)
	}

	topic, err := topicMapper(events.EventType(eventType))
	if err != nil {
		return nil, "", err
	}

	return codec, topic, nil
}

func encode(codec *goavro.Codec, event events.Event) ([]byte, error) {
	encodedBody, err := codec.BinaryFromNative(nil, event.ToMap())
	if err != nil {
		return nil, ierrors.Data("encode", event.GetEventType(), err)
	}
	return encodedBody, nil
}

func publish(
	ctx context.Context,
	body []byte,
	url string,
	event events.Event,
	httpDoer HTTPDoer,
) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return ierrors.Config("http_request", fmt.Sprintf("failed to create request for URL %s: %v", url, err))
	}

	request.Header.Set("Content-Type", "application/avro-binary")
	request.Header.Set("partitionKey", event.GetPairAddress())

	response, err := httpDoer(request)
	if err != nil {
		return ierrors.Connection("dapr", err)
	}
	defer response.Body.Close()

	if response.StatusCode >= 300 {
		return ierrors.Publish(url, fmt.Errorf("HTTP %d: %s", response.StatusCode, response.Status))
	}

	return nil
}
