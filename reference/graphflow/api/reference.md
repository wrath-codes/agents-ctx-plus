# Complete API Reference

Complete API reference for GraphFlow.

---

## Core Types

### Task Trait

```rust
#[async_trait]
pub trait Task: Send + Sync {
    fn id(&self) -> &str;
    async fn run(&self, context: Context) -> Result<TaskResult>;
}
```

### TaskResult

```rust
pub struct TaskResult {
    pub response: Option<String>,
    pub next_action: NextAction,
    pub task_id: String,
    pub status_message: Option<String>,
}

impl TaskResult {
    pub fn new(response: Option<String>, next_action: NextAction) -> Self;
    pub fn new_with_status(
        response: Option<String>,
        next_action: NextAction,
        status_message: Option<String>
    ) -> Self;
    pub fn move_to_next() -> Self;
    pub fn move_to_next_direct() -> Self;
}
```

### NextAction

```rust
pub enum NextAction {
    Continue,
    ContinueAndExecute,
    WaitForInput,
    End,
    GoTo(String),
    GoBack,
}
```

---

## Graph API

### Graph

```rust
pub struct Graph {
    pub id: String,
    tasks: DashMap<String, Arc<dyn Task>>,
    edges: Mutex<Vec<Edge>>,
    start_task_id: Mutex<Option<String>>,
    task_timeout: Duration,
}

impl Graph {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn set_task_timeout(&mut self, timeout: Duration);
    pub fn add_task(&self, task: Arc<dyn Task>) -> &Self;
    pub fn set_start_task(&self, task_id: impl Into<String>) -> &Self;
    pub fn add_edge(&self, from: impl Into<String>, to: impl Into<String>) -> &Self;
    pub fn add_conditional_edge<F>(
        &self,
        from: impl Into<String>,
        condition: F,
        yes: impl Into<String>,
        no: impl Into<String>,
    ) -> &Self
    where F: Fn(&Context) -> bool + Send + Sync + 'static;
    
    pub async fn execute_session(&self, session: &mut Session) -> Result<ExecutionResult>;
    pub async fn execute(&self, task_id: &str, context: Context) -> Result<TaskResult>;
    pub fn find_next_task(&self, current_task_id: &str, context: &Context) -> Option<String>;
    pub fn start_task_id(&self) -> Option<String>;
    pub fn get_task(&self, task_id: &str) -> Option<Arc<dyn Task>>;
}
```

### GraphBuilder

```rust
pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn add_task(self, task: Arc<dyn Task>) -> Self;
    pub fn add_edge(self, from: impl Into<String>, to: impl Into<String>) -> Self;
    pub fn add_conditional_edge<F>(
        self,
        from: impl Into<String>,
        condition: F,
        yes: impl Into<String>,
        no: impl Into<String>,
    ) -> Self
    where F: Fn(&Context) -> bool + Send + Sync + 'static;
    pub fn set_start_task(self, task_id: impl Into<String>) -> Self;
    pub fn build(self) -> Graph;
}
```

---

## Execution API

### ExecutionResult

```rust
pub struct ExecutionResult {
    pub response: Option<String>,
    pub status: ExecutionStatus,
}

pub enum ExecutionStatus {
    Paused { next_task_id: String, reason: String },
    WaitingForInput,
    Completed,
    Error(String),
}
```

### FlowRunner

```rust
pub struct FlowRunner {
    graph: Arc<Graph>,
    storage: Arc<dyn SessionStorage>,
}

impl FlowRunner {
    pub fn new(graph: Arc<Graph>, storage: Arc<dyn SessionStorage>) -> Self;
    pub async fn run(&self, session_id: &str) -> Result<ExecutionResult>;
}
```

---

## Context API

### Context

