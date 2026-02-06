# Tools

## Overview

Tools allow agents to perform actions and retrieve information. They enable function calling capabilities in LLMs.

## Basic Tool

### Simple Tool

```rust
use rig::tool::Tool;

// Define input
#[derive(Deserialize)]
struct CalculatorInput {
    a: f64,
    b: f64,
    operation: String,
}

// Define output
#[derive(Serialize)]
struct CalculatorOutput {
    result: f64,
}

// Create tool
let calculator = Tool::new("calculator", |input: CalculatorInput| async move {
    let result = match input.operation.as_str() {
        "add" => input.a + input.b,
        "subtract" => input.a - input.b,
        "multiply" => input.a * input.b,
        "divide" => input.a / input.b,
        _ => return Err("Unknown operation".to_string()),
    };
    
    Ok(CalculatorOutput { result })
});
```

### Adding to Agent

```rust
let agent = client
    .agent("gpt-4")
    .tool(calculator)
    .build();

// Now the agent can use the calculator
let response = agent
    .prompt("What is 15 + 27?")
    .await?;
```

## Tool with Description

```rust
use rig::tool::ToolBuilder;

let weather_tool = ToolBuilder::new("weather")
    .description("Get weather information for a location")
    .parameter("location", "City name (e.g., 'New York')")
    .parameter("unit", "Temperature unit: 'celsius' or 'fahrenheit'")
    .build(|input: WeatherInput| async move {
        // Implementation
        Ok(WeatherOutput { temperature: 72.0 })
    });
```

## Async Tools

```rust
let search_tool = Tool::new("search", |input: SearchInput| async move {
    // Async operation
    let results = reqwest::get(format!("https://api.search.com?q={}", input.query))
        .await?
        .json::<SearchResponse>()
        .await?;
    
    Ok(results)
});
```

## Tool with State

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

struct DatabaseTool {
    db: Arc<Mutex<Connection>>,
}

impl DatabaseTool {
    fn new(db: Connection) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
        }
    }
    
    fn build_tool(&self) -> Tool<QueryInput, QueryOutput> {
        let db = self.db.clone();
        
        Tool::new("query", move |input: QueryInput| async move {
            let db = db.lock().await;
            let results = db.query(&input.sql).await?;
            Ok(QueryOutput { results })
        })
    }
}
```

## Multiple Tools

```rust
let agent = client
    .agent("gpt-4")
    .tool(calculator)
    .tool(weather_tool)
    .tool(search_tool)
    .build();
```

## Tool Execution

### Automatic

Agents automatically decide when to use tools:

```rust
let response = agent
    .prompt("What's the weather in Tokyo and calculate 25 * 4?")
    .await?;

// Agent will:
// 1. Call weather tool for Tokyo
// 2. Call calculator tool for 25 * 4
// 3. Combine results into response
```

### Manual

```rust
// Check if tool was used
if let Some(tool_calls) = response.tool_calls {
    for call in tool_calls {
        println!("Tool: {}", call.name);
        println!("Arguments: {}", call.arguments);
    }
}
```

## Error Handling

```rust
let tool = Tool::new("risky_operation", |input: Input| async move {
    match perform_operation(input).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Return descriptive error
            Err(format!("Operation failed: {}", e))
        }
    }
});
```

## Best Practices

1. **Clear Descriptions**: Help the LLM understand when to use tools
2. **Type Safety**: Use strongly typed inputs and outputs
3. **Error Messages**: Return clear, actionable errors
4. **Validation**: Validate inputs before processing
5. **Timeouts**: Add timeouts to long-running operations

## Next Steps

- **[Custom Tools](../advanced/custom-tools.md)** - Advanced tool patterns
- **[Agents](../core/agents.md)** - Agent configuration