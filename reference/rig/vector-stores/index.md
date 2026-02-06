# Vector Stores Overview

## Available Vector Store Integrations

Rig supports 10+ vector store integrations via companion crates:

| Vector Store | Crate | Best For |
|--------------|-------|----------|
| **MongoDB** | rig-mongodb | Document-based search, Atlas integration |
| **LanceDB** | rig-lancedb | Local/embedded, fast prototyping |
| **Neo4j** | rig-neo4j | Graph + vector combined |
| **Qdrant** | rig-qdrant | High-performance production |
| **SQLite** | rig-sqlite | Embedded, mobile apps |
| **SurrealDB** | rig-surrealdb | Multi-model, real-time |
| **Milvus** | rig-milvus | Enterprise scale, distributed |
| **ScyllaDB** | rig-scylladb | Cassandra-compatible, high throughput |

## Additional Integrations

### AWS S3Vectors
Store vectors in S3 with Athena querying:

```toml
[dependencies]
rig-s3vectors = "0.5"
```

Useful for data lake architectures and batch processing.

### HelixDB
Specialized for bioinformatics and scientific data:

```toml
[dependencies]
rig-helixdb = "0.5"
```

## Choosing a Vector Store

### For Development/Prototyping
- **SQLite** - Zero setup, embedded
- **LanceDB** - Fast, local, no server

### For Production Web Apps
- **MongoDB Atlas** - Managed, scalable, familiar
- **Qdrant** - High performance, filtering
- **SurrealDB** - Multi-model flexibility

### For Enterprise Scale
- **Milvus** - Billions of vectors, distributed
- **ScyllaDB** - High throughput, low latency

### For Graph Applications
- **Neo4j** - Knowledge graphs, relationships
- **SurrealDB** - Graph + document + vector

### For Edge/Mobile
- **SQLite** - Embedded, mobile apps
- **LanceDB** - Local ML, edge devices

## Quick Comparison

| Feature | MongoDB | Qdrant | Milvus | Neo4j | SQLite |
|---------|---------|---------|---------|-------|--------|
| Max Scale | Very High | High | Very High | High | Single Node |
| Latency | Low | Very Low | Low | Low | Very Low |
| Filtering | Excellent | Excellent | Good | Good | Basic |
| Graph | No | No | No | Yes | No |
| Self-hosted | Yes | Yes | Yes | Yes | Yes |
| Managed | Atlas | Cloud | Zilliz | Aura | N/A |

## Implementation Status

| Vector Store | Status | Documentation |
|--------------|--------|---------------|
| MongoDB | ✅ Stable | [Guide](mongodb.md) |
| LanceDB | ✅ Stable | [Guide](lancedb.md) |
| Neo4j | ✅ Stable | [Guide](neo4j.md) |
| Qdrant | ✅ Stable | [Guide](qdrant.md) |
| SQLite | ✅ Stable | [Guide](sqlite.md) |
| SurrealDB | ✅ Stable | [Guide](surrealdb.md) |
| Milvus | ✅ Stable | [Guide](milvus.md) |
| ScyllaDB | 🔄 Beta | Coming soon |
| S3Vectors | 🔄 Beta | Coming soon |
| HelixDB | 🔄 Beta | Coming soon |

## Next Steps

- **[MongoDB](mongodb.md)** - Get started with document-based vectors
- **[Qdrant](qdrant.md)** - Production-ready vector search
- **[RAG Systems](../examples/rag-system.md)** - Build complete RAG applications