# SSE Streaming

Server-Sent Events (SSE) provide real-time updates from OpenCode. The SDK offers comprehensive SSE support with automatic reconnection and event filtering.

## Overview

OpenCode streams events via SSE for:
- Session state changes
- Message updates
- Tool execution progress
- File operations
- Permission requests

```
┌──────────────┐     SSE      ┌──────────────┐
│  OpenCode    │◀────────────▶│  SDK Client  │
│   Server     │   Events     │              │
└──────────────┘              └──────────────┘
                                     │
                                     ▼
                              ┌──────────────┐
                              │  Your App    │
                              │  (handlers)  │
                              └──────────────┘
```

## SseSubscriber

The `SseSubscriber` manages SSE connections:

```rust
use opencode_rs::sse::{SseSubscriber, SseOptions};

let subscriber = client.sse_subscriber();
```

## Subscription Types

### Subscribe to Directory Events

Receive all events for the configured directory:

```rust
let subscription = client.subscribe().await?;

while let Some(event) = subscription.recv().await {
    match event {
        Ok(event) => handle_event(event).await,
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Subscribe to Session Events

Filter events for a specific session:

```rust
let subscription = client.subscribe_session(&session_id).await?;

while let Some(event) = subscription.recv().await {
    match event? {
        Event::MessageUpdated { props } => {
            println!("Message: {}", props.message_id);
        }
        Event::SessionIdle { .. } => {
            println!("Session complete");
            break;
        }
        _ => {}
    }
}
```

**Note:** Filtering happens client-side after receiving events from server.

### Subscribe to Global Events

Receive events from all directories:

```rust
let subscription = client.subscribe_global().await?;

while let Some(event) = subscription.recv().await {
    if let Ok(Event::GlobalEventEnvelope { directory, event }) = event {
        println!("[{}] {:?}", directory, event);
    }
}
```

## SseSubscription

The `SseSubscription` provides the event stream:

```rust
pub struct SseSubscription {
    receiver: mpsc::Receiver<Result<Event>>,
    cancel: CancellationToken,
}

impl SseSubscription {
    pub async fn recv(&mut self) -> Option<Result<Event>>;
    pub fn close(self);
}
```

### Receiving Events

```rust
let mut subscription = client.subscribe().await?;

loop {
    match subscription.recv().await {
        Some(Ok(event)) => {
            println!("Received: {:?}", event);
        }
        Some(Err(e)) => {
            eprintln!("Stream error: {}", e);
            break;
        }
        None => {
            println!("Stream closed");
            break;
        }
    }
}
```

### Closing Subscription

```rust
let subscription = client.subscribe().await?;

// Process some events
for _ in 0..10 {
    if let Some(event) = subscription.recv().await {
        println!("{:?}", event?);
    }
}

// Close the subscription
subscription.close();
```

## SseOptions

Configure subscription behavior:

```rust
use opencode_rs::sse::SseOptions;

let opts = SseOptions {
    reconnect: true,           // Auto-reconnect on disconnect
    max_retries: Some(5),      // Max reconnection attempts
    retry_delay_ms: 1000,      // Initial retry delay
    retry_backoff: 2.0,        // Backoff multiplier
};

