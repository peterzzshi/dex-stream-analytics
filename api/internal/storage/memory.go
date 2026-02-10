package storage

import "sync"

type Memory struct {
    mu      sync.RWMutex
    twap    map[string]float64
    volume  map[string]float64
    summary map[string]any
}

func NewMemory() *Memory {
    return &Memory{
        twap:    make(map[string]float64),
        volume:  make(map[string]float64),
        summary: make(map[string]any),
    }
}

func (m *Memory) LatestTWAP(pair string) (float64, bool) {
    m.mu.RLock()
    defer m.mu.RUnlock()
    v, ok := m.twap[pair]
    return v, ok
}

func (m *Memory) LatestVolume(pair string) (float64, bool) {
    m.mu.RLock()
    defer m.mu.RUnlock()
    v, ok := m.volume[pair]
    return v, ok
}

func (m *Memory) Summary() map[string]any {
    m.mu.RLock()
    defer m.mu.RUnlock()
    out := make(map[string]any, len(m.summary))
    for k, v := range m.summary {
        out[k] = v
    }
    return out
}

func (m *Memory) Upsert(pair string, twap, vol float64) {
    m.mu.Lock()
    defer m.mu.Unlock()
    m.twap[pair] = twap
    m.volume[pair] = vol
}
