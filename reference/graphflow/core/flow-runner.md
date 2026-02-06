# FlowRunner

High-level orchestrator for executing workflows with automatic session management.

---

## What is FlowRunner?

FlowRunner provides a convenient wrapper around the lower-level graph execution API. It automatically handles:

- Loading sessions from storage
- Executing one graph step
- Saving updated sessions
- Error handling

```rust
pub struct FlowRunner {
    graph: Arc<Graph>,
    storage: Arc<dyn SessionStorage>,
}
```

---

## When to Use FlowRunner

### Use FlowRunner When:

- Building web services
- Creating interactive applications
- Want minimal boilerplate
- Need step-by-step execution

### Use Manual Execution When:

- Batch processing
- Custom persistence logic
- Advanced diagnostics
- Maximum control needed

---

## Creating a FlowRunner

### Basic Creation

```rust
use graph_flow::{FlowRunner, Graph, InMemorySessionStorage};
use std::sync::Arc;

let graph = Arc::new(Graph::new("my_workflow"));
let storage = Arc::new(InMemorySessionStorage::new());

let runner = FlowRunner::new(graph, storage);
```

### With PostgreSQL

```rust
use graph_flow::PostgresSessionStorage;

let storage = Arc::new(
    PostgresSessionStorage::connect(&database_url).await?
);

let runner = FlowRunner::new(graph, storage);
```

---

## Executing Workflows

### Basic Execution

```rust
let result = runner.run("session_001").await?;

println!("Response: {:?}", result.response);
println!("Status: {:?}", result.status);
```

### Handling Results

```rust
match result.status {
    ExecutionStatus::Completed => {
        println!("Workflow completed!");
    }
    ExecutionStatus::Paused { next_task_id, .. } => {
        println!("Next task: {}", next_task_id);
    }
    ExecutionStatus::WaitingForInput => {
        println!("Waiting for user input");
    }
    ExecutionStatus::Error(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### Interactive Loop

```rust
loop {
    let result = runner.run(session_id).await?;
    
    match result.status {
        ExecutionStatus::Completed => {
            println!("Done: {:?}", result.response);
            break;
        }
        ExecutionStatus::WaitingForInput => {
            // Get user input
            let input = get_user_input();
            
            // Update session
            let mut session = storage.get(session_id).await?.unwrap();
            session.context.set("user_input", input).await;
            storage.save(session).await?;
        }
        ExecutionStatus::Paused { .. } => {
            // Continue automatically
            continue;
        }
        ExecutionStatus::Error(e) => {
            eprintln!("Error: {}", e);
            break;
        }
    }
}
```

---

## Web Service Pattern

### Shared FlowRunner (Recommended)

```rust
use axum::{
    Router,
    extract::State,
    routing::post,
    Json,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    flow_runner: Arc<FlowRunner>,
}

#[tokio::main]
async fn main() {
    let graph = Arc::new(create_graph());
    let storage = Arc::new(InMemorySessionStorage::new());
    let flow_runner = Arc::new(FlowRunner::new(graph, storage));
    
    let app_state = AppState { flow_runner };
    
    let app = Router::new()
        .route("/execute", post(execute_handler))
        .with_state(app_state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn execute_handler(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    // Set user input
    let mut session = state.flow_runner.storage()
        .get(&request.session_id).await.unwrap().unwrap();
    
    session.context.set("user_input", request.content).await;
    state.flow_runner.storage().save(session).await.unwrap();
    
    // Execute
    let result = state.flow_runner.run(&request.session_id).await.unwrap();
    
    Json(ExecuteResponse {
        response: result.response,
        status: format!("{:?}", result.status),
    })
}
```

### Per-Request FlowRunner

```rust
async fn handler(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    // Create fresh runner for this request
    let runner = FlowRunner::new(
        state.graph.clone(),
        state.storage.clone()
    );
    
    let result = runner.run(&request.session_id).await.unwrap();
    
    Json(ExecuteResponse {
        response: result.response,
        status: format!("{:?}", result.status),
    })
}
```

---

## Performance

FlowRunner is lightweight and efficient:

- **Creation Cost**: ~2 pointer copies (negligible)
- **Memory Overhead**: 16 bytes (2 × Arc<T>)
- **Runtime Cost**: Identical to manual approach

### Benchmarks

| Operation | Time |
|-----------|------|
| FlowRunner::new() | ~50ns |
| runner.run() | ~1μs + storage + task |
| Session save/load (in-memory) | ~1μs |
| Session save/load (PostgreSQL) | ~5-10ms |

---

## Error Handling

### Common Errors

```rust
match runner.run(session_id).await {
    Ok(result) => {
        // Handle success
    }
    Err(GraphError::SessionNotFound(id)) => {
        eprintln!("Session {} not found", id);
        // Create new session
    }
    Err(GraphError::TaskExecutionFailed(msg)) => {
        eprintln!("Task failed: {}", msg);
        // Handle task error
    }
    Err(GraphError::StorageError(msg)) => {
        eprintln!("Storage error: {}", msg);
        // Handle storage error
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

---

## Best Practices

### 1. Create Once, Share Across Requests

```rust
// At startup
let flow_runner = Arc::new(FlowRunner::new(graph, storage));

// In handlers
async fn handler(State(state): State<AppState>) {
    let result = state.flow_runner.run(session_id).await?;
}
```

### 2. Handle All Status Variants

```rust
match result.status {
    ExecutionStatus::Completed => { /* ... */ }
    ExecutionStatus::Paused { next_task_id, reason } => {
        tracing::info!("Paused, next: {}, reason: {}", next_task_id, reason);
    }
    ExecutionStatus::WaitingForInput => { /* ... */ }
    ExecutionStatus::Error(e) => { /* ... */ }
}
```

### 3. Set User Input Before Execution

```rust
// Update session
let mut session = storage.get(session_id).await?.unwrap();
session.context.set("user_input", user_input).await;
storage.save(session).await?;

// Then execute
let result = runner.run(session_id).await?;
```

### 4. Use Tracing

```rust
use tracing::{info, error};

match runner.run(session_id).await {
    Ok(result) => {
        info!(
            session_id = session_id,
            status = ?result.status,
            "Workflow step completed"
        );
    }
    Err(e) => {
        error!(
            session_id = session_id,
            error = %e,
            "Workflow step failed"
        );
    }
}
```

---

## Comparison with Manual Execution

| Aspect | FlowRunner | Manual |
|--------|-----------|--------|
| Boilerplate | Minimal | More |
| Session Management | Automatic | Manual |
| Error Handling | Automatic | Manual |
| Flexibility | Less | More |
| Performance | Same | Same |
| Control | Less | More |

### FlowRunner

```rust
// Simple and clean
let result = runner.run(session_id).await?;
```

### Manual

```rust
// More control, more boilerplate
let mut session = storage.get(session_id).await?
    .ok_or("Session not found")?;

let result = graph.execute_session(&mut session).await?;

storage.save(session).await?;
```

---

## Next Steps

- [Graph Execution](./graph-execution.md) - Understanding execution
- [Context and State](./context.md) - Managing state
- [Examples](../examples/simple.md) - Real-world patterns
