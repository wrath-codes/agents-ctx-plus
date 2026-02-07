package llm

import "context"

// Message represents a single message in a conversation.
type Message struct {
	Role    string `json:"role"` // "system", "user", "assistant"
	Content string `json:"content"`
}

// CompletionRequest is a provider-agnostic request for LLM completion.
type CompletionRequest struct {
	Messages    []Message
	Model       string  // model identifier (provider-specific)
	Temperature float64 // 0.0-2.0, 0 = deterministic
	MaxTokens   int     // max tokens in response, 0 = provider default
}

// CompletionResponse is the provider-agnostic response from an LLM.
type CompletionResponse struct {
	Content      string // the assistant's response text
	Model        string // model that actually served the request
	PromptTokens int    // tokens used in the prompt
	OutputTokens int    // tokens used in the response
	TotalTokens  int    // prompt + output
}

// Provider is the interface for any LLM backend (OpenRouter, Ollama, etc.).
type Provider interface {
	// Complete sends a completion request and returns the response.
	Complete(ctx context.Context, req CompletionRequest) (*CompletionResponse, error)

	// Name returns the provider name (e.g. "openrouter", "ollama").
	Name() string
}

// SystemMessage is a helper to create a system message.
func SystemMessage(content string) Message {
	return Message{Role: "system", Content: content}
}

// UserMessage is a helper to create a user message.
func UserMessage(content string) Message {
	return Message{Role: "user", Content: content}
}

// AssistantMessage is a helper to create an assistant message.
func AssistantMessage(content string) Message {
	return Message{Role: "assistant", Content: content}
}
