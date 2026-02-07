package monitoring

import (
	"testing"
	"time"
)

func TestMetricsCollectorWorkflowLifecycle(t *testing.T) {
	mc := NewMetricsCollector(nil)

	mc.RecordWorkflowStart("wf-1", "research", "research")
	mc.RecordWorkflowStart("wf-2", "poc", "poc")

	snap := mc.GetSnapshot()
	if snap.Workflows.TotalStarted != 2 {
		t.Errorf("started = %d, want 2", snap.Workflows.TotalStarted)
	}
	if snap.Workflows.ActiveCount != 2 {
		t.Errorf("active = %d, want 2", snap.Workflows.ActiveCount)
	}

	mc.RecordWorkflowEnd("wf-1", "research", "research", 100*time.Millisecond, true)
	mc.RecordWorkflowEnd("wf-2", "poc", "poc", 200*time.Millisecond, false)

	snap = mc.GetSnapshot()
	if snap.Workflows.TotalCompleted != 1 {
		t.Errorf("completed = %d, want 1", snap.Workflows.TotalCompleted)
	}
	if snap.Workflows.TotalFailed != 1 {
		t.Errorf("failed = %d, want 1", snap.Workflows.TotalFailed)
	}
	if snap.Workflows.SuccessRate != 50.0 {
		t.Errorf("success rate = %f, want 50.0", snap.Workflows.SuccessRate)
	}
	if snap.Workflows.AvgDurationMs != 150 {
		t.Errorf("avg duration = %f, want 150", snap.Workflows.AvgDurationMs)
	}
}

func TestMetricsCollectorPercentiles(t *testing.T) {
	mc := NewMetricsCollector(nil)

	// Record 10 workflow durations: 100ms, 200ms, ..., 1000ms
	for i := 1; i <= 10; i++ {
		mc.RecordWorkflowEnd("wf", "test", "test", time.Duration(i*100)*time.Millisecond, true)
	}

	snap := mc.GetSnapshot()
	if snap.Workflows.P95DurationMs < 900 {
		t.Errorf("p95 = %f, expected >= 900", snap.Workflows.P95DurationMs)
	}
	if snap.Workflows.P99DurationMs < 900 {
		t.Errorf("p99 = %f, expected >= 900", snap.Workflows.P99DurationMs)
	}
}

func TestMetricsCollectorTypeBreakdown(t *testing.T) {
	mc := NewMetricsCollector(nil)

	mc.RecordWorkflowStart("wf-1", "research", "research")
	mc.RecordWorkflowEnd("wf-1", "research", "research", 50*time.Millisecond, true)
	mc.RecordWorkflowStart("wf-2", "research", "research")
	mc.RecordWorkflowEnd("wf-2", "research", "research", 150*time.Millisecond, false)

	snap := mc.GetSnapshot()
	rm := snap.Workflows.ByType["research"]
	if rm.Started != 2 {
		t.Errorf("research started = %d, want 2", rm.Started)
	}
	if rm.Completed != 1 {
		t.Errorf("research completed = %d, want 1", rm.Completed)
	}
	if rm.SuccessRate != 50.0 {
		t.Errorf("research success rate = %f, want 50.0", rm.SuccessRate)
	}
}

func TestMetricsCollectorReset(t *testing.T) {
	mc := NewMetricsCollector(nil)
	mc.RecordWorkflowStart("wf-1", "research", "research")
	mc.Reset()

	snap := mc.GetSnapshot()
	if snap.Workflows.TotalStarted != 0 {
		t.Errorf("started = %d after reset, want 0", snap.Workflows.TotalStarted)
	}
}

func TestEventStreamPublishSubscribe(t *testing.T) {
	es := NewEventStream()

	// Subscribe to all events
	sub := es.Subscribe()
	defer es.Unsubscribe(sub.ID)

	// Publish event
	es.Publish(Event{
		Type:       EventWorkflowStarted,
		WorkflowID: "wf-1",
	})

	// Receive with timeout
	select {
	case ev := <-sub.Channel:
		if ev.WorkflowID != "wf-1" {
			t.Errorf("workflow ID = %q, want %q", ev.WorkflowID, "wf-1")
		}
	case <-time.After(time.Second):
		t.Fatal("timeout waiting for event")
	}
}

func TestEventStreamFilter(t *testing.T) {
	es := NewEventStream()

	// Subscribe only to workflow started events
	sub := es.Subscribe(EventWorkflowStarted)
	defer es.Unsubscribe(sub.ID)

	// Publish non-matching event
	es.Publish(Event{Type: EventWorkflowCompleted, WorkflowID: "wf-1"})
	// Publish matching event
	es.Publish(Event{Type: EventWorkflowStarted, WorkflowID: "wf-2"})

	select {
	case ev := <-sub.Channel:
		if ev.WorkflowID != "wf-2" {
			t.Errorf("expected wf-2, got %q", ev.WorkflowID)
		}
	case <-time.After(time.Second):
		t.Fatal("timeout waiting for event")
	}
}

func TestEventStreamHistory(t *testing.T) {
	es := NewEventStream()

	es.Publish(Event{Type: EventWorkflowStarted, WorkflowID: "wf-1"})
	es.Publish(Event{Type: EventWorkflowCompleted, WorkflowID: "wf-2"})
	es.Publish(Event{Type: EventStepStarted, WorkflowID: "wf-1"})

	all := es.GetHistory(10)
	if len(all) != 3 {
		t.Errorf("history len = %d, want 3", len(all))
	}

	filtered := es.GetHistory(10, EventWorkflowStarted)
	if len(filtered) != 1 {
		t.Errorf("filtered history len = %d, want 1", len(filtered))
	}
}

func TestEventStreamWorkflowEvents(t *testing.T) {
	es := NewEventStream()

	es.Publish(Event{Type: EventWorkflowStarted, WorkflowID: "wf-1"})
	es.Publish(Event{Type: EventStepStarted, WorkflowID: "wf-2"})
	es.Publish(Event{Type: EventStepCompleted, WorkflowID: "wf-1"})

	events := es.GetWorkflowEvents("wf-1")
	if len(events) != 2 {
		t.Errorf("wf-1 events = %d, want 2", len(events))
	}
}

func TestEventStreamUnsubscribe(t *testing.T) {
	es := NewEventStream()
	sub := es.Subscribe()

	if es.SubscriberCount() != 1 {
		t.Fatalf("subscriber count = %d, want 1", es.SubscriberCount())
	}

	es.Unsubscribe(sub.ID)

	if es.SubscriberCount() != 0 {
		t.Errorf("subscriber count after unsubscribe = %d, want 0", es.SubscriberCount())
	}
}