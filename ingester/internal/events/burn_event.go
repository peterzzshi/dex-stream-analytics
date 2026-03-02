package events

type BurnEvent struct {
	BaseEvent
	Sender    string `json:"sender"`
	Recipient string `json:"recipient"`
	Amount0   string `json:"amount0"` // Token0 amount removed from pool
	Amount1   string `json:"amount1"` // Token1 amount removed from pool
}

func (e BurnEvent) ToMap() map[string]interface{} {
	m := e.BaseEvent.ToMap()
	m["sender"] = e.Sender
	m["recipient"] = e.Recipient
	m["amount0"] = e.Amount0
	m["amount1"] = e.Amount1
	return m
}