let subscriber = client.sse_subscriber();
let subscription = subscriber.subscribe(opts).await?;
```

### Default Options

```rust
let opts = SseOptions::default();
// reconnect: true
// max_retries: None (unlimited)
// retry_delay_ms: 1000
// retry_backoff: 2.0
```

## Event Types

The SDK supports 40+ event types:

### Session Events

```rust
Event::SessionCreated { props }           // New session
Event::SessionUpdated { props }           // Session modified
Event::SessionDeleted { props }           // Session removed
Event::SessionIdle { props }              // Session paused
Event::SessionError { props }             // Error occurred
```

### Message Events

```rust
Event::MessageUpdated { props }           // New/updated message
Event::MessageRemoved { props }           // Message deleted
Event::MessagePartAdded { props }         // Content part added
Event::MessagePartUpdated { props }       // Content part changed
```

### Tool Events

```rust
Event::ToolStatePending { props }         // Tool queued
Event::ToolStateRunning { props }         // Tool executing
Event::ToolStateCompleted { props }       // Tool finished
Event::ToolStateError { props }           // Tool failed
```

### Permission Events

```rust
Event::PermissionAsked { props }          // Permission requested
Event::PermissionReplied { props }        // Permission responded
```

### File Events

```rust
Event::FileCreated { props }              // File created
Event::FileUpdated { props }              // File modified
Event::FileDeleted { props }              // File removed
```

## Handling Events

### Pattern Matching

```rust
while let Some(event) = subscription.recv().await {
    match event? {
        // Session lifecycle
        Event::SessionCreated { props } => {
            println!("Session started: {}", props.session_id);
        }
        Event::SessionIdle { props } => {
            println!("Session idle: {}", props.session_id);
        }
        
        // Message updates
        Event::MessageUpdated { props } => {
            if let Some(content) = &props.content {
                println!("New content: {}", content);
            }
        }
        
        // Tool execution
        Event::ToolStateRunning { props } => {
            println!("Tool running: {}", props.tool_id);
        }
        Event::ToolStateCompleted { props } => {
            println!("Tool completed: {}", props.tool_id);
            if let Some(output) = &props.output {
                println!("Output: {}", output);
            }
        }
        
        // Permission handling
        Event::PermissionAsked { props } => {
            println!("Permission needed: {}", props.permission_id);
            // Auto-grant or prompt user
            client.permissions().grant(&props.permission_id).await?;
        }
        
        _ => {}
    }
}
```

### Filtering Events

```rust
let mut subscription = client.subscribe().await?;

while let Some(event) = subscription.recv().await {
    let event = event?;
    
    // Only process specific event types
    if matches!(event, 
        Event::MessageUpdated { .. } | 
        Event::SessionIdle { .. }
    ) {
        println!("{:?}", event);
    }
}
```

## Reconnection

The SDK automatically reconnects on disconnect:

```rust
let opts = SseOptions {
    reconnect: true,
    max_retries: Some(10),
    retry_delay_ms: 1000,
    retry_backoff: 2.0,  // 1s, 2s, 4s, 8s...
};

let subscription = subscriber.subscribe(opts).await?;
```

### Last Event ID

The SDK tracks the last event ID for resume:

```rust
// If connection drops, reconnects with Last-Event-ID header
// Server resumes from where it left off (if supported)
```

## Complete Example

```rust
use opencode_rs::{Client, Result};
use opencode_rs::types::event::Event;
use opencode_rs::types::message::PromptRequest;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().build()?;
    
    // Create session
    let session = client.run_simple_text(
        "Write a Rust function to calculate fibonacci"
    ).await?;
    
    println!("Session: {}", session.id);
    
    // Subscribe to session events
    let mut subscription = client.subscribe_session(&session.id).await?;
    
    while let Some(event) = subscription.recv().await {
        match event? {
            Event::MessageUpdated { props } => {
                if let Some(text) = &props.text {
                    print!("{}", text);
                }
            }
            Event::ToolStateCompleted { props } => {
                println!("\n[Tool completed: {}]", props.tool_id);
            }
            Event::SessionIdle { .. } => {
                println!("\n[Session complete]");
                break;
            }
            Event::SessionError { props } => {
                eprintln!("\n[Error: {:?}]", props.error);
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Error Handling

### Stream Errors

```rust
while let Some(result) = subscription.recv().await {
    match result {
        Ok(event) => handle_event(event).await?,
        Err(OpencodeError::Connection(e)) => {
            eprintln!("Connection lost: {}", e);
            // Reconnect if needed
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            break;
        }
    }
}
```

### Timeout Handling

```rust
use tokio::time::{timeout, Duration};

let mut subscription = client.subscribe().await?;

loop {
    match timeout(Duration::from_secs(30), subscription.recv()).await {
        Ok(Some(event)) => println!("{:?}", event?),
        Ok(None) => break,  // Stream closed
        Err(_) => {
            println!("Timeout - no events for 30s");
            break;
        }
    }
}
```

## Performance Tips

1. **Reuse Subscriptions**: Create one subscription and process multiple events
2. **Filter Early**: Use `subscribe_session` to reduce network traffic
3. **Handle Backpressure**: Process events quickly or buffer them
4. **Close When Done**: Always close subscriptions to free resources

## Debugging

Enable tracing to see SSE activity:

```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

Log output:
```
DEBUG opencode_rs::sse: Connecting to SSE endpoint: http://127.0.0.1:4096/event
DEBUG opencode_rs::sse: Connected successfully
DEBUG opencode_rs::sse: Received event: message.updated
DEBUG opencode_rs::sse: Received event: tool.state.completed
```