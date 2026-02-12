package publisher

import (
	"context"
	"fmt"
	"net/http"
	"strings"

	"ingester/internal/avro"
	"ingester/internal/config"
	"ingester/logger"
	"ingester/pkg/events"

	"github.com/linkedin/goavro/v2"
)

// Publisher pushes Avro-encoded swap events to Dapr pub/sub.
type Publisher struct {
	httpClient        *http.Client
	codec             *goavro.Codec
	configuration     *config.Config
	applicationLogger *logger.Logger
}

func New(configuration *config.Config, applicationLogger *logger.Logger, codec *goavro.Codec) *Publisher {
	return &Publisher{
		httpClient:        &http.Client{},
		codec:             codec,
		configuration:     configuration,
		applicationLogger: applicationLogger,
	}
}

func (publisher *Publisher) Publish(executionContext context.Context, event events.SwapEvent) error {
	encodedBody, errorValue := avro.EncodeSwap(publisher.codec, event)
	if errorValue != nil {
		return errorValue
	}

	request, errorValue := http.NewRequestWithContext(executionContext, http.MethodPost,
		fmt.Sprintf("http://localhost:%s/v1.0/publish/%s/%s", publisher.configuration.DaprHTTPPort, publisher.configuration.PubSubName, publisher.configuration.TopicName),
		strings.NewReader(string(encodedBody)))
	if errorValue != nil {
		return errorValue
	}
	request.Header.Set("Content-Type", "application/avro-binary")

	response, errorValue := publisher.httpClient.Do(request)
	if errorValue != nil {
		return errorValue
	}
	defer response.Body.Close()
	if response.StatusCode >= 300 {
		return fmt.Errorf("publish failed: %s", response.Status)
	}
	return nil
}

func (publisher *Publisher) Close() {}
