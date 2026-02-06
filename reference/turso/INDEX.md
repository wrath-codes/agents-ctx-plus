# turso — Sub-Index

> SQLite platform with libSQL, embedded replicas, and AgentFS (37 files)

| subsection     | file                                                                  | description                                     |
| -------------- | --------------------------------------------------------------------- | ----------------------------------------------- |
| root           | [index.md](index.md)                                                  | Main index — overview, architecture             |
| root           | [README.md](README.md)                                                | Getting started guide                           |
| turso-database | [01-overview.md](turso-database/01-overview.md)                       | Database overview — libSQL fork, features       |
| turso-database | [02-architecture.md](turso-database/02-architecture.md)               | Architecture — sqld, replication, WAL           |
| turso-database | [03-async-features.md](turso-database/03-async-features.md)           | Async — async API, batching                     |
| turso-database | [04-vector-search.md](turso-database/04-vector-search.md)             | Vector search — vector columns, ANN queries     |
| turso-database | [05-extensions.md](turso-database/05-extensions.md)                   | Extensions — loadable extensions                |
| turso-database | [06-advanced-features.md](turso-database/06-advanced-features.md)     | Advanced — ATTACH, JSON, FTS                    |
| turso-database | [07-mcp-server.md](turso-database/07-mcp-server.md)                   | MCP server — Model Context Protocol integration |
| turso-cloud    | [01-overview.md](turso-cloud/01-overview.md)                          | Cloud overview — managed platform               |
| turso-cloud    | [02-database-management.md](turso-cloud/02-database-management.md)    | Database management — create, delete, groups    |
| turso-cloud    | [03-organizations.md](turso-cloud/03-organizations.md)                | Organizations — team management                 |
| turso-cloud    | [04-locations-regions.md](turso-cloud/04-locations-regions.md)        | Locations — multi-region deployment             |
| turso-cloud    | [05-authentication.md](turso-cloud/05-authentication.md)              | Authentication — API tokens, auth               |
| turso-cloud    | [06-embedded-replicas.md](turso-cloud/06-embedded-replicas.md)        | Embedded replicas — local read replicas         |
| turso-cloud    | [07-branching.md](turso-cloud/07-branching.md)                        | Branching — database branching for dev/staging  |
| turso-cloud    | [08-advanced-features.md](turso-cloud/08-advanced-features.md)        | Advanced — multi-db schemas, encryption         |
| turso-cloud    | [09-platform-api.md](turso-cloud/09-platform-api.md)                  | Platform API — REST API reference               |
| sdks           | [rust-crate/01-overview.md](sdks/rust-crate/01-overview.md)           | Rust SDK — libsql crate                         |
| sdks           | [bindings/index.md](sdks/bindings/index.md)                           | SDK bindings — overview                         |
| sdks           | [bindings/go-binding.md](sdks/bindings/go-binding.md)                 | Go — go-libsql binding                          |
| sdks           | [bindings/java-binding.md](sdks/bindings/java-binding.md)             | Java — JDBC binding                             |
| sdks           | [bindings/javascript-binding.md](sdks/bindings/javascript-binding.md) | JavaScript — @libsql/client                     |
| sdks           | [bindings/python-binding.md](sdks/bindings/python-binding.md)         | Python — libsql-client                          |
| sdks           | [bindings/wasm-binding.md](sdks/bindings/wasm-binding.md)             | WASM — browser binding                          |
| agentfs        | [01-overview.md](agentfs/01-overview.md)                              | AgentFS overview — filesystem for AI agents     |
| agentfs        | [02-core-concepts.md](agentfs/02-core-concepts.md)                    | Core concepts — agents, knowledge, memories     |
| agentfs        | [03-installation.md](agentfs/03-installation.md)                      | Installation — setup                            |
| agentfs        | [04-cli-reference.md](agentfs/04-cli-reference.md)                    | CLI — agentfs commands                          |
| agentfs        | [05-configuration.md](agentfs/05-configuration.md)                    | Configuration — agentfs config                  |
| agentfs        | [06-sdks/index.md](agentfs/06-sdks/index.md)                          | AgentFS SDKs — overview                         |
| agentfs        | [06-sdks/python-sdk.md](agentfs/06-sdks/python-sdk.md)                | Python SDK — agentfs-py                         |
| agentfs        | [06-sdks/rust-sdk.md](agentfs/06-sdks/rust-sdk.md)                    | Rust SDK — agentfs-rs                           |
| agentfs        | [06-sdks/typescript-sdk.md](agentfs/06-sdks/typescript-sdk.md)        | TypeScript SDK — agentfs-ts                     |
| agentfs        | [07-mcp-integration.md](agentfs/07-mcp-integration.md)                | MCP — Model Context Protocol bridge             |
| agentfs        | [08-cloud-sync.md](agentfs/08-cloud-sync.md)                          | Cloud sync — Turso cloud replication            |
| agentfs        | [09-nfs-export.md](agentfs/09-nfs-export.md)                          | NFS — network filesystem export                 |
| agentfs        | [10-security.md](agentfs/10-security.md)                              | Security — access control, encryption           |

---

_37 files · Related: [document-store](../document-store/INDEX.md), [rig](../rig/INDEX.md), [duckdb](../duckdb/INDEX.md)_
