# Qdrant Vector Store

## Overview

Qdrant is a high-performance vector similarity search engine with extended filtering support. It's designed for production-scale AI applications.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-qdrant = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Qdrant Setup

**Option 1: Cloud (Qdrant Cloud)**
```bash
export QDRANT_URL="https://your-cluster.qdrant.io"
export QDRANT_API_KEY="your-api-key"
```

**Option 2: Local (Docker)**
```bash
docker run -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage \
    qdrant/qdrant
```

```bash
export QDRANT_URL="http://localhost:6333"
```

## Basic Usage

### Connecting to Qdrant

```rust
use rig::providers::openai;
use rig_qdrant::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Connect to Qdrant
    let qdrant_client = Client::new(
        &std::env::var("QDRANT_URL")?,
        std::env::var("QDRANT_API_KEY").ok(),
    ).await?;
    
    // Create or open collection
    let collection = qdrant_client
        .collection("documents", &embedding_model, 1536)
        .await?;
    
    println!("Connected to Qdrant!");
    
    Ok(())
}
```

### Adding Points

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: String,
    title: String,
    content: String,
    category: String,
    tags: Vec<String>,
}

// Single document
collection.add_point(Document {
    id: "1".to_string(),
    title: "Rust Basics".to_string(),
    content: "Rust is a systems programming language...".to_string(),
    category: "programming".to_string(),
    tags: vec!["rust", "tutorial"],
}).await?;

// Batch upload
let documents = vec![
    Document { /* ... */ },
    Document { /* ... */ },
];

collection.add_points_batch(documents).await?;
```

### Semantic Search

```rust
use rig::vector_store::VectorStoreIndex;

// Basic search
let results = collection.search("How to learn Rust?", 5).await?;

// Search with filtering
use qdrant_client::qdrant::Filter;

let filter = Filter::must(["category".eq("programming")]);

let results = collection
    .search_with_filter("Rust tutorial", 5, filter)
    .await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Title: {}", result.document.title);
    println!("Category: {}", result.document.category);
}
```

## Advanced Features

### Payload Filtering

```rust
use qdrant_client::qdrant::{Condition, Filter};

// Multiple conditions
let filter = Filter::must([
    Condition::matches("category", "programming"),
    Condition::matches("tags", "rust"),
]);

let results = collection
    .search_with_filter("memory management", 10, filter)
    .await?;

// Range filters
let filter = Filter::must([
    Condition::range("created_at", Range {
        gt: Some(1609459200.0),  // After Jan 1, 2021
        lt: None,
    }),
]);
```

### Hybrid Search

```rust
// Sparse vectors (BM25) + Dense vectors
let results = collection
    .hybrid_search(
        "Rust ownership",
        5,
        0.7, // Dense weight
        0.3, // Sparse weight
    )
    .await?;
```

### Recommendations

```rust
// Recommend based on positive/negative examples
let recommendations = collection
    .recommend(RecommendRequest {
        positive: vec!["doc_1", "doc_2"],
        negative: vec!["doc_3"],
        limit: 5,
        filter: Some(category_filter),
    })
    .await?;
```

### Scroll API

```rust
// Iterate through all points
let mut offset = None;

loop {
    let (points, next_offset) = collection
        .scroll(ScrollRequest {
            limit: 100,
            offset,
            filter: Some(filter.clone()),
        })
        .await?;
    
    for point in points {
        process_document(point);
    }
    
    if next_offset.is_none() {
        break;
    }
    offset = next_offset;
}
```

## Collection Management

### Creating Collections

```rust
// Create with configuration
use qdrant_client::qdrant::{VectorsConfig, Distance};

let config = CollectionConfig {
    vectors: VectorsConfig {
        size: 1536,
        distance: Distance::Cosine,
        hnsw_config: Some(HnswConfigDiff {
            m: Some(16),
            ef_construct: Some(100),
            ..Default::default()
        }),
    },
    optimizers: OptimizersConfigDiff {
        indexing_threshold: Some(10000),
        ..Default::default()
    },
};

