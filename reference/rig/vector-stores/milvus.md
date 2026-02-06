# Milvus Vector Store

## Overview

Milvus is a high-performance, distributed vector database designed for enterprise-scale AI applications. It supports billion-scale vector search with high availability.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-milvus = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Milvus Setup

**Option 1: Docker Compose (Local)**
```yaml
# docker-compose.yml
version: '3.5'
services:
  etcd:
    image: quay.io/coreos/etcd:v3.5.5
  minio:
    image: minio/minio:RELEASE.2023-03-20T20-16-18Z
  milvus:
    image: milvusdb/milvus:v2.3.3
    ports:
      - "19530:19530"
```

**Option 2: Zilliz Cloud (Managed)**
```bash
export MILVUS_URI="https://your-cluster.zillizcloud.com"
export MILVUS_TOKEN="your-token"
```

**Option 3: Local Standalone**
```bash
docker run -p 19530:19530 \
    -v $(pwd)/milvus_data:/var/lib/milvus \
    milvusdb/milvus:latest
```

## Basic Usage

### Connecting to Milvus

```rust
use rig::providers::openai;
use rig_milvus::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Connect to Milvus
    let milvus_client = Client::new(
        &std::env::var("MILVUS_URI")?,
        std::env::var("MILVUS_TOKEN").ok(),
    ).await?;
    
    // Create collection
    let collection = milvus_client
        .collection("documents", &embedding_model, 1536)
        .await?;
    
    println!("Connected to Milvus!");
    
    Ok(())
}
```

### Defining Schema

```rust
use rig_milvus::FieldSchema;

// Define collection schema
let schema = vec![
    FieldSchema::primary_int64("id"),
    FieldSchema::varchar("title", 512),
    FieldSchema::varchar("content", 65535),
    FieldSchema::varchar("category", 64),
    FieldSchema::float_vector("embedding", 1536),
];

collection.create_with_schema(schema).await?;
```

### Adding Entities

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: i64,
    title: String,
    content: String,
    category: String,
}

// Insert single document
collection.insert(Document {
    id: 1,
    title: "Rust Basics".to_string(),
    content: "Rust is a systems programming language...".to_string(),
    category: "programming".to_string(),
}).await?;

// Batch insert
let docs = vec![
    Document { /* ... */ },
    Document { /* ... */ },
];

collection.insert_batch(docs).await?;
```

### Semantic Search

```rust
use rig::vector_store::VectorStoreIndex;

// Basic search
let results = collection.search("How to learn Rust?", 5).await?;

// Search with expression filter
let results = collection
    .search_with_expr("Rust tutorial", 5, "category == 'programming'")
    .await?;

// Partition search
let results = collection
    .search_in_partition("Rust guide", "programming", 5)
    .await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Title: {}", result.document.title);
}
```

## Advanced Features

### Partition Management

```rust
// Create partition
collection.create_partition("programming").await?;
collection.create_partition("science").await?;

// Load specific partitions
collection.load_partitions(["programming", "science"]).await?;

// Search within partition
let results = collection
    .search_in_partition("Rust", "programming", 10)
    .await?;
```

### Index Management

```rust
// Create IVF_FLAT index
collection.create_index(
    "embedding",
    IndexParams::ivf_flat()
        .nlist(128)
        .metric_type(MetricType::Cosine),
).await?;

// Create HNSW index for faster search
collection.create_index(
    "embedding",
    IndexParams::hnsw()
        .m(16)
        .ef_construction(200)
        .metric_type(MetricType::Cosine),
).await?;

// Load collection into memory
collection.load().await?;
```

### Hybrid Search

```rust
// Sparse + Dense vectors
use rig_milvus::SparseVector;

let sparse_vector = SparseVector::new(
    vec![0, 100, 500],     // indices
    vec![0.5, 0.3, 0.2],   // values
);

let results = collection
    .hybrid_search(
        "Rust ownership",
        sparse_vector,
        5,
        0.7, // Dense weight
        0.3, // Sparse weight
    )
    .await?;
```

### Multi-Vector Search

```rust
// Search with multiple vectors (reranking)
let queries = vec![
    "Rust tutorial",
    "Rust programming guide",
    "Learn Rust",
];

