# FastEmbed Usage Guide

Practical examples and patterns for using FastEmbed effectively.

## 🚀 Basic Usage

### Simple Text Embedding

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

fn main() -> anyhow::Result<()> {
    // Initialize model
    let model = TextEmbedding::try_new(Default::default())?;
    
    // Single document
    let doc = vec!["Hello, World!"];
    let embedding = model.embed(doc, None)?;
    
    println!("Dimensions: {}", embedding[0].len());
    println!("First 5 values: {:?}", &embedding[0][0..5]);
    
    Ok(())
}
```

### Multiple Documents

```rust
let documents = vec![
    "The quick brown fox",
    "jumps over the lazy dog",
    "Machine learning is fascinating",
];

let embeddings = model.embed(documents, None)?;

for (i, emb) in embeddings.iter().enumerate() {
    println!("Document {}: {} dimensions", i, emb.len());
}
```

### With Query/Passage Prefixes

```rust
// Recommended: Use prefixes for better results
let documents = vec![
    "query: What is Rust programming language?",
    "passage: Rust is a systems programming language.",
    "passage: It focuses on safety and performance.",
];

let embeddings = model.embed(documents, None)?;
```

## 📦 Batch Processing

### Default Batch Size

```rust
// Default batch size is 256
let large_dataset = (0..1000)
    .map(|i| format!("Document number {}", i))
    .collect::<Vec<_>>();

// Automatically batched
let embeddings = model.embed(large_dataset, None)?;
```

### Custom Batch Size

```rust
// Smaller batches for memory-constrained environments
let embeddings = model.embed(documents, Some(64))?;

// Larger batches for maximum throughput
let embeddings = model.embed(documents, Some(512))?;
```

### Processing Large Files

```rust
use std::fs::File;
use std::io::{BufRead, BufReader};

fn process_large_file(path: &str) -> anyhow::Result<Vec<Embedding>> {
    let model = TextEmbedding::try_new(Default::default())?;
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let mut batch = Vec::new();
    let mut all_embeddings = Vec::new();
    
    for line in reader.lines() {
        batch.push(line?);
        
        if batch.len() >= 256 {
            let embeddings = model.embed(batch.clone(), None)?;
            all_embeddings.extend(embeddings);
            batch.clear();
        }
    }
    
    // Process remaining
    if !batch.is_empty() {
        let embeddings = model.embed(batch, None)?;
        all_embeddings.extend(embeddings);
    }
    
    Ok(all_embeddings)
}
```

## 🎯 Advanced Patterns

### Semantic Search

```rust
use ndarray::Array;

fn semantic_search(
    model: &TextEmbedding,
    documents: &[String],
    query: &str,
    top_k: usize,
) -> anyhow::Result<Vec<(usize, f32)>> {
    // Embed documents
    let doc_embeddings = model.embed(documents.to_vec(), None)?;
    
    // Embed query
    let query_doc = vec![format!("query: {}", query)];
    let query_embedding = model.embed(query_doc, None)?;
    
    // Calculate similarities
    let query_vec = Array::from(query_embedding[0].clone());
    let mut scores: Vec<(usize, f32)> = doc_embeddings
        .iter()
        .enumerate()
        .map(|(idx, emb)| {
            let doc_vec = Array::from(emb.clone());
            let similarity = cosine_similarity(&query_vec, &doc_vec);
            (idx, similarity)
        })
        .collect();
    
    // Sort by similarity
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scores.truncate(top_k);
    
    Ok(scores)
}

fn cosine_similarity(a: &Array<f32, ndarray::Ix1>, b: &Array<f32, ndarray::Ix1>) -> f32 {
    let dot = a.dot(b);
    let norm_a = a.dot(a).sqrt();
    let norm_b = b.dot(b).sqrt();
    dot / (norm_a * norm_b)
}
```

### Document Clustering

```rust
use ndarray::{Array2, Axis};

fn cluster_documents(
    model: &TextEmbedding,
    documents: &[String],
    n_clusters: usize,
) -> anyhow::Result<Vec<usize>> {
    // Generate embeddings
    let embeddings = model.embed(documents.to_vec(), None)?;
    
    // Convert to ndarray
    let n_samples = embeddings.len();
    let n_features = embeddings[0].len();
    let flat: Vec<f32> = embeddings.into_iter().flatten().collect();
    let data = Array2::from_shape_vec((n_samples, n_features), flat)?;
    
    // Apply k-means (using your preferred clustering library)
    // let clusters = kmeans(&data, n_clusters)?;
    
    Ok(vec![]) // Placeholder
}
```

### Similarity Matrix

```rust
fn similarity_matrix(embeddings: &[Embedding]) -> Vec<Vec<f32>> {
    let n = embeddings.len();
    let mut matrix = vec![vec![0.0; n]; n];
    
    for i in 0..n {
        for j in i..n {
            let sim = cosine_similarity(
                &Array::from(embeddings[i].clone()),
                &Array::from(embeddings[j].clone()),
            );
            matrix[i][j] = sim;
            matrix[j][i] = sim;
        }
    }
    
    matrix
}
```

## 🔄 Reranking Example

```rust
use fastembed::{TextRerank, RerankInitOptions, RerankerModel};

fn rerank_results(
    query: &str,
    candidates: Vec<String>,
) -> anyhow::Result<Vec<String>> {
    let reranker = TextRerank::try_new(Default::default())?;
    
    let results = reranker.rerank(
        &format!("query: {}", query),
        candidates,
        true,  // return documents
        None,  // default batch size
    )?;
    
    // Results already sorted by relevance score
    let ranked: Vec<String> = results
        .into_iter()
        .map(|r| r.text)
        .collect();
    
    Ok(ranked)
}
```

## 🖼️ Image Embeddings

```rust
use fastembed::{ImageEmbedding, ImageInitOptions, ImageEmbeddingModel};

