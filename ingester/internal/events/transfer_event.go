package events

// TransferEvent represents LP token movement for the pair contract token.
//
// This is used to correlate Mint/Burn activity with LP token accounting.
type TransferEvent struct {
	BaseEvent
	From  string `json:"from"`
	To    string `json:"to"`
	Value string `json:"value"`
}

func (e TransferEvent) ToMap() map[string]interface{} {
	m := e.BaseEvent.ToMap()
	m["from"] = e.From
	m["to"] = e.To
	m["value"] = e.Value
	return m
}
