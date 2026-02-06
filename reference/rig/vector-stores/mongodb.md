# MongoDB Vector Store

## Overview

Rig's MongoDB integration enables semantic search and RAG (Retrieval-Augmented Generation) using MongoDB Atlas Vector Search.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-mongodb = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### MongoDB Atlas Setup

1. Create a MongoDB Atlas cluster
2. Enable Vector Search (Atlas Search)
3. Get connection string

```bash
export MONGODB_URI="mongodb+srv://user:password@cluster.mongodb.net"
```

## Basic Usage

### Creating an Index

```rust
use rig::providers::openai;
use rig_mongodb::{Client, Index};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Connect to MongoDB
    let mongodb_client = Client::new(&std::env::var("MONGODB_URI")?).await?;
    
    // Create or get index
    let index = mongodb_client
        .index("my_database", "documents", &embedding_model)
        .await?;
    
    println!("Index ready!");
    
    Ok(())
}
```

### Adding Documents

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    #[serde(rename = "_id")]
    id: String,
    title: String,
    content: String,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    author: String,
    category: String,
}

// Add documents
let docs = vec![
    Document {
        id: "1".to_string(),
        title: "Introduction to Rust".to_string(),
        content: "Rust is a systems programming language...".to_string(),
        metadata: Metadata {
            author: "Alice".to_string(),
            category: "programming".to_string(),
        },
    },
    Document {
        id: "2".to_string(),
        title: "Async Programming".to_string(),
        content: "Async/await is a concurrency pattern...".to_string(),
        metadata: Metadata {
            author: "Bob".to_string(),
            category: "programming".to_string(),
        },
    },
];

for doc in docs {
    index.add_document(&doc).await?;
}
```

### Semantic Search

```rust
use rig::vector_store::VectorStoreIndex;

// Search for documents
let results = index.search("How do I write async code?", 5).await?;

for result in results {
    println!("Score: {}", result.score);
    println!("Title: {}", result.document.title);
    println!("Content: {}", result.document.content);
    println!("---");
}
```

## RAG Implementation

### Building a RAG Agent

```rust
use rig::completion::Prompt;

async fn rag_query(
    agent: &Agent,
    index: &Index,
    query: &str,
) -> Result<String, anyhow::Error> {
    // Retrieve relevant documents
    let results = index.search(query, 3).await?;
    
    // Build context
    let context = results
        .iter()
        .map(|r| format!("{}: {}", r.document.title, r.document.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    
    // Create prompt with context
    let prompt = format!(
        r#"Context:
{}

Question: {}

Answer based on the context above."#,
        context, query
    );
    
    // Get response
    let response = agent.prompt(&prompt).await?;
    
    Ok(response)
}

// Usage
let response = rag_query(&agent, &index, "What is ownership in Rust?").await?;
println!("{}", response);
```

### Streaming RAG

```rust
async fn streaming_rag(
    agent: &Agent,
    index: &Index,
    query: &str,
) -> Result<(), anyhow::Error> {
    // Retrieve context
    let results = index.search(query, 3).await?;
    let context = format_results(&results);
    
    // Stream response
    let prompt = format!("Context:\n{}\n\nQuestion: {}", context, query);
    let mut stream = agent.stream_prompt(&prompt).await?;
    
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
    }
    
    Ok(())
}
```

## Advanced Features

### Filtering

```rust
use mongodb::bson::doc;

// Search with filter
let filter = doc! {
    "metadata.category": "programming"
};

let results = index
    .search_with_filter("Rust tutorial", 5, filter)
    .await?;
```

### Batch Operations

```rust
// Add multiple documents at once
let documents: Vec<Document> = load_documents().await?;
index.add_documents(&documents).await?;

// Search multiple queries
let queries = vec!["Rust ownership", "Async programming", "Error handling"];
for query in queries {
    let results = index.search(query, 3).await?;
    println!("Query: {}", query);
    println!("Results: {:?}", results.len());
}
```

### Document Updates

```rust
// Update document
let updated_doc = Document {
    id: "1".to_string(),
    title: "Introduction to Rust (Updated)".to_string(),
    content: "Updated content...".to_string(),
    metadata: Metadata {
        author: "Alice".to_string(),
        category: "programming".to_string(),
    },
};

index.update_document(&updated_doc).await?;

// Delete document
index.delete_document("1").await?;
```

## Best Practices

### 1. Index Management

```rust
// Check if index exists
if !index.exists().await? {
    index.create().await?;
}

// Optimize index
index.optimize().await?;
```

### 2. Error Handling

```rust
match index.add_document(&doc).await {
    Ok(_) => println!("Document added"),
    Err(e) => {
        eprintln!("Failed to add document: {}", e);
        // Retry or log
    }
}
```

### 3. Connection Pooling

```rust
use std::sync::Arc;

let client = Arc::new(Client::new(&uri).await?);

// Share across tasks
let client_clone = client.clone();
tokio::spawn(async move {
    let index = client_clone.index("db", "coll", &model).await?;
    // Use index...
});
```

## Complete Example

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_mongodb::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Article {
    #[serde(rename = "_id")]
    id: String,
    title: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Setup
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    let agent = openai_client.agent("gpt-4").build();
    
    let mongodb_client = Client::new(&std::env::var("MONGODB_URI")?).await?;
    let index = mongodb_client
        .index("knowledge_base", "articles", &embedding_model)
        .await?;
    
    // Add sample articles
    let articles = vec![
        Article {
            id: "1".to_string(),
            title: "Rust Ownership".to_string(),
            content: "Ownership is Rust's most unique feature...".to_string(),
        },
        Article {
            id: "2".to_string(),
            title: "Borrowing".to_string(),
            content: "Borrowing allows references without taking ownership...".to_string(),
        },
    ];
    
    for article in articles {
        index.add_document(&article).await?;
    }
    
    // RAG query
    let query = "How does ownership work in Rust?";
    let results = index.search(query, 3).await?;
    
    let context = results
        .iter()
        .map(|r| format!("{}: {}", r.document.title, r.document.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    
    let prompt = format!(
        "Based on the following articles:\n\n{}\n\nAnswer: {}",
        context, query
    );
    
    let response = agent.prompt(&prompt).await?;
    println!("Answer: {}", response);
    
    Ok(())
}
```

## Next Steps

- **[LanceDB](lancedb.md)** - Local vector store
- **[Neo4j](neo4j.md)** - Graph + vector database
- **[RAG Systems](../examples/rag-system.md)** - Complete RAG examples