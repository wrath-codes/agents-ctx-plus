# Turso Cloud Overview

## What is Turso Cloud?

Turso Cloud is a managed platform for SQLite-compatible databases, providing global distribution, automatic scaling, and advanced features while maintaining the simplicity and performance of SQLite.

## Key Value Propositions

### 1. SQLite-Compatible
- Drop-in replacement for SQLite
- No migration needed from existing SQLite databases
- Familiar SQL interface
- Works with existing SQLite tools and libraries

### 2. Global Distribution
- Deploy databases across 30+ locations worldwide
- Automatic data placement closest to users
- Low-latency reads from edge locations
- Built-in replication and failover

### 3. Serverless Scaling
- Automatic scaling from zero to thousands of requests
- Pay only for what you use
- No capacity planning needed
- Handles traffic spikes automatically

### 4. Developer Experience
- Simple CLI and API
- Git-like database branching
- Local development with embedded replicas
- Comprehensive SDKs for all major languages

## Core Features

### Global Deployment
```bash
# Create database in specific region
turso db create mydb --location lhr

# List available locations
turso locations list

# Deploy to multiple locations
turso db replicate mydb cdg  # Paris
turso db replicate mydb nrt  # Tokyo
```

### Database Branching
```bash
# Create branch for testing
turso db branch mydb mydb-staging

# Test schema changes
turso db shell mydb-staging

# Merge to production
turso db destroy mydb
turso db branch mydb-staging mydb --overwrite
```

### Embedded Replicas
```rust
// Local replica for development
let db = Builder::new_sync(
    "local.db",
    "libsql://mydb-org.turso.io",
    token
).build().await?;

// Reads are local, writes sync to cloud
```

### Vector Search
```sql
-- Vector search works out of the box
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
);

CREATE INDEX idx_docs_embedding ON documents(
    libsql_vector_idx(embedding)
);

SELECT * FROM vector_top_k('idx_docs_embedding', vector('[...]'), 5);
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Turso Cloud Platform                      │
├─────────────────────────────────────────────────────────────┤
│                    Global Load Balancer                      │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  Region  │  │  Region  │  │  Region  │  │  Region  │    │
│  │   LHR    │  │   CDG    │  │   NRT    │  │   IAD    │    │
│  │ (London) │  │ (Paris)  │  │ (Tokyo)  │  │(Virginia)│    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    Control Plane                             │
│  (Database Management, Authentication, Billing)              │
├─────────────────────────────────────────────────────────────┤
│                    Storage Layer                             │
│  (Encrypted, Replicated, Durable)                            │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Install CLI
```bash
curl -sSfL https://get.tur.so/install.sh | bash
```

### 2. Sign Up
```bash
turso auth signup
```

### 3. Create Database
```bash
# Create database
turso db create myapp

# Get connection URL
turso db show myapp --url

# Connect and query
turso db shell myapp
```

### 4. Connect from Application
```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Builder::new_remote(
        "libsql://myapp-username.turso.io",
        std::env::var("TURSO_TOKEN")?
    ).build().await?;
    
    let conn = db.connect()?;
    
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
        ()
    ).await?;
    
    Ok(())
}
```

## Pricing Tiers

### Starter (Free)
- 1 database
- 500MB storage
- 1B rows read/month
- 1M rows written/month
- Community support

### Scaler ($29/month)
- 10 databases
- 10GB storage per database
- 100B rows read/month
- 100M rows written/month
- Email support

### Enterprise (Custom)
- Unlimited databases
- Custom storage limits
- Custom row limits
- Dedicated support
- SLA guarantees

## Use Cases

### 1. Multi-tenant SaaS
```rust
// Separate database per customer
for customer in customers {
    let db_name = format!("customer-{}", customer.id);
    turso.create_database(&db_name).await?;
}
```

### 2. Edge Computing
```rust
// Embedded replica at edge
let edge_db = Builder::new_sync(
    "/tmp/edge-cache.db",
    "libsql://central-db.turso.io",
    token
).build().await?;

// Ultra-low latency reads locally
// Writes sync to central database
```

### 3. Global Applications
```bash
# Deploy to 3 continents
turso db create global-app --location lhr
turso db replicate global-app iad
turso db replicate global-app nrt
```

### 4. AI/ML Applications
```sql
-- Store embeddings
CREATE TABLE knowledge (
    id INTEGER PRIMARY KEY,
    text TEXT,
    embedding F32_BLOB(1536)
);

-- Semantic search
SELECT * FROM vector_top_k('idx_knowledge_embedding', query_vector, 10);
```

## Comparison with Alternatives

| Feature | Turso Cloud | SQLite | PostgreSQL | MongoDB Atlas |
|---------|-------------|---------|------------|---------------|
| Serverless | ✅ | ❌ | Partial | ✅ |
| Global Edge | ✅ | ❌ | ❌ | Partial |
| SQLite Compatible | ✅ | ✅ | ❌ | ❌ |
| Vector Search | ✅ | Extension | Extension | ✅ |
| Branching | ✅ | ❌ | ❌ | Partial |
| Embedded Replicas | ✅ | File copy | ❌ | ❌ |
| Pricing | Usage-based | Free | Instance-based | Usage-based |

## Security

### Encryption
- Data encrypted at rest (AES-256)
- TLS 1.3 for all connections
- Per-database encryption keys available

### Authentication
- Database tokens with fine-grained permissions
- Platform API keys for management
- Token rotation support

### Compliance
- SOC 2 Type II certified
- GDPR compliant
- HIPAA eligible (Enterprise)

## Next Steps

- **Database Management**: [02-database-management.md](./02-database-management.md)
- **Organizations**: [03-organizations.md](./03-organizations.md)
- **Locations**: [04-locations-regions.md](./04-locations-regions.md)
- **Authentication**: [05-authentication.md](./05-authentication.md)
- **Embedded Replicas**: [06-embedded-replicas.md](./06-embedded-replicas.md)
- **Branching**: [07-branching.md](./07-branching.md)