```rust
pub struct Context {
    data: Arc<DashMap<String, Value>>,
    chat_history: Arc<RwLock<ChatHistory>>,
}

impl Context {
    pub fn new() -> Self;
    pub fn with_max_chat_messages(max: usize) -> Self;
    
    // Data storage
    pub async fn set(&self, key: impl Into<String>, value: impl Serialize);
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    pub async fn remove(&self, key: &str) -> Option<Value>;
    pub async fn clear(&self);
    
    // Sync access
    pub fn get_sync<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    pub fn set_sync(&self, key: impl Into<String>, value: impl Serialize);
    
    // Chat history
    pub async fn add_user_message(&self, content: String);
    pub async fn add_assistant_message(&self, content: String);
    pub async fn add_system_message(&self, content: String);
    pub async fn get_chat_history(&self) -> ChatHistory;
    pub async fn clear_chat_history(&self);
    pub async fn chat_history_len(&self) -> usize;
    pub async fn is_chat_history_empty(&self) -> bool;
    pub async fn get_last_messages(&self, n: usize) -> Vec<SerializableMessage>;
    pub async fn get_all_messages(&self) -> Vec<SerializableMessage>;
    
    // Rig integration (requires "rig" feature)
    #[cfg(feature = "rig")]
    pub async fn get_rig_messages(&self) -> Vec<Message>;
    #[cfg(feature = "rig")]
    pub async fn get_last_rig_messages(&self, n: usize) -> Vec<Message>;
}
```

### SerializableMessage

```rust
pub struct SerializableMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl SerializableMessage {
    pub fn new(role: MessageRole, content: String) -> Self;
    pub fn user(content: String) -> Self;
    pub fn assistant(content: String) -> Self;
    pub fn system(content: String) -> Self;
}

pub enum MessageRole {
    User,
    Assistant,
    System,
}
```

### ChatHistory

```rust
pub struct ChatHistory {
    messages: Vec<SerializableMessage>,
    max_messages: Option<usize>,
}

impl ChatHistory {
    pub fn new() -> Self;
    pub fn with_max_messages(max: usize) -> Self;
    pub fn add_user_message(&mut self, content: String);
    pub fn add_assistant_message(&mut self, content: String);
    pub fn add_system_message(&mut self, content: String);
    pub fn clear(&mut self);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn messages(&self) -> &[SerializableMessage];
    pub fn last_messages(&self, n: usize) -> &[SerializableMessage];
}
```

---

## Storage API

### Session

```rust
pub struct Session {
    pub id: String,
    pub graph_id: String,
    pub current_task_id: String,
    pub status_message: Option<String>,
    pub context: Context,
}

impl Session {
    pub fn new_from_task(sid: String, task_name: &str) -> Self;
}
```

### SessionStorage Trait

```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: Session) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Session>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

### InMemorySessionStorage

```rust
pub struct InMemorySessionStorage {
    sessions: Arc<DashMap<String, Session>>,
}

impl InMemorySessionStorage {
    pub fn new() -> Self;
}

#[async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn save(&self, session: Session) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Session>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

### PostgresSessionStorage

```rust
pub struct PostgresSessionStorage {
    pool: sqlx::PgPool,
}

impl PostgresSessionStorage {
    pub async fn connect(database_url: &str) -> Result<Self>;
}

#[async_trait]
impl SessionStorage for PostgresSessionStorage {
    async fn save(&self, session: Session) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Session>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

---

## FanOut API

### FanOutTask

```rust
pub struct FanOutTask {
    id: String,
    children: Vec<Arc<dyn Task>>,
    prefix: Option<String>,
}

impl FanOutTask {
    pub fn new(id: impl Into<String>, children: Vec<Arc<dyn Task>>) -> Self;
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self;
}

#[async_trait]
impl Task for FanOutTask {
    fn id(&self) -> &str;
    async fn run(&self, context: Context) -> Result<TaskResult>;
}
```

---

## Error Types

### GraphError

```rust
pub enum GraphError {
    TaskNotFound(String),
    TaskExecutionFailed(String),
    SessionNotFound(String),
    StorageError(String),
    SerializationError(String),
    Other(String),
}

