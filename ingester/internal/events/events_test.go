package events

import (
	"testing"
)

func TestEventTypes(t *testing.T) {
	// Test that event type constants have expected values
	tests := []struct {
		eventType EventType
		expected  string
	}{
		{EventTypeSwap, "Swap"},
		{EventTypeMint, "Mint"},
		{EventTypeBurn, "Burn"},
	}

	for _, tt := range tests {
		t.Run(string(tt.eventType), func(t *testing.T) {
			if string(tt.eventType) != tt.expected {
				t.Errorf("Expected %q, got %q", tt.expected, string(tt.eventType))
			}
		})
	}
}

func TestAllEventTypes(t *testing.T) {
	// Test that AllEventTypes contains all expected types
	if len(AllEventTypes) != 3 {
		t.Errorf("Expected 3 event types, got %d", len(AllEventTypes))
	}

	// Check each type is present
	expected := map[EventType]bool{
		EventTypeSwap: false,
		EventTypeMint: false,
		EventTypeBurn: false,
	}

	for _, et := range AllEventTypes {
		if _, exists := expected[et]; exists {
			expected[et] = true
		}
	}

	for et, found := range expected {
		if !found {
			t.Errorf("Event type %s not found in AllEventTypes", et)
		}
	}
}

func TestSwapEvent_GetMethods(t *testing.T) {
	event := SwapEvent{
		BaseEvent: BaseEvent{
			EventType:   "Swap",
			EventID:     "test-event-id",
			PairAddress: "0x1234",
		},
	}

	if event.GetEventType() != "Swap" {
		t.Errorf("Expected event type 'Swap', got %q", event.GetEventType())
	}

	if event.GetEventID() != "test-event-id" {
		t.Errorf("Expected event ID 'test-event-id', got %q", event.GetEventID())
	}

	if event.GetPairAddress() != "0x1234" {
		t.Errorf("Expected pair address '0x1234', got %q", event.GetPairAddress())
	}
}

func TestMintEvent_GetMethods(t *testing.T) {
	event := MintEvent{
		BaseEvent: BaseEvent{
			EventType:   "Mint",
			EventID:     "mint-id",
			PairAddress: "0x5678",
		},
	}

	if event.GetEventType() != "Mint" {
		t.Errorf("Expected event type 'Mint', got %q", event.GetEventType())
	}

	if event.GetEventID() != "mint-id" {
		t.Errorf("Expected event ID 'mint-id', got %q", event.GetEventID())
	}

	if event.GetPairAddress() != "0x5678" {
		t.Errorf("Expected pair address '0x5678', got %q", event.GetPairAddress())
	}
}

func TestBurnEvent_GetMethods(t *testing.T) {
	event := BurnEvent{
		BaseEvent: BaseEvent{
			EventType:   "Burn",
			EventID:     "burn-id",
			PairAddress: "0x9abc",
		},
	}

	if event.GetEventType() != "Burn" {
		t.Errorf("Expected event type 'Burn', got %q", event.GetEventType())
	}

	if event.GetEventID() != "burn-id" {
		t.Errorf("Expected event ID 'burn-id', got %q", event.GetEventID())
	}

	if event.GetPairAddress() != "0x9abc" {
		t.Errorf("Expected pair address '0x9abc', got %q", event.GetPairAddress())
	}
}

func TestSwapEvent_ToMap(t *testing.T) {
	volumeUSD := 123.45
	event := SwapEvent{
		BaseEvent: BaseEvent{
			EventID:     "event-1",
			BlockNumber: 100,
		},
		Sender:     "0xsender",
		Recipient:  "0xrecipient",
		Amount0In:  "1000",
		Amount1In:  "0",
		Amount0Out: "0",
		Amount1Out: "850",
		Price:      1.18,
		VolumeUSD:  &volumeUSD,
		GasUsed:    21000,
		GasPrice:   "50000000000",
	}

	m := event.ToMap()

	if m["eventId"] != "event-1" {
		t.Errorf("Expected eventId 'event-1', got %v", m["eventId"])
	}

	if m["sender"] != "0xsender" {
		t.Errorf("Expected sender '0xsender', got %v", m["sender"])
	}

	if m["price"] != 1.18 {
		t.Errorf("Expected price 1.18, got %v", m["price"])
	}

	if m["volumeUSD"] != 123.45 {
		t.Errorf("Expected volumeUSD 123.45, got %v", m["volumeUSD"])
	}
}

func TestSwapEvent_ToMap_NilVolumeUSD(t *testing.T) {
	event := SwapEvent{
		BaseEvent: BaseEvent{
			EventID: "event-2",
		},
		VolumeUSD: nil, // Not calculated
	}

	m := event.ToMap()

	if m["volumeUSD"] != nil {
		t.Errorf("Expected volumeUSD nil, got %v", m["volumeUSD"])
	}
}

func TestMintEvent_ToMap(t *testing.T) {
	event := MintEvent{
		BaseEvent: BaseEvent{
			EventID:     "mint-1",
			BlockNumber: 200,
		},
		Sender:  "0xminter",
		Amount0: "5000",
		Amount1: "4250",
	}

	m := event.ToMap()

	if m["eventId"] != "mint-1" {
		t.Errorf("Expected eventId 'mint-1', got %v", m["eventId"])
	}

	if m["sender"] != "0xminter" {
		t.Errorf("Expected sender '0xminter', got %v", m["sender"])
	}

	if m["amount0"] != "5000" {
		t.Errorf("Expected amount0 '5000', got %v", m["amount0"])
	}
}

func TestBurnEvent_ToMap(t *testing.T) {
	event := BurnEvent{
		BaseEvent: BaseEvent{
			EventID: "burn-1",
		},
		Sender:    "0xburner",
		Recipient: "0xreceiver",
		Amount0:   "2500",
		Amount1:   "2125",
	}

	m := event.ToMap()

	if m["eventId"] != "burn-1" {
		t.Errorf("Expected eventId 'burn-1', got %v", m["eventId"])
	}

	if m["recipient"] != "0xreceiver" {
		t.Errorf("Expected recipient '0xreceiver', got %v", m["recipient"])
	}
}

func TestBaseEvent_ToMap_WithOptionalFields(t *testing.T) {
	token0Symbol := "WMATIC"
	token1Symbol := "USDC"

	base := BaseEvent{
		EventID:      "base-1",
		Token0Symbol: &token0Symbol,
		Token1Symbol: &token1Symbol,
	}

	m := base.ToMap()

	if m["token0Symbol"] != "WMATIC" {
		t.Errorf("Expected token0Symbol 'WMATIC', got %v", m["token0Symbol"])
	}

	if m["token1Symbol"] != "USDC" {
		t.Errorf("Expected token1Symbol 'USDC', got %v", m["token1Symbol"])
	}
}

func TestBaseEvent_ToMap_WithoutOptionalFields(t *testing.T) {
	base := BaseEvent{
		EventID:      "base-2",
		Token0Symbol: nil,
		Token1Symbol: nil,
	}

	m := base.ToMap()

	if m["token0Symbol"] != nil {
		t.Errorf("Expected token0Symbol nil, got %v", m["token0Symbol"])
	}

	if m["token1Symbol"] != nil {
		t.Errorf("Expected token1Symbol nil, got %v", m["token1Symbol"])
	}
}
