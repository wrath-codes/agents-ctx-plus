# Basic Usage Examples

Practical examples demonstrating common OpenCode SDK patterns.

## Example 1: Simple Session

Create a session and send a prompt:

```rust
use opencode_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .directory(".")
        .build()?;
    
    let session = client.run_simple_text(
        "Write a Rust function to reverse a string"
    ).await?;
    
    println!("Session: {}", session.id);
    Ok(())
}
```

## Example 2: Session with Events

Listen to events while processing:

```rust
use opencode_rs::{Client, Result};
use opencode_rs::types::event::Event;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().build()?;
    
    let session = client.run_simple_text(
        "Refactor main function"
    ).await?;
    
    let mut subscription = client.subscribe_session(&session.id).await?;
    
    while let Some(event) = subscription.recv().await {
        match event? {
            Event::MessageUpdated { props } => {
                if let Some(text) = &props.text {
                    print!("{}", text);
                }
            }
            Event::SessionIdle { .. } => {
                println!("Done");
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Example 3: List and Manage Sessions

```rust
use opencode_rs::types::session::CreateSessionRequest;

let sessions = client.sessions().list().await?;

for session in sessions {
    println!("{}: {:?}", session.id, session.status);
}

let new_session = client.sessions().create(
    CreateSessionRequest {
        description: Some("My task".to_string()),
        ..Default::default()
    }
).await?;
```