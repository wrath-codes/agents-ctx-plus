# Tasks

Tasks are the building blocks of workflows in GraphFlow.

---

## What is a Task?

A task is a unit of work that:
- Receives input via context
- Performs operations
- Returns a result with next action
- Controls workflow flow

```rust
#[async_trait]
pub trait Task: Send + Sync {
    fn id(&self) -> &str;
    async fn run(&self, context: Context) -> Result<TaskResult>;
}
```

---

## Creating Tasks

### Basic Task

```rust
use async_trait::async_trait;
use graph_flow::{Context, Task, TaskResult, NextAction};

struct GreetingTask;

#[async_trait]
impl Task for GreetingTask {
    fn id(&self) -> &str {
        "greeting_task"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let name: String = context.get("name").await.unwrap_or("World".to_string());
        let greeting = format!("Hello, {}!", name);
        
        Ok(TaskResult::new(
            Some(greeting),
            NextAction::Continue
        ))
    }
}
```

### Task with Default ID

Use the default ID implementation:

```rust
struct MyTask;

#[async_trait]
impl Task for MyTask {
    // Uses type_name::<Self>() as ID
    // Result: "my_crate::MyTask"
    
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        Ok(TaskResult::new(None, NextAction::End))
    }
}
```

### Task with Custom ID

Override the ID for clarity:

```rust
struct ValidationTask {
    validator_type: String,
}

#[async_trait]
impl Task for ValidationTask {
    fn id(&self) -> &str {
        &self.validator_type  // e.g., "email_validator"
    }
    
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Validation logic
        Ok(TaskResult::new(None, NextAction::Continue))
    }
}
```

---

## TaskResult

Tasks return `TaskResult` with response and next action:

```rust
pub struct TaskResult {
    pub response: Option<String>,      // Response to user
    pub next_action: NextAction,       // What to do next
    pub task_id: String,               // Auto-set by graph
    pub status_message: Option<String>, // Debug/logging info
}
```

### Creating Results

```rust
// Basic result
TaskResult::new(
    Some("Processing complete".to_string()),
    NextAction::Continue
)

// With status message
TaskResult::new_with_status(
    Some("Validated".to_string()),
    NextAction::Continue,
    Some("All checks passed".to_string())
)

// Convenience methods
TaskResult::move_to_next()        // Continue, no response
TaskResult::move_to_next_direct() // ContinueAndExecute
```

---

## NextAction

Controls workflow execution flow:

### Continue

Execute next task, return control to caller:

```rust
Ok(TaskResult::new(
    Some("Step 1 complete".to_string()),
    NextAction::Continue
))
```

**Best for:** Interactive workflows, web services

### ContinueAndExecute

Execute next task immediately:

```rust
Ok(TaskResult::new(
    Some("Processing...".to_string()),
    NextAction::ContinueAndExecute
))
```

**Best for:** Batch processing, automated chains

### WaitForInput

Pause for user input:

```rust
Ok(TaskResult::new(
    Some("Please provide your email".to_string()),
    NextAction::WaitForInput
))
```

**Best for:** Interactive applications, forms

### End

Complete workflow:

```rust
Ok(TaskResult::new(
    Some("Workflow completed!".to_string()),
    NextAction::End
))
```

**Best for:** Final tasks

### GoTo

Jump to specific task:

```rust
Ok(TaskResult::new(
    Some("Retrying...".to_string()),
    NextAction::GoTo("validation_task".to_string())
))
```

**Best for:** Loops, error handling

### GoBack

Return to previous task:

```rust
Ok(TaskResult::new(
    Some("Going back...".to_string()),
    NextAction::GoBack
))
```

**Best for:** Back navigation

---

## Context Operations

### Reading Data

```rust
// Async access
let name: Option<String> = context.get("name").await;
let count: Option<i32> = context.get("count").await;

// Sync access (for edge conditions)
let name: Option<String> = context.get_sync("name");
```

### Writing Data

```rust
// Store any serializable type
context.set("name", "Alice".to_string()).await;
context.set("count", 42).await;
context.set("active", true).await;

// Sync write
context.set_sync("key", value);
```

### Chat History

```rust
// Add messages
context.add_user_message("Hello".to_string()).await;
context.add_assistant_message("Hi!".to_string()).await;
context.add_system_message("Session started".to_string()).await;

// Retrieve history
let history = context.get_chat_history().await;
let recent = context.get_last_messages(5).await;

// With Rig feature
#[cfg(feature = "rig")]
{
    let rig_messages = context.get_rig_messages().await;
}
```

---

## Task Patterns

### Validation Task

```rust
struct ValidationTask;

#[async_trait]
impl Task for ValidationTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let email: Option<String> = context.get("email").await;
        
        match email {
            Some(email) if is_valid_email(&email) => {
                Ok(TaskResult::new(
                    Some("Email valid".to_string()),
                    NextAction::Continue
                ))
            }
            Some(_) => {
                Ok(TaskResult::new(
                    Some("Invalid email".to_string()),
                    NextAction::WaitForInput
                ))
            }
            None => {
                Ok(TaskResult::new(
                    Some("Please provide email".to_string()),
                    NextAction::WaitForInput
                ))
            }
        }
    }
}

fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}
```

### Processing Task

