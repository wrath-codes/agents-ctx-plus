# SQLite Vector Store

## Overview

SQLite with vector extensions provides a lightweight, embedded vector database perfect for local applications and edge deployments.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-sqlite = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
libsqlite3-sys = { version = "0.28", features = ["bundled"] }
```

## Basic Usage

### Creating a Database

```rust
use rig::providers::openai;
use rig_sqlite::{Client, Table};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Create SQLite client
    let sqlite_client = Client::new("./data.db").await?;
    
    // Create table with vector support
    let table = sqlite_client
        .table("documents", &embedding_model, 1536)
        .await?;
    
    println!("SQLite database ready!");
    
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
}

// Insert document
table.insert(Document {
    id: "1".to_string(),
    title: "Introduction to Rust".to_string(),
    content: "Rust is a systems programming language...".to_string(),
}).await?;

// Batch insert
let docs = vec![
    Document { /* ... */ },
    Document { /* ... */ },
];

table.insert_batch(&docs).await?;
```

### Semantic Search

```rust
use rig::vector_store::VectorStoreIndex;

// Basic search
let results = table.search("How does Rust work?", 5).await?;

// Search with filter
let results = table
    .search_with_filter("Rust tutorial", 5, "title LIKE '%Rust%'")
    .await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Title: {}", result.document.title);
}
```

## Features

### Full-Text Search Integration

```rust
// Enable FTS5
table.enable_fts5(["title", "content"]).await?;

// Hybrid search (vector + fulltext)
let results = table
    .hybrid_search(
        "Rust ownership",
        5,
        0.7, // Vector weight
        0.3, // Fulltext weight
    )
    .await?;
```

### Custom Schema

```rust
// Define custom table schema
let table = sqlite_client
    .table_with_schema(
        "documents",
        &embedding_model,
        1536,
        r#"
            CREATE TABLE documents (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                embedding BLOB
            );
            CREATE INDEX idx_category ON documents(category);
        "#,
    )
    .await?;
```

### Transactions

```rust
use rig_sqlite::Transaction;

let mut tx = table.begin_transaction().await?;

try {
    for doc in documents {
        tx.insert(&doc).await?;
    }
    tx.commit().await?;
} catch {
    tx.rollback().await?;
}
```

## Best Practices

### 1. Connection Management

```rust
use std::sync::Arc;

let client = Arc::new(Client::new("./data.db").await?);

// Clone for concurrent use
let client2 = client.clone();
tokio::spawn(async move {
    let table = client2.table("docs", &model, 1536).await?;
    // Use table...
});
```

### 2. Index Optimization

```rust
// Create vector index
table.create_vector_index("idx_documents_embedding").await?;

// Vacuum for optimization
sqlite_client.vacuum().await?;
```

### 3. Backup

```rust
// Backup database
sqlite_client.backup("./data_backup.db").await?;
```

## Use Cases

SQLite is ideal for:
- **Desktop applications**
- **Mobile apps**
- **Edge devices**
- **Testing/development**
- **Single-user applications**

## Next Steps

- **[SurrealDB](surrealdb.md)** - Multi-model database
- **[LanceDB](lancedb.md)** - Local vector database
- **[RAG Systems](../examples/rag-system.md)** - Complete RAG examples