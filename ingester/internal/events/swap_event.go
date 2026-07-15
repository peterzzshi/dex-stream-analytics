package events

type SwapEvent struct {
	BaseEvent
	Sender     string   `json:"sender"`
	Recipient  string   `json:"recipient"`
	Amount0In  string   `json:"amount0In"`
	Amount1In  string   `json:"amount1In"`
	Amount0Out string   `json:"amount0Out"`
	Amount1Out string   `json:"amount1Out"`
	Price      float64  `json:"price"`
	VolumeUSD  *float64 `json:"volumeUSD"`
	GasUsed    int64    `json:"gasUsed"`
	GasPrice   string   `json:"gasPrice"`
}

func (e SwapEvent) WithPrice(price float64) SwapEvent {
	e.Price = price
	return e
}

func (e SwapEvent) ToMap() map[string]interface{} {
	m := e.BaseEvent.ToMap()
	m["sender"] = e.Sender
	m["recipient"] = e.Recipient
	m["amount0In"] = e.Amount0In
	m["amount1In"] = e.Amount1In
	m["amount0Out"] = e.Amount0Out
	m["amount1Out"] = e.Amount1Out
	m["price"] = e.Price
	m["volumeUSD"] = toNullable(e.VolumeUSD)
	m["gasUsed"] = e.GasUsed
	m["gasPrice"] = e.GasPrice
	return m
}
