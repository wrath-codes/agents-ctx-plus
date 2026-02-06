# Quick Start Guide

Build your first workflow with GraphFlow in 5 minutes.

---

## Simple Greeting Workflow

Let's build a simple workflow that greets a user and adds excitement.

### Step 1: Create Project

```bash
cargo new greeting-workflow
cd greeting-workflow
```

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
graph-flow = "0.4"
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
```

### Step 2: Define Tasks

Create `src/main.rs`:

```rust
use async_trait::async_trait;
use graph_flow::{
    Context, FlowRunner, GraphBuilder, InMemorySessionStorage,
    NextAction, Session, Task, TaskResult,
};
use std::sync::Arc;

// Task 1: Generate greeting
struct HelloTask;

#[async_trait]
impl Task for HelloTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get name from context
        let name: String = context.get("name").await.unwrap_or("World".to_string());
        let greeting = format!("Hello, {}", name);
        
        // Store for next task
        context.set("greeting", greeting.clone()).await;
        
        // Return response and continue
        Ok(TaskResult::new(Some(greeting), NextAction::Continue))
    }
}

// Task 2: Add excitement
struct ExcitementTask;

#[async_trait]
impl Task for ExcitementTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get greeting from context
        let greeting: String = context.get_sync("greeting").unwrap();
        let excited = format!("{} !!!", greeting);
        
        // End workflow
        Ok(TaskResult::new(Some(excited), NextAction::End))
    }
}
```

### Step 3: Build and Execute Graph

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create tasks
    let hello_task = Arc::new(HelloTask);
    let excitement_task = Arc::new(ExcitementTask);
    
    // Build graph
    let graph = Arc::new(
        GraphBuilder::new("greeting_workflow")
            .add_task(hello_task.clone())
            .add_task(excitement_task.clone())
            .add_edge(hello_task.id(), excitement_task.id())
            .build()
    );
    
    // Create storage
    let storage = Arc::new(InMemorySessionStorage::new());
    
    // Create FlowRunner
    let runner = FlowRunner::new(graph, storage.clone());
    
    // Create session with initial data
    let session_id = "session_001";
    let session = Session::new_from_task(session_id.to_string(), hello_task.id());
    session.context.set("name", "Alice".to_string()).await;
    storage.save(session).await?;
    
    // Execute workflow
    let result = runner.run(session_id).await?;
    
    println!("Response: {:?}", result.response);
    println!("Status: {:?}", result.status);
    
    Ok(())
}
```

### Step 4: Run

```bash
cargo run
```

Output:

```
Response: Some("Hello, Alice !!!")
Status: Completed
```

---

## Step-by-Step Execution

For more control, execute step by step:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... setup code ...
    
    // Execute step by step
    loop {
        let result = runner.run(session_id).await?;
        
        match result.status {
            graph_flow::ExecutionStatus::Completed => {
                println!("Done: {:?}", result.response);
                break;
            }
            graph_flow::ExecutionStatus::Paused { next_task_id, .. } => {
                println!("Step complete. Next: {}", next_task_id);
                continue;
            }
            _ => break,
        }
    }
    
    Ok(())
}
```

---

## Adding User Input

Make it interactive:

```rust
struct GetNameTask;

#[async_trait]
impl Task for GetNameTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Check if name is already set
        if let Some(name) = context.get::<String>("name").await {
            // Name exists, continue
            return Ok(TaskResult::new(
                Some(format!("Hello, {}", name)),
                NextAction::Continue
            ));
        }
        
        // Wait for user input
        Ok(TaskResult::new(
            Some("What's your name?".to_string()),
            NextAction::WaitForInput
        ))
    }
}
```

Execute with user input:

```rust
// First execution - will wait for input
let result = runner.run(session_id).await?;
println!("{}", result.response.unwrap()); // "What's your name?"

// Get user input and update context
let mut session = storage.get(session_id).await?.unwrap();
session.context.set("name", "Bob".to_string()).await;
storage.save(session).await?;

// Continue execution
let result = runner.run(session_id).await?;
println!("{}", result.response.unwrap()); // "Hello, Bob"
```

---

## Using Conditional Routing

Add a conditional branch:

```rust
struct MoodTask;

#[async_trait]
impl Task for MoodTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let mood: String = context.get("mood").await.unwrap_or("neutral".to_string());
        
        if mood == "happy" {
            Ok(TaskResult::new(
                Some("😊 Great to hear!".to_string()),
                NextAction::Continue
            ))
        } else {
            Ok(TaskResult::new(
                Some("😔 Hope things get better!".to_string()),
                NextAction::Continue
            ))
        }
    }
}

// Build with conditional edge
let graph = GraphBuilder::new("mood_workflow")
    .add_task(mood_task.clone())
    .add_task(happy_task.clone())
    .add_task(sad_task.clone())
    .add_conditional_edge(
        mood_task.id(),
        |ctx| ctx.get_sync::<String>("mood").map(|m| m == "happy").unwrap_or(false),
        happy_task.id(),
        sad_task.id(),
    )
    .build();
```

---

## Next Steps

- [First Workflow Tutorial](./first-workflow.md) - Detailed walkthrough
- [Task System](../concepts/tasks.md) - Deep dive into tasks
- [Context and State](../concepts/context.md) - Managing workflow state
- [Examples](../examples/simple.md) - More complete examples
