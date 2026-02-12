package events

type SwapEvent struct {
	EventID         string   `json:"eventId"`
	BlockNumber     int64    `json:"blockNumber"`
	BlockTimestamp  int64    `json:"blockTimestamp"`
	TransactionHash string   `json:"transactionHash"`
	LogIndex        int32    `json:"logIndex"`
	PairAddress     string   `json:"pairAddress"`
	Token0          string   `json:"token0"`
	Token1          string   `json:"token1"`
	Token0Symbol    *string  `json:"token0Symbol"`
	Token1Symbol    *string  `json:"token1Symbol"`
	Sender          string   `json:"sender"`
	Recipient       string   `json:"recipient"`
	Amount0In       string   `json:"amount0In"`
	Amount1In       string   `json:"amount1In"`
	Amount0Out      string   `json:"amount0Out"`
	Amount1Out      string   `json:"amount1Out"`
	Price           float64  `json:"price"`
	VolumeUSD       *float64 `json:"volumeUSD"`
	GasUsed         int64    `json:"gasUsed"`
	GasPrice        string   `json:"gasPrice"`
	EventTimestamp  int64    `json:"eventTimestamp"`
}

func (swapEvent SwapEvent) WithPrice(price float64) SwapEvent {
	swapEvent.Price = price
	return swapEvent
}
