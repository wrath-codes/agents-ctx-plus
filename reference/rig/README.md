# Rig

Complete documentation for Rig - a Rust framework for building scalable, modular, and ergonomic LLM-powered applications.

## Overview

Rig is a Rust library that provides portable, modular, and lightweight full-stack AI agents. It offers simple but powerful abstractions over LLM providers and vector stores with minimal boilerplate.

## Why Rig?

### Performance
- **Lightweight**: Rust runs orders of magnitude faster than Python
- **Safety**: Type system and ownership model help handle unexpected LLM outputs
- **Portability**: Compile to WebAssembly for browser deployment

### Features
- Full support for LLM completion and embedding workflows
- 20+ model providers under one unified interface
- 10+ vector store integrations
- Agentic workflows with multi-turn streaming
- Full WASM compatibility (core library)

## Quick Start

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() {
    // Create OpenAI client
    let client = openai::Client::from_env();
    
    // Create agent
    let agent = client
        .agent("gpt-4")
        .preamble("You are a helpful assistant.")
        .build();
    
    // Prompt the agent
    let response = agent
        .prompt("Hello, how are you?")
        .await
        .expect("Failed to get response");
    
    println!("Agent: {response}");
}
```

## Installation

```toml
[dependencies]
rig-core = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Documentation Map

```
reference/rig/
├── README.md                    # This file
├── index.md                     # Overview and navigation
├── core/                        # Core concepts
│   ├── getting-started.md
│   ├── agents.md
│   ├── completion.md
│   ├── embeddings.md
│   ├── pipelines.md
│   └── tools.md
├── providers/                   # Model providers
│   ├── openai.md
│   ├── anthropic.md
│   ├── gemini.md
│   ├── cohere.md
│   └── ollama.md
├── vector-stores/              # Vector store integrations
│   ├── mongodb.md
│   ├── lancedb.md
│   ├── neo4j.md
│   ├── qdrant.md
│   └── sqlite.md
├── integrations/               # Companion integrations
│   ├── bedrock.md
│   ├── fastembed.md
│   └── onchain-kit.md
├── examples/                   # Examples and patterns
│   ├── basic-agent.md
│   ├── rag-system.md
│   ├── multi-agent.md
│   └── streaming.md
├── advanced/                   # Advanced topics
│   ├── custom-providers.md
│   ├── custom-tools.md
│   ├── error-handling.md
│   └── wasm-deployment.md
└── deployment/                 # Deployment guides
    ├── production.md
    └── monitoring.md
```

## Key Concepts

### 1. Agents
Agents are the core abstraction for LLM interactions:

```rust
let agent = client
    .agent("gpt-4")
    .preamble("You are an expert programmer.")
    .context(&Context::new("Rust is a systems programming language..."))
    .build();
```

### 2. Completion Models
Unified interface for text generation:

```rust
let model = client.completion_model("gpt-4");
let response = model.complete("Hello").await?;
```

### 3. Embedding Models
For vector representations:

```rust
let model = client.embedding_model("text-embedding-3-small");
let embedding = model.embed("Hello world").await?;
```

### 4. Vector Stores
For semantic search and RAG:

```rust
let index = vector_store.index("documents");
let results = index.search("query").await?;
```

### 5. Tools
Extensible function calling:

```rust
let tool = Tool::new("calculator", |input: CalculatorInput| {
    Ok(input.a + input.b)
});

let agent = client
    .agent("gpt-4")
    .tool(tool)
    .build();
```

## Model Providers

Rig natively supports:

| Provider | Completion | Embeddings | Notes |
|----------|-----------|------------|-------|
| OpenAI | ✅ | ✅ | GPT-4, GPT-3.5, DALL-E |
| Anthropic | ✅ | ❌ | Claude 3, Claude 2 |
| Gemini | ✅ | ✅ | Google's models |
| Cohere | ✅ | ✅ | Command, Embed |
| Ollama | ✅ | ✅ | Local models |
| Perplexity | ✅ | ❌ | Search-augmented |
| Hugging Face | ✅ | ✅ | Various models |
| XAI | ✅ | ❌ | Grok |
| DeepSeek | ✅ | ❌ | DeepSeek models |

## Vector Stores

Available integrations:

| Store | Crate | Features |
|-------|-------|----------|
| MongoDB | rig-mongodb | Atlas Vector Search |
| LanceDB | rig-lancedb | Local/Cloud |
| Neo4j | rig-neo4j | Graph + Vector |
| Qdrant | rig-qdrant | High performance |
| SQLite | rig-sqlite | Local, embedded |
| SurrealDB | rig-surrealdb | Multi-model |
| Milvus | rig-milvus | Distributed |
| ScyllaDB | rig-scylladb | Cassandra-compatible |

## Next Steps

1. **[Getting Started](core/getting-started.md)** - Installation and first agent
2. **[Core Concepts](core/agents.md)** - Learn about agents and workflows
3. **[Model Providers](providers/openai.md)** - Configure your LLM provider
4. **[Examples](examples/basic-agent.md)** - Build real applications