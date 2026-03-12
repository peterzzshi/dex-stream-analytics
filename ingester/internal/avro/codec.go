package avro

import (
	_ "embed"
	"fmt"

	"ingester/internal/events"

	"github.com/linkedin/goavro/v2"
)

//go:embed SwapEvent.avsc
var swapEventSchemaText string

//go:embed MintEvent.avsc
var mintEventSchemaText string

//go:embed BurnEvent.avsc
var burnEventSchemaText string

var schemaRegistry = map[events.EventType]string{
	events.EventTypeSwap: swapEventSchemaText,
	events.EventTypeMint: mintEventSchemaText,
	events.EventTypeBurn: burnEventSchemaText,
}

func NewCodec(eventType events.EventType) (*goavro.Codec, error) {
	schemaText, ok := schemaRegistry[eventType]
	if !ok {
		return nil, fmt.Errorf("event type not found: %s", eventType)
	}
	codec, err := goavro.NewCodec(schemaText)
	if err != nil {
		return nil, fmt.Errorf("failed to parse Avro schema for %s: %w", eventType, err)
	}
	return codec, nil
}
