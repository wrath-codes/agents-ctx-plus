# Vector Search

## Overview

libSQL includes native vector search capabilities, enabling semantic similarity queries without external dependencies. This is ideal for RAG applications, recommendation systems, and semantic search.

## Vector Data Type

### F32_BLOB
```sql
-- Define a column that stores 384-dimensional vectors
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
);
```

The `F32_BLOB` type:
- Stores 32-bit floating point arrays
- Fixed dimension specified in declaration
- Efficient binary storage
- SIMD-optimized operations

## Vector Functions

### Creating Vectors
```sql
-- From array literal
INSERT INTO documents (content, embedding)
VALUES ('Hello world', vector('[0.1, 0.2, 0.3, ...]'));

-- From JSON array
INSERT INTO documents (content, embedding)
VALUES ('Hello world', vector('[0.1, 0.2, 0.3, ...]'));
```

### Distance Functions
```sql
-- Euclidean distance
SELECT vector_distance_l2(embedding, vector('[0.1, 0.2, ...]')) AS distance
FROM documents;

-- Cosine distance (1 - cosine similarity)
SELECT vector_distance_cosine(embedding, vector('[0.1, 0.2, ...]')) AS distance
FROM documents;

-- Inner product
SELECT vector_distance_inner(embedding, vector('[0.1, 0.2, ...]')) AS score
FROM documents;
```

### Top-K Search
```sql
-- Find 5 most similar documents
SELECT content, vector_distance_cosine(embedding, vector('[0.1, ...]')) AS distance
FROM documents
WHERE embedding MATCH vector('[0.1, ...]')
ORDER BY distance
LIMIT 5;
```

## Vector Indexing

### Creating Vector Indexes
```sql
-- Create vector index for fast similarity search
CREATE INDEX idx_documents_embedding ON documents(
    libsql_vector_idx(embedding, 'metric=cosine')
);
```

Index options:
- `metric`: `cosine`, `l2`, or `inner` (default: cosine)
- Index type: HNSW (Hierarchical Navigable Small World)

### Querying with Index
```sql
-- Use vector_top_k for indexed search
SELECT * FROM vector_top_k(
    'idx_documents_embedding',  -- Index name
    vector('[0.1, 0.2, ...]'),   -- Query vector
    10                            -- Top K results
);
```

## Rust API

### Storing Vectors
```rust
use libsql::Builder;

let db = Builder::new_local("vectors.db").build().await?;
let conn = db.connect()?;

// Create table with vector column
conn.execute(
    "CREATE TABLE documents (
        id INTEGER PRIMARY KEY,
        content TEXT,
        embedding F32_BLOB(384)
    )",
    (),
).await?;

// Insert with vector
let embedding: Vec<f32> = vec![0.1, 0.2, 0.3, /* ... 384 values ... */];
conn.execute(
    "INSERT INTO documents (content, embedding) VALUES (?, ?)",
    ("Hello world", embedding),
).await?;
```

### Searching Vectors
```rust
// Search for similar documents
let query_vector: Vec<f32> = generate_embedding("search query");

let mut rows = conn.query(
    "SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
     FROM documents
     ORDER BY distance
     LIMIT 5",
    [format!("{:?}", query_vector)],
).await?;

while let Some(row) = rows.next().await? {
    let content: String = row.get(0)?;
    let distance: f64 = row.get(1)?;
    println!("Content: {}, Distance: {}", content, distance);
}
```

### Batch Operations
```rust
// Insert multiple vectors efficiently
let tx = conn.transaction().await?;

for (content, embedding) in documents {
    tx.execute(
        "INSERT INTO documents (content, embedding) VALUES (?, ?)",
        (content, embedding),
    ).await?;
}

tx.commit().await?;
```

## Complete RAG Example

```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup database
    let db = Builder::new_local("rag.db").build().await?;
    let conn = db.connect()?;
    
    // Create table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS knowledge_base (
            id INTEGER PRIMARY KEY,
            content TEXT,
            embedding F32_BLOB(384),
            source TEXT
        )",
        (),
    ).await?;
    
    // Create vector index
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_kb_embedding ON knowledge_base(
            libsql_vector_idx(embedding, 'metric=cosine')
        )",
        (),
    ).await?;
    
    // Insert documents with embeddings
    let documents = vec![
        ("Rust is a systems programming language", generate_embedding("Rust...")),
        ("SQLite is an embedded database", generate_embedding("SQLite...")),
        ("Vector search enables semantic similarity", generate_embedding("Vector...")),
    ];
    
    let tx = conn.transaction().await?;
    for (content, embedding) in documents {
        tx.execute(
            "INSERT INTO knowledge_base (content, embedding, source) VALUES (?, ?, ?)",
            (content, embedding, "docs"),
        ).await?;
    }
    tx.commit().await?;
    
    // Query with natural language
    let query = "programming languages";
    let query_embedding = generate_embedding(query);
    
    let mut rows = conn.query(
        "SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
         FROM knowledge_base
         ORDER BY distance
         LIMIT 3",
        [format!("{:?}", query_embedding)],
    ).await?;
    
    println!("Query: {}", query);
    while let Some(row) = rows.next().await? {
        let content: String = row.get(0)?;
        let distance: f64 = row.get(1)?;
        println!("  {:.3}: {}", distance, content);
    }
    
    Ok(())
}

fn generate_embedding(text: &str) -> Vec<f32> {
    // Use FastEmbed or similar to generate embeddings
    // This is a placeholder
    vec![0.0; 384]
}
```

## Hybrid Search

Combine vector similarity with traditional filters:

```sql
-- Semantic search filtered by category
SELECT content, vector_distance_cosine(embedding, vector('[...]')) as distance
FROM documents
WHERE category = 'technology'
  AND created_at > date('now', '-1 month')
ORDER BY distance
LIMIT 10;
```

## Performance Considerations

### Index vs Full Scan
```sql
-- Without index: O(n) full table scan
SELECT * FROM documents
ORDER BY vector_distance_cosine(embedding, vector('[...]'))
LIMIT 5;

-- With index: O(log n) approximate search
SELECT * FROM vector_top_k('idx_documents_embedding', vector('[...]'), 5);
```

### Batch Insertions
```rust
// Much faster than individual inserts
let tx = conn.transaction().await?;
for doc in large_dataset {
    tx.execute("INSERT ...", ...).await?;
}
tx.commit().await?;
```

### Memory Usage
- Vector index requires additional memory
- Each vector: dimension × 4 bytes (f32)
- HNSW index adds ~50% overhead
- 100K vectors × 384 dims = ~150MB

## Integration with FastEmbed

```rust
use fastembed::{TextEmbedding, EmbeddingModel};
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FastEmbed
    let model = TextEmbedding::try_new(
        InitializationOptions::new(EmbeddingModel::BGESmallENV15)
    )?;
    
    // Setup Turso
    let db = Builder::new_local("search.db").build().await?;
    let conn = db.connect()?;
    
    // Generate embeddings and store
    let texts = vec!["First document", "Second document"];
    let embeddings = model.embed(texts, None)?;
    
    for (text, embedding) in texts.iter().zip(embeddings.iter()) {
        conn.execute(
            "INSERT INTO docs (content, embedding) VALUES (?, ?)",
            (*text, embedding.as_slice()),
        ).await?;
    }
    
    Ok(())
}
```

## Next Steps

- **Extensions**: [05-extensions.md](./05-extensions.md)
- **Advanced Features**: [06-advanced-features.md](./06-advanced-features.md)
- **MCP Server**: [07-mcp-server.md](./07-mcp-server.md)