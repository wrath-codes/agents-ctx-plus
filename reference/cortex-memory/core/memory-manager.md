# Memory Manager

The Memory Manager is the central orchestrator of all memory operations in Cortex Memory. It coordinates between the vector store, LLM client, and various processing components.

---

## Overview

The `MemoryManager` provides the primary API for:
- Creating and storing memories
- Searching and retrieving memories
- Updating and deleting memories
- Managing memory lifecycle
- Performing health checks

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   MemoryManager                         │
├──────────────┬──────────────┬───────────────────────────┤
│  VectorStore │   LLMClient  │      Components           │
├──────────────┼──────────────┼───────────────────────────┤
│ - insert()   │ - complete() │ - FactExtractor           │
│ - search()   │ - embed()    │ - MemoryUpdater           │
│ - update()   │ - extract()  │ - ImportanceEvaluator     │
│ - delete()   │ - summarize()│ - DuplicateDetector       │
│ - list()     │              │ - MemoryClassifier        │
└──────────────┴──────────────┴───────────────────────────┘
```

---

## Creating a Memory Manager

### Basic Creation

```rust
use cortex_mem_core::{
    memory::MemoryManager,
    vector_store::qdrant::QdrantVectorStore,
    llm::create_llm_client,
};
use cortex_mem_config::Config;

// Load configuration
let config = Config::load("config.toml")?;

// Create vector store
let vector_store = QdrantVectorStore::new(&config.qdrant).await?;

// Create LLM client
let llm_client = create_llm_client(&config.llm, &config.embedding)?;

// Create memory manager
let memory_manager = MemoryManager::new(
    Box::new(vector_store),
    llm_client,
    config.memory.clone(),
);
```

### Using the Initialization Helper

```rust
use cortex_mem_core::init::initialize_memory_system;

// One-line initialization
let (vector_store, llm_client) = initialize_memory_system(&config).await?;

let memory_manager = MemoryManager::new(
    vector_store,
    llm_client,
    config.memory.clone(),
);
```

---

## Core Operations

### 1. Storing Memories

#### Store Simple Content

```rust
use cortex_mem_core::types::{MemoryMetadata, MemoryType};

let metadata = MemoryMetadata::new(MemoryType::Personal)
    .with_user_id("user123".to_string())
    .with_importance_score(0.8);

let memory_id = memory_manager.store(
    "User prefers dark mode in all applications".to_string(),
    metadata
).await?;

println!("Stored memory with ID: {}", memory_id);
```

#### Store from Conversation

```rust
use cortex_mem_core::types::Message;

let messages = vec![
    Message::user("I'm learning Rust programming"),
    Message::assistant("That's a great choice! Rust is excellent for systems programming.")
        .with_name("Tutor"),
    Message::user("Yes, I want to build high-performance applications"),
];

let metadata = MemoryMetadata::new(MemoryType::Conversational)
    .with_user_id("user123".to_string())
    .with_agent_id("learning-assistant".to_string());

let results = memory_manager.add_memory(&messages, metadata).await?;

for result in results {
    println!("Extracted: {}", result.memory);
}
```

#### Store with Full Metadata

```rust
let metadata = MemoryMetadata::new(MemoryType::Factual)
    .with_user_id("user123".to_string())
    .with_agent_id("research-assistant".to_string())
    .with_run_id("session-456".to_string())
    .with_actor_id("researcher-001".to_string())
    .with_role("assistant".to_string())
    .with_importance_score(0.9)
    .with_entities(vec!["Rust".to_string(), "performance".to_string()])
    .with_topics(vec!["programming".to_string(), "systems".to_string()]);

let custom_metadata = serde_json::json!({
    "source": "research_paper",
    "verified": true
});

memory_manager.store(
    "Rust provides zero-cost abstractions for high-performance systems.".to_string(),
    metadata
).await?;
```

### 2. Searching Memories

#### Basic Search

```rust
use cortex_mem_core::types::Filters;

let results = memory_manager.search(
    "What programming language is the user learning?",
    &Filters::for_user("user123"),
    5
).await?;

for scored_memory in results {
    println!("Score: {:.2}", scored_memory.score);
    println!("Content: {}", scored_memory.memory.content);
}
```

#### Search with Filters

```rust
let mut filters = Filters::for_user("user123");
filters.memory_type = Some(MemoryType::Personal);
filters.min_importance = Some(0.7);
filters.entities = Some(vec!["preference".to_string()]);

let results = memory_manager.search(
    "What are the user's preferences?",
    &filters,
    10
).await?;
```

#### Search with Similarity Threshold

```rust
// Only return highly similar results
let results = memory_manager.search_with_threshold(
    "What does the user like?",
    &Filters::for_user("user123"),
    10,
    Some(0.80)  // Minimum similarity score
).await?;
```

#### Search with Time Range

```rust
use chrono::{Utc, Duration};

