# Context and State Management

The Context system provides thread-safe state management across workflow tasks.

---

## What is Context?

Context is a thread-safe container for workflow state:

```rust
pub struct Context {
    data: Arc<DashMap<String, Value>>,        // Key-value storage
    chat_history: Arc<RwLock<ChatHistory>>,   // Conversation history
}
```

**Features:**
- Type-safe storage via serde
- Thread-safe (Send + Sync)
- Synchronous and async access
- Chat history management
- Automatic serialization

---

## Creating Context

### New Context

```rust
let context = Context::new();
```

### With Message Limit

```rust
// Limit chat history to 100 messages
let context = Context::with_max_chat_messages(100);
```

---

## Storing Data

### Basic Storage

```rust
// Store any serializable type
context.set("name", "Alice".to_string()).await;
context.set("count", 42).await;
context.set("active", true).await;
context.set("data", vec![1, 2, 3]).await;

// Custom structs
#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
}

let user = User { id: 1, name: "Bob".to_string() };
context.set("user", user).await;
```

### Synchronous Storage

```rust
// For use in edge conditions
context.set_sync("key", "value".to_string());
```

---

## Retrieving Data

### Async Access

```rust
// Must specify type
let name: Option<String> = context.get("name").await;
let count: Option<i32> = context.get("count").await;
let user: Option<User> = context.get("user").await;

// With default
let name = context.get("name").await.unwrap_or("Unknown".to_string());
```

### Synchronous Access

```rust
// For edge conditions
let name: Option<String> = context.get_sync("name");
let count: Option<i32> = context.get_sync("count");
```

---

## Data Types

### Supported Types

Any type implementing `Serialize` and `DeserializeOwned`:

- Primitives: `i32`, `f64`, `bool`, `String`
- Collections: `Vec<T>`, `HashMap<K, V>`
- Structs: Custom structs with derive macros
- Enums: With derive macros

### Custom Types

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ClaimDetails {
    claim_id: String,
    amount: f64,
    status: ClaimStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
}

// Store and retrieve
let details = ClaimDetails {
    claim_id: "CLM-001".to_string(),
    amount: 1000.0,
    status: ClaimStatus::Pending,
};

context.set("claim", details).await;

let claim: Option<ClaimDetails> = context.get("claim").await;
```

---

## Chat History

### Adding Messages

```rust
// User message
context.add_user_message("Hello, assistant!".to_string()).await;

// Assistant message
context.add_assistant_message("Hello! How can I help?".to_string()).await;

// System message
context.add_system_message("Session started".to_string()).await;
```

### Retrieving History

```rust
// Get all messages
let history = context.get_chat_history().await;

// Get recent messages
let recent = context.get_last_messages(5).await;

// Get all messages as SerializableMessage
let all = context.get_all_messages().await;

// Check length
let count = context.chat_history_len().await;
let is_empty = context.is_chat_history_empty().await;
```

### With Rig Integration

```rust
#[cfg(feature = "rig")]
{
    // Get messages in Rig format
    let rig_messages = context.get_rig_messages().await;
    
    // Get last N messages
    let recent = context.get_last_rig_messages(10).await;
    
    // Use with Rig agent
    let response = agent.chat(&user_input, rig_messages).await?;
}
```

### Managing History

```rust
// Clear history
context.clear_chat_history().await;

// Set max messages (on creation)
let context = Context::with_max_chat_messages(50);
```

---

## Serialization

Context fully supports serialization:

```rust
// Serialize
let json = serde_json::to_string(&context)?;

// Deserialize
let context: Context = serde_json::from_str(&json)?;

// Retrieve data after deserialization
let name: Option<String> = context.get("name").await;
let history = context.get_chat_history().await;
```

**Use Cases:**
- Session persistence
- Debugging
- Logging
- Testing

---

## Best Practices

### 1. Use Descriptive Keys

```rust
// Good: Clear key names
context.set("user_email", email).await;
context.set("claim_amount", amount).await;

// Avoid: Generic keys
context.set("data", value).await;
context.set("temp", value).await;
```

### 2. Handle Missing Data

```rust
let value: Option<String> = context.get("key").await;

match value {
    Some(v) => process(v),
    None => {
        // Handle missing data
        return Ok(TaskResult::new(
            Some("Please provide data".to_string()),
            NextAction::WaitForInput
        ));
    }
}
```

### 3. Store Intermediate Results

```rust
// Store results for later tasks
let processed = process_data(raw_data).await;
context.set("processed_data", processed).await;

// Later task can retrieve
let data: Option<ProcessedData> = context.get("processed_data").await;
```

### 4. Use Custom Types for Complex Data

```rust
#[derive(Serialize, Deserialize)]
struct WorkflowState {
    step: u32,
    data: Vec<String>,
    errors: Vec<String>,
}

// Store as single value
let state = WorkflowState { /* ... */ };
context.set("workflow_state", state).await;

// Retrieve
let state: Option<WorkflowState> = context.get("workflow_state").await;
```

### 5. Limit Chat History in Production

```rust
// Prevent unbounded growth
let context = Context::with_max_chat_messages(100);
```

---

## Examples

### Simple State Management

```rust
struct SetNameTask;

#[async_trait]
impl Task for SetNameTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        context.set("name", "Alice".to_string()).await;
        Ok(TaskResult::new(None, NextAction::Continue))
    }
}

struct GreetTask;

#[async_trait]
impl Task for GreetTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let name: String = context.get_sync("name").unwrap();
        let greeting = format!("Hello, {}!", name);
        
        Ok(TaskResult::new(
            Some(greeting),
            NextAction::End
        ))
    }
}
```

### Conversation Management

```rust
struct ChatTask;

#[async_trait]
impl Task for ChatTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let user_input: String = context.get("user_input").await
            .unwrap_or_default();
        
        // Get conversation history
        let history = context.get_rig_messages().await;
        
        // Call LLM
        let response = agent.chat(&user_input, history).await?;
        
        // Store conversation
        context.add_user_message(user_input).await;
        context.add_assistant_message(response.clone()).await;
        
        Ok(TaskResult::new(
            Some(response),
            NextAction::Continue
        ))
    }
}
```

---

## Next Steps

- [Storage](./storage.md) - Persistence options
- [Task API](./task-api.md) - Working with tasks
- [Examples](../examples/simple.md) - Real-world usage
