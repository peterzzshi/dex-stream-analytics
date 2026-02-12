package logger

type LogContextData struct {
	Tags      map[string]bool
	Category  string
	Metadata  map[string]string
	SessionID string
}
