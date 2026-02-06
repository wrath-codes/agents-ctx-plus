# Model Providers

## Overview

Rig provides a unified interface for working with multiple LLM providers. This guide covers configuration and usage for each supported provider.

## OpenAI

### Setup

```toml
[dependencies]
rig-core = "0.5"
```

```bash
export OPENAI_API_KEY="sk-..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = openai::Client::from_env();
    
    let agent = client
        .agent("gpt-4")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{}", response);
    
    Ok(())
}
```

### Available Models

| Model | Description | Context |
|-------|-------------|---------|
| `gpt-4` | GPT-4 | 8K |
| `gpt-4-turbo` | GPT-4 Turbo | 128K |
| `gpt-4o` | GPT-4o (omni) | 128K |
| `gpt-3.5-turbo` | GPT-3.5 | 16K |
| `text-embedding-3-small` | Embeddings | - |
| `text-embedding-3-large` | Embeddings | - |

### Advanced Configuration

```rust
let client = openai::Client::from_env();

let agent = client
    .agent("gpt-4")
    .preamble("You are a helpful assistant.")
    .temperature(0.7)
    .max_tokens(2000)
    .top_p(0.9)
    .frequency_penalty(0.5)
    .presence_penalty(0.5)
    .build();
```

### Direct Model Access

```rust
use rig::completion::CompletionModel;

let model = client.completion_model("gpt-4");

let request = CompletionRequest {
    prompt: "Hello".to_string(),
    preamble: Some("You are helpful.".to_string()),
    temperature: Some(0.7),
    max_tokens: Some(1000),
};

let response = model.completion(request).await?;
```

### Embeddings

```rust
use rig::embeddings::EmbeddingModel;

let model = client.embedding_model("text-embedding-3-small");
let embedding = model.embed("Hello world").await?;

println!("Vector: {:?}", embedding.vec);
println!("Dimensions: {}", embedding.dimensions);
```

## Anthropic

### Setup

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::anthropic};

let client = anthropic::Client::from_env();

let agent = client
    .agent("claude-3-opus-20240229")
    .build();

let response = agent.prompt("Hello!").await?;
```

### Available Models

| Model | Description | Context |
|-------|-------------|---------|
| `claude-3-opus-20240229` | Most powerful | 200K |
| `claude-3-sonnet-20240229` | Balanced | 200K |
| `claude-3-haiku-20240307` | Fastest | 200K |

## Gemini (Google)

### Setup

```bash
export GEMINI_API_KEY="..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::gemini};

let client = gemini::Client::from_env();

let agent = client
    .agent("gemini-1.5-pro")
    .build();

let response = agent.prompt("Hello!").await?;
```

### Available Models

| Model | Description | Context |
|-------|-------------|---------|
| `gemini-1.5-pro` | Pro model | 1M |
| `gemini-1.5-flash` | Fast model | 1M |
| `gemini-pro` | Standard | 32K |

## Ollama (Local Models)

### Setup

Install Ollama: https://ollama.ai

Pull a model:
```bash
ollama pull llama2
ollama pull codellama
ollama pull mistral
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::ollama};

let client = ollama::Client::new("http://localhost:11434");

let agent = client
    .agent("llama2")
    .build();

let response = agent.prompt("Hello!").await?;
```

### Configuration

```rust
let client = ollama::Client::new("http://localhost:11434");

let agent = client
    .agent("llama2")
    .temperature(0.8)
    .num_ctx(4096)  // Context window
    .num_predict(256)  // Max tokens
    .top_p(0.9)
    .top_k(40)
    .build();
```

### Available Models

Any model from Ollama's library:
- `llama2` - Meta's Llama 2
- `codellama` - Code-specialized
- `mistral` - Mistral AI
- `mixtral` - Mixture of Experts
- `llava` - Vision model
- And 100+ more

## Cohere

### Setup

```bash
export COHERE_API_KEY="..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::cohere};

let client = cohere::Client::from_env();

let agent = client
    .agent("command")
    .build();

let response = agent.prompt("Hello!").await?;
```

### Available Models

| Model | Description |
|-------|-------------|
| `command` | General purpose |
| `command-r` | RAG-optimized |
| `command-r-plus` | Advanced RAG |
| `embed-english-v3.0` | Embeddings |

## Perplexity

### Setup

```bash
export PERPLEXITY_API_KEY="pplx-..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::perplexity};