collection.create_with_config(config).await?;
```

### Collection Operations

```rust
// Get collection info
let info = collection.info().await?;
println!("Points count: {}", info.points_count);

// Delete collection
collection.delete().await?;

// Optimize
collection.optimize().await?;
```

## Production Deployment

### Clustering

```rust
// Connect to cluster
let client = Client::new("http://qdrant-node-1:6333", None).await?;

// Replication factor
let config = CollectionConfig {
    replication_factor: Some(3),
    write_consistency_factor: Some(2),
    ..Default::default()
};
```

### Performance Tuning

```rust
// Index configuration
let hnsw_config = HnswConfigDiff {
    m: Some(32),              // Higher = better recall, slower
    ef_construct: Some(200),  // Higher = better index quality
    ef: Some(128),            // Search time trade-off
    ..Default::default()
};

// Search with custom ef
let results = collection
    .search_with_params("query", 5, SearchParams {
        hnsw_ef: Some(256),
        exact: Some(false),
    })
    .await?;
```

## Complete Example

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_qdrant::{Client, Collection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Article {
    id: String,
    title: String,
    content: String,
    category: String,
    author: String,
    created_at: i64,
}

struct ArticleStore {
    collection: Collection,
    agent: rig::Agent,
}

impl ArticleStore {
    async fn new() -> Result<Self, anyhow::Error> {
        let openai_client = openai::Client::from_env();
        let embedding_model = openai_client.embedding_model("text-embedding-3-small");
        let agent = openai_client.agent("gpt-4").build();
        
        let qdrant_client = Client::new(
            &std::env::var("QDRANT_URL")?,
            std::env::var("QDRANT_API_KEY").ok(),
        ).await?;
        
        let collection = qdrant_client
            .collection("articles", &embedding_model, 1536)
            .await?;
        
        Ok(Self { collection, agent })
    }
    
    async fn add_article(&self, article: Article) -> Result<(), anyhow::Error> {
        self.collection.add_point(article).await?;
        Ok(())
    }
    
    async fn search_by_category(
        &self,
        query: &str,
        category: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult<Article>>, anyhow::Error> {
        use qdrant_client::qdrant::{Condition, Filter};
        
        let filter = Filter::must([Condition::matches("category", category)]);
        
        self.collection
            .search_with_filter(query, limit, filter)
            .await
    }
    
    async fn answer_question(&self, question: &str) -> Result<String, anyhow::Error> {
        // Search across all categories
        let results = self.collection.search(question, 5).await?;
        
        let context = results
            .iter()
            .map(|r| format!("{}: {}", r.document.title, r.document.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let prompt = format!(
            "Based on these articles:\n\n{}\n\nAnswer: {}",
            context, question
        );
        
        self.agent.prompt(&prompt).await
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let store = ArticleStore::new().await?;
    
    // Add sample articles
    store.add_article(Article {
        id: "1".to_string(),
        title: "Getting Started with Rust".to_string(),
        content: "Rust is a modern systems programming language...".to_string(),
        category: "tutorial".to_string(),
        author: "Alice".to_string(),
        created_at: chrono::Utc::now().timestamp(),
    }).await?;
    
    // Search tutorials
    let tutorials = store
        .search_by_category("beginner guide", "tutorial", 3)
        .await?;
    
    println!("Found {} tutorials", tutorials.len());
    
    // Ask question
    let answer = store.answer_question("How do I start with Rust?").await?;
    println!("Answer: {}", answer);
    
    Ok(())
}
```

## Use Cases

Qdrant is ideal for:
- **Large-scale semantic search** (millions of vectors)
- **Real-time recommendations**
- **Multi-tenant applications**
- **Dynamic filtering** with payload conditions
- **High-throughput** production systems

## Next Steps

- **[SQLite](sqlite.md)** - Embedded vector database
- **[SurrealDB](surrealdb.md)** - Multi-model database
- **[RAG Systems](../examples/rag-system.md)** - Complete RAG examples