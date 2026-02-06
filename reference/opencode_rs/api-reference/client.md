# Client API Reference

The `Client` is the primary interface for interacting with OpenCode. It provides high-level, ergonomic methods for all OpenCode operations.

## Client Overview

```rust
use opencode_rs::{Client, ClientBuilder, Result};

// Create a client
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .directory("/path/to/project")
    .timeout_secs(300)
    .build()?;
```

## ClientBuilder

### Creating a Builder

```rust
let builder = Client::builder();
// or
let builder = ClientBuilder::new();
```

### Configuration Methods

#### base_url

Set the OpenCode server URL.

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .build()?;
```

**Parameters:**
- `url`: String or &str - The base URL for the OpenCode server

**Default:** `"http://127.0.0.1:4096"`

#### directory

Set the working directory for all operations.

```rust
let client = Client::builder()
    .directory("/home/user/my-project")
    .build()?;
```

**Parameters:**
- `dir`: String or &str - Absolute or relative path

**Effect:** Sets the `x-opencode-directory` header on all requests

#### timeout_secs

Set the HTTP request timeout.

```rust
let client = Client::builder()
    .timeout_secs(600)  // 10 minutes
    .build()?;
```

**Parameters:**
- `secs`: u64 - Timeout in seconds

**Default:** `300` (5 minutes)

### Building

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .directory(".")
    .timeout_secs(300)
    .build()?;
```

**Returns:** `Result<Client>`

**Errors:**
- Invalid URL format
- Failed to create HTTP client
- Missing required features

## Client Methods

### Convenience Methods

#### run_simple_text

Create a session and send a text prompt in one call.

```rust
let session = client
    .run_simple_text("Write a Rust function to parse JSON")
    .await?;
```

**Parameters:**
- `text`: impl Into<String> - The prompt text

**Returns:** `Result<Session>`

**Note:** This returns immediately after sending. The AI response arrives asynchronously via SSE events.

**Example:**
```rust
use opencode_rs::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().build()?;
    
    // Create session and send prompt
    let session = client.run_simple_text(
        "Explain borrow checking in Rust"
    ).await?;
    
    println!("Session ID: {}", session.id);
    
    // Subscribe to receive the response
    let mut events = client.subscribe_session(&session.id).await?;
    while let Some(event) = events.recv().await {
        println!("Event: {:?}", event?);
    }
    
    Ok(())
}
```

### API Accessors

The client provides typed accessors to individual API modules:

#### sessions

Access the Sessions API.

```rust
let sessions_api = client.sessions();

// Create session
let session = sessions_api.create(request).await?;

// List sessions
let sessions = sessions_api.list().await?;

// Get session
let session = sessions_api.get(&id).await?;

// Update session
let updated = sessions_api.update(&id, request).await?;

// Delete session
sessions_api.delete(&id).await?;
```

**Returns:** `SessionsApi`

#### messages

Access the Messages API.

```rust
let messages_api = client.messages();

// Send prompt
let message = messages_api.create_prompt(request).await?;

// Execute command
let result = messages_api.create_command(request).await?;

// List messages
let messages = messages_api.list(&session_id).await?;
```

**Returns:** `MessagesApi`

#### parts

Access the Parts API.

```rust
let parts_api = client.parts();

// Get message parts
let parts = parts_api.list(&session_id, &message_id).await?;
```

**Returns:** `PartsApi`

#### files

Access the Files API.

```rust
let files_api = client.files();

// Read file
let content = files_api.read(&path).await?;

// Write file
files_api.write(&path, &content).await?;

// Delete file
files_api.delete(&path).await?;
```

**Returns:** `FilesApi`

#### tools

Access the Tools API.

```rust
let tools_api = client.tools();

// List available tools
let tools = tools_api.list().await?;

// Get tool details
let tool = tools_api.get(&tool_id).await?;
```

**Returns:** `ToolsApi`

#### mcp

Access the MCP (Model Context Protocol) API.

```rust
let mcp_api = client.mcp();

// List MCP servers
let servers = mcp_api.list_servers().await?;

// Call MCP tool
let result = mcp_api.call_tool(&server_id, &tool_name, params).await?;
```

**Returns:** `McpApi`

#### providers

Access the Providers API.

```rust
let providers_api = client.providers();

// List available providers
let providers = providers_api.list().await?;

// Get provider details
let provider = providers_api.get(&provider_id).await?;
```

**Returns:** `ProvidersApi`

#### permissions

Access the Permissions API.

```rust
let permissions_api = client.permissions();

// List pending permissions
let pending = permissions_api.list_pending().await?;

// Grant permission
permissions_api.grant(&permission_id).await?;

// Deny permission
permissions_api.deny(&permission_id).await?;
```

**Returns:** `PermissionsApi`

#### config

Access the Config API.

```rust
let config_api = client.config();

// Get configuration
let config = config_api.get().await?;

// Update configuration
config_api.update(request).await?;
```

**Returns:** `ConfigApi`

#### project

Access the Project API.

```rust
let project_api = client.project();

// Get project info
let info = project_api.info().await?;
```

**Returns:** `ProjectApi`

#### worktree

