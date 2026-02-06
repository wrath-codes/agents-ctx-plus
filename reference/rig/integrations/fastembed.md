# FastEmbed Integration

## Overview

FastEmbed provides fast, lightweight, and accurate embeddings locally without requiring external API calls. Rig's FastEmbed integration enables on-device embedding generation.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-fastembed = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Basic Usage

### Local Embeddings

```rust
use rig::providers::fastembed;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize FastEmbed
    let client = fastembed::Client::new();
    
    // Create embedding model
    let model = client.embedding_model("BAAI/bge-small-en-v1.5");
    
    // Generate embedding
    let embedding = model.embed("Hello world").await?;
    
    println!("Dimensions: {}", embedding.dimensions);
    println!("Vector: {:?}", &embedding.vec[..5]);
    
    Ok(())
}
```

### Batch Processing

```rust
use rig::embeddings::EmbeddingModel;

let texts = vec![
    "First document",
    "Second document",
    "Third document",
];

// Batch embed
let embeddings = model.embed_batch(&texts).await?;

for (i, embedding) in embeddings.iter().enumerate() {
    println!("Document {}: {} dimensions", i, embedding.dimensions);
}
```

## Available Models

| Model | Dimensions | Best For |
|-------|------------|----------|
| BAAI/bge-small-en-v1.5 | 384 | General purpose, fast |
| BAAI/bge-base-en-v1.5 | 768 | Higher quality |
| sentence-transformers/all-MiniLM-L6-v2 | 384 | General similarity |
| intfloat/multilingual-e5-large | 1024 | Multilingual |
| BAAI/bge-large-en-v1.5 | 1024 | Best quality |

## Usage with Vector Stores

```rust
use rig::providers::fastembed;
use rig_mongodb::Client as MongoClient;

// Use FastEmbed with any vector store
let fastembed_client = fastembed::Client::new();
let embedding_model = fastembed_client.embedding_model("BAAI/bge-small-en-v1.5");

let mongodb_client = MongoClient::new(&uri).await?;
let index = mongodb_client
    .index("documents", &embedding_model, 384)  // Note: 384 dimensions
    .await?;

// Now you can add documents without API calls
index.add_document(&doc).await?;
```

## Benefits

- **No API costs** - Run locally
- **Privacy** - Data stays on device
- **Speed** - No network latency
- **Offline** - Works without internet

## Performance

```
Model: BAAI/bge-small-en-v1.5
- Size: ~100MB
- Speed: ~1000 docs/sec (CPU)
- Memory: ~500MB RAM
- Dimensions: 384
```

## Next Steps

- **[Vector Stores](../vector-stores/index.md)** - Store local embeddings
- **[SQLite](../vector-stores/sqlite.md)** - Perfect for local vector storage
- **[RAG Systems](../examples/rag-system.md)** - Build offline RAG