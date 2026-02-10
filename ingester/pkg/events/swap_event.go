package events

// SwapEvent mirrors the Avro schema while keeping fields immutable once built.
type SwapEvent struct {
    EventID        string
    BlockNumber    int64
    BlockTimestamp int64
    TransactionHash string
    LogIndex        int32
    PairAddress     string
    Token0          string
    Token1          string
    Token0Symbol    *string
    Token1Symbol    *string
    Sender          string
    Recipient       string
    Amount0In       string
    Amount1In       string
    Amount0Out      string
    Amount1Out      string
    Price           float64
    VolumeUSD       *float64
    GasUsed         int64
    GasPrice        string
    EventTimestamp  int64
}

// WithPrice returns a new event with price set, preserving immutability.
func (e SwapEvent) WithPrice(price float64) SwapEvent {
    e.Price = price
    return e
}
