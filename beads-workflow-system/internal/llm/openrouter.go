package llm

import (
	"context"
	"fmt"

	openrouter "github.com/revrost/go-openrouter"
)

const (
	// DefaultModel is the free model we use by default.
	DefaultModel = "stepfun/step-3.5-flash:free"
)

// OpenRouterProvider implements Provider using the OpenRouter API.
type OpenRouterProvider struct {
	client *openrouter.Client
	model  string
}

// OpenRouterOption configures the OpenRouter provider.
type OpenRouterOption func(*OpenRouterProvider)

// WithModel sets the model to use. Defaults to DefaultModel.
func WithModel(model string) OpenRouterOption {
	return func(p *OpenRouterProvider) {
		p.model = model
	}
}

// NewOpenRouterProvider creates a new OpenRouter provider.
// apiKey is the OPENROUTER_API_KEY value.
func NewOpenRouterProvider(apiKey string, opts ...OpenRouterOption) *OpenRouterProvider {
	client := openrouter.NewClient(
		apiKey,
		openrouter.WithXTitle("beads-workflow-system"),
	)

	p := &OpenRouterProvider{
		client: client,
		model:  DefaultModel,
	}

	for _, opt := range opts {
		opt(p)
	}

	return p
}

// Name returns the provider name.
func (p *OpenRouterProvider) Name() string {
	return "openrouter"
}

// Complete sends a completion request to OpenRouter.
func (p *OpenRouterProvider) Complete(ctx context.Context, req CompletionRequest) (*CompletionResponse, error) {
	model := req.Model
	if model == "" {
		model = p.model
	}

	// Convert our messages to OpenRouter messages.
	msgs := make([]openrouter.ChatCompletionMessage, len(req.Messages))
	for i, m := range req.Messages {
		msgs[i] = openrouter.ChatCompletionMessage{
			Role:    m.Role,
			Content: openrouter.Content{Text: m.Content},
		}
	}

	orReq := openrouter.ChatCompletionRequest{
		Model:    model,
		Messages: msgs,
	}

	if req.Temperature > 0 {
		orReq.Temperature = float32(req.Temperature)
	}
	if req.MaxTokens > 0 {
		orReq.MaxTokens = req.MaxTokens
	}

	resp, err := p.client.CreateChatCompletion(ctx, orReq)
	if err != nil {
		return nil, fmt.Errorf("openrouter completion failed: %w", err)
	}

	if len(resp.Choices) == 0 {
		return nil, fmt.Errorf("openrouter returned no choices")
	}

	content := resp.Choices[0].Message.Content.Text

	return &CompletionResponse{
		Content:      content,
		Model:        resp.Model,
		PromptTokens: resp.Usage.PromptTokens,
		OutputTokens: resp.Usage.CompletionTokens,
		TotalTokens:  resp.Usage.TotalTokens,
	}, nil
}
