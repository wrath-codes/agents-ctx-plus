# Basic Agent Example

## Overview

This example demonstrates building a simple conversational agent with Rig.

## Code

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create OpenAI client
    let client = openai::Client::from_env();
    
    // Create a helpful assistant agent
    let agent = client
        .agent("gpt-4")
        .preamble("You are a helpful assistant.")
        .temperature(0.7)
        .build();
    
    println!("Assistant: Hello! How can I help you today?");
    
    // Interactive loop
    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") {
            break;
        }
        
        // Get response
        match agent.prompt(input).await {
            Ok(response) => println!("Assistant: {}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    println!("Goodbye!");
    Ok(())
}
```

## Running the Example

```bash
export OPENAI_API_KEY="sk-..."
cargo run --example basic_agent
```

## Sample Interaction

```
Assistant: Hello! How can I help you today?
> What is Rust?
Assistant: Rust is a systems programming language that focuses on safety, speed, and concurrency...
> How do I install it?
Assistant: You can install Rust using rustup, the official installer...
> exit
Goodbye!
```

## Key Concepts

1. **Client**: Connects to OpenAI API
2. **Agent**: Configured with system message and parameters
3. **Prompt**: Sends user input to the agent
4. **Error Handling**: Gracefully handles API errors

## Variations

### With Context

```rust
let agent = client
    .agent("gpt-4")
    .preamble("You are a Rust expert.")
    .context(&Context::new("Rust is a systems language focused on safety."))
    .build();
```

### With Streaming

```rust
let mut stream = agent.stream_prompt(&input).await?;
print!("Assistant: ");
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?);
}
println!();
```