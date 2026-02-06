# FastEmbed Models

Complete reference of all supported embedding and reranking models.

## 📊 Model Categories

FastEmbed supports 4 types of models:

1. **Text Embedding Models** (30+) - Dense vector representations
2. **Sparse Text Embedding Models** (2) - Sparse vectors for hybrid search
3. **Image Embedding Models** (5) - Vision embeddings
4. **Reranker Models** (4) - Re-ranking for better relevance

## 📝 Text Embedding Models

### Default Model

**BAAI/bge-small-en-v1.5** (Recommended for most use cases)
- Dimensions: 384
- Size: ~100MB
- Speed: Very Fast
- Quality: Good
- Best for: General purpose, resource-constrained environments

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

// This uses BGE-Small by default
let model = TextEmbedding::try_new(Default::default())?;
```

### BGE Series (Best Overall)

| Model | Enum | Dimensions | Size | Speed | Best For |
|-------|------|-----------|------|-------|----------|
| BGE-Small | `BGESmallENV15` | 384 | ~100MB | Very Fast | Balanced |
| BGE-Base | `BGEBaseENV15` | 768 | ~300MB | Fast | Better quality |
| BGE-Large | `BGELargeENV15` | 1024 | ~1GB | Medium | Best quality |
| BGE-M3 | `BGEM3` | 1024 | ~2GB | Medium | Multilingual |

```rust
// BGE-Base
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::BGEBaseENV15)
)?;

// BGE-Large (best quality)
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::BGELargeENV15)
)?;
```

### Sentence Transformers (Fast & Light)

| Model | Enum | Dimensions | Size | Speed |
|-------|------|-----------|------|-------|
| All-MiniLM-L6 | `AllMiniLML6V2` | 384 | ~80MB | Very Fast |
| All-MiniLM-L12 | `AllMiniLML12V2` | 384 | ~100MB | Fast |
| All-MPNet-Base | `AllMPNetBaseV2` | 768 | ~300MB | Fast |
| Paraphrase-MiniLM | `ParaphraseMiniLML12V2` | 384 | ~100MB | Fast |
| Paraphrase-MPNet | `ParaphraseMultilingualMPNetBaseV2` | 768 | ~300MB | Fast |

```rust
// Fastest option
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::AllMiniLML6V2)
)?;
```

### Multilingual Models

| Model | Enum | Dimensions | Languages | Best For |
|-------|------|-----------|-----------|----------|
| Multilingual-E5-Small | `MultilingualE5Small` | 384 | 100+ | Fast multilingual |
| Multilingual-E5-Base | `MultilingualE5Base` | 768 | 100+ | Balanced |
| Multilingual-E5-Large | `MultilingualE5Large` | 1024 | 100+ | Best quality |
| BGE-Small-ZH | `BGESmallZHV15` | 512 | Chinese | Chinese text |
| BGE-Large-ZH | `BGELargeZHV15` | 1024 | Chinese | Chinese text |

```rust
// Multilingual support
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::MultilingualE5Base)
)?;
```

### Nomic Models (Long Context)

| Model | Enum | Dimensions | Context | Best For |
|-------|------|-----------|---------|----------|
| Nomic-Embed-Text-v1 | `NomicEmbedTextV1` | 768 | 8192 | Long documents |
| Nomic-Embed-Text-v1.5 | `NomicEmbedTextV15` | 768 | 8192 | Vision pairs |

```rust
// Long context support
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
)?;
```

### Modern & Specialized Models

| Model | Enum | Dimensions | Size | Specialization |
|-------|------|-----------|------|----------------|
| Mxbai-Embed-Large | `MxbaiEmbedLargeV1` | 1024 | ~1GB | General purpose |
| GTE-Base | `GTEBaseENV15` | 768 | ~300MB | Alibaba NLP |
| GTE-Large | `GTELargeENV15` | 1024 | ~1GB | Alibaba NLP |
| ModernBERT-Embed | `ModernBERTEmbedLarge` | 1024 | ~1GB | Modern BERT |
| Jina-Embed-V2-Code | `JinaEmbedV2BaseCode` | 768 | ~300MB | Code embeddings |
| Jina-Embed-V2-Base | `JinaEmbedV2BaseEn` | 768 | ~300MB | English |
| Snowflake-Arctic-XS | `SnowflakeArcticEmbedXS` | 384 | ~50MB | Ultra fast |
| Snowflake-Arctic-S | `SnowflakeArcticEmbedS` | 384 | ~100MB | Fast |
| Snowflake-Arctic-M | `SnowflakeArcticEmbedM` | 768 | ~300MB | Balanced |
| Snowflake-Arctic-L | `SnowflakeArcticEmbedL` | 1024 | ~1GB | Best quality |
| Google-Gemma-300M | `GoogleGeminiEmbeddingGemma300M` | 768 | ~1GB | Google model |
| Qwen3-0.6B | `Qwen3Embedding0_6B`* | 768 | ~600MB | Requires qwen3 feature |
| Qwen3-4B | `Qwen3Embedding4B`* | 3584 | ~4GB | Requires qwen3 feature |
| Qwen3-8B | `Qwen3Embedding8B`* | 3584 | ~8GB | Requires qwen3 feature |

*Requires `qwen3` feature flag

```rust
// Code embeddings
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::JinaEmbedV2BaseCode)
)?;
```

### Vision-Text Models

| Model | Enum | Dimensions | Pairs With |
|-------|------|-----------|------------|
| CLIP-ViT-B-32-Text | `ClipVitB32` | 512 | CLIP-ViT-B-32-Vision |

```rust
// For image-text search
let text_model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::ClipVitB32)
)?;
```

### Quantized Models

Quantized versions available (append `Q` to enum name):

```rust
// Quantized BGE-Small (smaller, faster)
let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::BGESmallENV15Q)
)?;

