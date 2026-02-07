package monitoring

import (
	"encoding/json"
	"fmt"
	"sync"
	"time"
)

// EventType represents a workflow event type
type EventType string

const (
	EventWorkflowStarted   EventType = "workflow:started"
	EventWorkflowCompleted EventType = "workflow:completed"
	EventWorkflowFailed    EventType = "workflow:failed"
	EventWorkflowCancelled EventType = "workflow:cancelled"
	EventStepStarted       EventType = "step:started"
	EventStepCompleted     EventType = "step:completed"
	EventStepFailed        EventType = "step:failed"
	EventAgentAssigned     EventType = "agent:assigned"
	EventAgentHandoff      EventType = "agent:handoff"
	EventAgentHeartbeat    EventType = "agent:heartbeat"
	EventResultStored      EventType = "result:stored"
)

// Event represents a workflow system event
type Event struct {
	ID         string                 `json:"id"`
	Type       EventType              `json:"type"`
	WorkflowID string                 `json:"workflow_id,omitempty"`
	AgentID    string                 `json:"agent_id,omitempty"`
	StepName   string                 `json:"step_name,omitempty"`
	Data       map[string]interface{} `json:"data,omitempty"`
	Timestamp  time.Time              `json:"timestamp"`
}

// EventStream provides pub/sub for workflow events
type EventStream struct {
	mu          sync.RWMutex
	subscribers map[string]*Subscriber
	history     []Event
	maxHistory  int
	nextID      int64
}

// Subscriber represents an event subscriber
type Subscriber struct {
	ID       string
	Channel  chan Event
	Filters  []EventType
	closed   bool
	mu       sync.Mutex
}

// NewEventStream creates a new event stream
func NewEventStream() *EventStream {
	return &EventStream{
		subscribers: make(map[string]*Subscriber),
		history:     make([]Event, 0, 1000),
		maxHistory:  1000,
	}
}

// Subscribe creates a new subscription with optional event type filters
func (es *EventStream) Subscribe(filters ...EventType) *Subscriber {
	es.mu.Lock()
	defer es.mu.Unlock()

	es.nextID++
	sub := &Subscriber{
		ID:      fmt.Sprintf("sub-%d", es.nextID),
		Channel: make(chan Event, 256),
		Filters: filters,
	}

	es.subscribers[sub.ID] = sub
	return sub
}

// Unsubscribe removes a subscription
func (es *EventStream) Unsubscribe(subID string) {
	es.mu.Lock()
	defer es.mu.Unlock()

	if sub, ok := es.subscribers[subID]; ok {
		sub.mu.Lock()
		sub.closed = true
		close(sub.Channel)
		sub.mu.Unlock()
		delete(es.subscribers, subID)
	}
}

// Publish publishes an event to all matching subscribers
func (es *EventStream) Publish(event Event) {
	if event.Timestamp.IsZero() {
		event.Timestamp = time.Now()
	}

	es.mu.Lock()
	// Add to history
	es.history = append(es.history, event)
	if len(es.history) > es.maxHistory {
		es.history = es.history[len(es.history)-es.maxHistory:]
	}

	// Copy subscriber list to avoid holding the lock during send
	subs := make([]*Subscriber, 0, len(es.subscribers))
	for _, sub := range es.subscribers {
		subs = append(subs, sub)
	}
	es.mu.Unlock()

	// Deliver to subscribers
	for _, sub := range subs {
		sub.mu.Lock()
		if sub.closed {
			sub.mu.Unlock()
			continue
		}

		if sub.matchesFilter(event.Type) {
			select {
			case sub.Channel <- event:
			default:
				// Channel full, skip
			}
		}
		sub.mu.Unlock()
	}
}

// GetHistory returns recent events, optionally filtered by type
func (es *EventStream) GetHistory(limit int, eventTypes ...EventType) []Event {
	es.mu.RLock()
	defer es.mu.RUnlock()

	filterSet := make(map[EventType]bool)
	for _, t := range eventTypes {
		filterSet[t] = true
	}

	var results []Event
	// Walk backwards for most recent first
	for i := len(es.history) - 1; i >= 0 && len(results) < limit; i-- {
		ev := es.history[i]
		if len(filterSet) == 0 || filterSet[ev.Type] {
			results = append(results, ev)
		}
	}

	return results
}

// GetWorkflowEvents returns events for a specific workflow
func (es *EventStream) GetWorkflowEvents(workflowID string) []Event {
	es.mu.RLock()
	defer es.mu.RUnlock()

	var results []Event
	for _, ev := range es.history {
		if ev.WorkflowID == workflowID {
			results = append(results, ev)
		}
	}

	return results
}

// SubscriberCount returns the number of active subscribers
func (es *EventStream) SubscriberCount() int {
	es.mu.RLock()
	defer es.mu.RUnlock()
	return len(es.subscribers)
}

// matchesFilter checks if an event type matches the subscriber's filter
func (sub *Subscriber) matchesFilter(eventType EventType) bool {
	if len(sub.Filters) == 0 {
		return true // No filter = receive all
	}

	for _, f := range sub.Filters {
		if f == eventType {
			return true
		}
	}
	return false
}

// ToJSON serializes an event to JSON
func (e Event) ToJSON() (string, error) {
	bytes, err := json.Marshal(e)
	if err != nil {
		return "", err
	}
	return string(bytes), nil
}