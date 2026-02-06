# Memory Types

Cortex Memory supports multiple types of memories to organize and categorize information effectively. Each type serves a different purpose and is optimized for specific use cases.

---

## Overview

```rust
pub enum MemoryType {
    Conversational,  // Conversational memories from user interactions
    Procedural,      // How-to information and procedures
    Factual,         // General factual information
    Semantic,        // Concepts and meanings
    Episodic,        // Specific events and experiences
    Personal,        // Personal preferences and characteristics
}
```

---

## Memory Type Details

### 1. Conversational

**Purpose**: Store raw conversation history and extracted facts from dialogues.

**Use Cases**:
- Chat history preservation
- Context maintenance across sessions
- Dialogue pattern analysis
- User interaction tracking

**Characteristics**:
- Most common memory type
- Automatically extracted from conversations
- Contains both user and assistant messages
- Supports fact extraction

**Example**:
```rust
let metadata = MemoryMetadata::new(MemoryType::Conversational)
    .with_user_id("user123".to_string())
    .with_agent_id("support-bot".to_string());

memory_manager.add_memory(&messages, metadata).await?;
```

**Storage Format**:
```json
{
  "content": "User asked about pricing plans",
  "memory_type": "Conversational",
  "entities": ["pricing", "plans"],
  "topics": ["sales", "inquiry"]
}
```

---

### 2. Procedural

**Purpose**: Store step-by-step instructions and how-to information.

**Use Cases**:
- Tutorial preservation
- Process documentation
- Workflow instructions
- Best practices

**Characteristics**:
- Structured step-by-step format
- Action-result pattern recognition
- Extracted from tool usage
- Supports replay and execution

**Example**:
```rust
let messages = vec![
    Message::user("How do I reset my password?"),
    Message::assistant("I'll guide you through the password reset process:
    1. Go to the login page
    2. Click 'Forgot Password'
    3. Enter your email
    4. Check your inbox for reset link
    5. Create new password")
];

let metadata = MemoryMetadata::new(MemoryType::Procedural)
    .with_user_id("user123".to_string());

memory_manager.add_memory(&messages, metadata).await?;
```

**Storage Format**:
```json
{
  "content": "Password reset procedure: 1. Navigate to login 2. Click forgot password...",
  "memory_type": "Procedural",
  "entities": ["password", "reset", "email"],
  "topics": ["authentication", "security"]
}
```

**Procedural Memory System Prompt**:
```
You are a Procedural Memory System specialized in creating comprehensive 
step-by-step action records from tool/agent interactions.

For each interaction, create a detailed procedural memory that includes:
1. Action Description: What action was performed
2. Context: Why this action was taken
3. Steps: Detailed breakdown of the procedure
4. Results: Outcomes and observations
5. Dependencies: Prerequisites and requirements
```

---

### 3. Factual

**Purpose**: Store objective facts about the world, entities, or relationships.

**Use Cases**:
- Knowledge base building
- Entity relationships
- Domain-specific facts
- Reference information

**Characteristics**:
- Objective and verifiable
- Entity-centric
- Relationship mapping
- Supports inference

**Example**:
```rust
let metadata = MemoryMetadata::new(MemoryType::Factual)
    .with_user_id("user123".to_string())
    .with_entities(vec!["Rust", "programming language".to_string()]);

memory_manager.store(
    "Rust is a systems programming language focused on safety and performance.".to_string(),
    metadata
).await?;
```

**Storage Format**:
```json
{
  "content": "Rust guarantees memory safety without garbage collection",
  "memory_type": "Factual",
  "entities": ["Rust", "memory safety", "garbage collection"],
  "topics": ["programming", "systems"]
}
```

---

### 4. Semantic

**Purpose**: Store conceptual understanding and meanings.

**Use Cases**:
- Concept definitions
- Category hierarchies
- Meaning associations
- Abstract relationships

**Characteristics**:
- Conceptual rather than specific
- Supports abstraction
- Category-based organization
- Meaning preservation

**Example**:
```rust
let metadata = MemoryMetadata::new(MemoryType::Semantic)
    .with_user_id("user123".to_string())
    .with_topics(vec!["programming concepts".to_string()]);

memory_manager.store(
    "Ownership in Rust represents a unique reference to data with clear lifetime rules.".to_string(),
    metadata
).await?;
```

**Storage Format**:
```json
{
  "content": "Ownership: unique reference with deterministic cleanup",
  "memory_type": "Semantic",
  "entities": ["ownership", "reference", "lifetime"],
  "topics": ["concepts", "memory management"]
}
```

---

### 5. Episodic

**Purpose**: Store specific events and experiences.

**Use Cases**:
- Event tracking
- Experience logging
- Timeline construction
- Historical records

**Characteristics**:
- Time-stamped
- Event-specific
- Context-rich
- Temporal ordering

