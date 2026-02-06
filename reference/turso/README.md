# Turso

Complete documentation for Turso - the next-generation SQLite platform for modern applications and AI agents.

## Quick Start

Turso provides SQLite-compatible databases with modern features:

```bash
# Install Turso CLI
curl -sSfL https://get.tur.so/install.sh | bash

# Create a local database
turso db create mydb

# Get connection URL
turso db show mydb --url

# Connect and query
turso db shell mydb
```

## What's Inside

This documentation covers three main Turso components:

### 📦 Turso Database (libSQL)
- **What**: Open-source SQLite engine in Rust
- **Features**: Async I/O, vector search, concurrent writes, CDC, WASM
- **Best for**: Embedded apps, edge computing, local-first software

### ☁️ Turso Cloud
- **What**: Managed SQLite platform
- **Features**: Global deployment, branching, replicas, vector search
- **Best for**: Production apps, multi-tenant SaaS, global distribution

### 🤖 AgentFS
- **What**: Copy-on-write filesystem for AI agents
- **Features**: Workspace isolation, auditing, MCP integration
- **Best for**: AI agents, reproducible workflows, compliance

## Documentation Map

```
reference/turso/
├── index.md                 # This file - overview and navigation
├── turso-database/          # Core libSQL engine docs
│   ├── 01-overview.md
│   ├── 02-architecture/
│   ├── 03-async-features/
│   ├── 04-vector-search/
│   ├── 05-extensions/
│   ├── 06-advanced-features/
│   └── 07-mcp-server.md
├── turso-cloud/             # Managed platform docs
│   ├── 01-overview.md
│   ├── 02-database-management/
│   ├── 03-organizations/
│   ├── 04-locations-regions/
│   ├── 05-authentication/
│   ├── 06-embedded-replicas/
│   ├── 07-branching/
│   ├── 08-advanced-features/
│   ├── 09-platform-api/
│   └── 10-sdks/
├── agentfs/                 # AI agent filesystem docs
│   ├── 01-overview.md
│   ├── 02-core-concepts/
│   ├── 03-installation/
│   ├── 04-cli-reference/
│   ├── 05-configuration/
│   ├── 06-sdks/
│   ├── 07-mcp-integration/
│   ├── 08-cloud-sync/
│   ├── 09-nfs-export/
│   └── 10-security/
└── sdks/                    # Language bindings and SDKs
    ├── rust-crate/
    └── bindings/
```

## Common Use Cases

### Building a RAG Application

```rust
// 1. Create vector table
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
);

// 2. Insert with embeddings
INSERT INTO documents (content, embedding)
VALUES ('text here', vector('[0.1, 0.2, ...]'));

// 3. Search similar documents
SELECT content, vector_distance_cosine(embedding, vector('[...]')) as distance
FROM documents
WHERE embedding MATCH vector('[...]')
ORDER BY distance
LIMIT 5;
```

### Local Development with Replicas

```bash
# Start embedded replica for local development
turso dev --db-file local.db

# Application connects to local file
libsql://local.db
```

### AI Agent Workspace

```bash
# Create isolated workspace
agentfs run --workspace my-agent bash

# All changes tracked and auditable
agentfs commit -m "Made changes"

# Sync to cloud
agentfs push
```

## Key Features at a Glance

| Feature | libSQL | Turso Cloud | AgentFS |
|---------|---------|-------------|---------|
| SQLite Compatible | ✅ | ✅ | ✅ |
| Async I/O | ✅ | ✅ | ✅ |
| Vector Search | ✅ | ✅ | ✅ |
| Global Distribution | ❌ | ✅ | ✅ |
| Embedded Replicas | ✅ | ✅ | ❌ |
| Database Branching | ❌ | ✅ | ❌ |
| MCP Server | ✅ | ❌ | ✅ |
| Workspace Isolation | ❌ | ❌ | ✅ |
| Audit Logging | ❌ | ❌ | ✅ |

## Performance Characteristics

- **Reads**: Sub-millisecond latency for local databases
- **Writes**: Optimized with io_uring and concurrent transactions
- **Vector Search**: Similarity queries in <10ms for 100k vectors
- **Sync**: Near real-time replication between replicas
- **Startup**: <100ms for embedded replicas

## Community and Support

- **Discord**: https://discord.gg/turso
- **GitHub**: https://github.com/tursodatabase
- **Twitter**: @tursodatabase
- **Email**: support@turso.tech

## Contributing

Turso is open source and welcomes contributions:

- **libSQL**: https://github.com/tursodatabase/libsql
- **Turso CLI**: https://github.com/tursodatabase/turso

## Next Steps

1. **[Turso Database](./turso-database/01-overview.md)** - Learn about the core engine
2. **[Turso Cloud](./turso-cloud/01-overview.md)** - Deploy managed databases
3. **[AgentFS](./agentfs/01-overview.md)** - Build AI agent workflows
4. **[SDKs](./sdks/)** - Integrate with your language of choice

---

*Turso - SQLite for the modern era*