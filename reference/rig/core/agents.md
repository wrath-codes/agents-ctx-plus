# Agents

## Overview

Agents are the primary abstraction in Rig for interacting with LLMs. They encapsulate model configuration, context management, and provide a high-level interface for completions.

## Creating Agents

### Basic Agent

```rust
use rig::{completion::Prompt, providers::openai};

let client = openai::Client::from_env();

let agent = client
    .agent("gpt-4")
    .preamble("You are a helpful assistant.")
    .build();

let response = agent.prompt("Hello!").await?;
```

### Agent Builder Pattern

```rust
let agent = client
    .agent("gpt-4")
    // System message
    .preamble("You are an expert programmer.")
    // Additional context
    .context(&Context::new("You specialize in Rust."))
    .context(&Context::new("You value safety and performance."))
    // Model parameters
    .temperature(0.7)
    .max_tokens(2000)
    .top_p(0.9)
    .frequency_penalty(0.5)
    .presence_penalty(0.5)
    // Build
    .build();
```

## Agent Configuration

### Preamble (System Message)

The preamble defines the agent's behavior and personality:

```rust
let agent = client
    .agent("gpt-4")
    .preamble(r#"
You are a helpful coding assistant.

Guidelines:
- Write clean, idiomatic code
- Explain your reasoning
- Ask clarifying questions when needed
"#)
    .build();
```

### Context

Add context to provide additional information:

```rust
use rig::completion::Context;

let context = Context::new(r#"
Project: MyApp
Framework: Axum
Database: PostgreSQL
"#);

let agent = client
    .agent("gpt-4")
    .context(&context)
    .build();
```

### Model Parameters

Control the generation behavior:

```rust
let agent = client
    .agent("gpt-4")
    .temperature(0.7)        // Creativity (0.0 - 2.0)
    .max_tokens(1000)        // Response length
    .top_p(0.9)             // Nucleus sampling
    .frequency_penalty(0.5)  // Reduce repetition
    .presence_penalty(0.5)   // Encourage new topics
    .build();
```

## Prompting

### Basic Prompts

```rust
use rig::completion::Prompt;

// Simple prompt
let response = agent.prompt("What is Rust?").await?;

// Multi-line prompt
let response = agent.prompt(r#"
Explain the concept of ownership in Rust.
Include examples.
"#).await?;
```

### Structured Prompts

```rust
let response = agent.prompt(r#"
Task: Write a function to calculate fibonacci numbers

Requirements:
- Use recursion
- Handle edge cases
- Include documentation

Provide only the code.
"#).await?;
```

### Dynamic Prompts

```rust
fn ask_about_topic(agent: &Agent, topic: &str) -> Result<String, PromptError> {
    let prompt = format!(
        "Explain {} in simple terms. Keep it under 100 words.",
        topic
    );
    
    agent.prompt(&prompt).await
}

let response = ask_about_topic(&agent, "async/await").await?;
```

## Streaming Responses

### Basic Streaming

```rust
use rig::completion::StreamingPrompt;

let mut stream = agent.stream_prompt("Tell me a story").await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(text) => print!("{}", text),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Collecting Stream

```rust
let mut stream = agent.stream_prompt("Generate code").await?;
let mut full_response = String::new();

while let Some(chunk) = stream.next().await {
    let text = chunk?;
    full_response.push_str(&text);
}

println!("Complete response: {}", full_response);
```

## Multi-Turn Conversations

### Maintaining Context

```rust
use rig::completion::Conversation;

let mut conversation = Conversation::new(&agent);

// First turn
let response1 = conversation
    .send("My name is Alice")
    .await?;

// Second turn (remembers context)
let response2 = conversation
    .send("What's my name?")
    .await?;

// response2 will mention "Alice"
```

### Manual Context Management

```rust
let mut history = Vec::new();

// User message
history.push(Message::user("Hello"));

// Assistant response
let response = agent.chat(&history).await?;
history.push(Message::assistant(&response));

