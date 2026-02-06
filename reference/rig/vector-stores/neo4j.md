# Neo4j Vector Store

## Overview

Neo4j is a graph database that combines vector search with graph relationships. Rig's Neo4j integration enables knowledge graphs with semantic search capabilities.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-neo4j = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Neo4j Setup

1. Install Neo4j (Desktop or Server)
2. Enable the GDS (Graph Data Science) library
3. Create a database with vector index support

```bash
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="password"
```

## Basic Usage

### Connecting to Neo4j

```rust
use rig::providers::openai;
use rig_neo4j::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize OpenAI for embeddings
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    
    // Connect to Neo4j
    let neo4j_client = Client::new(
        &std::env::var("NEO4J_URI")?,
        &std::env::var("NEO4J_USER")?,
        &std::env::var("NEO4J_PASSWORD")?,
    ).await?;
    
    // Create vector index
    let index = neo4j_client
        .index("document_vectors", &embedding_model, 1536)
        .await?;
    
    println!("Connected to Neo4j!");
    
    Ok(())
}
```

### Adding Documents with Relationships

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: String,
    title: String,
    content: String,
    category: String,
}

// Add document as node
let doc = Document {
    id: "1".to_string(),
    title: "Rust Ownership".to_string(),
    content: "Ownership is Rust's most unique feature...".to_string(),
    category: "programming".to_string(),
};

index.add_node(&doc, "Document").await?;

// Create relationships
index.add_relationship("1", "2", "RELATED_TO").await?;
index.add_relationship("1", "3", "PART_OF", json!({"chapter": 1})).await?;
```

### Semantic Search with Graph

```rust
use rig::vector_store::VectorStoreIndex;

// Basic vector search
let results = index.search("How does ownership work?", 5).await?;

// Search with graph traversal
let results = index
    .search_with_traversal(
        "Rust memory safety",
        5,
        "MATCH (d:Document)-[:RELATED_TO]->(related) RETURN related"
    )
    .await?;

