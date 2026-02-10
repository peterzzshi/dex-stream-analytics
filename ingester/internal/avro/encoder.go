package avro

import (
	"ingester/pkg/events"

	"github.com/linkedin/goavro/v2"
)

// EncodeSwap serialises a SwapEvent to Avro binary using a provided codec.
func EncodeSwap(codec *goavro.Codec, event events.SwapEvent) ([]byte, error) {
	record := map[string]interface{}{
		"eventId":         event.EventID,
		"blockNumber":     event.BlockNumber,
		"blockTimestamp":  event.BlockTimestamp,
		"transactionHash": event.TransactionHash,
		"logIndex":        event.LogIndex,
		"pairAddress":     event.PairAddress,
		"token0":          event.Token0,
		"token1":          event.Token1,
		"token0Symbol":    toNullable(event.Token0Symbol),
		"token1Symbol":    toNullable(event.Token1Symbol),
		"sender":          event.Sender,
		"recipient":       event.Recipient,
		"amount0In":       event.Amount0In,
		"amount1In":       event.Amount1In,
		"amount0Out":      event.Amount0Out,
		"amount1Out":      event.Amount1Out,
		"price":           event.Price,
		"volumeUSD":       toNullable(event.VolumeUSD),
		"gasUsed":         event.GasUsed,
		"gasPrice":        event.GasPrice,
		"eventTimestamp":  event.EventTimestamp,
	}
	return codec.BinaryFromNative(nil, record)
}

func toNullable[T any](ptr *T) interface{} {
	if ptr == nil {
		return nil
	}
	return *ptr
}
