package llm

import (
	"context"
	"fmt"
	"time"

	openrouter "github.com/revrost/go-openrouter"
)

const (
	// DefaultModel auto-routes to any available free model on OpenRouter.
	DefaultModel = "openrouter/free"
)

// OpenRouterProvider implements Provider using the OpenRouter API.
// It supports model fallback and retry with backoff.
type OpenRouterProvider struct {
	client     *openrouter.Client
	model      string
	fallbacks  []string // models to try if primary fails
	maxRetries int
}

// OpenRouterOption configures the OpenRouter provider.
type OpenRouterOption func(*OpenRouterProvider)

// WithModel sets the primary model. Defaults to DefaultModel.
func WithModel(model string) OpenRouterOption {
	return func(p *OpenRouterProvider) {
		p.model = model
	}
}

// WithFallbacks sets fallback models tried in order when primary fails.
func WithFallbacks(models ...string) OpenRouterOption {
	return func(p *OpenRouterProvider) {
		p.fallbacks = models
	}
}

// WithMaxRetries sets the max retries per model. Default 2.
func WithMaxRetries(n int) OpenRouterOption {
	return func(p *OpenRouterProvider) {
		p.maxRetries = n
	}
}

// NewOpenRouterProvider creates a new OpenRouter provider.
func NewOpenRouterProvider(apiKey string, opts ...OpenRouterOption) *OpenRouterProvider {
	client := openrouter.NewClient(
		apiKey,
		openrouter.WithXTitle("beads-workflow-system"),
	)

	p := &OpenRouterProvider{
		client:     client,
		model:      DefaultModel,
		fallbacks:  []string{"stepfun/step-3.5-flash:free"},
		maxRetries: 2,
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
// It tries the primary model first, then fallbacks, with retry+backoff per model.
func (p *OpenRouterProvider) Complete(ctx context.Context, req CompletionRequest) (*CompletionResponse, error) {
	model := req.Model
	if model == "" {
		model = p.model
	}

	// Build the list of models to try: primary + fallbacks.
	models := []string{model}
	for _, fb := range p.fallbacks {
		if fb != model {
			models = append(models, fb)
		}
	}

	var lastErr error
	for _, m := range models {
		for attempt := 0; attempt <= p.maxRetries; attempt++ {
			if attempt > 0 {
				// Exponential backoff: 1s, 2s, 4s...
				backoff := time.Duration(1<<uint(attempt-1)) * time.Second
				select {
				case <-time.After(backoff):
				case <-ctx.Done():
					return nil, ctx.Err()
				}
			}

			resp, err := p.doComplete(ctx, m, req)
			if err != nil {
				lastErr = err
				continue
			}

			// Check for empty response (rate limit symptom on free models).
			if resp.Content == "" {
				lastErr = fmt.Errorf("model %s returned empty response (possible rate limit)", m)
				continue
			}

			return resp, nil
		}
	}

	return nil, fmt.Errorf("all models exhausted: %w", lastErr)
}

// doComplete makes a single completion call to OpenRouter.
func (p *OpenRouterProvider) doComplete(ctx context.Context, model string, req CompletionRequest) (*CompletionResponse, error) {
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
		return nil, fmt.Errorf("openrouter %s: %w", model, err)
	}

	if len(resp.Choices) == 0 {
		return nil, fmt.Errorf("openrouter %s: no choices returned", model)
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
