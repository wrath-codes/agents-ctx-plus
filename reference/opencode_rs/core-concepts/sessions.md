# Sessions

Sessions are the core concept in OpenCode. A session represents a conversation or task context where you interact with the AI coding agent.

## What is a Session?

A session in OpenCode is:
- A unique workspace for a coding task
- A container for messages exchanged with the AI
- A persistent context across multiple interactions
- Associated with a specific directory/project

## Session Lifecycle

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│ Created  │────▶│ Active   │────▶│Completed │────▶│ Archived │
│          │     │          │     │          │     │          │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
      │                │                │
      │                │                │
      ▼                ▼                ▼
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Error   │     │  Idle    │     │ Reverted │
└──────────┘     └──────────┘     └──────────┘
```

### States

| State | Description |
|-------|-------------|
| `created` | Session initialized, ready for messages |
| `active` | AI is processing or waiting for input |
| `idle` | Session paused, waiting for user action |
| `completed` | Task finished successfully |
| `error` | An error occurred during processing |
| `reverted` | Changes have been reverted |

## Creating Sessions

### Simple Creation

```rust
use opencode_rs::{Client, Result};
use opencode_rs::types::session::CreateSessionRequest;

let client = Client::builder().build()?;

// Quick method: create and run
let session = client.run_simple_text("Write a Rust function").await?;
```

### Advanced Creation

```rust
use opencode_rs::types::session::{CreateSessionRequest, Provider};

let request = CreateSessionRequest {
    description: Some("Refactor authentication".to_string()),
    initial_prompt: Some("Review and refactor the auth module".to_string()),
    provider: Some(Provider::Claude),
    model: Some("claude-3-opus-20240229".to_string()),
    agent: Some("default".to_string()),
    tools: Some(vec!["file".to_string(), "shell".to_string()]),
    ephemeral: Some(false),
};

let session = client.sessions().create(request).await?;
```

## Session Properties

```rust
pub struct Session {
    pub id: String,                    // Unique identifier
    pub status: SessionStatus,         // Current state
    pub description: Option<String>,   // User-defined description
    pub created_at: DateTime<Utc>,     // Creation timestamp
    pub updated_at: DateTime<Utc>,     // Last update timestamp
    pub provider: Option<Provider>,    // AI provider
    pub model: Option<String>,         // Model name
    pub agent: Option<String>,         // Agent configuration
    pub directory: String,             // Working directory
    pub todo_list: Vec<TodoItem>,      // Task list
}
```

## Managing Sessions

### Listing Sessions

```rust
// List all sessions for the directory
let sessions = client.sessions().list().await?;

for session in sessions {
    println!("{}: {} ({:?})", 
        session.id, 
        session.description.as_deref().unwrap_or("Untitled"),
        session.status
    );
}
```

### Getting Session Details

```rust
let session = client.sessions().get(&session_id).await?;
println!("Session status: {:?}", session.status);
```

### Updating Sessions

```rust
use opencode_rs::types::session::UpdateSessionRequest;

let update = UpdateSessionRequest {
    description: Some("Updated description".to_string()),
    status: Some(SessionStatus::Active),
};

let updated = client.sessions().update(&session_id, update).await?;
```

### Deleting Sessions

```rust
client.sessions().delete(&session_id).await?;
println!("Session deleted");
```

## Session Operations

### Reverting Sessions

Revert a session to undo changes:

```rust
use opencode_rs::types::session::RevertRequest;

let request = RevertRequest {
    message: Some("Reverting due to errors".to_string()),
};

let result = client.sessions().revert(&session_id, request).await?;
println!("Reverted files: {:?}", result.reverted_files);
```

### Summarizing Sessions

Generate a summary of session activity:

```rust
use opencode_rs::types::session::SummarizeRequest;

let request = SummarizeRequest {
    include_file_changes: Some(true),
    include_messages: Some(true),
};

let summary = client.sessions().summarize(&session_id, request).await?;
println!("Summary: {}", summary.summary);
println!("Files changed: {}", summary.file_changes.len());
```

### Getting Session Diff

View changes made during a session:

```rust
let diff = client.sessions().diff(&session_id).await?;

for file_diff in &diff.file_diffs {
    println!("File: {}", file_diff.path);
    println!("Added: {}, Removed: {}", 
        file_diff.lines_added, 
        file_diff.lines_removed
    );
}
```

## Session Context

### Directory Association

Every session is associated with a directory:

```rust
// Set at client level
let client = Client::builder()
    .directory("/path/to/project")
    .build()?;

