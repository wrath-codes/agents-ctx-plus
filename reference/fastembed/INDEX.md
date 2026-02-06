# fastembed — Sub-Index

> Rust embedding models with ONNX runtime (5 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
|[DOCUMENTATION_SUMMARY.md](DOCUMENTATION_SUMMARY.md)|Documentation overview|

### [models](models/)

|file|description|
|---|---|
|[text-models.md](models/text-models.md)|Text models — BAAI/bge, all-MiniLM, JINA, nomic|

### [usage](usage/)

|file|description|
|---|---|
|[basic.md](usage/basic.md)|Basic usage — TextEmbedding::try_new(), embed()|

### Key Patterns
```rust
let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::BGESmallENV15,
    ..Default::default()
})?;
let embeddings = model.embed(vec!["text to embed"], None)?;
```

---
*5 files · Related: [rig](../rig/INDEX.md), [cortex-memory](../cortex-memory/INDEX.md)*
