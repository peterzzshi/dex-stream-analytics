package publisher

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"testing"

	"ingester/internal/events"
)

func mockTopicMapper(eventType events.EventType) (string, error) {
	switch eventType {
	case events.EventTypeSwap:
		return "dex-trading-events", nil
	case events.EventTypeMint, events.EventTypeBurn, events.EventTypeTransfer:
		return "dex-liquidity-events", nil
	default:
		return "", nil
	}
}

func mockURLBuilder(topic string) string {
	return "http://localhost:3500/v1.0/publish/kafka-pubsub/" + topic
}

func mockHTTPError(statusCode int) HTTPDoer {
	return func(req *http.Request) (*http.Response, error) {
		return &http.Response{
			StatusCode: statusCode,
			Body:       io.NopCloser(strings.NewReader("Error")),
		}, nil
	}
}

func mockHTTPNetworkError() HTTPDoer {
	return func(req *http.Request) (*http.Response, error) {
		return nil, http.ErrServerClosed
	}
}

func TestCreateCodecMap(t *testing.T) {
	codecs, err := CreateCodecMap()

	if err != nil {
		t.Fatalf("CreateCodecMap() failed: %v", err)
	}

	if len(codecs) != 4 {
		t.Errorf("Expected 4 codecs, got %d", len(codecs))
	}

	// Verify all event types have codecs
	for _, eventType := range events.AllEventTypes() {
		if _, ok := codecs[eventType]; !ok {
			t.Errorf("Missing codec for event type: %s", eventType)
		}
	}
}

func TestPublish_SwapEvent(t *testing.T) {
	codecs, _ := CreateCodecMap()

	// Track requests with closure
	var capturedReq *http.Request
	var capturedBody []byte
	mockDoer := func(req *http.Request) (*http.Response, error) {
		capturedReq = req
		capturedBody, _ = io.ReadAll(req.Body)
		return &http.Response{
			StatusCode: 200,
			Body:       io.NopCloser(strings.NewReader("")),
		}, nil
	}

	event := events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:       events.EventTypeSwap,
			EventID:         "tx-123-0",
			PairAddress:     "0xpair",
			BlockNumber:     1000,
			BlockTimestamp:  1640000000,
			TransactionHash: "0xabc123",
			EventTimestamp:  1640000010,
		},
		Sender:     "0xsender",
		Recipient:  "0xrecipient",
		Amount0In:  "1000",
		Amount1In:  "0",
		Amount0Out: "0",
		Amount1Out: "850",
		Price:      1.18,
		VolumeUSD:  nil,
		GasUsed:    21000,
		GasPrice:   "50000000000",
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)

	if capturedReq == nil {
		t.Fatal("Expected HTTP request to be made")
	}

	// Verify HTTP method
	if capturedReq.Method != http.MethodPost {
		t.Errorf("Expected POST, got %s", capturedReq.Method)
	}

	// Verify content type is CloudEvents JSON
	if capturedReq.Header.Get("Content-Type") != "application/cloudevents+json" {
		t.Errorf("Expected application/cloudevents+json content type, got %s", capturedReq.Header.Get("Content-Type"))
	}

	// Verify partition key
	if capturedReq.Header.Get("partitionKey") != "0xpair" {
		t.Errorf("Expected partitionKey 0xpair, got %s", capturedReq.Header.Get("partitionKey"))
	}

	// Verify CloudEvents structure
	var cloudEvent map[string]interface{}
	if err := json.Unmarshal(capturedBody, &cloudEvent); err != nil {
		t.Fatalf("Failed to parse CloudEvents JSON: %v", err)
	}

	// Verify CloudEvents required fields
	if cloudEvent["specversion"] != "1.0" {
		t.Errorf("Expected specversion 1.0, got %v", cloudEvent["specversion"])
	}
	if cloudEvent["type"] != "com.dex.events.swap" {
		t.Errorf("Expected type com.dex.events.swap, got %v", cloudEvent["type"])
	}
	if cloudEvent["source"] != "ingester/uniswap-v2" {
		t.Errorf("Expected source ingester/uniswap-v2, got %v", cloudEvent["source"])
	}
	if cloudEvent["id"] != "tx-123-0" {
		t.Errorf("Expected id tx-123-0, got %v", cloudEvent["id"])
	}
	if cloudEvent["datacontenttype"] != "application/avro-binary" {
		t.Errorf("Expected datacontenttype application/avro-binary, got %v", cloudEvent["datacontenttype"])
	}

	// Verify data_base64 field exists
	if _, ok := cloudEvent["data_base64"]; !ok {
		t.Error("Expected data_base64 field in CloudEvents")
	}
}