// All sessions created by this client use this directory
let session = client.run_simple_text("Analyze code").await?;
assert_eq!(session.directory, "/path/to/project");
```

### Session-Specific Tools

Configure which tools are available to the AI:

```rust
let request = CreateSessionRequest {
    tools: Some(vec![
        "file".to_string(),      // File operations
        "shell".to_string(),     // Shell commands
        "grep".to_string(),      // Text search
        "edit".to_string(),      // Code editing
    ]),
    ..Default::default()
};

let session = client.sessions().create(request).await?;
```

## Session Messages

Sessions contain messages exchanged with the AI:

```rust
// Get all messages in a session
let messages = client.messages().list(&session.id).await?;

for message in messages {
    println!("Role: {:?}", message.role);
    for part in &message.parts {
        match part {
            Part::Text { text } => println!("Text: {}", text),
            Part::File { file } => println!("File: {}", file.path),
            _ => {}
        }
    }
}
```

## Session Events

Listen to session-specific events:

```rust
// Subscribe to events for a specific session
let mut subscription = client.subscribe_session(&session.id).await?;

while let Some(event) = subscription.recv().await {
    match event? {
        Event::SessionCreated { props } => {
            println!("Session created: {}", props.session_id);
        }
        Event::MessageUpdated { props } => {
            println!("Message updated: {}", props.message_id);
        }
        Event::SessionIdle { props } => {
            println!("Session idle: {}", props.session_id);
        }
        _ => {}
    }
}
```

## Best Practices

### 1. Session Naming

Provide descriptive descriptions:

```rust
let request = CreateSessionRequest {
    description: Some("Refactor auth middleware for JWT support".to_string()),
    ..Default::default()
};
```

### 2. Session Cleanup

Delete ephemeral sessions when done:

```rust
let request = CreateSessionRequest {
    ephemeral: Some(true),  // Will be auto-deleted
    ..Default::default()
};
```

### 3. Error Handling

Always handle session errors:

```rust
match client.sessions().create(request).await {
    Ok(session) => {
        println!("Created: {}", session.id);
    }
    Err(OpencodeError::Api { code, message }) => {
        eprintln!("API Error {}: {}", code, message);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### 4. Resource Management

Don't create too many concurrent sessions:

```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(5)); // Max 5 concurrent

for task in tasks {
    let permit = semaphore.clone().acquire_owned().await?;
    tokio::spawn(async move {
        let _permit = permit; // Hold permit until done
        client.run_simple_text(task).await
    });
}
```

## Common Patterns

### Pattern 1: Quick Task

For simple, one-off tasks:

```rust
let result = client.run_simple_text("Review this code").await?;
// Result returns immediately; listen for events for actual response
```

### Pattern 2: Long-Running Session

For complex, multi-step tasks:

```rust
let session = client.sessions().create(CreateSessionRequest {
    description: Some("Implement feature X".to_string()),
    ..Default::default()
}).await?;

// Multiple interactions
let msg1 = client.messages().create_prompt(PromptRequest {
    session_id: session.id.clone(),
    content: vec![PromptPart::Text { 
        text: "Step 1: Analyze requirements".to_string() 
    }],
    ..Default::default()
}).await?;

// ... more steps
```

### Pattern 3: Session with File Context

Include files in the session:

```rust
let request = CreateSessionRequest {
    initial_prompt: Some("Review the attached files".to_string()),
    ..Default::default()
};

let session = client.sessions().create(request).await?;

// Add file references
let file_part = Part::File {
    file: FilePart {
        path: "src/main.rs".to_string(),
        content: Some(file_content),
    },
};
```

## Session Timeouts

Sessions have configurable timeouts:

```rust
let client = Client::builder()
    .timeout_secs(600)  // 10 minutes
    .build()?;
```

Note: This is the HTTP request timeout. Session duration on the server may have separate limits.

## Monitoring Sessions

Track session progress:

```rust
let mut subscription = client.subscribe_session(&session.id).await?;

let mut completed = false;
while let Some(event) = subscription.recv().await {
    match event? {
        Event::ToolStateCompleted { props } => {
            println!("Tool completed: {}", props.tool_id);
        }
        Event::SessionIdle { props } => {
            println!("Session idle");
            completed = true;
            break;
        }
        Event::SessionError { props } => {
            eprintln!("Session error: {:?}", props.error);
            break;
        }
        _ => {}
    }
}
```

## Related Topics

- [Messages and Parts](messages-parts.md) - Working with session content
- [Event System](event-system.md) - Real-time session events
- [Client Lifecycle](client-lifecycle.md) - Managing client connections