let results = collection
    .batch_search(&queries, 5)
    .await?;
```

## Production Deployment

### Cluster Setup

```rust
// Connect to Milvus cluster
let client = Client::new("http://milvus-proxy:19530", None).await?;

// Configure replication
collection.set_replicas(3).await?;
```

### Performance Tuning

```rust
// Search with custom params
let results = collection
    .search_with_params(
        "query",
        5,
        SearchParams::new()
            .ef(128)           // HNSW search depth
            .nprobe(128)       // IVF clusters to search
            .round_decimal(4),  // Result precision
    )
    .await?;
```

### Monitoring

```rust
// Get collection statistics
let stats = collection.get_stats().await?;
println!("Entity count: {}", stats.row_count);
println!("Index type: {}", stats.index_type);

// Query nodes info
let nodes = client.get_query_nodes().await?;
for node in nodes {
    println!("Node: {}, Load: {}", node.id, node.load);
}
```

## Complete Example

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_milvus::{Client, IndexParams, MetricType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Article {
    id: i64,
    title: String,
    content: String,
    category: String,
    views: i64,
}

struct ArticleStore {
    collection: rig_milvus::Collection,
    agent: rig::Agent,
}

impl ArticleStore {
    async fn new() -> Result<Self, anyhow::Error> {
        let openai_client = openai::Client::from_env();
        let embedding_model = openai_client.embedding_model("text-embedding-3-small");
        let agent = openai_client.agent("gpt-4").build();
        
        let milvus_client = Client::new(
            &std::env::var("MILVUS_URI")?,
            std::env::var("MILVUS_TOKEN").ok(),
        ).await?;
        
        let collection = milvus_client
            .collection("articles", &embedding_model, 1536)
            .await?;
        
        // Create index if not exists
        if !collection.has_index().await? {
            collection.create_index(
                "embedding",
                IndexParams::hnsw()
                    .m(16)
                    .ef_construction(200)
                    .metric_type(MetricType::Cosine),
            ).await?;
        }
        
        collection.load().await?;
        
        Ok(Self { collection, agent })
    }
    
    async fn add_article(&self, article: Article) -> Result<(), anyhow::Error> {
        self.collection.insert(article).await?;
        Ok(())
    }
    
    async fn search_popular(
        &self,
        query: &str,
        min_views: i64,
        limit: usize,
    ) -> Result<Vec<SearchResult<Article>>, anyhow::Error> {
        let filter = format!("views >= {}", min_views);
        
        self.collection
            .search_with_expr(query, limit, &filter)
            .await
    }
    
    async fn summarize_category(&self, category: &str) -> Result<String, anyhow::Error> {
        let results = self.collection
            .search_in_partition("*", category, 10)
            .await?;
        
        let content = results
            .iter()
            .map(|r| format!("{}: {}", r.document.title, r.document.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let prompt = format!(
            "Summarize the following {} articles:\n\n{}",
            category, content
        );
        
        self.agent.prompt(&prompt).await
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let store = ArticleStore::new().await?;
    
    // Add articles
    store.add_article(Article {
        id: 1,
        title: "Getting Started with Rust".to_string(),
        content: "Rust is a modern systems language...".to_string(),
        category: "programming".to_string(),
        views: 10000,
    }).await?;
    
    // Search popular articles
    let popular = store.search_popular("Rust", 5000, 5).await?;
    println!("Found {} popular articles", popular.len());
    
    // Summarize category
    let summary = store.summarize_category("programming").await?;
    println!("Category summary: {}", summary);
    
    Ok(())
}
```

## Use Cases

Milvus is ideal for:
- **Enterprise-scale** vector search (billions of vectors)
- **Distributed deployments** with high availability
- **Hybrid search** (dense + sparse vectors)
- **Multi-tenant** applications with partitions
- **Real-time** recommendation systems

## Next Steps

- **[ScyllaDB](scylladb.md)** - Cassandra-compatible vector store
- **[MongoDB](mongodb.md)** - Document-based vector search
- **[Production Deployment](../deployment/production.md)** - Best practices