func TestPublish_MintEvent(t *testing.T) {
	codecs, _ := CreateCodecMap()
	requestMade := false
	mockDoer := func(req *http.Request) (*http.Response, error) {
		requestMade = true
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader(""))}, nil
	}

	event := events.MintEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeMint,
			EventID:     "tx-456-1",
			PairAddress: "0xpair2",
		},
		Sender:  "0xminter",
		Amount0: "5000",
		Amount1: "4250",
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)

	if !requestMade {
		t.Error("Expected HTTP request to be made")
	}
}

func TestPublish_BurnEvent(t *testing.T) {
	codecs, _ := CreateCodecMap()
	requestMade := false
	mockDoer := func(req *http.Request) (*http.Response, error) {
		requestMade = true
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader(""))}, nil
	}

	event := events.BurnEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeBurn,
			EventID:     "tx-789-2",
			PairAddress: "0xpair3",
		},
		Sender:    "0xburner",
		Recipient: "0xreceiver",
		Amount0:   "2500",
		Amount1:   "2125",
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)

	if !requestMade {
		t.Error("Expected HTTP request to be made")
	}
}

func TestPublish_TransferEvent(t *testing.T) {
	codecs, _ := CreateCodecMap()
	requestMade := false
	mockDoer := func(req *http.Request) (*http.Response, error) {
		requestMade = true
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader(""))}, nil
	}

	event := events.TransferEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeTransfer,
			EventID:     "tx-999-3",
			PairAddress: "0xpair3",
		},
		From:  "0xfrom",
		To:    "0xto",
		Value: "777",
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)

	if !requestMade {
		t.Error("Expected HTTP request to be made")
	}
}

func TestPublish_HTTPError(t *testing.T) {
	codecs, _ := CreateCodecMap()
	mockDoer := mockHTTPError(500)

	event := events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeSwap,
			EventID:     "test",
			PairAddress: "0xpair",
		},
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)
}

func TestPublish_NetworkError(t *testing.T) {
	codecs, _ := CreateCodecMap()
	mockDoer := mockHTTPNetworkError()

	event := events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeSwap,
			EventID:     "test",
			PairAddress: "0xpair",
		},
	}

	Publish(context.Background(), event, codecs, mockTopicMapper, mockURLBuilder, mockDoer)
}

func TestCreateCloudEvent(t *testing.T) {
	codecs, _ := CreateCodecMap()

	tests := []struct {
		name          string
		event         events.Event
		expectedTopic string
		expectError   bool
	}{
		{
			name: "Swap event",
			event: events.SwapEvent{
				BaseEvent: events.BaseEvent{EventType: events.EventTypeSwap},
			},
			expectedTopic: "dex-trading-events",
			expectError:   false,
		},
		{
			name: "Mint event",
			event: events.MintEvent{
				BaseEvent: events.BaseEvent{EventType: events.EventTypeMint},
			},
			expectedTopic: "dex-liquidity-events",
			expectError:   false,
		},
		{
			name: "Burn event",
			event: events.BurnEvent{
				BaseEvent: events.BaseEvent{EventType: events.EventTypeBurn},
			},
			expectedTopic: "dex-liquidity-events",
			expectError:   false,
		},
		{
			name: "Transfer event",
			event: events.TransferEvent{
				BaseEvent: events.BaseEvent{EventType: events.EventTypeTransfer},
			},
			expectedTopic: "dex-liquidity-events",
			expectError:   false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			payload, topic, err := createCloudEvent(tt.event, codecs, mockTopicMapper)

			if tt.expectError && err == nil {
				t.Error("Expected error but got nil")
			}
			if !tt.expectError && err != nil {
				t.Errorf("Unexpected error: %v", err)
			}
			if !tt.expectError {
				if payload == nil {
					t.Error("Expected non-nil payload")
				}
				if topic != tt.expectedTopic {
					t.Errorf("Expected topic %s, got %s", tt.expectedTopic, topic)
				}
			}
		})
	}
}

func TestEncode(t *testing.T) {
	codecs, _ := CreateCodecMap()

	event := events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeSwap,
			EventID:     "test-id",
			PairAddress: "0xpair",
		},
		Sender: "0xsender",
	}

	payload, _, err := createCloudEvent(event, codecs, mockTopicMapper)

	if err != nil {
		t.Fatalf("createCloudEvent() failed: %v", err)
	}

	if len(payload) == 0 {
		t.Error("Expected non-empty encoded data")
	}
}

func TestURLBuilder(t *testing.T) {
	url := mockURLBuilder("test-topic")

	if !strings.Contains(url, "test-topic") {
		t.Errorf("URL should contain topic: %s", url)
	}

	if !strings.Contains(url, "/v1.0/publish/") {
		t.Errorf("URL should contain Dapr publish path: %s", url)
	}

	expected := "http://localhost:3500/v1.0/publish/kafka-pubsub/test-topic"
	if url != expected {
		t.Errorf("Expected URL %s, got %s", expected, url)
	}
}
