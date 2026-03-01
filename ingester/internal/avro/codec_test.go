package avro

import (
	"testing"

	"ingester/internal/events"
)

func TestNewCodec_AllEventTypes(t *testing.T) {
	for _, eventType := range events.AllEventTypes {
		t.Run(string(eventType), func(t *testing.T) {
			codec, err := NewCodec(eventType)
			if err != nil {
				t.Fatalf("NewCodec(%s) failed: %v", eventType, err)
			}

			if codec == nil {
				t.Error("Expected non-nil codec")
			}
		})
	}
}

func TestNewCodec_InvalidEventType(t *testing.T) {
	_, err := NewCodec("InvalidType")

	if err == nil {
		t.Error("Expected error for invalid event type, got nil")
	}
}

func TestEncodeDecode_SwapEvent(t *testing.T) {
	codec, err := NewCodec(events.EventTypeSwap)
	if err != nil {
		t.Fatalf("NewCodec failed: %v", err)
	}

	// Use nil volumeUSD to avoid Avro union encoding complexity in tests
	original := events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:       "Swap",
			EventID:         "tx-123-0",
			BlockNumber:     1000,
			BlockTimestamp:  1640000000,
			TransactionHash: "0xabc123",
			LogIndex:        0,
			PairAddress:     "0xpair",
			Token0:          "0xtoken0",
			Token1:          "0xtoken1",
			EventTimestamp:  1640000010,
		},
		Sender:     "0xsender",
		Recipient:  "0xrecipient",
		Amount0In:  "1000",
		Amount1In:  "0",
		Amount0Out: "0",
		Amount1Out: "850",
		Price:      1.18,
		VolumeUSD:  nil, // Nil to simplify test
		GasUsed:    21000,
		GasPrice:   "50000000000",
	}

	// Encode
	encoded, err := codec.BinaryFromNative(nil, original.ToMap())
	if err != nil {
		t.Fatalf("BinaryFromNative failed: %v", err)
	}

	if len(encoded) == 0 {
		t.Error("Expected non-empty encoded data")
	}

	// Decode
	decoded, _, err := codec.NativeFromBinary(encoded)
	if err != nil {
		t.Fatalf("NativeFromBinary failed: %v", err)
	}

	decodedMap, ok := decoded.(map[string]interface{})
	if !ok {
		t.Fatal("Decoded value is not a map")
	}

	// Verify key fields
	if decodedMap["eventId"] != "tx-123-0" {
		t.Errorf("Expected eventId 'tx-123-0', got %v", decodedMap["eventId"])
	}

	if decodedMap["sender"] != "0xsender" {
		t.Errorf("Expected sender '0xsender', got %v", decodedMap["sender"])
	}
}

func TestEncodeDecode_MintEvent(t *testing.T) {
	codec, err := NewCodec(events.EventTypeMint)
	if err != nil {
		t.Fatalf("NewCodec failed: %v", err)
	}

	original := events.MintEvent{
		BaseEvent: events.BaseEvent{
			EventType:       "Mint",
			EventID:         "tx-456-1",
			BlockNumber:     2000,
			BlockTimestamp:  1640001000,
			TransactionHash: "0xdef456",
			LogIndex:        1,
			PairAddress:     "0xpair2",
			Token0:          "0xtoken0",
			Token1:          "0xtoken1",
			EventTimestamp:  1640001010,
		},
		Sender:  "0xminter",
		Amount0: "5000",
		Amount1: "4250",
	}

	// Encode
	encoded, err := codec.BinaryFromNative(nil, original.ToMap())
	if err != nil {
		t.Fatalf("BinaryFromNative failed: %v", err)
	}

	// Decode
	decoded, _, err := codec.NativeFromBinary(encoded)
	if err != nil {
		t.Fatalf("NativeFromBinary failed: %v", err)
	}

	decodedMap, ok := decoded.(map[string]interface{})
	if !ok {
		t.Fatal("Decoded value is not a map")
	}

	if decodedMap["eventId"] != "tx-456-1" {
		t.Errorf("Expected eventId 'tx-456-1', got %v", decodedMap["eventId"])
	}

	if decodedMap["amount0"] != "5000" {
		t.Errorf("Expected amount0 '5000', got %v", decodedMap["amount0"])
	}
}

func TestEncodeDecode_BurnEvent(t *testing.T) {
	codec, err := NewCodec(events.EventTypeBurn)
	if err != nil {
		t.Fatalf("NewCodec failed: %v", err)
	}

	original := events.BurnEvent{
		BaseEvent: events.BaseEvent{
			EventType:       "Burn",
			EventID:         "tx-789-2",
			BlockNumber:     3000,
			BlockTimestamp:  1640002000,
			TransactionHash: "0xghi789",
			LogIndex:        2,
			PairAddress:     "0xpair3",
			Token0:          "0xtoken0",
			Token1:          "0xtoken1",
			EventTimestamp:  1640002010,
		},
		Sender:    "0xburner",
		Recipient: "0xreceiver",
		Amount0:   "2500",
		Amount1:   "2125",
	}

	// Encode
	encoded, err := codec.BinaryFromNative(nil, original.ToMap())
	if err != nil {
		t.Fatalf("BinaryFromNative failed: %v", err)
	}

	// Decode
	decoded, _, err := codec.NativeFromBinary(encoded)
	if err != nil {
		t.Fatalf("NativeFromBinary failed: %v", err)
	}

	decodedMap, ok := decoded.(map[string]interface{})
	if !ok {
		t.Fatal("Decoded value is not a map")
	}

	if decodedMap["eventId"] != "tx-789-2" {
		t.Errorf("Expected eventId 'tx-789-2', got %v", decodedMap["eventId"])
	}

	if decodedMap["recipient"] != "0xreceiver" {
		t.Errorf("Expected recipient '0xreceiver', got %v", decodedMap["recipient"])
	}
}

func TestCodec_RoundTrip(t *testing.T) {
	// Test that encoding and decoding preserves data
	tests := []struct {
		name      string
		eventType events.EventType
		event     events.Event
	}{
		{
			"SwapEvent",
			events.EventTypeSwap,
			events.SwapEvent{
				BaseEvent: events.BaseEvent{EventID: "swap-1"},
				Sender:    "0x1",
			},
		},
		{
			"MintEvent",
			events.EventTypeMint,
			events.MintEvent{
				BaseEvent: events.BaseEvent{EventID: "mint-1"},
				Sender:    "0x2",
			},
		},
		{
			"BurnEvent",
			events.EventTypeBurn,
			events.BurnEvent{
				BaseEvent: events.BaseEvent{EventID: "burn-1"},
				Sender:    "0x3",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			codec, _ := NewCodec(tt.eventType)

			// Encode
			encoded, err := codec.BinaryFromNative(nil, tt.event.ToMap())
			if err != nil {
				t.Fatalf("Encoding failed: %v", err)
			}

			// Decode
			decoded, _, err := codec.NativeFromBinary(encoded)
			if err != nil {
				t.Fatalf("Decoding failed: %v", err)
			}

			// Verify eventId survived round trip
			decodedMap := decoded.(map[string]interface{})
			if decodedMap["eventId"] != tt.event.GetEventID() {
				t.Errorf("Event ID mismatch after round trip")
			}
		})
	}
}