fn image_search_example() -> anyhow::Result<()> {
    let model = ImageEmbedding::try_new(Default::default())?;
    
    // Embed images
    let images = vec![
        "path/to/cat.png",
        "path/to/dog.jpg",
        "path/to/bird.png",
    ];
    
    let embeddings = model.embed(images, None)?;
    
    println!("Image embeddings: {}", embeddings.len());
    println!("Dimensions per image: {}", embeddings[0].len());
    
    Ok(())
}
```

## 🎯 Sparse Embeddings

```rust
use fastembed::{SparseTextEmbedding, SparseInitOptions, SparseModel};

fn sparse_embedding_example() -> anyhow::Result<()> {
    let model = SparseTextEmbedding::try_new(Default::default())?;
    
    let documents = vec![
        "Machine learning and artificial intelligence",
        "Deep learning neural networks",
    ];
    
    let embeddings = model.embed(documents, None)?;
    
    for (i, emb) in embeddings.iter().enumerate() {
        println!("Document {}:", i);
        println!("  Indices: {:?}", emb.indices);
        println!("  Values: {:?}", emb.values);
    }
    
    Ok(())
}
```

## ⚡ Optimization Tips

### 1. Use Appropriate Batch Size

```rust
// For small datasets (< 1000 docs)
let embeddings = model.embed(docs, None)?;  // Default 256

// For medium datasets (1K - 10K docs)
let embeddings = model.embed(docs, Some(512))?;

// For large datasets (> 10K docs)
let embeddings = model.embed(docs, Some(1024))?;
```

### 2. Reuse Model Instance

```rust
// ✅ Good: Reuse model
let model = TextEmbedding::try_new(Default::default())?;
for batch in batches {
    let embeddings = model.embed(batch, None)?;
}

// ❌ Bad: Create model each time
for batch in batches {
    let model = TextEmbedding::try_new(Default::default())?;  // Slow!
    let embeddings = model.embed(batch, None)?;
}
```

### 3. Enable GPU Acceleration

```toml
# Cargo.toml
[dependencies]
fastembed = { version = "5", features = ["ort-cuda"] }
```

### 4. Cache Models Locally

Models are automatically cached in:
- Linux/macOS: `~/.cache/fastembed/`
- Windows: `%USERPROFILE%\.cache\fastembed\`

### 5. Use Smaller Models for Development

```rust
// Fast iteration
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::BGESmallENV15)
)?;
```

## 🐛 Error Handling

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

fn robust_embedding() {
    // Handle initialization errors
    let model = match TextEmbedding::try_new(Default::default()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model: {}", e);
            return;
        }
    };
    
    // Handle embedding errors
    match model.embed(vec!["test"], None) {
        Ok(embeddings) => {
            println!("Success: {} embeddings", embeddings.len());
        }
        Err(e) => {
            eprintln!("Embedding failed: {}", e);
        }
    }
}
```

## 📊 Performance Benchmarks

### Throughput (docs/second)

| Model | Batch Size | CPU | GPU (CUDA) |
|-------|-----------|-----|------------|
| BGE-Small | 256 | ~2000 | ~8000 |
| BGE-Base | 256 | ~800 | ~4000 |
| BGE-Large | 256 | ~300 | ~1500 |

### Memory Usage

| Model | Model Size | Runtime Memory |
|-------|-----------|----------------|
| BGE-Small | ~100MB | ~300MB |
| BGE-Base | ~300MB | ~600MB |
| BGE-Large | ~1GB | ~1.5GB |

### Latency (single document)

| Model | Cold Start | Warm |
|-------|-----------|------|
| BGE-Small | ~500ms | ~5ms |
| BGE-Base | ~800ms | ~10ms |
| BGE-Large | ~1200ms | ~20ms |

## 🔗 Related Documentation

- [Models](../models/text-models.md) - All available models
- [Configuration](../configuration/init-options.md) - Configuration options
- [Integration](../integration/vector-databases.md) - Vector DB integration
- [API Reference](https://docs.rs/fastembed) - Rust API docs

## 📚 Example Projects

### Semantic Search CLI

```rust
// search.rs
use fastembed::TextEmbedding;

fn main() {
    let model = TextEmbedding::try_new(Default::default()).unwrap();
    
    // Load documents from file
    let docs = std::fs::read_to_string("docs.txt")
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    
    // Create search index
    let embeddings = model.embed(docs.clone(), None).unwrap();
    
    // Search loop
    loop {
        let query = read_line("Query: ");
        let query_emb = model.embed(vec![query], None).unwrap();
        
        // Find most similar
        let results = find_similar(&query_emb[0], &embeddings, 5);
        
        for (idx, score) in results {
            println!("{:.4}: {}", score, &docs[idx]);
        }
    }
}
```

### RAG Pipeline

```rust
// rag.rs
struct RAGSystem {
    embedding_model: TextEmbedding,
    documents: Vec<String>,
    doc_embeddings: Vec<Embedding>,
}

impl RAGSystem {
    fn new(docs: Vec<String>) -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(Default::default())?;
        let embeddings = model.embed(docs.clone(), None)?;
        
        Ok(Self {
            embedding_model: model,
            documents: docs,
            doc_embeddings: embeddings,
        })
    }
    
    fn query(&self, question: &str) -> anyhow::Result<String> {
        // 1. Embed question
        let query_emb = self.embedding_model.embed(
            vec![format!("query: {}", question)],
            None
        )?;
        
        // 2. Find relevant documents
        let relevant = self.find_relevant(&query_emb[0], 3);
        
        // 3. Format context
        let context = relevant.join("\n\n");
        
        Ok(context)
    }
}
```