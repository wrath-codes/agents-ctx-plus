# Getting Started with OpenCode Rust SDK

This guide will help you get up and running with the OpenCode Rust SDK in just a few minutes.

## Prerequisites

Before you begin, ensure you have:

1. **Rust installed** (version 1.70 or later)
   ```bash
   rustc --version
   ```

2. **OpenCode server running**
   The SDK connects to a running OpenCode instance. By default, it expects OpenCode to be available at `http://127.0.0.1:4096`.

   To start OpenCode:
   ```bash
   # Install OpenCode if you haven't already
   npm install -g opencode
   
   # Or use npx
   npx opencode
   ```

## Installation

### Add to Your Project

Add `opencode_rs` to your `Cargo.toml`:

```toml
[dependencies]
opencode_rs = "0.1.2"
tokio = { version = "1", features = ["full"] }
```

Or use cargo add:

```bash
cargo add opencode_rs
cargo add tokio --features full
```

### Feature Flags

The SDK uses feature flags to control dependencies:

- **`http`** (default) - Enables HTTP client functionality using reqwest
- **`sse`** (default) - Enables SSE streaming support

To customize features:

```toml
[dependencies]
opencode_rs = { version = "0.1.2", default-features = false, features = ["http", "sse"] }
```

## Your First Program

Create a simple program that creates a session and sends a text prompt:

```rust
// src/main.rs
use opencode_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging (optional but recommended)
    tracing_subscriber::fmt::init();

    // Create a client with default configuration
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .directory(".")  // Current directory
        .timeout_secs(300)  // 5 minute timeout
        .build()?;

    println!("Client created successfully!");

    // Create a session and send a simple text prompt
    let session = client.run_simple_text(
        "Write a Hello World program in Rust"
    ).await?;

    println!("Session created with ID: {}", session.id);
    println!("Status: {:?}", session.status);

    Ok(())
}
```

Add tracing-subscriber to your dependencies:

```toml
[dependencies]
opencode_rs = "0.1.2"
tokio = { version = "1", features = ["full"] }
tracing-subscriber = "0.3"
```

Run the program:

```bash
cargo run
```

## Understanding the Basics

### 1. Client Creation

The `Client` is your main interface to OpenCode. Use `Client::builder()` to configure:

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")  // OpenCode server URL
    .directory("/path/to/project")       // Working directory
    .timeout_secs(300)                   // Request timeout
    .build()?;
```

### 2. Sessions

Sessions are conversations with OpenCode. Each session has:
- A unique ID
- A status (active, completed, error, etc.)
- Associated messages
- Configuration settings

### 3. Messages

Messages are the content exchanged with OpenCode:

```rust
// Send a text prompt
let session = client.run_simple_text("Your prompt here").await?;

// Or use the messages API for more control
let messages_api = client.messages();
let prompt_request = PromptRequest {
    session_id: session.id.clone(),
    content: vec![PromptPart::Text {
        text: "Your detailed prompt".to_string(),
    }],
    ephemeral: Some(false),
};
let message = messages_api.create_prompt(prompt_request).await?;
```

### 4. Event Streaming

OpenCode sends real-time updates via SSE. Subscribe to events:

```rust
// Subscribe to all events for the directory
let mut subscription = client.subscribe().await?;

// Process events as they arrive
while let Some(event) = subscription.recv().await {
    match event {
        Ok(event) => {
            println!("Received event: {:?}", event);
        }
        Err(e) => {
            eprintln!("Error receiving event: {}", e);
        }
    }
}
```

## Next Steps

Now that you have the basics working:

1. **Explore the API**: Learn about [sessions](../core-concepts/sessions.md), [messages](../core-concepts/messages-parts.md), and [events](../core-concepts/event-system.md)
2. **Try Examples**: Check out the [examples directory](../examples/basic-usage.md)
3. **Configure Advanced Options**: Learn about [client configuration](../configuration/client-config.md)
4. **Handle Errors**: Understand [error handling](../api-reference/error-handling.md)

## Troubleshooting

### Connection Refused

If you get a connection error, ensure OpenCode is running:

```
Error: error sending request for url (http://127.0.0.1:4096/...)
```

**Solution**: Start OpenCode in your terminal or check the URL/port.

### Timeout Errors

If requests timeout, increase the timeout:

```rust
let client = Client::builder()
    .timeout_secs(600)  // 10 minutes
    .build()?;
```

### Directory Not Found

Ensure the directory path exists and is accessible:

```rust
let client = Client::builder()
    .directory("/absolute/path/to/project")
    .build()?;
```

## Resources

- [API Documentation](https://docs.rs/opencode_rs)
- [OpenCode Documentation](https://opencode.ai)
- [GitHub Repository](https://github.com/allisoneer/agentic_auxilary)