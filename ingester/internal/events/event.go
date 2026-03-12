package events

import "strings"

// EventType represents the type of DEX event
type EventType string

const (
	EventTypeSwap EventType = "Swap"
	EventTypeMint EventType = "Mint"
	EventTypeBurn EventType = "Burn"
)

// CloudEventType returns the CloudEvents type string for this event type
func (et EventType) CloudEventType() string {
	return "com.dex.events." + strings.ToLower(string(et))
}

// AllEventTypes contains all valid event types
var AllEventTypes = []EventType{
	EventTypeSwap,
	EventTypeMint,
	EventTypeBurn,
}

// Event is the interface that all DEX events must implement
type Event interface {
	GetEventType() EventType
	GetEventID() string
	GetPairAddress() string
	GetEventTimestamp() int64
	ToMap() map[string]interface{}
}

// BaseEvent contains fields common to all DEX events
type BaseEvent struct {
	EventType       EventType `json:"eventType"`
	EventID         string    `json:"eventId"`
	BlockNumber     int64     `json:"blockNumber"`
	BlockTimestamp  int64     `json:"blockTimestamp"`
	TransactionHash string    `json:"transactionHash"`
	LogIndex        int32     `json:"logIndex"`
	PairAddress     string    `json:"pairAddress"`
	Token0          string    `json:"token0"`
	Token1          string    `json:"token1"`
	Token0Symbol    *string   `json:"token0Symbol,omitempty"`
	Token1Symbol    *string   `json:"token1Symbol,omitempty"`
	EventTimestamp  int64     `json:"eventTimestamp"`
}

func (base BaseEvent) GetEventType() EventType {
	return base.EventType
}

func (base BaseEvent) GetEventID() string {
	return base.EventID
}

func (base BaseEvent) GetPairAddress() string {
	return base.PairAddress
}

func (base BaseEvent) GetEventTimestamp() int64 {
	return base.EventTimestamp
}

// ToMap converts BaseEvent fields to a map for Avro serialization
func (base BaseEvent) ToMap() map[string]interface{} {
	return map[string]interface{}{
		"eventId":         base.EventID,
		"blockNumber":     base.BlockNumber,
		"blockTimestamp":  base.BlockTimestamp,
		"transactionHash": base.TransactionHash,
		"logIndex":        base.LogIndex,
		"pairAddress":     base.PairAddress,
		"token0":          base.Token0,
		"token1":          base.Token1,
		"token0Symbol":    toNullable(base.Token0Symbol),
		"token1Symbol":    toNullable(base.Token1Symbol),
		"eventTimestamp":  base.EventTimestamp,
	}
}

func toNullable[T any](ptr *T) interface{} {
	if ptr == nil {
		return nil
	}
	return *ptr
}

// Compile-time assertions that all event types implement the Event interface
var (
	_ Event = (*SwapEvent)(nil)
	_ Event = (*MintEvent)(nil)
	_ Event = (*BurnEvent)(nil)
)
