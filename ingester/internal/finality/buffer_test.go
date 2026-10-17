package finality

import (
	"testing"

	"go.uber.org/zap"

	"ingester/internal/events"
)

func makeEvent(blockNumber int64, eventID string) events.Event {
	return &events.SwapEvent{
		BaseEvent: events.BaseEvent{
			EventType:   events.EventTypeSwap,
			EventID:     eventID,
			BlockNumber: blockNumber,
		},
	}
}

func TestBufferPassthroughWhenZeroConfirmations(t *testing.T) {
	buf := NewBuffer(0, zap.NewNop())

	evt := makeEvent(100, "evt-1")
	result := buf.Add(evt)

	if len(result.Confirmed) != 1 {
		t.Fatalf("expected 1 confirmed event, got %d", len(result.Confirmed))
	}
	if len(result.Evicted) != 0 {
		t.Fatalf("expected 0 evicted events, got %d", len(result.Evicted))
	}
	if result.Confirmed[0].GetEventID() != "evt-1" {
		t.Errorf("expected evt-1, got %s", result.Confirmed[0].GetEventID())
	}
	if buf.PendingCount() != 0 {
		t.Errorf("expected 0 pending, got %d", buf.PendingCount())
	}
}

func TestBufferHoldsEventsUntilConfirmed(t *testing.T) {
	buf := NewBuffer(3, zap.NewNop()) // need 3 confirmations

	// Add event at block 100 — not yet confirmed
	result := buf.Add(makeEvent(100, "evt-100"))
	if len(result.Confirmed) != 0 {
		t.Fatalf("expected 0 confirmed, got %d", len(result.Confirmed))
	}

	// Block 101 — still not enough
	result = buf.Add(makeEvent(101, "evt-101"))
	if len(result.Confirmed) != 0 {
		t.Fatalf("expected 0 confirmed at block 101, got %d", len(result.Confirmed))
	}

	// Block 102 — still not enough (need tip=103 for block 100 to be confirmed)
	result = buf.Add(makeEvent(102, "evt-102"))
	if len(result.Confirmed) != 0 {
		t.Fatalf("expected 0 released at block 102, got %d", len(result.Confirmed))
	}

	// Block 103 — now block 100 is confirmed (103 - 3 = 100)
	result = buf.Add(makeEvent(103, "evt-103"))
	if len(result.Confirmed) != 1 {
		t.Fatalf("expected 1 released at block 103, got %d", len(result.Confirmed))
	}
	if result.Confirmed[0].GetEventID() != "evt-100" {
		t.Errorf("expected evt-100, got %s", result.Confirmed[0].GetEventID())
	}

	// Block 104 — block 101 confirmed (104 - 3 = 101)
	result = buf.Add(makeEvent(104, "evt-104"))
	if len(result.Confirmed) != 1 {
		t.Fatalf("expected 1 released at block 104, got %d", len(result.Confirmed))
	}
	if result.Confirmed[0].GetEventID() != "evt-101" {
		t.Errorf("expected evt-101, got %s", result.Confirmed[0].GetEventID())
	}
}

func TestBufferReleasesMultipleEventsFromSameBlock(t *testing.T) {
	buf := NewBuffer(2, zap.NewNop())

	// Two events at block 50
	buf.Add(makeEvent(50, "evt-50a"))
	buf.Add(makeEvent(50, "evt-50b"))

	// Block 51 — not confirmed yet
	result := buf.Add(makeEvent(51, "evt-51"))
	if len(result.Confirmed) != 0 {
		t.Fatalf("expected 0 released at block 51, got %d", len(result.Confirmed))
	}

	// Block 52 — block 50 confirmed (52 - 2 = 50)
	result = buf.Add(makeEvent(52, "evt-52"))
	if len(result.Confirmed) != 2 {
		t.Fatalf("expected 2 released at block 52, got %d", len(result.Confirmed))
	}
}

func TestAdvanceTipReleasesEvents(t *testing.T) {
	buf := NewBuffer(2, zap.NewNop())

	buf.Add(makeEvent(10, "evt-10"))

	// Advance tip without new event
	result := buf.AdvanceTip(12)
	if len(result.Confirmed) != 1 {
		t.Fatalf("expected 1 released from AdvanceTip, got %d", len(result.Confirmed))
	}
	if result.Confirmed[0].GetEventID() != "evt-10" {
		t.Errorf("expected evt-10, got %s", result.Confirmed[0].GetEventID())
	}
}

func TestFlushReleasesAllPending(t *testing.T) {
	buf := NewBuffer(100, zap.NewNop()) // very high threshold

	buf.Add(makeEvent(1, "evt-1"))
	buf.Add(makeEvent(2, "evt-2"))
	buf.Add(makeEvent(3, "evt-3"))

	if buf.PendingCount() != 3 {
		t.Fatalf("expected 3 pending, got %d", buf.PendingCount())
	}

	released := buf.Flush()
	if len(released) != 3 {
		t.Fatalf("expected 3 flushed, got %d", len(released))
	}
	if buf.PendingCount() != 0 {
		t.Errorf("expected 0 pending after flush, got %d", buf.PendingCount())
	}
}

func TestFlushReleasesInBlockOrder(t *testing.T) {
	buf := NewBuffer(100, zap.NewNop())

	buf.Add(makeEvent(30, "evt-30"))
	buf.Add(makeEvent(10, "evt-10"))
	buf.Add(makeEvent(20, "evt-20"))

	released := buf.Flush()
	if len(released) != 3 {
		t.Fatalf("expected 3, got %d", len(released))
	}
	// Events should come in block order: 10, 20, 30
	if released[0].GetBlockNumber() != 10 {
		t.Errorf("expected block 10 first, got %d", released[0].GetBlockNumber())
	}
	if released[1].GetBlockNumber() != 20 {
		t.Errorf("expected block 20 second, got %d", released[1].GetBlockNumber())
	}
	if released[2].GetBlockNumber() != 30 {
		t.Errorf("expected block 30 third, got %d", released[2].GetBlockNumber())
	}
}

func TestPendingBlocks(t *testing.T) {
	buf := NewBuffer(10, zap.NewNop())

	buf.Add(makeEvent(1, "evt-1a"))
	buf.Add(makeEvent(1, "evt-1b"))
	buf.Add(makeEvent(2, "evt-2"))

	if buf.PendingBlocks() != 2 {
		t.Errorf("expected 2 pending blocks, got %d", buf.PendingBlocks())
	}
}
