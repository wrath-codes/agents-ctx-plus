# RAG System Example

## Overview

This example demonstrates building a complete RAG (Retrieval-Augmented Generation) system with Rig.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    RAG System                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  User Query                                             │
│     │                                                   │
│     ▼                                                   │
│  ┌─────────────────┐                                    │
│  │ 1. Embed Query │                                    │
│  │   (Embedding   │                                    │
│  │    Model)      │                                    │
│  └────────┬────────┘                                    │
│           │                                             │
│           ▼                                             │
│  ┌─────────────────┐                                    │
│  │ 2. Search       │                                    │
│  │   (Vector Store)│                                    │
│  └────────┬────────┘                                    │
│           │                                             │
│           ▼                                             │
│  ┌─────────────────┐                                    │
│  │ 3. Build Context│                                    │
│  │   (Top K docs)  │                                    │
│  └────────┬────────┘                                    │
│           │                                             │
│           ▼                                             │
│  ┌─────────────────┐                                    │
│  │ 4. Generate     │                                    │
│  │   (LLM Agent)   │                                    │
│  └────────┬────────┘                                    │
│           │                                             │
│           ▼                                             │
│      Response                                           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Complete Implementation

```rust
use rig::{
    completion::Prompt,
    providers::openai,
    vector_store::VectorStoreIndex,
};
use rig_mongodb::{Client as MongoClient, Index};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Document {
    #[serde(rename = "_id")]
    id: String,
    title: String,
    content: String,
    category: String,
}

struct RagSystem {
    agent: Arc<openai::Agent>,
    index: Arc<Index>,
}

impl RagSystem {
    async fn new() -> Result<Self, anyhow::Error> {
        // Initialize OpenAI
        let openai_client = openai::Client::from_env();
        let embedding_model = openai_client.embedding_model("text-embedding-3-small");
        
        // Create agent
        let agent = openai_client
            .agent("gpt-4")
            .preamble(r#"
You are a helpful assistant that answers questions based on the provided context.

Guidelines:
- Use only the information from the context
- If the answer isn't in the context, say so
- Be concise but complete
- Cite sources when possible
"#)
            .build();
        
        // Initialize MongoDB
        let mongodb_client = MongoClient::new(&std::env::var("MONGODB_URI")?).await?;
        let index = mongodb_client
            .index("rag_db", "documents", &embedding_model)
            .await?;
        
        Ok(Self {
            agent: Arc::new(agent),
            index: Arc::new(index),
        })
    }
    
    async fn add_document(&self, doc: &Document) -> Result<(), anyhow::Error> {
        self.index.add_document(doc).await?;
        Ok(())
    }
    
    async fn query(&self, question: &str, top_k: usize) -> Result<String, anyhow::Error> {
        // 1. Search for relevant documents
        let results = self.index.search(question, top_k).await?;
        
        if results.is_empty() {
            return Ok("I don't have enough information to answer that question.".to_string());
        }
        
        // 2. Build context
        let context = results
            .iter()
            .map(|r| {
                format!(
                    "[Source: {}]\nTitle: {}\nContent: {}",
                    r.document.id, r.document.title, r.document.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        
        // 3. Build prompt
        let prompt = format!(
            r#"Context:
{}

Question: {}

Please answer the question based on the context above. If the answer isn't in the context, say "I don't have enough information to answer that question."

Answer:"#,
            context, question
        );
        
        // 4. Generate response
        let response = self.agent.prompt(&prompt).await?;
        
        Ok(response)
    }
    
    async fn query_with_sources(
        &self,
        question: &str,
        top_k: usize,
    ) -> Result<(String, Vec<Document>), anyhow::Error> {
        let results = self.index.search(question, top_k).await?;
        
        let context = results
            .iter()
            .map(|r| {
                format!(
                    "[Source: {}]\nTitle: {}\nContent: {}",
                    r.document.id, r.document.title, r.document.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        
        let prompt = format!(
            r#"Context:
{}

Question: {}

Please answer the question based on the context above. Include citations like [Source: id] in your answer.

Answer:"#,
            context, question
        );
        
        let response = self.agent.prompt(&prompt).await?;
        let sources: Vec<Document> = results.iter().map(|r| r.document.clone()).collect();
        
        Ok((response, sources))
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize RAG system
    let rag = RagSystem::new().await?;
    
    // Add sample documents
    let docs = vec![
        Document {
            id: "1".to_string(),
            title: "Rust Ownership".to_string(),
            content: "Ownership is Rust's most unique feature. It enables Rust to make memory safety guarantees without needing a garbage collector.".to_string(),
            category: "rust".to_string(),
        },
        Document {
            id: "2".to_string(),
            title: "Borrowing".to_string(),
            content: "Borrowing allows you to have references to data without taking ownership. This is done through references, which are indicated by the & symbol.".to_string(),
            category: "rust".to_string(),
        },
        Document {
            id: "3".to_string(),
            title: "Structs in Rust".to_string(),
            content: "Structs are custom data types that let you package together multiple related values. They can represent objects with named fields.".to_string(),
            category: "rust".to_string(),
        },
    ];
    
    println!("Adding documents...");
    for doc in &docs {
        rag.add_document(doc).await?;
    }
    println!("Documents added!\n");
    
    // Interactive queries
    let questions = vec![
        "What is ownership in Rust?",
        "How does borrowing work?",
        "Explain structs",
        "What is the capital of France?",
    ];
    
    for question in questions {
        println!("Question: {}", question);
        println!("-" .repeat(50));
        
        let (answer, sources) = rag.query_with_sources(question, 2).await?;
        
        println!("Answer: {}", answer);
        println!("\nSources:");
        for source in sources {
            println!("  - {}: {}", source.id, source.title);
        }
        println!("\n");
    }
    
    Ok(())
}
```