Access the Worktree API.

```rust
let worktree_api = client.worktree();

// Get worktree status
let status = worktree_api.status().await?;
```

**Returns:** `WorktreeApi`

#### find

Access the Find API for searching.

```rust
let find_api = client.find();

// Find files
let results = find_api.files(&pattern).await?;

// Find symbols
let results = find_api.symbols(&query).await?;
```

**Returns:** `FindApi`

#### pty

Access the PTY (Pseudo-Terminal) API.

```rust
let pty_api = client.pty();

// Execute terminal command
let output = pty_api.execute(&command).await?;
```

**Returns:** `PtyApi`

#### misc

Access miscellaneous endpoints.

```rust
let misc_api = client.misc();

// Health check
let health = misc_api.health().await?;
```

**Returns:** `MiscApi`

### SSE Streaming

#### sse_subscriber

Get an SSE subscriber for streaming events.

```rust
let subscriber = client.sse_subscriber();
```

**Returns:** `SseSubscriber`

#### subscribe

Subscribe to all events for the configured directory.

```rust
let subscription = client.subscribe().await?;

while let Some(event) = subscription.recv().await {
    println!("Event: {:?}", event?);
}
```

**Returns:** `Result<SseSubscription>`

**Events:** All events for the client's directory

#### subscribe_session

Subscribe to events filtered by session ID.

```rust
let subscription = client.subscribe_session(&session_id).await?;

while let Some(event) = subscription.recv().await {
    match event? {
        Event::MessageUpdated { props } => {
            println!("Message updated: {}", props.message_id);
        }
        Event::SessionIdle { .. } => {
            println!("Session complete");
            break;
        }
        _ => {}
    }
}
```

**Parameters:**
- `session_id`: &str - The session ID to filter by

**Returns:** `Result<SseSubscription>`

**Note:** Events are filtered client-side after receiving from server

#### subscribe_global

Subscribe to global events across all directories.

```rust
let subscription = client.subscribe_global().await?;

while let Some(event) = subscription.recv().await {
    if let Ok(Event::GlobalEventEnvelope { directory, event }) = event {
        println!("Event from {}: {:?}", directory, event);
    }
}
```

**Returns:** `Result<SseSubscription>`

**Note:** Uses the `/global/event` endpoint

## Thread Safety

The `Client` is both `Send` and `Sync`, allowing it to be shared across threads:

```rust
use std::sync::Arc;

let client = Arc::new(Client::builder().build()?);

// Clone for multiple tasks
let client1 = client.clone();
let client2 = client.clone();

let task1 = tokio::spawn(async move {
    client1.sessions().list().await
});

let task2 = tokio::spawn(async move {
    client2.messages().create_prompt(request).await
});

let (result1, result2) = tokio::join!(task1, task2);
```

## Error Handling

All client methods return `Result<T, OpencodeError>`:

```rust
use opencode_rs::error::OpencodeError;

match client.sessions().create(request).await {
    Ok(session) => {
        println!("Created: {}", session.id);
    }
    Err(OpencodeError::Connection(e)) => {
        eprintln!("Connection failed: {}", e);
    }
    Err(OpencodeError::Api { code, message }) => {
        eprintln!("API error {}: {}", code, message);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Complete Example

```rust
use opencode_rs::{Client, Result};
use opencode_rs::types::session::CreateSessionRequest;
use opencode_rs::types::message::{PromptRequest, PromptPart};

#[tokio::main]
async fn main() -> Result<()> {
    // Create client
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .directory(".")
        .timeout_secs(300)
        .build()?;
    
    // Create session
    let session = client.sessions().create(CreateSessionRequest {
        description: Some("Analyze code".to_string()),
        ..Default::default()
    }).await?;
    
    println!("Session: {}", session.id);
    
    // Send prompt
    let message = client.messages().create_prompt(PromptRequest {
        session_id: session.id.clone(),
        content: vec![PromptPart::Text {
            text: "Review the main.rs file".to_string(),
        }],
        ephemeral: Some(false),
    }).await?;
    
    println!("Message sent: {}", message.id);
    
    // Subscribe to events
    let mut subscription = client.subscribe_session(&session.id).await?;
    
    while let Some(event) = subscription.recv().await {
        println!("Event: {:?}", event?);
    }
    
    Ok(())
}
```

## Advanced Usage

### Custom Configuration

```rust
let client = Client::builder()
    .base_url(std::env::var("OPENCODE_URL")?)
    .directory(std::env::var("OPENCODE_DIR")?)
    .timeout_secs(600)
    .build()?;
```

### Connection Pooling

The underlying `reqwest::Client` automatically pools connections:

```rust
// Create one client and reuse
let client = Client::builder().build()?;

// Multiple operations share connections
for i in 0..100 {
    let session = client.run_simple_text(format!("Task {}", i)).await?;
}
```

### Request Timeouts

Set appropriate timeouts for your use case:

```rust
// For quick operations
let quick_client = Client::builder()
    .timeout_secs(30)
    .build()?;

// For long-running tasks
let long_client = Client::builder()
    .timeout_secs(3600)  // 1 hour
    .build()?;
```