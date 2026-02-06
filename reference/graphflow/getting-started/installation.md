# Installation Guide

Get GraphFlow installed and running in minutes.

---

## Prerequisites

- **Rust** 1.70 or later
- **Cargo** (comes with Rust)

Check your Rust version:

```bash
rustc --version
cargo --version
```

---

## Installation Methods

### Method 1: Add to Existing Project

Add GraphFlow to your `Cargo.toml`:

```toml
[dependencies]
graph-flow = "0.4"
```

With LLM integration (Rig):

```toml
[dependencies]
graph-flow = { version = "0.4", features = ["rig"] }
```

---

### Method 2: Clone the Repository

Clone the full repository with examples:

```bash
git clone https://github.com/a-agmon/rs-graph-llm.git
cd rs-graph-llm
```

Build the project:

```bash
cargo build --release
```

Run examples:

```bash
# Simple example
cargo run --example simple_example

# Complex example with conditional routing
cargo run --example complex_example

# FanOut parallel execution
cargo run --example fanout_basic
```

---

### Method 3: Create New Project

Create a new Rust project:

```bash
cargo new my-graphflow-app
cd my-graphflow-app
```

Add dependencies:

```toml
[dependencies]
graph-flow = "0.4"
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
```

For LLM support:

```toml
[dependencies]
graph-flow = { version = "0.4", features = ["rig"] }
rig-core = "0.19"
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
```

---

## Feature Flags

GraphFlow provides optional features:

| Feature | Description | Default |
|---------|-------------|---------|
| `rig` | LLM integration via Rig crate | Disabled |

Enable features in `Cargo.toml`:

```toml
[dependencies]
graph-flow = { version = "0.4", features = ["rig"] }
```

---

## Verify Installation

Create a simple test file:

```rust
// src/main.rs
use graph_flow::{Context, Task, TaskResult, NextAction};
use async_trait::async_trait;

struct TestTask;

#[async_trait]
impl Task for TestTask {
    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Ok(TaskResult::new(
            Some("GraphFlow is working!".to_string()),
            NextAction::End
        ))
    }
}

#[tokio::main]
async fn main() {
    println!("✅ GraphFlow installed successfully!");
}
```

Run:

```bash
cargo run
```

Expected output:

```
✅ GraphFlow installed successfully!
```

---

## Environment Setup

### For LLM Integration

Set your LLM API key:

```bash
export OPENROUTER_API_KEY="your-api-key"
# or
export OPENAI_API_KEY="your-api-key"
```

### For PostgreSQL Storage

Set database URL:

```bash
export DATABASE_URL="postgresql://user:password@localhost/dbname"
```

---

## IDE Setup

### VS Code

Recommended extensions:
- **rust-analyzer** - Rust language support
- **CodeLLDB** - Debugging
- **Even Better TOML** - TOML file support

Configuration (`.vscode/settings.json`):

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### IntelliJ/RustRover

1. Install Rust plugin
2. Import project
3. Enable feature flags in settings if needed

---

## Troubleshooting

### Compilation Errors

**Error**: `cannot find struct, variant or union type TaskResult`

**Solution**: Ensure you're importing from graph_flow:

```rust
use graph_flow::{TaskResult, NextAction};
```

---

**Error**: `feature 'rig' is not enabled`

**Solution**: Enable the feature in `Cargo.toml`:

```toml
graph-flow = { version = "0.4", features = ["rig"] }
```

---

**Error**: `the trait bound MyTask: Task is not satisfied`

**Solution**: Implement all required trait methods:

```rust
#[async_trait]
impl Task for MyTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Implementation
    }
}
```

---

### Runtime Errors

**Error**: `Session not found`

**Solution**: Create and save session before execution:

```rust
let session = Session::new_from_task(session_id, task_id);
storage.save(session).await?;
```

---

**Error**: `Task not found`

**Solution**: Ensure task is added to graph:

```rust
let graph = GraphBuilder::new("workflow")
    .add_task(task)  // Don't forget this!
    .build();
```

---

## Next Steps

- [Quick Start Guide](./quickstart.md) - Build your first workflow
- [First Workflow Tutorial](./first-workflow.md) - Step-by-step guide
- [Core Concepts](../concepts/architecture.md) - Understand the architecture