impl std::error::Error for GraphError {}
impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}
```

### Result Type

```rust
pub type Result<T> = std::result::Result<T, GraphError>;
```

---

## TypeScript Definitions

```typescript
// Core types
interface TaskResult {
  response?: string;
  next_action: NextAction;
  task_id: string;
  status_message?: string;
}

type NextAction = 
  | { type: "Continue" }
  | { type: "ContinueAndExecute" }
  | { type: "WaitForInput" }
  | { type: "End" }
  | { type: "GoTo"; task_id: string }
  | { type: "GoBack" };

interface ExecutionResult {
  response?: string;
  status: ExecutionStatus;
}

type ExecutionStatus =
  | { type: "Paused"; next_task_id: string; reason: string }
  | { type: "WaitingForInput" }
  | { type: "Completed" }
  | { type: "Error"; message: string };

interface Session {
  id: string;
  graph_id: string;
  current_task_id: string;
  status_message?: string;
  context: Context;
}

interface Context {
  // Data storage
  get<T>(key: string): Promise<T | undefined>;
  set<T>(key: string, value: T): Promise<void>;
  
  // Chat history
  add_user_message(content: string): Promise<void>;
  add_assistant_message(content: string): Promise<void>;
  add_system_message(content: string): Promise<void>;
  get_chat_history(): Promise<ChatHistory>;
}

interface ChatHistory {
  messages: SerializableMessage[];
}

interface SerializableMessage {
  role: "User" | "Assistant" | "System";
  content: string;
  timestamp: string; // ISO 8601
}
```

---

## Python Types

```python
from typing import Optional, Dict, Any, Literal
from datetime import datetime
from enum import Enum

class NextActionType(Enum):
    CONTINUE = "Continue"
    CONTINUE_AND_EXECUTE = "ContinueAndExecute"
    WAIT_FOR_INPUT = "WaitForInput"
    END = "End"
    GO_TO = "GoTo"
    GO_BACK = "GoBack"

class ExecutionStatusType(Enum):
    PAUSED = "Paused"
    WAITING_FOR_INPUT = "WaitingForInput"
    COMPLETED = "Completed"
    ERROR = "Error"

class TaskResult:
    response: Optional[str]
    next_action: Dict[str, Any]  # NextAction variant
    task_id: str
    status_message: Optional[str]

class ExecutionResult:
    response: Optional[str]
    status: Dict[str, Any]  # ExecutionStatus variant

class Session:
    id: str
    graph_id: str
    current_task_id: str
    status_message: Optional[str]
    context: Dict[str, Any]

class SerializableMessage:
    role: Literal["User", "Assistant", "System"]
    content: str
    timestamp: datetime
```

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `rig` | LLM integration via Rig crate | Disabled |

Enable in `Cargo.toml`:

```toml
[dependencies]
graph-flow = { version = "0.4", features = ["rig"] }
```

---

## Re-exports

```rust
pub use context::{ChatHistory, Context, MessageRole, SerializableMessage};
pub use error::{GraphError, Result};
pub use graph::{ExecutionResult, ExecutionStatus, Graph, GraphBuilder};
pub use runner::FlowRunner;
pub use storage::{GraphStorage, InMemoryGraphStorage, InMemorySessionStorage, Session, SessionStorage};
pub use storage_postgres::PostgresSessionStorage;
pub use task::{NextAction, Task, TaskResult};
pub use fanout::FanOutTask;
```

---

## Version Compatibility

| graph-flow | Rust Edition | Status |
|------------|--------------|--------|
| 0.4.x | 2024 | Current |
| 0.3.x | 2021 | Legacy |
| 0.2.x | 2021 | Legacy |

---

## Next Steps

- [Installation](../getting-started/installation.md)
- [Quick Start](../getting-started/quickstart.md)
- [Core Concepts](../concepts/architecture.md)