## Running the Example

```bash
# Set environment variables
export OPENAI_API_KEY="sk-..."
export MONGODB_URI="mongodb+srv://..."

# Run
cargo run --example rag_system
```

## Expected Output

```
Adding documents...
Documents added!

Question: What is ownership in Rust?
--------------------------------------------------
Answer: Ownership is Rust's most unique feature. It enables Rust to make memory safety guarantees without needing a garbage collector.

Sources:
  - 1: Rust Ownership

Question: How does borrowing work?
--------------------------------------------------
Answer: Borrowing allows you to have references to data without taking ownership. This is done through references, which are indicated by the & symbol.

Sources:
  - 2: Borrowing

Question: What is the capital of France?
--------------------------------------------------
Answer: I don't have enough information to answer that question.

Sources:
  (none)
```

## Key Components

1. **Document Structure**: Defines your document schema
2. **RagSystem**: Encapsulates agent and vector store
3. **add_document**: Indexes documents for retrieval
4. **query**: Performs RAG pipeline
5. **query_with_sources**: Returns answer with citations

## Advanced Features

### Category Filtering

```rust
async fn query_by_category(
    &self,
    question: &str,
    category: &str,
) -> Result<String, anyhow::Error> {
    use mongodb::bson::doc;
    
    let filter = doc! { "category": category };
    let results = self.index
        .search_with_filter(question, 5, filter)
        .await?;
    
    // ... rest of query logic
}
```

### Streaming Responses

```rust
async fn streaming_query(&self, question: &str) -> Result<(), anyhow::Error> {
    let results = self.index.search(question, 3).await?;
    let context = build_context(&results);
    
    let prompt = format!("Context:\n{}\n\nQuestion: {}", context, question);
    let mut stream = self.agent.stream_prompt(&prompt).await?;
    
    print!("Answer: ");
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
    }
    println!();
    
    Ok(())
}
```

## Next Steps

- **[Multi-Agent System](multi-agent.md)** - Complex agent workflows
- **[Streaming](streaming.md)** - Real-time responses
- **[MongoDB](../vector-stores/mongodb.md)** - Vector store details