for result in results {
    println!("Document: {}", result.document.title);
    println!("Score: {}", result.score);
    
    // Get related documents through graph
    if let Some(related) = result.related {
        println!("Related: {:?}", related);
    }
}
```

## Graph-Enhanced RAG

### Knowledge Graph RAG

```rust
async fn graph_rag_query(
    agent: &Agent,
    index: &Index,
    query: &str,
) -> Result<String, anyhow::Error> {
    // 1. Vector search
    let vector_results = index.search(query, 3).await?;
    
    // 2. Graph exploration
    let graph_context = index
        .query_graph(r#"
            MATCH (d:Document)<-[:MENTIONS]-(entity:Entity)
            WHERE d.id IN $doc_ids
            RETURN entity.name, entity.type, count(*) as mentions
            ORDER BY mentions DESC
            LIMIT 10
        "#)
        .param("doc_ids", vector_results.iter().map(|r| &r.document.id).collect::<Vec<_>>())
        .await?;
    
    // 3. Combine contexts
    let context = format!(
        "Documents:\n{}\n\nKey Entities:\n{}",
        format_documents(&vector_results),
        format_entities(&graph_context)
    );
    
    // 4. Generate response
    let prompt = format!(
        "Context:\n{}\n\nQuestion: {}\n\nAnswer:",
        context, query
    );
    
    agent.prompt(&prompt).await
}
```

## Advanced Features

### Hybrid Search (Vector + Fulltext)

```rust
// Create fulltext index
index.create_fulltext_index("document_text", ["title", "content"]).await?;

// Hybrid search
let results = index
    .hybrid_search(
        "Rust ownership model",
        5,
        0.7, // vector weight
        0.3, // fulltext weight
    )
    .await?;
```

### Cypher Query Integration

```rust
// Custom Cypher queries
let results = index
    .query(r#"
        MATCH (d:Document)
        WHERE d.category = $category
        WITH d, vector.similarity.cosine(d.embedding, $query_vector) as score
        RETURN d, score
        ORDER BY score DESC
        LIMIT $limit
    "#)
    .param("category", "programming")
    .param("query_vector", query_embedding.vec)
    .param("limit", 5)
    .await?;
```

### Path Finding

```rust
// Find paths between documents
let paths = index
    .find_paths("doc_1", "doc_10", "RELATED_TO", 4)
    .await?;

for path in paths {
    println!("Path: {:?}", path.nodes);
    println!("Length: {}", path.length);
}
```

## Best Practices

### 1. Index Management

```rust
// Check if index exists
if !index.exists().await? {
    index.create_vector_index("document_vectors", 1536).await?;
}

// Optimize index
index.optimize().await?;
```

### 2. Batch Operations

```rust
// Batch insert with relationships
let documents = load_documents().await?;
let relationships = load_relationships().await?;

index.add_nodes_batch(&documents, "Document").await?;
index.add_relationships_batch(&relationships).await?;
```

### 3. Transaction Management

```rust
use rig_neo4j::Transaction;

let mut tx = index.begin_transaction().await?;

try {
    tx.add_node(&doc1, "Document").await?;
    tx.add_node(&doc2, "Document").await?;
    tx.add_relationship("1", "2", "RELATED_TO").await?;
    
    tx.commit().await?;
} catch {
    tx.rollback().await?;
}
```

## Complete Example: Knowledge Base

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_neo4j::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Concept {
    id: String,
    name: String,
    description: String,
    domain: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Setup
    let openai_client = openai::Client::from_env();
    let embedding_model = openai_client.embedding_model("text-embedding-3-small");
    let agent = openai_client.agent("gpt-4").build();
    
    let neo4j_client = Client::new(
        &std::env::var("NEO4J_URI")?,
        &std::env::var("NEO4J_USER")?,
        &std::env::var("NEO4J_PASSWORD")?,
    ).await?;
    
    let index = neo4j_client
        .index("concepts", &embedding_model, 1536)
        .await?;
    
    // Add concepts with relationships
    let concepts = vec![
        Concept {
            id: "rust".to_string(),
            name: "Rust".to_string(),
            description: "A systems programming language focused on safety and performance".to_string(),
            domain: "programming".to_string(),
        },
        Concept {
            id: "ownership".to_string(),
            name: "Ownership".to_string(),
            description: "Rust's memory management system without garbage collection".to_string(),
            domain: "programming".to_string(),
        },
        Concept {
            id: "borrowing".to_string(),
            name: "Borrowing".to_string(),
            description: "References to data without taking ownership".to_string(),
            domain: "programming".to_string(),
        },
    ];
    
    for concept in &concepts {
        index.add_node(concept, "Concept").await?;
    }
    
    // Create relationships
    index.add_relationship("ownership", "rust", "PART_OF").await?;
    index.add_relationship("borrowing", "ownership", "RELATED_TO").await?;
    
    // Query with graph context
    let query = "How does Rust manage memory?";
    let results = index.search(query, 3).await?;
    
    // Get related concepts
    let mut context = String::new();
    for result in &results {
        context.push_str(&format!("{}: {}\n", 
            result.document.name, 
            result.document.description
        ));
        
        // Find related concepts
        let related = index
            .query(r#"
                MATCH (c:Concept {id: $id})-[:RELATED_TO|PART_OF]-(related)
                RETURN related.name, related.description
            "#)
            .param("id", &result.document.id)
            .await?;
        
        for rel in related {
            context.push_str(&format!("  Related: {}\n", rel.name));
        }
    }
    
    let prompt = format!(
        "Based on these concepts:\n\n{}\n\nAnswer: {}",
        context, query
    );
    
    let response = agent.prompt(&prompt).await?;
    println!("Answer: {}", response);
    
    Ok(())
}
```

## Use Cases

Neo4j is ideal for:
- **Knowledge graphs** with semantic search
- **Recommendation systems** using graph relationships
- **Entity resolution** with vector similarity
- **Multi-hop reasoning** across connected documents
- **Hierarchical data** with vector search

## Next Steps

- **[Qdrant](qdrant.md)** - High-performance vector database
- **[SQLite](sqlite.md)** - Embedded vector store
- **[RAG Systems](../examples/rag-system.md)** - Complete RAG examples