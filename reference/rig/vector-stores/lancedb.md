# LanceDB Vector Store

## Overview

LanceDB is a serverless, low-latency vector database for AI applications. Rig's LanceDB integration enables local and embedded vector search.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-lancedb = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Basic Usage

### Creating a Table

```rust
use rig::providers::openai;
use rig_lancedb::{Client, Table};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: String,
    text: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Create LanceDB client
    let client = Client::new("./data").await?;
    
    // Create table
    let table = client
        .table("documents", &embedding_model)
        .await?;
    
    println!("Table ready!");
    
    Ok(())
}
```

### Adding Documents

```rust
// Single document
table.add(Document {
    id: "1".to_string(),
    text: "Rust is a systems programming language...".to_string(),
}).await?;

// Batch
let docs = vec![
    Document {
        id: "1".to_string(),
        text: "Rust is safe...".to_string(),
    },
    Document {
        id: "2".to_string(),
        text: "Rust is fast...".to_string(),
    },
];

table.add_batch(docs).await?;
```

### Searching

```rust
use rig::vector_store::VectorStoreIndex;

// Basic search
let results = table.search("How is Rust safe?", 5).await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Text: {}", result.document.text);
}
```

## Features

- **Local Storage**: No server required
- **Fast Search**: ANN indexing
- **Persistent**: Data stored on disk
- **Embeddings**: Automatic embedding generation

## Use Cases

Perfect for:
- Local development
- Desktop applications
- Edge deployments
- Prototyping