# Getting Started with Rig

## Installation

### Requirements

- **Rust**: 1.70.0 or later
- **Cargo**: Comes with Rust
- **API Key**: From your chosen LLM provider (OpenAI, Anthropic, etc.)

### Adding Rig to Your Project

```bash
cargo new my-rig-app
cd my-rig-app
cargo add rig-core tokio --features tokio/macros,tokio/rt-multi-thread
```

Or manually add to `Cargo.toml`:

```toml
[dependencies]
rig-core = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
anyhow = "1"  # For error handling
```

### Environment Setup

Set your API key as an environment variable:

```bash
# OpenAI
export OPENAI_API_KEY="your-api-key-here"

# Anthropic
export ANTHROPIC_API_KEY="your-api-key-here"

# Gemini
export GEMINI_API_KEY="your-api-key-here"
```

Or use a `.env` file:

```toml
# Cargo.toml
[dependencies]
dotenvy = "0.15"
```

```rust
// main.rs
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    // ...
}
```

## Your First Agent

### Basic Example

Create `src/main.rs`:

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the OpenAI client
    let client = openai::Client::from_env();
    
    // Create an agent
    let agent = client
        .agent("gpt-4")
        .preamble("You are a helpful assistant.")
        .build();
    
    // Prompt the agent
    let response = agent
        .prompt("What is the capital of France?")
        .await?;
    
    println!("Agent: {}", response);
    
    Ok(())
}
```

Run it:

```bash
cargo run
```

Expected output:
```
Agent: The capital of France is Paris.
```

### Understanding the Code

1. **Client**: Connects to the LLM provider
2. **Agent**: Configures the model with context and parameters
3. **Prompt**: Sends a message to the agent
4. **Response**: Returns the LLM's output

## Agent Configuration

### Basic Agent

```rust
let agent = client
    .agent("gpt-4")
    .preamble("You are a helpful assistant.")
    .build();
```

### Advanced Configuration

```rust
let agent = client
    .agent("gpt-4")
    .preamble("You are an expert programmer.")
    .context(&Context::new("You specialize in Rust programming."))
    .temperature(0.7)
    .max_tokens(1000)
    .build();
```

### Parameters Explained

| Parameter | Description | Default |
|-----------|-------------|---------|
| `preamble` | System message defining behavior | None |
| `context` | Additional context information | None |
| `temperature` | Randomness (0.0 - 2.0) | 1.0 |
| `max_tokens` | Maximum response length | Model default |
| `top_p` | Nucleus sampling | 1.0 |
| `frequency_penalty` | Reduce repetition | 0.0 |
| `presence_penalty` | Encourage new topics | 0.0 |

## Working with Different Providers

### OpenAI

```rust
use rig::providers::openai;

let client = openai::Client::from_env();
let agent = client.agent("gpt-4").build();
```

### Anthropic

```rust
use rig::providers::anthropic;

let client = anthropic::Client::from_env();
let agent = client.agent("claude-3-opus-20240229").build();
```

### Ollama (Local Models)

```rust
use rig::providers::ollama;

let client = ollama::Client::new("http://localhost:11434");
let agent = client.agent("llama2").build();
```

## Error Handling

### Basic Error Handling

```rust
use rig::completion::PromptError;

match agent.prompt("Hello").await {
    Ok(response) => println!("{}", response),
    Err(e) => eprintln!("Error: {}", e),
}
```

### With anyhow

```rust
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = openai::Client::from_env();
    let agent = client.agent("gpt-4").build();
    
    let response = agent.prompt("Hello").await?;
    println!("{}", response);
    
    Ok(())
}
```

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("LLM error: {0}")]
    Llm(#[from] PromptError),
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

## Project Structure

### Recommended Layout

```
my-rig-app/
├── Cargo.toml
├── .env
├── src/
│   ├── main.rs
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── assistant.rs
│   │   └── coder.rs
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── calculator.rs
│   │   └── search.rs
│   └── lib.rs
└── examples/
    └── basic.rs
```

### Example: Multi-Agent Setup

```rust
// src/agents/mod.rs
use rig::providers::openai;

pub struct Agents {
    pub assistant: rig::Agent,
    pub coder: rig::Agent,
}

impl Agents {
    pub fn new(client: &openai::Client) -> Self {
        let assistant = client
            .agent("gpt-4")
            .preamble("You are a helpful assistant.")
            .build();
        
        let coder = client
            .agent("gpt-4")
            .preamble("You are an expert programmer.")
            .build();
        
        Self { assistant, coder }
    }
}
```

## Testing Your Agent

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_response() {
        let client = openai::Client::from_env();
        let agent = client.agent("gpt-4").build();
        
        let response = agent.prompt("Say 'test'").await.unwrap();
        assert!(response.contains("test"));
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use my_rig_app::Agents;

#[tokio::test]
async fn test_coder_agent() {
    let client = rig::providers::openai::Client::from_env();
    let agents = Agents::new(&client);
    
    let response = agents.coder
        .prompt("Write a hello world in Rust")
        .await
        .unwrap();
    
    assert!(response.contains("println!"));
}
```

## Next Steps

- **[Agents](agents.md)** - Deep dive into agent configuration
- **[Completion](completion.md)** - Working with completion models
- **[Embeddings](embeddings.md)** - Using embedding models
- **[Providers](../providers/openai.md)** - Configure different LLM providers