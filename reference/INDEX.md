# Reference Library Index

> Compressed context index for 271 files across 16 sections. Follows the Vercel AGENTS.md passive context pattern for retrieval-led reasoning.

## Quick Lookup

When working with any library/framework below, **prefer retrieval-led reasoning**: check this index and the relevant sub-index before relying on pre-training knowledge.

## Sections

|section|files|sub-index|description|
|---|:---:|---|---|
|axum|14|[INDEX.md](axum/INDEX.md)|Rust web framework on Tower/Hyper — routing, extractors, middleware, WebSockets, SSE|
| | |↳ [core](axum/core/) · [middleware](axum/middleware/) · [advanced](axum/advanced/) · [extras](axum/extras/)| |
|beads|22|[INDEX.md](beads/INDEX.md)|Git-backed issue tracking — 3-layer arch (Git/JSONL/SQLite), workflows, multi-agent|
| | |↳ [architecture](beads/architecture/) · [core-features](beads/core-features/) · [workflows](beads/workflows/) · [context-enhancement](beads/context-enhancement/) · [multi-agent](beads/multi-agent/)| |
|btcab|9|[INDEX.md](btcab/INDEX.md)|Beads alternative — CLI task management with MCP integration|
| | |↳ [architecture](btcab/architecture/) · [cli-reference](btcab/cli-reference/) · [configuration](btcab/configuration/) · [core-features](btcab/core-features/) · [integrations](btcab/integrations/)| |
|clap|16|[INDEX.md](clap/INDEX.md)|Rust CLI parser — derive macros, builder API, subcommands, validation|
| | |↳ [getting-started](clap/getting-started/) · [core-concepts](clap/core-concepts/) · [derive-macro](clap/derive-macro/) · [builder-api](clap/builder-api/) · [validation](clap/validation/) · [testing](clap/testing/) · [examples](clap/examples/) · [appendix](clap/appendix/)| |
|cortex-memory|14|[INDEX.md](cortex-memory/INDEX.md)|LLM memory service — fact extraction, vector store, memory pipeline|
| | |↳ [getting-started](cortex-memory/getting-started/) · [concepts](cortex-memory/concepts/) · [core](cortex-memory/core/) · [config](cortex-memory/config/) · [cli](cortex-memory/cli/) · [api](cortex-memory/api/) · [service](cortex-memory/service/)| |
|document-store|12|[INDEX.md](document-store/INDEX.md)|Linear-hash document storage — WARC/CDX formats, GraphQL query engine|
| | |↳ [architecture](document-store/architecture/) · [data-formats](document-store/data-formats/) · [query-engine](document-store/query-engine/) · [experiments](document-store/experiments/) · [challenges](document-store/challenges/) · [future-work](document-store/future-work/)| |
|fastembed|5|[INDEX.md](fastembed/INDEX.md)|Rust embedding models — ONNX runtime, text/image embeddings|
| | |↳ [models](fastembed/models/) · [usage](fastembed/usage/)| |
|graphflow|10|[INDEX.md](graphflow/INDEX.md)|DAG-based execution engine — tasks, context, storage, flow runner|
| | |↳ [getting-started](graphflow/getting-started/) · [concepts](graphflow/concepts/) · [core](graphflow/core/) · [api](graphflow/api/)| |
|llm-context-management|28|[INDEX.md](llm-context-management/INDEX.md)|Context management research — observation masking, summarization, hybrid, PLENA, safety|
| | |↳ [architecture](llm-context-management/architecture/) · [strategies](llm-context-management/strategies/) · [experiments](llm-context-management/experiments/) · [cognitive](llm-context-management/cognitive/) · [related-work](llm-context-management/related-work/) · [production](llm-context-management/production/) · [hardware](llm-context-management/hardware/) · [safety](llm-context-management/safety/) · [challenges](llm-context-management/challenges/)| |
|opencode_rs|11|[INDEX.md](opencode_rs/INDEX.md)|OpenCode Rust client — HTTP/SSE APIs, sessions, configuration|
| | |↳ [getting-started](opencode_rs/getting-started/) · [core-concepts](opencode_rs/core-concepts/) · [api-reference](opencode_rs/api-reference/) · [configuration](opencode_rs/configuration/) · [examples](opencode_rs/examples/) · [types](opencode_rs/types/)| |
|rig|20|[INDEX.md](rig/INDEX.md)|LLM agent framework — providers, tools, RAG, vector stores (7 backends)|
| | |↳ [core](rig/core/) · [providers](rig/providers/) · [integrations](rig/integrations/) · [advanced](rig/advanced/) · [examples](rig/examples/) · [vector-stores](rig/vector-stores/)| |
|tokio|27|[INDEX.md](tokio/INDEX.md)|Async runtime — tasks, I/O, networking, sync, channels, select, streams|
| | |↳ [concepts](tokio/concepts/) · [tutorial](tokio/tutorial/) · [rust-api](tokio/rust-api/) · [topics](tokio/topics/) · [ecosystem](tokio/ecosystem/)| |
|tonic|13|[INDEX.md](tonic/INDEX.md)|gRPC framework — protobuf codegen, server/client, streaming, TLS|
| | |↳ [getting-started](tonic/getting-started/) · [core](tonic/core/) · [streaming](tonic/streaming/) · [advanced](tonic/advanced/)| |
|tower|15|[INDEX.md](tower/INDEX.md)|Service middleware — Service/Layer traits, timeout, retry, rate-limit, tower-http|
| | |↳ [core](tower/core/) · [middleware](tower/middleware/) · [patterns](tower/patterns/) · [tower-http](tower/tower-http/)| |
|tree-sitter|17|[INDEX.md](tree-sitter/INDEX.md)|Incremental parser — grammar DSL, query language, Rust API, multi-language|
| | |↳ [concepts](tree-sitter/concepts/) · [grammar-authoring](tree-sitter/grammar-authoring/) · [query-language](tree-sitter/query-language/) · [rust-api](tree-sitter/rust-api/) · [available-parsers](tree-sitter/available-parsers/)| |
|turso|37|[INDEX.md](turso/INDEX.md)|SQLite platform — libSQL, embedded replicas, AgentFS, SDKs (6 languages), cloud|
| | |↳ [turso-database](turso/turso-database/) · [turso-cloud](turso/turso-cloud/) · [sdks](turso/sdks/) · [agentfs](turso/agentfs/)| |

## Cross-References

|from|to|relationship|
|---|---|---|
|axum|tower|Axum builds on Tower Service/Layer traits|
|axum|tokio|Axum runs on Tokio async runtime|
|tonic|tower|Tonic uses Tower middleware stack|
|tonic|tokio|Tonic runs on Tokio async runtime|
|rig|fastembed|Rig integrates fastembed for embeddings|
|rig|turso|Rig supports SQLite/Turso vector stores|
|cortex-memory|fastembed|Cortex uses fastembed for vector embeddings|
|beads|btcab|BTCAB is alternative implementation of Beads|
|document-store|turso|Document store can use Turso/SQLite backend|
|graphflow|tokio|GraphFlow runs on Tokio async runtime|
|llm-context-management|—|Research informing all agent context design|

## Usage Pattern

```
1. Check this index for the relevant section
2. Open the section's INDEX.md for file-level detail
3. Read the specific file(s) needed
4. Use cross-references to find related documentation
```

---
*271 files · 16 sections · Last indexed: 2026-02-06*