let mut filters = Filters::for_user("user123");
filters.created_after = Some(Utc::now() - Duration::days(30));  // Last 30 days

let results = memory_manager.search(
    "Recent activities",
    &filters,
    20
).await?;
```

### 3. Retrieving Memories

#### Get by ID

```rust
if let Some(memory) = memory_manager.get("memory-uuid").await? {
    println!("Content: {}", memory.content);
    println!("Type: {:?}", memory.metadata.memory_type);
    println!("Created: {}", memory.created_at);
} else {
    println!("Memory not found");
}
```

#### List Memories

```rust
let memories = memory_manager.list(
    &Filters::for_user("user123"),
    Some(50)  // Limit to 50 results
).await?;

for memory in memories {
    println!("{}: {}", memory.id, memory.content);
}
```

#### List with Multiple Filters

```rust
let filters = Filters {
    user_id: Some("user123".to_string()),
    agent_id: Some("assistant".to_string()),
    memory_type: Some(MemoryType::Conversational),
    topics: Some(vec!["programming".to_string()]),
    ..Default::default()
};

let memories = memory_manager.list(&filters, Some(100)).await?;
```

### 4. Updating Memories

#### Update Content

```rust
memory_manager.update(
    "memory-uuid",
    "Updated content with more details".to_string()
).await?;
```

#### Smart Update (with Fact Extraction)

```rust
memory_manager.smart_update(
    "memory-uuid",
    "Additional information to merge with existing memory".to_string()
).await?;
```

#### Update Metadata Only

```rust
memory_manager.update_metadata(
    "memory-uuid",
    MemoryType::Personal
).await?;
```

#### Complete Update

```rust
memory_manager.update_complete_memory(
    "memory-uuid",
    Some("New content".to_string()),              // Content
    Some(MemoryType::Factual),                    // Type
    Some(0.95),                                   // Importance
    Some(vec!["entity1".to_string()]),            // Entities
    Some(vec!["topic1".to_string()]),             // Topics
    Some(custom_metadata)                         // Custom
).await?;
```

### 5. Deleting Memories

#### Delete by ID

```rust
memory_manager.delete("memory-uuid").await?;
```

#### Batch Delete

```rust
for id in memory_ids {
    memory_manager.delete(&id).await?;
}
```

---

## Advanced Operations

### Health Check

```rust
let health = memory_manager.health_check().await?;

println!("Overall: {}", if health.overall { "Healthy" } else { "Unhealthy" });
println!("Vector Store: {}", if health.vector_store { "OK" } else { "Error" });
println!("LLM Service: {}", if health.llm_service { "OK" } else { "Error" });
```

### Statistics

```rust
let stats = memory_manager.get_stats(&Filters::for_user("user123")).await?;

println!("Total memories: {}", stats.total_count);

for (memory_type, count) in stats.by_type {
    println!("  {:?}: {}", memory_type, count);
}

for (user_id, count) in stats.by_user {
    println!("User {}: {} memories", user_id, count);
}
```

### Creating Procedural Memories

```rust
use cortex_mem_core::types::MemoryType;