```rust
struct DataProcessingTask;

#[async_trait]
impl Task for DataProcessingTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input: String = context.get("raw_data").await
            .ok_or(GraphError::TaskExecutionFailed("No input data".to_string()))?;
        
        let processed = input.to_uppercase();
        context.set("processed_data", processed.clone()).await;
        
        Ok(TaskResult::new(
            Some(format!("Processed {} bytes", processed.len())),
            NextAction::Continue
        ))
    }
}
```

### LLM Integration Task

```rust
#[cfg(feature = "rig")]
struct LLMTask;

#[cfg(feature = "rig")]
#[async_trait]
impl Task for LLMTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        use rig::providers::openrouter;
        
        let user_input: String = context.get("user_input").await
            .unwrap_or_default();
        
        let client = openrouter::Client::new(
            &std::env::var("OPENROUTER_API_KEY").unwrap()
        );
        
        let agent = client.agent("openai/gpt-4o-mini")
            .preamble("You are a helpful assistant")
            .build();
        
        let chat_history = context.get_rig_messages().await;
        let response = agent.chat(&user_input, chat_history).await
            .map_err(|e| GraphError::TaskExecutionFailed(e.to_string()))?;
        
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

### Decision Task

```rust
struct DecisionTask {
    threshold: f64,
}

#[async_trait]
impl Task for DecisionTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let score: Option<f64> = context.get("confidence_score").await;
        
        match score {
            Some(s) if s >= self.threshold => {
                context.set("decision", "approved".to_string()).await;
                Ok(TaskResult::new(
                    Some("Approved".to_string()),
                    NextAction::Continue
                ))
            }
            Some(_) => {
                context.set("decision", "rejected".to_string()).await;
                Ok(TaskResult::new(
                    Some("Rejected".to_string()),
                    NextAction::Continue
                ))
            }
            None => Err(GraphError::TaskExecutionFailed(
                "No score available".to_string()
            )),
        }
    }
}
```

---

## Error Handling

### Task Errors

Return errors using `GraphError`:

```rust
use graph_flow::GraphError;

#[async_trait]
impl Task for MyTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let data: Option<String> = context.get("data").await;
        
        let data = data.ok_or_else(|| {
            GraphError::TaskExecutionFailed("Missing data".to_string())
        })?;
        
        // Process...
        
        Ok(TaskResult::new(None, NextAction::Continue))
    }
}
```

### Error Recovery

Use `GoTo` for retry logic:

```rust
struct RetryTask {
    max_retries: usize,
}

#[async_trait]
impl Task for RetryTask {
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let retries: usize = context.get("retry_count").await.unwrap_or(0);
        
        match perform_operation().await {
            Ok(result) => {
                context.set("retry_count", 0).await;
                Ok(TaskResult::new(
                    Some(result),
                    NextAction::Continue
                ))
            }
            Err(_) if retries < self.max_retries => {
                context.set("retry_count", retries + 1).await;
                Ok(TaskResult::new(
                    Some(format!("Retry {} of {}", retries + 1, self.max_retries)),
                    NextAction::GoTo(self.id())  // Retry this task
                ))
            }
            Err(e) => Err(GraphError::TaskExecutionFailed(e.to_string())),
        }
    }
}
```

---

## Best Practices

### 1. Keep Tasks Focused

Each task should do one thing:

```rust
// Good: Single responsibility
struct ValidateEmailTask;
struct SendEmailTask;

// Avoid: Doing too much
struct ValidateAndSendEmailTask;  // Don't do this
```

### 2. Use Descriptive IDs

```rust
// Good: Clear ID
fn id(&self) -> &str {
    "email_validation"
}

// Avoid: Generic ID
fn id(&self) -> &str {
    "task_1"
}
```

### 3. Handle Missing Data Gracefully

```rust
async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
    let data: Option<String> = context.get("data").await;
    
    let data = match data {
        Some(d) => d,
        None => {
            return Ok(TaskResult::new(
                Some("Please provide data".to_string()),
                NextAction::WaitForInput
            ));
        }
    };
    
    // Process data...
}
```

### 4. Store Intermediate Results

```rust
async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
    let input = fetch_data().await?;
    let processed = process_data(input).await?;
    
    // Store for later tasks
    context.set("processed_data", processed).await;
    
    Ok(TaskResult::new(None, NextAction::Continue))
}
```

### 5. Use Status Messages

```rust
Ok(TaskResult::new_with_status(
    Some("Processing complete".to_string()),
    NextAction::Continue,
    Some("Validated 5 fields, found 0 errors".to_string())
))
```

---

## Testing Tasks

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_greeting_task() {
        let task = GreetingTask;
        let context = Context::new();
        
        context.set("name", "Alice".to_string()).await;
        
        let result = task.run(context).await.unwrap();
        
        assert_eq!(
            result.response,
            Some("Hello, Alice!".to_string())
        );
        assert!(matches!(result.next_action, NextAction::Continue));
    }
}
```

---

## Next Steps

- [Graph Builder](./graph-builder.md) - Connecting tasks
- [Context API](./context-api.md) - State management
- [Conditional Routing](../advanced/conditional-routing.md) - Dynamic flows
- [Examples](../examples/simple.md) - Real-world patterns