// Next user message
history.push(Message::user("How are you?"));
let response = agent.chat(&history).await?;
```

## Agent Types

### Specialized Agents

```rust
// Code Reviewer
let code_reviewer = client
    .agent("gpt-4")
    .preamble(r#"
You are a senior code reviewer.

Check for:
- Bugs and errors
- Performance issues
- Code style violations
- Security vulnerabilities

Provide actionable feedback.
"#)
    .build();

// Technical Writer
let tech_writer = client
    .agent("gpt-4")
    .preamble(r#"
You are a technical writer.

Create clear, concise documentation.
Use examples liberally.
Follow style guide.
"#)
    .build();
```

### Agent with Tools

```rust
use rig::tool::Tool;

// Define a tool
#[derive(Deserialize)]
struct SearchInput {
    query: String,
}

#[derive(Serialize)]
struct SearchOutput {
    results: Vec<String>,
}

let search_tool = Tool::new("search", |input: SearchInput| async move {
    // Perform search
    Ok(SearchOutput {
        results: vec!["Result 1".to_string()],
    })
});

// Create agent with tool
let agent = client
    .agent("gpt-4")
    .tool(search_tool)
    .build();
```

## Advanced Patterns

### Agent Pool

```rust
use std::sync::Arc;

struct AgentPool {
    agents: Vec<Arc<Agent>>,
    current: AtomicUsize,
}

impl AgentPool {
    fn new(client: &Client, count: usize) -> Self {
        let agents: Vec<_> = (0..count)
            .map(|_| Arc::new(client.agent("gpt-4").build()))
            .collect();
        
        Self {
            agents,
            current: AtomicUsize::new(0),
        }
    }
    
    fn get_agent(&self) -> Arc<Agent> {
        let idx = self.current.fetch_add(1, Ordering::SeqCst) % self.agents.len();
        self.agents[idx].clone()
    }
}
```

### Agent with Memory

```rust
struct AgentWithMemory {
    agent: Agent,
    memory: Vec<String>,
}

impl AgentWithMemory {
    async fn prompt(&mut self, message: &str) -> Result<String, PromptError> {
        // Add context from memory
        let context = self.memory.join("\n");
        let full_prompt = format!("Context:\n{}\n\nUser: {}", context, message);
        
        let response = self.agent.prompt(&full_prompt).await?;
        
        // Store in memory
        self.memory.push(format!("User: {}\nAssistant: {}", message, response));
        
        Ok(response)
    }
}
```

## Best Practices

### 1. Clear Preambles

```rust
// Good: Specific and detailed
let agent = client
    .agent("gpt-4")
    .preamble(r#"
You are a Rust expert.

When writing code:
- Use idiomatic Rust patterns
- Prefer Result over panic
- Document public APIs
- Handle all error cases
"#)
    .build();
```

### 2. Appropriate Temperature

```rust
// Creative tasks: higher temperature
let creative_agent = client
    .agent("gpt-4")
    .temperature(0.9)
    .build();

// Factual tasks: lower temperature
let factual_agent = client
    .agent("gpt-4")
    .temperature(0.2)
    .build();
```

### 3. Error Handling

```rust
match agent.prompt("Generate code").await {
    Ok(response) => {
        // Validate response
        if response.is_empty() {
            println!("Warning: Empty response");
        }
        response
    }
    Err(e) => {
        eprintln!("Agent error: {}", e);
        // Fallback or retry
        "Default response".to_string()
    }
}
```

### 4. Rate Limiting

```rust
use tokio::time::{sleep, Duration};

async fn prompt_with_backoff(agent: &Agent, message: &str) -> Result<String, PromptError> {
    let mut retries = 0;
    
    loop {
        match agent.prompt(message).await {
            Ok(response) => return Ok(response),
            Err(e) if retries < 3 => {
                retries += 1;
                sleep(Duration::from_secs(2u64.pow(retries))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Next Steps

- **[Completion Models](completion.md)** - Direct model access
- **[Tools](tools.md)** - Adding tool calling
- **[Pipelines](pipelines.md)** - Building agent workflows