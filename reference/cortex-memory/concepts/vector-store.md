# Vector Store

Cortex Memory uses vector databases to store and retrieve memories using semantic similarity search. This document covers the vector store architecture, implementation, and usage.

---

## Overview

The Vector Store layer provides:
- **Semantic Search**: Find memories by meaning, not just keywords
- **High Performance**: Fast similarity search with vector indexing
- **Metadata Filtering**: Combine vector search with structured filters
- **Scalability**: Handle millions of memories efficiently

### Supported Backends

Currently supported vector databases:
- **Qdrant** (primary) - High-performance vector search engine
- Extensible trait system for future backends

---

## Architecture

### Vector Store Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync + dyn_clone::DynClone {
    /// Insert a memory into the vector store
    async fn insert(&self, memory: &Memory) -> Result<()>;

    /// Search for similar memories
    async fn search(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>>;

    /// Search with similarity threshold
    async fn search_with_threshold(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>>;

    /// Update an existing memory
    async fn update(&self, memory: &Memory) -> Result<()>;

    /// Delete a memory by ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// Get a memory by ID
    async fn get(&self, id: &str) -> Result<Option<Memory>>;

    /// List all memories with optional filters
    async fn list(&self, filters: &Filters, limit: Option<usize>) -> Result<Vec<Memory>>;

    /// Check if the vector store is healthy
    async fn health_check(&self) -> Result<bool>;
}
```

---

## Qdrant Implementation

### Connection Setup

```rust
use cortex_mem_core::vector_store::qdrant::QdrantVectorStore;
use cortex_mem_config::QdrantConfig;

let config = QdrantConfig {
    url: "http://localhost:6333".to_string(),
    collection_name: "cortex-memory".to_string(),
    timeout_secs: 5,
};

let store = QdrantVectorStore::new(&config).await?;
```

### Auto-Detection of Embedding Dimensions

Cortex Memory can automatically detect embedding dimensions:

```rust
// Create with LLM client for auto-detection
let store = QdrantVectorStore::new_with_llm_client(
    &config,
    &llm_client
).await?;

// The system will:
// 1. Generate a test embedding
// 2. Detect the dimension (e.g., 1536 for text-embedding-3-small)
// 3. Create/update collection with correct size
```

### Collection Management

Collections are automatically created with proper configuration:

```rust
// Collection parameters
VectorParams {
    size: 1536,              // Embedding dimension
    distance: Distance::Cosine,  // Similarity metric
    ..Default::default()
}
```

---

## Memory Storage Format

### Point Structure

Memories are stored as points in Qdrant:

```rust
PointStruct {
    id: "uuid",                    // Unique memory ID
    vector: [0.1, -0.2, ...],      // Embedding vector
    payload: {
        // Core fields
        "content": "Memory content",
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z",
        
        // Timestamps for filtering
        "created_at_ts": 1705317000000i64,
        "updated_at_ts": 1705317000000i64,
        
        // Metadata
        "user_id": "user123",
        "agent_id": "agent456",
        "run_id": "session789",
        "actor_id": "actor001",
        "role": "assistant",
        
        // Memory properties
        "memory_type": "Personal",
        "hash": "sha256_hash",
        "importance_score": 0.85,
        
        // Entities and topics (arrays)
        "entities": ["Rust", "programming"],
        "topics": ["development", "systems"],
        
        // Custom metadata
        "custom_keywords": ["memory", "safety"],
    }
}
```

---

## Search Operations

### Basic Semantic Search

```rust
// Generate query embedding
let query = "What does the user like to eat?";
let query_vector = llm_client.embed(query).await?;

// Search
let results = vector_store.search(
    &query_vector,
    &Filters::for_user("user123"),
    10
).await?;

for result in results {
    println!("Score: {:.2}, Content: {}", 
        result.score, 
        result.memory.content
    );
}
```

### Search with Threshold

Filter results by minimum similarity:

```rust
let results = vector_store.search_with_threshold(
    &query_vector,
    &filters,
    10,
    Some(0.70)  // Only return results with score >= 0.70
).await?;
```

### Filtered Search

Combine vector search with metadata filters:

```rust
let filters = Filters {
    user_id: Some("user123".to_string()),
    agent_id: Some("assistant".to_string()),
    memory_type: Some(MemoryType::Personal),
    min_importance: Some(0.7),
    entities: Some(vec!["preference".to_string()]),
    topics: Some(vec!["food".to_string()]),
    created_after: Some(DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")?),
    ..Default::default()
};

let results = vector_store.search(&query_vector, &filters, 10).await?;
```

---

## Filter Types

### Available Filters

```rust
pub struct Filters {
    // Identity filters
    pub user_id: Option<String>,      // Filter by user
    pub agent_id: Option<String>,     // Filter by agent
    pub run_id: Option<String>,       // Filter by session/run
    pub actor_id: Option<String>,     // Filter by actor
    
    // Type filters
    pub memory_type: Option<MemoryType>,  // Filter by memory type
    
    // Quality filters
    pub min_importance: Option<f32>,  // Minimum importance score
    pub max_importance: Option<f32>,  // Maximum importance score
    
    // Time filters (using millisecond timestamps)
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    
    // Content filters
    pub entities: Option<Vec<String>>,    // Must have these entities
    pub topics: Option<Vec<String>>,      // Must have these topics
    
    // Custom filters
    pub custom: HashMap<String, serde_json::Value>,
}
```

### Filter Construction Helpers

```rust
// Filter for specific user
let filters = Filters::for_user("user123");

// Filter for specific agent
let filters = Filters::for_agent("assistant-bot");

// Filter for specific run/session
let filters = Filters::for_run("session-456");

// Chain additional filters
let filters = Filters::for_user("user123")
    .with_memory_type(MemoryType::Personal);
```

---

## CRUD Operations

### Insert Memory

```rust
let memory = Memory {
    id: Uuid::new_v4().to_string(),
    content: "User prefers dark mode".to_string(),
    embedding: vec![0.1, -0.2, 0.3, ...],  // Generated by LLM
    metadata: MemoryMetadata::new(MemoryType::Personal)
        .with_user_id("user123".to_string()),
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

vector_store.insert(&memory).await?;
```

### Retrieve Memory

```rust
// Get by ID
if let Some(memory) = vector_store.get("memory-uuid").await? {
    println!("Content: {}", memory.content);
}
```

### Update Memory

```rust
// Update content and regenerate embedding
memory.content = "User prefers dark mode in all apps".to_string();
memory.embedding = llm_client.embed(&memory.content).await?;
memory.updated_at = Utc::now();

vector_store.update(&memory).await?;
```

### Delete Memory

```rust
vector_store.delete("memory-uuid").await?;
```

### List Memories

```rust
// List with filters
let memories = vector_store.list(
    &Filters::for_user("user123"),
    Some(100)  // Limit
).await?;

println!("Found {} memories", memories.len());
```

---

## Similarity Metrics

### Cosine Similarity

Cortex Memory uses cosine similarity for vector comparison:

```
similarity = (A · B) / (||A|| × ||B||)

Where:
- A · B = dot product of vectors
- ||A|| = magnitude of vector A
- ||B|| = magnitude of vector B
```

**Range**: -1.0 to 1.0
- **1.0**: Identical direction (perfect match)
- **0.0**: Orthogonal (unrelated)
- **-1.0**: Opposite direction (rare in practice)

### Score Interpretation

| Score Range | Interpretation |
|-------------|----------------|
| 0.90 - 1.00 | Very high similarity (near duplicate) |
| 0.80 - 0.90 | High similarity (strongly related) |
| 0.70 - 0.80 | Good similarity (related) |
| 0.60 - 0.70 | Moderate similarity (somewhat related) |
| 0.50 - 0.60 | Low similarity (weakly related) |
| < 0.50 | Very low similarity (unrelated) |

---

## Performance Optimization

### Indexing

Qdrant automatically indexes vectors for fast search:
- **HNSW Index**: Hierarchical Navigable Small World
- **Build Time**: During insertion
- **Search Complexity**: O(log n)

### Batch Operations

```rust
// Insert multiple memories efficiently
let points: Vec<PointStruct> = memories
    .iter()
    .map(|m| memory_to_point(m))
    .collect();

client.upsert_points(UpsertPoints {
    collection_name: collection_name.clone(),
    points,
    ..Default::default()
}).await?;
```

### Connection Pooling

```rust
// Qdrant client maintains connection pool
let client = Qdrant::from_url(&config.url)
    .build()?;
```

### Timeout Configuration

```rust
let config = QdrantConfig {
    url: "http://localhost:6333".to_string(),
    timeout_secs: 10,  // Adjust based on network latency
    ..Default::default()
};
```

---

## Scaling Considerations

### Horizontal Scaling

Qdrant supports distributed deployment:

```yaml
# docker-compose.yml for distributed Qdrant
version: '3.8'
services:
  qdrant-node-1:
    image: qdrant/qdrant
    ports:
      - "6333:6333"
    environment:
      - QDRANT__CLUSTER__ENABLED=true
      
  qdrant-node-2:
    image: qdrant/qdrant
    environment:
      - QDRANT__CLUSTER__ENABLED=true
```

### Collection Sharding

```rust
// Qdrant automatically shards collections
CreateCollection {
    collection_name: "cortex-memory".to_string(),
    shards_number: Some(6),  // Number of shards
    ..Default::default()
}
```

### Memory Estimation

Estimate storage requirements:

```
Storage per memory ≈ 
    (embedding_dim × 4 bytes) +  // Vector (f32)
    (content length) +           // Text content
    (metadata overhead)          // JSON metadata

Example for 1536-dim embeddings:
- Vector: 1536 × 4 = 6,144 bytes
- Content: ~500 bytes (average)
- Metadata: ~1,000 bytes
- Total: ~7.5 KB per memory

For 1 million memories: ~7.5 GB
```

---

## Health Monitoring

### Health Check

```rust
match vector_store.health_check().await? {
    true => println!("Vector store is healthy"),
    false => println!("Vector store is unavailable"),
}
```

### Collection Info

```rust
let info = client.collection_info(&collection_name).await?;
println!("Points count: {}", info.result.points_count);
println!("Indexed: {}", info.result.indexed_vectors_count);
```

---

## Configuration

### Qdrant Configuration

```toml
[qdrant]
url = "http://localhost:6333"           # Qdrant server URL
collection_name = "cortex-memory"       # Collection name
timeout_secs = 5                        # Request timeout

# Optional: for auto-detection
# embedding_dim is auto-detected, no need to set
```

### Environment Variables

```bash
export QDRANT_URL="http://localhost:6333"
export QDRANT_COLLECTION="cortex-memory"
```

---

## Best Practices

### 1. Use Appropriate Filters

Always filter by user/agent to ensure data isolation:

```rust
// Good: Scoped to specific user
let filters = Filters::for_user("user123");

// Avoid: Unfiltered searches in multi-tenant systems
let filters = Filters::new();
```

### 2. Set Similarity Thresholds

Adjust thresholds based on use case:

```rust
// High precision: Only very similar results
let threshold = Some(0.80);

// Balanced: Good similarity
let threshold = Some(0.65);

// High recall: Include more results
let threshold = Some(0.50);
```

### 3. Monitor Vector Store Health

```rust
// Regular health checks
match memory_manager.health_check().await? {
    HealthStatus { vector_store: true, .. } => {
        // Proceed with operations
    }
    _ => {
        // Handle unavailable vector store
    }
}
```

### 4. Batch Operations for Bulk Imports

```rust
// For bulk operations, use batches
const BATCH_SIZE: usize = 100;

for chunk in memories.chunks(BATCH_SIZE) {
    // Process batch
}
```

### 5. Index Important Metadata

Ensure frequently filtered fields are indexed:
- user_id
- agent_id
- memory_type
- created_at_ts

---

## Troubleshooting

### Common Issues

#### Connection Refused
```
Error: Vector store error: connection refused

Solution: 
- Check Qdrant is running: curl http://localhost:6333/health
- Verify URL in config.toml
- Check firewall settings
```

#### Dimension Mismatch
```
Error: Collection has dimension 1536 but expected 768

Solution:
- Use new_with_llm_client() for auto-detection
- Or manually specify embedding_dim in config
- Consider recreating collection with correct dimension
```

#### Slow Search Performance
```
Search taking too long

Solutions:
- Increase Qdrant resources (CPU/Memory)
- Enable indexing
- Use filters to reduce search space
- Consider sharding for large collections
```

#### Memory Not Found
```
Error: Memory not found: uuid

Solutions:
- Verify ID is correct
- Check if memory was deleted
- Ensure using correct collection
```

---

## Next Steps

- [Memory Pipeline](./memory-pipeline.md) - Understand how memories flow through the system
- [Architecture Overview](./architecture.md) - System architecture overview
- [Configuration](../config/qdrant.md) - Qdrant-specific configuration
