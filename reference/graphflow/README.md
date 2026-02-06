# GraphFlow Documentation

## 🔄 High-Performance Workflow Framework for Rust

GraphFlow is a high-performance, type-safe framework for building multi-agent workflow systems in Rust. It provides a graph-based execution engine for orchestrating complex, stateful workflows with built-in support for AI agent integration.

---

## What is GraphFlow?

GraphFlow combines the power of graph-based workflow execution with Rust's performance and type safety. It's inspired by LangGraph but built from the ground up for Rust, offering:

- **Graph Execution Library** - Orchestrate complex, stateful workflows
- **LLM Integration** - Optional integration with the Rig crate for AI capabilities
- **Flexible Execution** - Step-by-step, batch, or mixed execution modes
- **Human-in-the-Loop** - Natural workflow interruption and resumption
- **Type Safety** - Compile-time guarantees for workflow correctness

### Philosophy

GraphFlow follows the same philosophy as LangGraph:
- **Graph execution library** for orchestrating workflows
- **LLM ecosystem integration** for AI agent capabilities

But built for Rust with:
- **Performance** - Zero-cost abstractions, minimal overhead
- **Type Safety** - Compile-time guarantees
- **Clean Database Schema** - Simple, efficient storage
- **Production Ready** - Battle-tested patterns

---

## Documentation Structure

### Getting Started
- [Installation](./getting-started/installation.md)
- [Quick Start Guide](./getting-started/quickstart.md)
- [First Workflow](./getting-started/first-workflow.md)

### Core Concepts
- [Architecture Overview](./concepts/architecture.md)
- [Tasks](./concepts/tasks.md)
- [Graph Execution](./concepts/graph-execution.md)
- [Context and State](./concepts/context.md)
- [Storage](./concepts/storage.md)

### Core API
- [Task API](./core/task-api.md)
- [Graph Builder](./core/graph-builder.md)
- [Context API](./core/context-api.md)
- [FlowRunner](./core/flow-runner.md)

### Advanced Topics
- [Conditional Routing](./advanced/conditional-routing.md)
- [Parallel Execution](./advanced/parallel-execution.md)
- [LLM Integration](./advanced/llm-integration.md)
- [Human-in-the-Loop](./advanced/human-in-loop.md)

### Examples
- [Simple Example](./examples/simple.md)
- [Insurance Claims Service](./examples/insurance-claims.md)
- [Recommendation Service](./examples/recommendation.md)

### API Reference
- [Types and Traits](./api/types.md)
- [Complete API](./api/reference.md)

---

## Quick Example

```rust
use async_trait::async_trait;
use graph_flow::{
    Context, FlowRunner, GraphBuilder, InMemorySessionStorage,
    NextAction, Session, Task, TaskResult,
};
use std::sync::Arc;

// Define a task
struct HelloTask;

#[async_trait]
impl Task for HelloTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let name: String = context.get("name").await.unwrap_or("World".to_string());
        let greeting = format!("Hello, {}!", name);
        
        context.set("greeting", greeting.clone()).await;
        
        Ok(TaskResult::new(Some(greeting), NextAction::Continue))
    }
}

#[tokio::main]
async fn main() -> graph_flow::Result<()> {
    // Build workflow
    let hello_task = Arc::new(HelloTask);
    let graph = Arc::new(
        GraphBuilder::new("greeting_workflow")
            .add_task(hello_task.clone())
            .build()
    );
    
    // Create runner
    let storage = Arc::new(InMemorySessionStorage::new());
    let runner = FlowRunner::new(graph, storage.clone());
    
    // Create and run session
    let session = Session::new_from_task("session_001".to_string(), hello_task.id());
    session.context.set("name", "Alice".to_string()).await;
    storage.save(session).await?;
    
    let result = runner.run("session_001").await?;
    println!("Response: {:?}", result.response);
    
    Ok(())
}
```

---

## Key Features

### Type-Safe Workflows
Compile-time guarantees for workflow correctness. The Rust type system ensures your workflows are valid before they run.

### Flexible Execution
Choose how your workflow executes:
- **Step-by-Step** - Execute one task at a time, return control to caller
- **Continuous** - Execute tasks automatically until completion
- **Mixed** - Combine both approaches in the same workflow