// Available quantized models:
// - BGESmallENV15Q
// - BGEBaseENV15Q
// - BGELargeENV15Q
// - AllMiniLML6V2Q
// - AllMiniLML12V2Q
// - And more...
```

## 🎯 Sparse Text Embedding Models

Sparse embeddings combine keyword and semantic search.

### Available Models

| Model | Enum | Best For |
|-------|------|----------|
| SPLADE-PP-en-v1 | `SPLADEPPV1` (default) | English sparse |
| BGE-M3 | `BGEM3Sparse` | Multilingual sparse |

```rust
use fastembed::{SparseTextEmbedding, SparseInitOptions, SparseModel};

// Default: SPLADE-PP-en-v1
let model = SparseTextEmbedding::try_new(Default::default())?;

// Or specify model
let model = SparseTextEmbedding::try_new(
    SparseInitOptions::new(SparseModel::BGEM3Sparse)
)?;

// Generate sparse embeddings
let embeddings = model.embed(documents, None)?;
// Returns Vec<SparseEmbedding> with indices and values
```

## 🖼️ Image Embedding Models

Generate embeddings for images (useful for image search).

### Available Models

| Model | Enum | Dimensions | Best For |
|-------|------|-----------|----------|
| CLIP-ViT-B-32-Vision | `ClipVitB32` (default) | 512 | General vision |
| ResNet50 | `Resnet50` | 2048 | Traditional CV |
| Unicom-ViT-B-16 | `UnicomVitB16` | 768 | Fine-grained |
| Unicom-ViT-B-32 | `UnicomVitB32` | 512 | General |
| Nomic-Vision-v1.5 | `NomicEmbedVisionV15` | 768 | Pairs with Nomic-Text-v1.5 |

```rust
use fastembed::{ImageEmbedding, ImageInitOptions, ImageEmbeddingModel};

// Default: CLIP-ViT-B-32
let model = ImageEmbedding::try_new(Default::default())?;

// Embed images
let images = vec!["path/to/image1.png", "path/to/image2.jpg"];
let embeddings = model.embed(images, None)?;
```

## 🔄 Reranker Models

Rerankers improve search relevance by re-ranking candidate documents.

### Available Models

| Model | Enum | Best For |
|-------|------|----------|
| BGE-Reranker-Base | `BGERerankerBase` (default) | General reranking |
| BGE-Reranker-v2-M3 | `BGERerankerV2M3` | Multilingual |
| Jina-Reranker-v1-Turbo | `JinaRerankerV1TurboEn` | Fast English |
| Jina-Reranker-v2-Base | `JinaRerankerV2BaseMultilingual` | Best multilingual |

```rust
use fastembed::{TextRerank, RerankInitOptions, RerankerModel};

// Initialize reranker
let reranker = TextRerank::try_new(Default::default())?;

// Or specific model
let reranker = TextRerank::try_new(
    RerankInitOptions::new(RerankerModel::JinaRerankerV2BaseMultilingual)
)?;