let messages = vec![
    Message::user("How do I create a new Rust project?"),
    Message::assistant("Here are the steps:
        1. Run `cargo new project_name`
        2. Navigate to the directory
        3. Open in your editor")
        .with_name("Assistant"),
];

let metadata = MemoryMetadata::new(MemoryType::Procedural)
    .with_user_id("user123".to_string())
    .with_agent_id("tutorial-bot".to_string());

// This will use the procedural memory extraction prompt
let results = memory_manager.add_memory(&messages, metadata).await?;
```

---

## MemoryManager Structure

```rust
pub struct MemoryManager {
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,
    config: MemoryConfig,
    
    // Processing components
    fact_extractor: Box<dyn FactExtractor>,
    memory_updater: Box<dyn MemoryUpdater>,
    importance_evaluator: Box<dyn ImportanceEvaluator>,
    duplicate_detector: Box<dyn DuplicateDetector>,
    memory_classifier: Box<dyn MemoryClassifier>,
}
```

### Component Details

#### Fact Extractor
- Extracts facts from conversations
- Uses LLM-based extraction
- Supports multiple strategies

#### Memory Updater
- Determines memory actions (create, update, merge, delete)
- Compares new facts with existing memories
- Uses similarity thresholds

#### Importance Evaluator
- Scores memories by importance (0.0 - 1.0)
- Considers content, entities, and context
- Used for ranking search results

#### Duplicate Detector
- Finds similar memories
- Uses semantic similarity
- Can merge duplicates automatically

#### Memory Classifier
- Classifies memory type
- Extracts entities and topics
- Uses LLM for analysis

---

## Configuration Options

### MemoryConfig

```rust
pub struct MemoryConfig {
    pub max_memories: usize,                    // Max memories to keep
    pub similarity_threshold: f32,             // Similarity threshold
    pub max_search_results: usize,             // Default search limit
    pub memory_ttl_hours: Option<u64>,         // Time-to-live
    pub auto_summary_threshold: usize,         // Auto-summary trigger
    pub auto_enhance: bool,                    // Enable auto-enhancement
    pub deduplicate: bool,                     // Enable deduplication
    pub merge_threshold: f32,                  // Merge threshold
    pub search_similarity_threshold: Option<f32>, // Search threshold
}
```

### Default Values

```toml
[memory]
max_memories = 10000
similarity_threshold = 0.65
max_search_results = 50
auto_summary_threshold = 32768
auto_enhance = true
deduplicate = true
merge_threshold = 0.75
search_similarity_threshold = 0.50
```

---

## Error Handling

### Common Errors

```rust
use cortex_mem_core::error::MemoryError;

match memory_manager.get("invalid-id").await {
    Ok(Some(memory)) => println!("Found: {}", memory.content),
    Ok(None) => println!("Memory not found"),
    Err(MemoryError::NotFound { id }) => {
        eprintln!("Memory {} not found", id);
    }
    Err(MemoryError::VectorStore(e)) => {
        eprintln!("Vector store error: {}", e);
    }
    Err(MemoryError::LLM(msg)) => {
        eprintln!("LLM error: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### Validation Errors

```rust
// Empty content
match memory_manager.store("".to_string(), metadata).await {
    Err(MemoryError::Validation(msg)) => {
        eprintln!("Validation error: {}", msg);
    }
    _ => {}
}
```

---

## Best Practices

### 1. Always Filter by User

```rust
// Good: Scoped to user
let filters = Filters::for_user("user123");
let results = memory_manager.search(query, &filters, 10).await?;

// Avoid: Unscoped search in multi-tenant systems
let results = memory_manager.search(query, &Filters::new(), 10).await?;
```

### 2. Use Appropriate Memory Types

```rust
// Personal information
let metadata = MemoryMetadata::new(MemoryType::Personal)
    .with_importance_score(0.9);

// Step-by-step instructions
let metadata = MemoryMetadata::new(MemoryType::Procedural);

// General facts
let metadata = MemoryMetadata::new(MemoryType::Factual);
```

### 3. Handle Empty Results

```rust
let results = memory_manager.search(query, &filters, 10).await?;

if results.is_empty() {
    println!("No relevant memories found");
    // Provide fallback response
} else {
    // Process results
}
```

### 4. Check Health Before Operations

```rust
let health = memory_manager.health_check().await?;

if !health.overall {
    eprintln!("Memory system is not healthy");
    return Err("System unavailable".into());
}
```

### 5. Use Batching for Bulk Operations

```rust
const BATCH_SIZE: usize = 100;

for chunk in memories.chunks(BATCH_SIZE) {
    for memory in chunk {
        memory_manager.store(memory.content.clone(), metadata.clone()).await?;
    }
    // Optional: Add delay between batches
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

---

## Examples

### Complete Example: Personal Assistant

```rust
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
    types::{Filters, MemoryMetadata, MemoryType, Message},
};
use cortex_mem_config::Config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize
    let config = Config::load("config.toml")?;
    let (vector_store, llm_client) = initialize_memory_system(&config).await?;
    
    let memory_manager = Arc::new(MemoryManager::new(
        vector_store,
        llm_client,
        config.memory.clone(),
    ));
    
    // Store user preference
    let metadata = MemoryMetadata::new(MemoryType::Personal)
        .with_user_id("alice".to_string())
        .with_importance_score(0.9);
    
    let memory_id = memory_manager.store(
        "Alice prefers tea over coffee, especially green tea".to_string(),
        metadata
    ).await?;
    
    println!("Stored preference: {}", memory_id);
    
    // Retrieve preference
    let results = memory_manager.search(
        "What does Alice like to drink?",
        &Filters::for_user("alice"),
        5
    ).await?;
    
    for result in results {
        println!("Found: {} (score: {:.2})", 
            result.memory.content, 
            result.score
        );
    }
    
    Ok(())
}
```

---

## Next Steps

- [Fact Extraction](./fact-extraction.md) - How facts are extracted from conversations
- [Vector Store](./vector-store.md) - Understanding vector storage
- [Types and Data Structures](./types.md) - Memory data structures
- [Configuration](../config/memory.md) - Memory configuration options