### Stateful Execution
Workflows maintain state across interactions:
- **Sessions** - Persist workflow state
- **Context** - Thread-safe state sharing
- **Chat History** - Built-in conversation management

### Conditional Routing
Dynamic workflow paths based on runtime data:
```rust
.add_conditional_edge(
    classifier_id,
    |ctx| ctx.get_sync::<String>("sentiment") == Some("positive"),
    positive_task_id,
    negative_task_id,
)
```

### Parallel Execution
Execute tasks concurrently with FanOut:
```rust
let fanout = FanOutTask::new("parallel", vec![
    Arc::new(TaskA),
    Arc::new(TaskB),
    Arc::new(TaskC),
]);
```

### LLM Integration
Optional integration with Rig for AI agents:
```rust
let agent = client.agent("openai/gpt-4o-mini")
    .preamble("You are a helpful assistant")
    .build();

let response = agent.chat(&user_input, chat_history).await?;
```

### Storage Backends
- **In-Memory** - Fast, non-persistent (development)
- **PostgreSQL** - Persistent, scalable (production)

---

## Installation

```bash
cargo add graph-flow
```

With LLM support:
```bash
cargo add graph-flow --features rig
```

---

## Repository Structure

```
rs-graph-llm/
├── graph-flow/              # Core framework library
│   ├── src/
│   │   ├── lib.rs          # Main exports
│   │   ├── graph.rs        # Graph execution engine
│   │   ├── task.rs         # Task trait and results
│   │   ├── context.rs      # State management
│   │   ├── storage.rs      # Storage traits
│   │   ├── storage_postgres.rs  # PostgreSQL backend
│   │   ├── runner.rs       # FlowRunner
│   │   └── fanout.rs       # Parallel execution
│   └── Cargo.toml
│
├── examples/               # Learning examples
│   ├── simple_example.rs
│   ├── complex_example.rs
│   └── recommendation_flow.rs
│
├── insurance-claims-service/  # Production example
├── recommendation-service/    # Production example
└── medical-document-service/  # Production example
```

---

## Core Concepts

### Tasks
Tasks are the building blocks of workflows. Each task implements the `Task` trait:

```rust
#[async_trait]
impl Task for MyTask {
    async fn run(&self, context: Context) -> Result<TaskResult> {
        // Task logic here
        Ok(TaskResult::new(response, NextAction::Continue))
    }
}
```

### Graph
A graph defines the workflow structure with tasks and edges:

```rust
let graph = GraphBuilder::new("my_workflow")
    .add_task(task1)
    .add_task(task2)
    .add_edge(task1.id(), task2.id())
    .build();
```

### Context
Context provides thread-safe state management across tasks:

```rust
// Store data
context.set("key", value).await;

// Retrieve data
let value: Option<String> = context.get("key").await;

// Chat history
context.add_user_message("Hello".to_string()).await;
```

### Execution Control
Tasks control workflow execution through `NextAction`:

- `Continue` - Execute next task, return control
- `ContinueAndExecute` - Execute next task immediately
- `WaitForInput` - Pause for user input
- `End` - Complete workflow
- `GoTo(task_id)` - Jump to specific task

---

## Production Use Cases

### Insurance Claims Processing
Complete insurance workflow with:
- Multi-step claim processing
- Conditional routing based on insurance type
- LLM-driven natural language interactions
- Human-in-the-loop for high-value claims
- Business rule validation

### Recommendation Service
RAG-based recommendation system with:
- Vector search integration
- Multi-step reasoning
- Context accumulation
- Structured data extraction

---

## Performance Characteristics

- **Graph Execution**: Minimal overhead, zero-cost abstractions
- **Context Access**: O(1) via DashMap
- **Session Storage**: Pluggable backends
- **Memory Usage**: Efficient serialization
- **Concurrency**: Full async/await support

---

## Next Steps

- [Installation Guide](./getting-started/installation.md)
- [Quick Start Tutorial](./getting-started/quickstart.md)
- [Architecture Overview](./concepts/architecture.md)
- [API Reference](./api/reference.md)

---

## License

MIT License - see [LICENSE](https://github.com/a-agmon/rs-graph-llm/blob/main/LICENSE)

---

**GraphFlow** - Build complex, stateful workflows in Rust with type safety and performance.
