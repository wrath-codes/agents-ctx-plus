# fastembed — Sub-Index

> Rust embedding models with ONNX runtime (4 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [⚡ Quick Start](README.md#quick-start) · [🎯 What You Get](README.md#what-you-get) · [🔧 Customization](README.md#customization) · [📊 Available Models](README.md#available-models) · [🚀 Common Use Cases](README.md#common-use-cases) · [💡 Best Practices](README.md#best-practices) · [🔗 Next Steps](README.md#next-steps) · [📦 Installation Options](README.md#installation-options) · +3 more|
|[DOCUMENTATION_SUMMARY.md](DOCUMENTATION_SUMMARY.md)|Documentation overview|
| |↳ [📚 Documentation Complete](DOCUMENTATION_SUMMARY.md#documentation-complete) · [📁 Documentation Structure](DOCUMENTATION_SUMMARY.md#documentation-structure) · [🎯 What's Documented](DOCUMENTATION_SUMMARY.md#whats-documented) · [🚀 Quick Reference](DOCUMENTATION_SUMMARY.md#quick-reference) · [📊 Model Selection](DOCUMENTATION_SUMMARY.md#model-selection) · [🎯 Usage Patterns](DOCUMENTATION_SUMMARY.md#usage-patterns) · [📈 Performance](DOCUMENTATION_SUMMARY.md#performance) · [🔗 External Resources](DOCUMENTATION_SUMMARY.md#external-resources) · +6 more|

### [models](models/)

|file|description|
|---|---|
|[text-models.md](models/text-models.md)|Text models — BAAI/bge, all-MiniLM, JINA, nomic|
| |↳ [📊 Model Categories](models/text-models.md#model-categories) · [📝 Text Embedding Models](models/text-models.md#text-embedding-models) · [🎯 Sparse Text Embedding Models](models/text-models.md#sparse-text-embedding-models) · [🖼️ Image Embedding Models](models/text-models.md#image-embedding-models) · [🔄 Reranker Models](models/text-models.md#reranker-models) · [📈 Model Selection Guide](models/text-models.md#model-selection-guide) · [🎓 Model Comparison](models/text-models.md#model-comparison) · [🔗 Related Documentation](models/text-models.md#related-documentation) · +1 more|

### [usage](usage/)

|file|description|
|---|---|
|[basic.md](usage/basic.md)|Basic usage — TextEmbedding::try_new(), embed()|
| |↳ [🚀 Basic Usage](usage/basic.md#basic-usage) · [📦 Batch Processing](usage/basic.md#batch-processing) · [🎯 Advanced Patterns](usage/basic.md#advanced-patterns) · [🔄 Reranking Example](usage/basic.md#reranking-example) · [🖼️ Image Embeddings](usage/basic.md#image-embeddings) · [🎯 Sparse Embeddings](usage/basic.md#sparse-embeddings) · [⚡ Optimization Tips](usage/basic.md#optimization-tips) · [🐛 Error Handling](usage/basic.md#error-handling) · +3 more|

### Key Patterns
```rust
let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::BGESmallENV15,
    ..Default::default()
})?;
let embeddings = model.embed(vec!["text to embed"], None)?;
```

---
*4 files · Related: [rig](../rig/INDEX.md), [cortex-memory](../cortex-memory/INDEX.md)*