let client = perplexity::Client::from_env();

let agent = client
    .agent("sonar-medium-chat")
    .build();

let response = agent.prompt("What is Rust?").await?;
```

### Features

Perplexity models are search-augmented, providing up-to-date information.

## Hugging Face

### Setup

```bash
export HF_API_KEY="hf_..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::huggingface};

let client = huggingface::Client::from_env();

let agent = client
    .agent("meta-llama/Llama-2-70b-chat-hf")
    .build();

let response = agent.prompt("Hello!").await?;
```

## DeepSeek

### Setup

```bash
export DEEPSEEK_API_KEY="sk-..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::deepseek};

let client = deepseek::Client::from_env();

let agent = client
    .agent("deepseek-chat")
    .build();

let response = agent.prompt("Hello!").await?;
```

## XAI (Grok)

### Setup

```bash
export XAI_API_KEY="xai-..."
```

### Basic Usage

```rust
use rig::{completion::Prompt, providers::xai};

let client = xai::Client::from_env();

let agent = client
    .agent("grok-beta")
    .build();

let response = agent.prompt("Hello!").await?;
```

## Provider Comparison

| Provider | Best For | Context | Speed | Cost |
|----------|----------|---------|-------|------|
| OpenAI | General purpose | High | Fast | $$ |
| Anthropic | Long context | Very High | Medium | $$$ |
| Gemini | Multimodal | Very High | Fast | $$ |
| Ollama | Local/Privacy | Varies | Varies | Free |
| Cohere | Enterprise | High | Fast | $$ |
| Perplexity | Research | High | Medium | $$ |

## Multi-Provider Setup

### Using Multiple Providers

```rust
use rig::providers::{openai, anthropic};

struct MultiProviderClient {
    openai: openai::Client,
    anthropic: anthropic::Client,
}

impl MultiProviderClient {
    fn new() -> Self {
        Self {
            openai: openai::Client::from_env(),
            anthropic: anthropic::Client::from_env(),
        }
    }
    
    fn get_agent(&self, provider: &str) -> Box<dyn Prompt> {
        match provider {
            "openai" => Box::new(self.openai.agent("gpt-4").build()),
            "anthropic" => Box::new(self.anthropic.agent("claude-3-opus").build()),
            _ => panic!("Unknown provider"),
        }
    }
}
```

### Provider Selection

```rust
enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
}

fn create_agent(provider: Provider) -> Box<dyn Prompt> {
    match provider {
        Provider::OpenAI => {
            let client = openai::Client::from_env();
            Box::new(client.agent("gpt-4").build())
        }
        Provider::Anthropic => {
            let client = anthropic::Client::from_env();
            Box::new(client.agent("claude-3-opus").build())
        }
        Provider::Gemini => {
            let client = gemini::Client::from_env();
            Box::new(client.agent("gemini-1.5-pro").build())
        }
    }
}
```

## Best Practices

### 1. Environment Variables

Always use environment variables for API keys:

```rust
// Good
let client = openai::Client::from_env();

// Bad - hardcoded
let client = openai::Client::new("sk-...");
```

### 2. Fallback Providers

```rust
async fn prompt_with_fallback(message: &str) -> Result<String> {
    let providers = [
        || openai::Client::from_env().agent("gpt-4").build(),
        || anthropic::Client::from_env().agent("claude-3-sonnet").build(),
    ];
    
    for provider in &providers {
        let agent = provider();
        match agent.prompt(message).await {
            Ok(response) => return Ok(response),
            Err(_) => continue,
        }
    }
    
    Err(anyhow!("All providers failed"))
}
```

### 3. Rate Limiting

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn prompt_with_rate_limit(agent: &Agent, message: &str) -> Result<String> {
    let mut retries = 0;
    
    loop {
        match agent.prompt(message).await {
            Ok(response) => return Ok(response),
            Err(e) if e.to_string().contains("rate limit") && retries < 3 => {
                retries += 1;
                sleep(Duration::from_secs(2u64.pow(retries))).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

## Next Steps

- **[Vector Stores](../vector-stores/mongodb.md)** - Set up vector stores
- **[Tools](../core/tools.md)** - Add tool calling
- **[Advanced Patterns](../advanced/custom-providers.md)** - Build custom providers