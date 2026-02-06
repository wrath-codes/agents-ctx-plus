# Quick Start Guide

This guide will walk you through building a simple application with Cortex Memory in just a few minutes.

---

## Scenario: Personal AI Assistant

Let's build a simple personal assistant that remembers user preferences and facts about them.

### Step 1: Set Up Your Environment

Create a new Rust project:

```bash
cargo new my-memory-app
cd my-memory-app
```

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
cortex-mem-core = "1.0"
cortex-mem-config = "1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

Create a `config.toml` file:

```toml
[qdrant]
url = "http://localhost:6333"
collection_name = "personal-assistant"

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_efficient = "gpt-4o-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_name = "text-embedding-3-small"

[memory]
auto_enhance = true
deduplicate = true
```

### Step 2: Basic Memory Operations

Create `src/main.rs`:

```rust
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
    types::{Filters, MemoryMetadata, MemoryType},
};
use cortex_mem_config::Config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load("config.toml")?;
    
    // Initialize the memory system
    let (vector_store, llm_client) = initialize_memory_system(&config).await?;
    
    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(
        vector_store,
        llm_client,
        config.memory.clone(),
    ));
    
    println!("✅ Memory system initialized!");
    
    // Example user interactions
    let user_id = "alice";
    
    // Store some facts about the user
    store_user_facts(&memory_manager, user_id).await?;
    
    // Query memories
    query_memories(&memory_manager, user_id).await?;
    
    Ok(())
}

async fn store_user_facts(
    memory_manager: &Arc<MemoryManager>,
    user_id: &str,
) -> anyhow::Result<()> {
    println!("\n📝 Storing user facts...");
    
    let facts = vec![
        "Alice is a software engineer who loves Rust programming",
        "Alice prefers dark mode in all applications",
        "Alice has a dog named Max who is 3 years old",
        "Alice enjoys hiking on weekends",
        "Alice is allergic to peanuts",
    ];
    
    for fact in facts {
        let metadata = MemoryMetadata::new(MemoryType::Personal)
            .with_user_id(user_id.to_string());
        
        let memory_id = memory_manager.store(fact.to_string(), metadata).await?;
        println!("  ✅ Stored: {} (ID: {})", &fact[..40.min(fact.len())], memory_id);
    }
    
    Ok(())
}

async fn query_memories(
    memory_manager: &Arc<MemoryManager>,
    user_id: &str,
) -> anyhow::Result<()> {
    println!("\n🔍 Querying memories...");
    
    let queries = vec![
        "What programming language does Alice like?",
        "Does Alice have any pets?",
        "What are Alice's preferences?",
        "What should I know about Alice's health?",
    ];
    
    for query in queries {
        println!("\n  Question: {}", query);
        
        let filters = Filters::for_user(user_id);
        let results = memory_manager.search(query, &filters, 3).await?;
        
        if results.is_empty() {
            println!("    No relevant memories found");
        } else {
            for (i, scored_memory) in results.iter().enumerate() {
                println!(
                    "    {}. [{}] {}",
                    i + 1,
                    (scored_memory.score * 100.0) as u8,
                    scored_memory.memory.content
                );
            }
        }
    }
    
    Ok(())
}
```

### Step 3: Run Your Application

```bash
cargo run
```

You should see output showing memories being stored and retrieved!

---

## Advanced Example: Conversation Memory

Now let's add conversation history and automatic fact extraction:

```rust
use cortex_mem_core::types::{Message, MemoryMetadata, MemoryType};

async fn process_conversation(
    memory_manager: &Arc<MemoryManager>,
    user_id: &str,
) -> anyhow::Result<()> {
    println!("\n💬 Processing conversation...");
    
    // Simulate a conversation
    let messages = vec![
        Message::user("Hi! I'm planning a trip to Japan next month."),
        Message::assistant("That sounds exciting! Japan is beautiful in spring. Do you have any specific cities in mind?")
            .with_name("Assistant"),
        Message::user("I want to visit Tokyo and Kyoto. I love sushi and ramen!"),
        Message::assistant("Great choices! Tokyo has amazing sushi restaurants in Tsukiji area, and Kyoto is known for traditional kaiseki cuisine.")
            .with_name("Assistant"),
        Message::user("Thanks! I also need to remember to book my flight by next week."),
    ];
    
    // Add conversation to memory
    let metadata = MemoryMetadata::new(MemoryType::Conversational)
        .with_user_id(user_id.to_string())
        .with_agent_id("travel-assistant".to_string());
    
    let results = memory_manager.add_memory(&messages, metadata).await?;
    
    println!("  ✅ Extracted {} facts from conversation:", results.len());
    for result in results {
        println!("    - {}", result.memory);
    }
    
    // Now query for specific information
    let queries = vec![
        "Where is the user traveling?",
        "What food does the user like?",
        "What does the user need to do by next week?",
    ];
    
    println!("\n🔍 Querying extracted facts...");
    for query in queries {
        println!("\n  Question: {}", query);
        
        let filters = Filters::for_user(user_id);
        let results = memory_manager.search(query, &filters, 2).await?;
        
        for scored_memory in results {
            println!("    Answer: {}", scored_memory.memory.content);
        }
    }
    
    Ok(())
}
```

---

## Using the REST API

Instead of embedding the library, you can use the REST API:

### Start the Service

```bash
cortex-mem-service --config config.toml
```

### Store Memories via API

```bash
curl -X POST http://localhost:8000/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Bob is learning to play the guitar",
    "user_id": "bob",
    "memory_type": "personal"
  }'
```

### Search Memories via API

```bash
curl -X POST http://localhost:8000/memories/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is Bob learning?",
    "filters": {
      "user_id": "bob"
    },
    "limit": 5
  }'
```

---

## Using the CLI

For quick operations, use the command-line interface:

```bash
# Add a memory
cortex-mem-cli add \
  --content "Charlie works as a data scientist at TechCorp" \
  --user-id "charlie" \
  --memory-type "personal"

# Search memories
cortex-mem-cli search \
  --query "Where does Charlie work?" \
  --user-id "charlie" \
  --limit 3

# List all memories for a user
cortex-mem-cli list \
  --user-id "charlie" \
  --limit 20
```

---

## Next Steps

- [Explore Memory Types](../concepts/memory-types.md) - Learn about different types of memories
- [Configuration Guide](./configuration.md) - Customize Cortex Memory for your needs
- [Integration Examples](../examples/basic.md) - See more complex use cases
