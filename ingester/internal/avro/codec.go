package avro

import (
	_ "embed"

	"github.com/linkedin/goavro/v2"
)

//go:embed swap_event.avsc
var swapEventSchema string

func NewSwapEventCodec() (*goavro.Codec, error) {
	return goavro.NewCodec(swapEventSchema)
}
