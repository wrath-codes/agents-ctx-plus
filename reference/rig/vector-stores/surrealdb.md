# SurrealDB Vector Store

## Overview

SurrealDB is a multi-model database that combines document, graph, and vector capabilities. Rig's SurrealDB integration provides a flexible, real-time vector store solution.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-surrealdb = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### SurrealDB Setup

**Option 1: In-Memory (Development)**
```bash
export SURREALDB_URL="mem://"
```

**Option 2: File-based**
```bash
export SURREALDB_URL="file:///path/to/database.db"
```

**Option 3: Server**
```bash
# Start SurrealDB
docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start

export SURREALDB_URL="ws://localhost:8000"
export SURREALDB_NS="my_namespace"
export SURREALDB_DB="my_database"
export SURREALDB_USER="root"
export SURREALDB_PASS="root"
```

## Basic Usage

### Connecting to SurrealDB

```rust
use rig::providers::openai;
use rig_surrealdb::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Connect to SurrealDB
    let surreal_client = Client::new(
        &std::env::var("SURREALDB_URL")?,
        &std::env::var("SURREALDB_NS")?,
        &std::env::var("SURREALDB_DB")?,
        &std::env::var("SURREALDB_USER")?,
        &std::env::var("SURREALDB_PASS")?,
    ).await?;
    
    // Create table
    let table = surreal_client
        .table("documents", &embedding_model, 1536)
        .await?;
    
    println!("Connected to SurrealDB!");
    
    Ok(())
}
```

### Adding Documents

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
}

// Create document
let doc = Document {
    id: "doc:1".to_string(),
    title: "Rust Basics".to_string(),
    content: "Rust is a systems programming language...".to_string(),
    tags: vec!["rust", "programming"],
};

table.create(doc).await?;

// Batch insert
let docs = vec![
    Document { /* ... */ },
    Document { /* ... */ },
];

table.create_batch(docs).await?;
```

### Semantic Search

```rust
use rig::vector_store::VectorStoreIndex;

// Basic search
let results = table.search("How to learn Rust?", 5).await?;

// Search with SurrealQL filter
let results = table
    .search_with_query(
        "Rust tutorial",
        5,
        "SELECT * FROM documents WHERE tags CONTAINS 'rust'"
    )
    .await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Title: {}", result.document.title);
    println!("Tags: {:?}", result.document.tags);
}
```

## Advanced Features

### Real-time Subscriptions

```rust
// Subscribe to changes
let mut stream = table.subscribe().await?;

while let Some(event) = stream.next().await {
    match event {
        Event::Create { data } => println!("Created: {:?}", data),
        Event::Update { data } => println!("Updated: {:?}", data),
        Event::Delete { id } => println!("Deleted: {}", id),
    }
}
```

### Graph Relations

```rust
// Create related documents
table.create(doc1).await?;
table.create(doc2).await?;

// Create relation
table.relate("doc:1", "related_to", "doc:2")
    .content(json!({ "strength": 0.9 }))
    .await?;

// Search with graph traversal
let results = table
    .search_with_graph(
        "Rust concepts",
        5,
        "doc:1->related_to->documents"
    )
    .await?;
```

### Multi-Model Queries

```rust
// Combine vector search with graph and document queries
let results = surreal_client
    .query(r#"
        LET $query_vector = fn::embedding($query_text);
        
        SELECT 
            *,
            vector::similarity::cosine(embedding, $query_vector) AS score,
            ->related_to->documents AS related
        FROM documents
        WHERE tags CONTAINS 'rust'
        ORDER BY score DESC
        LIMIT 5
    "#)
    .bind(("query_text", "How does ownership work?"))
    .await?;
```

## Complete Example

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_surrealdb::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct KnowledgeNode {
    id: String,
    concept: String,
    definition: String,
    category: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Setup
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    let agent = openai_client.agent("gpt-4").build();
    
    let surreal_client = Client::new(
        "ws://localhost:8000",
        "knowledge_base",
        "concepts",
        "root",
        "root",
    ).await?;
    
    let table = surreal_client
        .table("concepts", &embedding_model, 1536)
        .await?;
    
    // Add knowledge nodes
    let concepts = vec![
        KnowledgeNode {
            id: "concept:rust".to_string(),
            concept: "Rust".to_string(),
            definition: "A systems programming language focused on safety".to_string(),
            category: "language".to_string(),
        },
        KnowledgeNode {
            id: "concept:ownership".to_string(),
            concept: "Ownership".to_string(),
            definition: "Memory management without garbage collection".to_string(),
            category: "concept".to_string(),
        },
    ];
    
    for concept in &concepts {
        table.create(concept).await?;
    }
    
    // Create relationships
    table.relate("concept:ownership", "part_of", "concept:rust").await?;
    
    // Search
    let query = "How does Rust manage memory?";
    let results = table.search(query, 3).await?;
    
    let context = results
        .iter()
        .map(|r| format!("{}: {}", r.document.concept, r.document.definition))
        .collect::<Vec<_>>()
        .join("\n");
    
    let prompt = format!("Based on:\n{}\n\nAnswer: {}", context, query);
    let response = agent.prompt(&prompt).await?;
    
    println!("Answer: {}", response);
    
    Ok(())
}
```

## Use Cases

SurrealDB is ideal for:
- **Real-time applications** with live queries
- **Graph + Vector** combined use cases
- **Multi-tenant** applications
- **Edge computing** deployments
- **Prototyping** with in-memory mode

## Next Steps

- **[Milvus](milvus.md)** - Distributed vector database
- **[ScyllaDB](scylladb.md)** - High-performance vector store
- **[Neo4j](neo4j.md)** - Graph-focused vector database