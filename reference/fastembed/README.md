# FastEmbed - Quick Start Guide

> **Rust library for generating vector embeddings and reranking locally**

## ⚡ Quick Start

### 1. Add Dependency

```toml
# Cargo.toml
[dependencies]
fastembed = "5"
```

### 2. Basic Text Embedding

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

fn main() -> anyhow::Result<()> {
    // Initialize with default model (BAAI/bge-small-en-v1.5)
    let model = TextEmbedding::try_new(Default::default())?;
    
    // Prepare documents
    let documents = vec![
        "Hello, World!",
        "This is an example.",
        "FastEmbed is great for embeddings.",
    ];
    
    // Generate embeddings
    let embeddings = model.embed(documents, None)?;
    
    println!("Generated {} embeddings", embeddings.len());
    println!("Embedding dimension: {}", embeddings[0].len());
    
    Ok(())
}
```

### 3. Run It

```bash
cargo run
```

## 🎯 What You Get

```
Generated 3 embeddings
Embedding dimension: 384
```

Each embedding is a vector of 384 floating-point numbers representing the semantic meaning of the text.

## 🔧 Customization

### Use Different Model

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::BGEBaseENV15)
        .with_show_download_progress(true),
)?;
```

### Query vs Passage Prefixes

```rust
let documents = vec![
    "query: What is machine learning?",  // For search queries
    "passage: Machine learning is a subset of AI.",  // For documents
];

let embeddings = model.embed(documents, None)?;
```

### Batch Processing

```rust
// Process 1000 documents with batch size 512
let embeddings = model.embed(large_document_set, Some(512))?;
```

## 📊 Available Models

### Text Embeddings (Default: bge-small)

| Model | Dimensions | Size | Use Case |
|-------|-----------|------|----------|
| BGE-Small | 384 | ~100MB | Fast, good quality |
| BGE-Base | 768 | ~300MB | Balanced |
| BGE-Large | 1024 | ~1GB | Best quality |
| MiniLM-L6 | 384 | ~80MB | Very fast |
| E5-Large | 1024 | ~1GB | Multilingual |

### Other Capabilities

```rust
// Sparse embeddings (for keyword + semantic)
use fastembed::{SparseTextEmbedding, SparseInitOptions};
let sparse_model = SparseTextEmbedding::try_new(Default::default())?;

// Image embeddings
use fastembed::{ImageEmbedding, ImageInitOptions};
let image_model = ImageEmbedding::try_new(Default::default())?;

// Reranking
use fastembed::{TextRerank, RerankInitOptions};
let reranker = TextRerank::try_new(Default::default())?;
```

## 🚀 Common Use Cases

### Semantic Search

```rust
// 1. Index your documents
let doc_embeddings = model.embed(documents.clone(), None)?;
// Store in vector database...

// 2. Search
let query = vec!["query: your search terms"];
let query_embedding = model.embed(query, None)?;
// Find similar vectors in database...
```

### RAG (Retrieval-Augmented Generation)

```rust
// 1. Retrieve relevant documents
let query_embedding = model.embed(vec![user_question], None)?;
let relevant_docs = vector_db.search(&query_embedding[0], 5);

// 2. Use with LLM
let context = format!("Based on: {:?}", relevant_docs);
// Send context + question to LLM...
```

### Similarity Comparison

```rust
use ndarray::Array;

let embeddings = model.embed(vec!["doc1", "doc2"], None)?;

// Calculate cosine similarity
let similarity = cosine_similarity(&embeddings[0], &embeddings[1]);
println!("Similarity: {}", similarity);
```

## 💡 Best Practices

### DO ✅

```rust
// Use batch processing for multiple documents
let embeddings = model.embed(documents, None)?;

// Add prefixes for better results
"query: user question"
"passage: document content"

// Handle errors properly
match model.embed(documents, None) {
    Ok(embeddings) => { /* use embeddings */ },
    Err(e) => eprintln!("Error: {}", e),
}
```

### DON'T ❌

```rust
// Don't embed one at a time in a loop
for doc in documents {
    let emb = model.embed(vec![doc], None)?; // Slow!
}

// Don't forget error handling
let embeddings = model.embed(docs, None).unwrap(); // May panic
```

## 🔗 Next Steps

- [Full Documentation](index.md) - Complete reference
- [Models](models/text-models.md) - All supported models
- [Usage Examples](usage/basic.md) - More examples
- [API Docs](https://docs.rs/fastembed) - Rust documentation

## 📦 Installation Options

### Basic

```toml
fastembed = "5"
```

### With GPU (CUDA)

```toml
fastembed = { version = "5", features = ["ort-cuda"] }
```

### With Qwen3 Models

```toml
fastembed = { version = "5", features = ["qwen3"] }
```

## 🎓 Learning Path

1. **Start Here**: Run the quick start example above
2. **Basic Usage**: Learn about different models and options
3. **Batch Processing**: Process large document sets efficiently
4. **Integration**: Connect with vector databases
5. **Optimization**: Tune for your specific use case

## 🆘 Troubleshooting

### Model Download Slow?

```rust
// Show download progress
InitOptions::new(model).with_show_download_progress(true)
```

### Out of Memory?

```rust
// Use smaller model
EmbeddingModel::BGESmallENV15  // 384d, ~100MB

// Or reduce batch size
model.embed(docs, Some(64))  // Instead of 256
```

### First Run Slow?

```rust
// Model is being downloaded and cached
// Subsequent runs will be much faster
```

## 🌟 Why FastEmbed?

- ✅ **Fast**: ONNX runtime + Rust = speed
- ✅ **Local**: No cloud dependencies
- ✅ **Easy**: Simple API, no async needed
- ✅ **Proven**: Production-ready models
- ✅ **Free**: Apache 2.0 license