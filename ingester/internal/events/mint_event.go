package events

type MintEvent struct {
	BaseEvent
	Sender  string `json:"sender"`
	Amount0 string `json:"amount0"` // Token0 amount added to pool
	Amount1 string `json:"amount1"` // Token1 amount added to pool
}

func (e MintEvent) ToMap() map[string]interface{} {
	m := e.BaseEvent.ToMap()
	m["sender"] = e.Sender
	m["amount0"] = e.Amount0
	m["amount1"] = e.Amount1
	return m
}