**Example**:
```rust
let metadata = MemoryMetadata::new(MemoryType::Episodic)
    .with_user_id("user123".to_string())
    .with_run_id("session-456".to_string());

memory_manager.store(
    "User successfully deployed their first Rust application to production.".to_string(),
    metadata
).await?;
```

**Storage Format**:
```json
{
  "content": "First production deployment completed at 2024-01-15 14:30",
  "memory_type": "Episodic",
  "entities": ["deployment", "production"],
  "topics": ["milestone", "devops"],
  "created_at": "2024-01-15T14:30:00Z"
}
```

---

### 6. Personal

**Purpose**: Store user preferences, characteristics, and personal information.

**Use Cases**:
- User preferences
- Personal details
- Preference learning
- Personalization

**Characteristics**:
- User-specific
- Preference-oriented
- Long-term relevance
- High importance score

**Example**:
```rust
let metadata = MemoryMetadata::new(MemoryType::Personal)
    .with_user_id("user123".to_string())
    .with_importance_score(0.9);

memory_manager.store(
    "User prefers dark mode in all applications and dislikes pop-up notifications.".to_string(),
    metadata
).await?;
```

**Storage Format**:
```json
{
  "content": "Prefers dark mode, dislikes pop-ups, favorite language is Rust",
  "memory_type": "Personal",
  "entities": ["dark mode", "notifications", "Rust"],
  "topics": ["preferences", "settings"],
  "importance_score": 0.9
}
```

---

## Memory Type Selection Guidelines

### When to Use Each Type

| Type | Use When | Example |
|------|----------|---------|
| **Conversational** | Storing dialogue history | Chat logs, Q&A sessions |
| **Procedural** | Documenting processes | Tutorials, workflows, how-tos |
| **Factual** | Storing objective knowledge | Facts, definitions, relationships |
| **Semantic** | Capturing concepts | Abstractions, meanings, categories |
| **Episodic** | Recording specific events | Milestones, experiences, incidents |
| **Personal** | Learning user preferences | Likes, dislikes, personal details |

### Auto-Classification

Cortex Memory can automatically classify memory types:

```rust
// Enable auto-enhancement in config
[memory]
auto_enhance = true

// The system will:
// 1. Analyze content
// 2. Extract entities and topics
// 3. Classify memory type using LLM
// 4. Evaluate importance
// 5. Check for duplicates
```

**Classification Logic**:
- **Personal**: Contains "I", "my", "prefer", "like", "dislike"
- **Procedural**: Contains "how to", "steps", "first", "then", "finally"
- **Factual**: Contains "is", "are", "was", "were" with objective statements
- **Episodic**: Contains time references, event descriptions
- **Semantic**: Contains conceptual language, abstractions
- **Conversational**: Default for dialogue content

---

## Memory Type Filtering

### Search by Type

```rust
use cortex_mem_core::types::{Filters, MemoryType};

// Filter for only personal memories
let mut filters = Filters::for_user("user123");
filters.memory_type = Some(MemoryType::Personal);

let results = memory_manager.search(
    "What does the user prefer?",
    &filters,
    10
).await?;
```

### List by Type

```bash
# CLI: List only procedural memories
cortex-mem-cli list --user-id "user123" --memory-type procedural

# API: Filter by memory type
curl -X GET "http://localhost:8000/memories?user_id=user123&memory_type=Personal"
```

---

## Memory Type Statistics

Get statistics about memory types:

```rust
let filters = Filters::for_user("user123");
let stats = memory_manager.get_stats(&filters).await?;

println!("Total memories: {}", stats.total_count);
for (memory_type, count) in stats.by_type {
    println!("  {:?}: {}", memory_type, count);
}
```

Example output:
```
Total memories: 150
  Conversational: 80
  Personal: 35
  Factual: 20
  Procedural: 10
  Episodic: 3
  Semantic: 2
```

---

## Best Practices

### 1. Choose Appropriate Types
- Use **Personal** for user preferences (high importance)
- Use **Procedural** for reusable instructions
- Use **Factual** for objective knowledge
- Use **Episodic** for time-sensitive events

### 2. Leverage Auto-Enhancement
```toml
[memory]
auto_enhance = true  # Enable automatic classification
```

### 3. Combine with Metadata
```rust
let metadata = MemoryMetadata::new(MemoryType::Personal)
    .with_user_id("user123".to_string())
    .with_importance_score(0.9)  // High importance for personal info
    .with_entities(vec!["preference".to_string(), "setting".to_string()])
    .with_topics(vec!["ui".to_string(), "ux".to_string()]);
```

### 4. Use Type-Specific Prompts
Different memory types use specialized extraction prompts:
- **User Memory Extraction**: Focus on personal facts
- **Assistant Memory Extraction**: Focus on capabilities and knowledge
- **Procedural Memory Extraction**: Focus on steps and actions

---

## Next Steps

- [Memory Pipeline](./memory-pipeline.md) - Learn how memories are processed
- [Fact Extraction](../core/fact-extraction.md) - Understand automatic extraction
- [Configuration](../config/memory.md) - Configure memory type behavior
