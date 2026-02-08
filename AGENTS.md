# AGENTS.md — Reference Library Context

> **Retrieval-led reasoning directive**: When working with any library or framework listed below, consult the reference documentation _before_ relying on pre-training knowledge. Pre-training knowledge is the fallback, not the default.

## Reference Map (267 files · 17 sections)

| section                | description                                                                         |
| ---------------------- | ----------------------------------------------------------------------------------- |
| axum                   | Rust web framework — routing, extractors, middleware, WebSockets, SSE               |
| beads                  | Git-backed issue tracking — 3-layer arch (Git/JSONL/SQLite), workflows, multi-agent |
| btcab                  | Beads alternative — CLI task management with MCP integration                        |
| clap                   | Rust CLI parser — derive macros, builder API, subcommands, validation               |
| cortex-memory          | LLM memory service — fact extraction, vector store, memory pipeline                 |
| document-store         | Linear-hash document storage — WARC/CDX formats, GraphQL query engine               |
| duckdb                 | Embedded analytical database — core extensions, community extensions, Rust SDK      |
| fastembed              | Rust embedding models — ONNX runtime, text/image embeddings                         |
| graphflow              | DAG-based execution engine — tasks, context, storage, flow runner                   |
| llm-context-management | Context management research — observation masking, summarization, hybrid, safety, KG extraction |
| opencode_rs            | OpenCode Rust client — HTTP/SSE APIs, sessions, configuration                       |
| rig                    | LLM agent framework — providers, tools, RAG, vector stores (7 backends)             |
| tokio                  | Async runtime — tasks, I/O, networking, sync, channels, select, streams             |
| tonic                  | gRPC framework — protobuf codegen, server/client, streaming, TLS                    |
| tower                  | Service middleware — Service/Layer traits, timeout, retry, rate-limit, tower-http   |
| tree-sitter            | Incremental parser — grammar DSL, query language, Rust API, multi-language          |
| turso                  | SQLite platform — libSQL, embedded replicas, AgentFS, SDKs (6 languages), cloud     |

## Navigation Protocol

Follow this 3-step lookup for any task involving the libraries above:

1. **Match** — Identify which section(s) the task involves using the map above
2. **Index** — Read `reference/{section}/INDEX.md` for file-level detail and heading anchors
3. **Read** — Open the specific file(s) needed for the task

Do not skip steps. The sub-indexes contain heading-level anchors that let you jump directly to the relevant section of each document.

## When to Consult

You MUST check the reference documentation when:

- Writing code that imports or uses any listed library
- Debugging errors in code that uses any listed library
- Answering questions about API surface, method signatures, or type constraints
- Reviewing code for correctness against documented patterns
- Unsure about feature flags, default behaviors, or configuration options

## Cross-References

When working with one library, also check its related libraries:

| working with           | also check                                                        |
| ---------------------- | ----------------------------------------------------------------- |
| axum                   | tower, tokio                                                      |
| tonic                  | tower, tokio                                                      |
| rig                    | fastembed, turso, duckdb                                          |
| cortex-memory          | fastembed                                                         |
| beads                  | btcab                                                             |
| document-store         | turso, duckdb                                                     |
| duckdb                 | turso (SQLite alternative)                                        |
| graphflow              | tokio, duckdb                                                     |
| llm-context-management | all sections (informs context design for every agent interaction) |

## Anti-Patterns

- **Do not** guess API signatures, method names, or type definitions from training data — look them up
- **Do not** assume feature flags or default behaviors — check the reference
- **Do not** skip the "Key Patterns" or "Gotchas" sections in sub-indexes — they prevent common mistakes
- **Do not** hallucinate documentation — if the reference doesn't cover something, say so and fall back to web search

## Scope

This reference library covers specific versions and snapshots of each library. For bleeding-edge changes not yet documented here, fall back to web search or official docs. When the reference _does_ cover a topic, prefer it over web results — it has been curated for accuracy and compressed for efficient consumption.
