package finality

import (
	"context"
	"log/slog"
	"os"
	"sort"
	"sync"

	"ingester/internal/events"
)

var logger = slog.New(slog.NewJSONHandler(os.Stdout, nil))

// Buffer holds events until their block reaches N confirmations.
// Events are released in block-number order once the chain tip advances
// past blockNumber + requiredConfirmations.
type Buffer struct {
	requiredConfirmations uint64
	mu                    sync.Mutex
	pending               map[uint64][]events.Event // blockNumber -> events
	highestBlock          uint64
}

// NewBuffer creates a finality buffer requiring N confirmations before release.
// If confirmations is 0, events pass through immediately (no buffering).
func NewBuffer(confirmations uint64) *Buffer {
	return &Buffer{
		requiredConfirmations: confirmations,
		pending:               make(map[uint64][]events.Event),
	}
}

// RequiredConfirmations returns the configured confirmation depth.
func (b *Buffer) RequiredConfirmations() uint64 {
	return b.requiredConfirmations
}

// Add buffers an event. Returns any events now confirmed given the current chain tip.
func (b *Buffer) Add(event events.Event) []events.Event {
	if b.requiredConfirmations == 0 {
		return []events.Event{event}
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	blockNum := uint64(event.GetBlockNumber())
	b.pending[blockNum] = append(b.pending[blockNum], event)

	if blockNum > b.highestBlock {
		b.highestBlock = blockNum
	}

	return b.releaseConfirmed()
}

// AdvanceTip updates the known chain tip and returns any newly confirmed events.
// Use this when receiving block-number updates without new events.
func (b *Buffer) AdvanceTip(blockNumber uint64) []events.Event {
	if b.requiredConfirmations == 0 {
		return nil
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	if blockNumber > b.highestBlock {
		b.highestBlock = blockNumber
	}

	return b.releaseConfirmed()
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
		logger.Info("Finality buffer released events",
			"count", len(released),
			"confirmedThreshold", confirmedThreshold,
			"chainTip", b.highestBlock,
			"pendingBlocks", len(b.pending),
		)
	}

	return released
}

// DrainConfirmed is a convenience for use in a select loop: it adds the event,
// then sends any confirmed events to the output channel.
func (b *Buffer) DrainConfirmed(ctx context.Context, event events.Event, output chan<- events.Event) error {
	confirmed := b.Add(event)
	for _, evt := range confirmed {
		select {
		case output <- evt:
		case <-ctx.Done():
			return ctx.Err()
		}
	}
	return nil
}

func sortedBlocks(pending map[uint64][]events.Event) []uint64 {
	blocks := make([]uint64, 0, len(pending))
	for blockNum := range pending {
		blocks = append(blocks, blockNum)
	}
	sort.Slice(blocks, func(i, j int) bool { return blocks[i] < blocks[j] })
	return blocks
}
