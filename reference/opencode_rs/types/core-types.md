# Core Types

The SDK provides comprehensive type definitions for all OpenCode data structures.

## Session Types

### Session

Represents a coding session.

```rust
pub struct Session {
    pub id: String,
    pub status: SessionStatus,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub directory: String,
    pub todo_list: Vec<TodoItem>,
}
```

### SessionStatus

```rust
pub enum SessionStatus {
    Created,
    Active,
    Idle,
    Completed,
    Error,
    Reverted,
}
```

### CreateSessionRequest

```rust
pub struct CreateSessionRequest {
    pub description: Option<String>,
    pub initial_prompt: Option<String>,
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub tools: Option<Vec<String>>,
    pub ephemeral: Option<bool>,
}
```

## Message Types

### Message

Represents a message in a session.

```rust
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: Role,
    pub parts: Vec<Part>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Role

```rust
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}
```

### Part

Content parts within messages.

```rust
pub enum Part {
    Text { text: String },
    File { file: FilePart },
    Image { image: ImagePart },
    ToolCall { tool_call: ToolCallPart },
    ToolResult { tool_result: ToolResultPart },
    // ... other variants
}
```

## Event Types

### Event

SSE events from OpenCode (40+ variants).

```rust
pub enum Event {
    // Session events
    SessionCreated { props: SessionInfoProps },
    SessionUpdated { props: SessionInfoProps },
    SessionDeleted { props: SessionInfoProps },
    SessionIdle { props: SessionIdleProps },
    SessionError { props: SessionErrorProps },
    
    // Message events
    MessageUpdated { props: MessageUpdatedProps },
    MessageRemoved { props: MessageRemovedProps },
    MessagePartAdded { props: MessagePartEventProps },
    MessagePartUpdated { props: MessagePartEventProps },
    
    // Tool events
    ToolStatePending { props: ToolStatePending },
    ToolStateRunning { props: ToolStateRunning },
    ToolStateCompleted { props: ToolStateCompleted },
    ToolStateError { props: ToolStateError },
    
    // Permission events
    PermissionAsked { props: PermissionAskedProps },
    PermissionReplied { props: PermissionRepliedProps },
    
    // ... many more
}
```

## Tool Types

### Tool

```rust
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
}
```

### Agent

```rust
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
}
```

## Provider Types

### Provider

AI provider configuration.

```rust
pub enum Provider {
    Claude,
    OpenAI,
    Gemini,
    Ollama,
    Custom(String),
}
```

## Error Types

### OpencodeError

```rust
pub enum OpencodeError {
    Connection(String),
    Api { code: u16, message: String },
    Serialization(String),
    InvalidUrl(String),
    Timeout,
    Other(String),
}
```

## Common Patterns

### Working with Option Fields

```rust
// Safe access to optional fields
if let Some(description) = &session.description {
    println!("Session: {}", description);
}

// Or use default values
let desc = session.description.as_deref().unwrap_or("Untitled");
```

### DateTime Handling

```rust
use chrono::{DateTime, Utc};

let created: DateTime<Utc> = session.created_at;
println!("Created: {}", created.format("%Y-%m-%d %H:%M:%S"));
```

### Enum Pattern Matching

```rust
match event {
    Event::MessageUpdated { props } => {
        // Handle message update
    }
    Event::ToolStateCompleted { props } => {
        // Handle tool completion
    }
    _ => {}
}
```