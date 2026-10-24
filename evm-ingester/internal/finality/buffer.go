package finality

import (
	"slices"
	"sync"

	"go.uber.org/zap"

	"evm-ingester/internal/events"
)

const (
	// DefaultMaxPendingBlocks limits buffer memory growth. At 1000 blocks with
	// ~100 events/block, memory footprint is ~10MB (100KB/event estimate).
	DefaultMaxPendingBlocks = 1000
	// DefaultMaxPendingEvents is a secondary limit if blocks are abnormally dense.
	DefaultMaxPendingEvents = 100_000
)

// Buffer holds events until their block reaches N confirmations.
// Events are released in block-number order once the chain tip advances
// past blockNumber + requiredConfirmations.
//
// Memory is bounded by evicting oldest unconfirmed blocks when limits are hit.
// Evicted events are returned so the caller can route them to a DLQ.
type Buffer struct {
	requiredConfirmations uint64
	maxPendingBlocks      int
	maxPendingEvents      int
	logger                *zap.Logger
	mu                    sync.Mutex
	pending               map[uint64][]events.Event // blockNumber -> events
	highestBlock          uint64
}

// NewBuffer creates a finality buffer requiring N confirmations before release.
// If confirmations is 0, events pass through immediately (no buffering).
func NewBuffer(confirmations uint64, logger *zap.Logger) *Buffer {
	return &Buffer{
		requiredConfirmations: confirmations,
		maxPendingBlocks:      DefaultMaxPendingBlocks,
		maxPendingEvents:      DefaultMaxPendingEvents,
		logger:                logger,
		pending:               make(map[uint64][]events.Event),
	}
}

// WithLimits returns a buffer with custom memory limits. Use for testing or
// resource-constrained deployments.
func (b *Buffer) WithLimits(maxBlocks, maxEvents int) *Buffer {
	b.maxPendingBlocks = maxBlocks
	b.maxPendingEvents = maxEvents
	return b
}

// RequiredConfirmations returns the configured confirmation depth.
func (b *Buffer) RequiredConfirmations() uint64 {
	return b.requiredConfirmations
}

// AddResult carries both confirmed events and any evicted due to buffer limits.
type AddResult struct {
	Confirmed []events.Event // Events released due to finality
	Evicted   []events.Event // Events evicted due to buffer pressure (route to DLQ)
}

// Add buffers an event. Returns confirmed events and any evicted due to memory limits.
// Evicted events should be routed to a DLQ rather than published.
func (b *Buffer) Add(event events.Event) AddResult {
	if b.requiredConfirmations == 0 {
		return AddResult{Confirmed: []events.Event{event}}
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	blockNum := uint64(event.GetBlockNumber())
	b.pending[blockNum] = append(b.pending[blockNum], event)

	if blockNum > b.highestBlock {
		b.highestBlock = blockNum
	}

	evicted := b.evictIfOverLimit()
	confirmed := b.releaseConfirmed()

	return AddResult{Confirmed: confirmed, Evicted: evicted}
}

// AdvanceTip updates the known chain tip and returns any newly confirmed events.
// Use this when receiving block-number updates without new events.
func (b *Buffer) AdvanceTip(blockNumber uint64) AddResult {
	if b.requiredConfirmations == 0 {
		return AddResult{}
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	if blockNumber > b.highestBlock {
		b.highestBlock = blockNumber
	}

	confirmed := b.releaseConfirmed()
	return AddResult{Confirmed: confirmed}
}

// Flush releases all pending events regardless of confirmation status.
// Use during graceful shutdown to avoid losing buffered events.
func (b *Buffer) Flush() []events.Event {
	b.mu.Lock()
	defer b.mu.Unlock()

	var all []events.Event
	blocks := sortedBlocks(b.pending)
	for _, blockNum := range blocks {
		all = append(all, b.pending[blockNum]...)
	}
	b.pending = make(map[uint64][]events.Event)
	return all
}

// PendingCount returns the number of events currently buffered.
func (b *Buffer) PendingCount() int {
	b.mu.Lock()
	defer b.mu.Unlock()

	count := 0
	for _, evts := range b.pending {
		count += len(evts)
	}
	return count
}

// PendingBlocks returns the number of distinct blocks buffered.
func (b *Buffer) PendingBlocks() int {
	b.mu.Lock()
	defer b.mu.Unlock()
	return len(b.pending)
}

// evictIfOverLimit removes oldest blocks when buffer exceeds size limits.
// Must be called with b.mu held. Returns evicted events for DLQ routing.
func (b *Buffer) evictIfOverLimit() []events.Event {
	pendingCount := b.countPending()

	if len(b.pending) <= b.maxPendingBlocks && pendingCount <= b.maxPendingEvents {
		return nil
	}

	blocks := sortedBlocks(b.pending)
	var evicted []events.Event

	// Evict oldest blocks until under both limits
	for _, blockNum := range blocks {
		if len(b.pending) <= b.maxPendingBlocks && pendingCount-len(b.pending[blockNum]) <= b.maxPendingEvents {
			break
		}
		evicted = append(evicted, b.pending[blockNum]...)
		pendingCount -= len(b.pending[blockNum])
		delete(b.pending, blockNum)
	}

	if len(evicted) > 0 {
		b.logger.Warn("Buffer evicted unconfirmed events due to memory limit",
			zap.Int("evictedCount", len(evicted)),
			zap.Int("remainingBlocks", len(b.pending)),
			zap.Uint64("oldestEvicted", blocks[0]),
			zap.Uint64("chainTip", b.highestBlock),
		)
	}

	return evicted
}

// releaseConfirmed emits events from blocks that are confirmed.
// Must be called with b.mu held.
func (b *Buffer) releaseConfirmed() []events.Event {
	if b.highestBlock < b.requiredConfirmations {
		return nil
	}

	confirmedThreshold := b.highestBlock - b.requiredConfirmations
	var released []events.Event

	blocks := sortedBlocks(b.pending)
	for _, blockNum := range blocks {
		if blockNum > confirmedThreshold {
			break
		}
		released = append(released, b.pending[blockNum]...)
		delete(b.pending, blockNum)
	}

	if len(released) > 0 {
		b.logger.Info("Finality buffer released events",
			zap.Int("count", len(released)),
			zap.Uint64("confirmedThreshold", confirmedThreshold),
			zap.Uint64("chainTip", b.highestBlock),
			zap.Int("pendingBlocks", len(b.pending)),
		)
	}

	return released
}

// countPending returns total events across all blocks. Must be called with b.mu held.
func (b *Buffer) countPending() int {
	count := 0
	for _, evts := range b.pending {
		count += len(evts)
	}
	return count
}

// DrainConfirmed is a convenience for use in a select loop: it adds the event,
// then sends confirmed events to the output channel and evicted events to the DLQ.
func (b *Buffer) DrainConfirmed(event events.Event, output chan<- events.Event, dlq chan<- events.Event) {
	result := b.Add(event)

	for _, evt := range result.Evicted {
		select {
		case dlq <- evt:
		default:
			b.logger.Error("DLQ channel full, dropping evicted event",
				zap.String("event_id", evt.GetEventID()),
			)
		}
	}

	for _, evt := range result.Confirmed {
		output <- evt
	}
}

func sortedBlocks(pending map[uint64][]events.Event) []uint64 {
	blocks := make([]uint64, 0, len(pending))
	for blockNum := range pending {
		blocks = append(blocks, blockNum)
	}
	slices.Sort(blocks)
	return blocks
}
