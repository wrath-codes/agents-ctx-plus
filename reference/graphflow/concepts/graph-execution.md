# Graph Execution

Understanding how GraphFlow executes workflows.

---

## Graph Structure

A graph consists of:
- **Tasks** - Units of work
- **Edges** - Connections between tasks
- **Context** - Shared state

```rust
let graph = GraphBuilder::new("my_workflow")
    .add_task(task_a)
    .add_task(task_b)
    .add_task(task_c)
    .add_edge(task_a.id(), task_b.id())
    .add_edge(task_b.id(), task_c.id())
    .build();
```

---

## Execution Models

### 1. Step-by-Step Execution

Execute one task at a time:

```rust
let runner = FlowRunner::new(graph, storage);

loop {
    let result = runner.run(session_id).await?;
    
    match result.status {
        ExecutionStatus::Completed => break,
        ExecutionStatus::Paused { next_task_id, .. } => {
            println!("Next task: {}", next_task_id);
            continue;
        }
        ExecutionStatus::WaitingForInput => {
            // Handle user input
            break;
        }
        ExecutionStatus::Error(e) => {
            eprintln!("Error: {}", e);
            break;
        }
    }
}
```

**Best for:**
- Web services
- Interactive applications
- Human-in-the-loop workflows

---

### 2. Continuous Execution

Execute tasks automatically:

```rust
// In your task
Ok(TaskResult::new(
    response,
    NextAction::ContinueAndExecute
))
```

**Best for:**
- Batch processing
- Automated workflows
- Data pipelines

---

### 3. Manual Execution

Direct control over execution:

```rust
let mut session = storage.get(session_id).await?.unwrap();
let result = graph.execute_session(&mut session).await?;
storage.save(session).await?;
```

**Best for:**
- Custom logic
- Batch operations
- Debugging

---

## Execution Flow

### Basic Flow

```
Start
  │
  ▼
Task A
  │
  ▼
Task B
  │
  ▼
Task C
  │
  ▼
 End
```

### With Conditional Branching

```
Start
  │
  ▼
Decision Task
  │
  ├─ Yes ─▶ Task A ─▶ End
  │
  └─ No ──▶ Task B ─▶ End
```

### With Loop

```
       ┌─────────────────┐
       │                 │
       ▼                 │
Validate ──▶ Error ──▶ Retry
  │
  ▼
Process
  │
  ▼
 End
```

---

## ExecutionResult

The result of graph execution:

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

### Status Variants

**Paused**
- Task completed with `NextAction::Continue`
- Contains next task ID
- Ready to continue

**WaitingForInput**
- Task completed with `NextAction::WaitForInput`
- Requires user input
- Stays at current task

**Completed**
- Task completed with `NextAction::End`
- Workflow finished
- No more tasks

**Error**
- Task failed
- Contains error message
- Workflow stopped

---

## Session Management

### Creating Sessions

```rust
// From a task
let session = Session::new_from_task(
    "session_001".to_string(),
    start_task.id()
);

// Set initial context
session.context.set("user_id", "123".to_string()).await;

// Save to storage
storage.save(session).await?;
```

### Session State

```rust
pub struct Session {
    pub id: String,                    // Unique identifier
    pub graph_id: String,              // Associated graph
    pub current_task_id: String,       // Current position
    pub status_message: Option<String>, // Last status
    pub context: Context,              // Workflow state
}
```

### Resuming Sessions

```rust
// Load existing session
let session = storage.get(session_id).await?
    .ok_or("Session not found")?;

// Update context
session.context.set("user_input", input).await;
storage.save(session).await?;

// Continue execution
let result = runner.run(session_id).await?;
```

---

## Task Timeout

Set execution timeout:

```rust
let mut graph = Graph::new("my_workflow");
graph.set_task_timeout(Duration::from_secs(60));

// Or via builder
let graph = GraphBuilder::new("my_workflow")
    .add_task(task)
    .build();
graph.set_task_timeout(Duration::from_secs(60));
```

Default timeout: 300 seconds (5 minutes)

---

## Error Handling

### Task Errors

Stop workflow on error:

```rust
async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
    let result = risky_operation().await
        .map_err(|e| GraphError::TaskExecutionFailed(e.to_string()))?;
    
    Ok(TaskResult::new(Some(result), NextAction::Continue))
}
```

### Handling at Application Level

```rust
match runner.run(session_id).await {
    Ok(result) => handle_success(result),
    Err(GraphError::TaskExecutionFailed(msg)) => {
        eprintln!("Task failed: {}", msg);
        // Handle error
    }
    Err(GraphError::SessionNotFound(id)) => {
        eprintln!("Session {} not found", id);
        // Handle missing session
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
        // Handle other errors
    }
}
```

---

## Best Practices

### 1. Use FlowRunner for Simplicity

```rust
// Recommended
let runner = FlowRunner::new(graph, storage);
let result = runner.run(session_id).await?;

// Manual (use when needed)
let mut session = storage.get(session_id).await?.unwrap();
let result = graph.execute_session(&mut session).await?;
storage.save(session).await?;
```

### 2. Handle All Status Variants

```rust
match result.status {
    ExecutionStatus::Completed => { /* Handle completion */ }
    ExecutionStatus::Paused { .. } => { /* Continue execution */ }
    ExecutionStatus::WaitingForInput => { /* Get input */ }
    ExecutionStatus::Error(e) => { /* Handle error */ }
}
```

### 3. Save Session After Updates

```rust
// Update context
session.context.set("key", value).await;

// Always save
storage.save(session).await?;

// Then execute
let result = runner.run(session_id).await?;
```

### 4. Set Appropriate Timeouts

```rust
// Fast operations
graph.set_task_timeout(Duration::from_secs(10));

// Slow operations (LLM calls)
graph.set_task_timeout(Duration::from_secs(120));
```

---

## Next Steps

- [FlowRunner](./flow-runner.md) - High-level orchestration
- [Conditional Routing](../advanced/conditional-routing.md) - Dynamic flows
- [Examples](../examples/simple.md) - Real-world patterns