// Rerank documents
let query = "what is machine learning?";
let documents = vec![
    "Machine learning is a subset of AI.",
    "The weather is nice today.",
    "AI and ML are related fields.",
];

let results = reranker.rerank(query, documents, true, None)?;

// Results sorted by relevance
for result in results {
    println!("Score: {:.4}, Text: {}", result.score, result.text);
}
```

## 📈 Model Selection Guide

### By Use Case

**General Purpose Search**
```rust
// Balanced speed and quality
EmbeddingModel::BGESmallENV15  // 384d, fast
EmbeddingModel::BGEBaseENV15   // 768d, better
```

**High-Quality Search**
```rust
// Best quality
EmbeddingModel::BGELargeENV15           // 1024d
EmbeddingModel::MultilingualE5Large     // 1024d, multilingual
```

**Fast/Resource-Constrained**
```rust
// Fastest options
EmbeddingModel::BGESmallENV15      // 384d
EmbeddingModel::AllMiniLML6V2      // 384d, very fast
EmbeddingModel::SnowflakeArcticEmbedXS  // 384d, ultra-small
```

**Multilingual**
```rust
// Multiple languages
EmbeddingModel::MultilingualE5Base
EmbeddingModel::BGEM3
EmbeddingModel::ParaphraseMultilingualMPNetBaseV2
```

**Long Documents**
```rust
// Long context (8192 tokens)
EmbeddingModel::NomicEmbedTextV1
EmbeddingModel::NomicEmbedTextV15
```

**Code Search**
```rust
// Code embeddings
EmbeddingModel::JinaEmbedV2BaseCode
EmbeddingModel::BGEBaseENV15
```

**Image-Text Search**
```rust
// CLIP models
EmbeddingModel::ClipVitB32              // Text
ImageEmbeddingModel::ClipVitB32         // Image
```

### By Constraints

**Memory-Constrained (< 200MB)**
```rust
EmbeddingModel::SnowflakeArcticEmbedXS  // ~50MB
EmbeddingModel::AllMiniLML6V2           // ~80MB
EmbeddingModel::BGESmallENV15           // ~100MB
```

**CPU-Only, Fast Inference**
```rust
EmbeddingModel::BGESmallENV15
EmbeddingModel::AllMiniLML6V2
EmbeddingModel::SnowflakeArcticEmbedS
```

**GPU Available**
```rust
// Any model with ort-cuda feature
// Larger models benefit more:
EmbeddingModel::BGELargeENV15
EmbeddingModel::MultilingualE5Large
```

## 🎓 Model Comparison

### Embedding Quality (Approximate)

Based on MTEB (Massive Text Embedding Benchmark):

| Model | Avg Score | Retrieval | Clustering | Classification |
|-------|-----------|-----------|------------|----------------|
| BGE-Large | 64.5 | 53.5 | 46.1 | 75.1 |
| GTE-Large | 63.8 | 52.2 | 46.8 | 73.8 |
| E5-Large | 63.1 | 50.9 | 46.6 | 73.4 |
| BGE-Base | 62.3 | 48.6 | 44.7 | 72.4 |
| BGE-Small | 60.1 | 45.2 | 41.8 | 69.8 |
| MiniLM-L6 | 56.3 | 41.8 | 39.1 | 64.8 |

### Speed Comparison

Documents/second (approximate, batch size 256):

| Model | CPU (single) | CPU (batch) | GPU |
|-------|--------------|-------------|-----|
| Snowflake-XS | 5000 | 20000 | 50000 |
| MiniLM-L6 | 3000 | 12000 | 40000 |
| BGE-Small | 2000 | 8000 | 30000 |
| BGE-Base | 800 | 4000 | 20000 |
| BGE-Large | 300 | 1500 | 10000 |

## 🔗 Related Documentation

- [Architecture](../architecture/text-embedding.md) - How embeddings work
- [Usage](../usage/basic.md) - Using models in code
- [Optimization](../usage/optimization.md) - Performance tuning
- [Integration](../integration/vector-databases.md) - Using with vector DBs

## 📚 See Also

- [MTEB Leaderboard](https://huggingface.co/spaces/mteb/leaderboard) - Model rankings
- [BGE Paper](https://arxiv.org/abs/2309.07597) - BGE model details
- [Sentence Transformers](https://www.sbert.net/) - ST documentation