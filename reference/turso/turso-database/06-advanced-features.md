# Advanced Features

## Overview

libSQL includes several advanced features for production deployments, including Change Data Capture, WebAssembly support, encryption, and replication capabilities.

## Change Data Capture (CDC)

CDC enables real-time streaming of database changes, essential for replication, caching, and event-driven architectures.

### How CDC Works
```
┌─────────────────────────────────────────────────────┐
│                  CDC Architecture                    │
├─────────────────────────────────────────────────────┤
│  Application → INSERT/UPDATE/DELETE                  │
│       ↓                                              │
│  libSQL → WAL → CDC Module → Change Stream          │
│       ↓                                              │
│  Consumers: Replicas, Cache, Analytics, Events      │
└─────────────────────────────────────────────────────┘
```

### Streaming Changes
```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Builder::new_local("data.db").build().await?;
    let conn = db.connect()?;
    
    // Start CDC stream
    let mut changes = conn.changes_stream();
    
    println!("Listening for changes...");
    
    while let Some(change) = changes.next().await {
        match change {
            Change::Insert { table, rowid, values } => {
                println!("INSERT: table={}, rowid={}, values={:?}", 
                    table, rowid, values);
            }
            Change::Update { table, rowid, old_values, new_values } => {
                println!("UPDATE: table={}, rowid={}", table, rowid);
                println!("  Old: {:?}", old_values);
                println!("  New: {:?}", new_values);
            }
            Change::Delete { table, rowid } => {
                println!("DELETE: table={}, rowid={}", table, rowid);
            }
        }
    }
    
    Ok(())
}
```

### Filtering Changes
```rust
// Stream changes for specific tables only
let mut changes = conn
    .changes_stream()
    .filter(|change| {
        matches!(change.table().as_str(), "users" | "orders")
    });

while let Some(change) = changes.next().await {
    // Only users and orders changes
}
```

### Use Cases

#### 1. Real-time Replication
```rust
// Replicate to downstream database
while let Some(change) = changes.next().await {
    downstream_conn.apply_change(&change).await?;
}
```

#### 2. Cache Invalidation
```rust
// Invalidate cache entries on changes
while let Some(change) = changes.next().await {
    if change.table() == "products" {
        let cache_key = format!("product:{}", change.rowid());
        redis.del(&cache_key).await?;
    }
}
```

#### 3. Audit Logging
```rust
// Log all changes to audit table
while let Some(change) = changes.next().await {
    audit_conn.execute(
        "INSERT INTO audit_log (table_name, operation, row_id, timestamp)
         VALUES (?, ?, ?, datetime('now'))",
        (change.table(), change.operation(), change.rowid()),
    ).await?;
}
```

#### 4. Event Broadcasting
```rust
// Broadcast changes to message queue
while let Some(change) = changes.next().await {
    let event = json!({
        "table": change.table(),
        "operation": change.operation(),
        "rowid": change.rowid(),
        "timestamp": Utc::now().to_rfc3339()
    });
    
    kafka.send("db-changes", event.to_string()).await?;
}
```

## WebAssembly Support

libSQL can run in WebAssembly environments, enabling browser-based and edge computing scenarios.

### Browser Usage
```javascript
// Load libSQL WASM module
import init, { Database } from './libsql.js';

async function setupDatabase() {
    await init();
    
    // Create in-memory database
    const db = new Database();
    
    // Execute SQL
    db.exec(`
        CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
        INSERT INTO users (name) VALUES ('Alice'), ('Bob');
    `);
    
    // Query data
    const results = db.query("SELECT * FROM users");
    console.log(results);
}
```

### Rust WASM Target
```rust
// Build for WASM
// Cargo.toml: crate-type = ["cdylib"]

use libsql::Builder;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn create_database() -> Result<JsValue, JsValue> {
    let db = Builder::new_in_memory()
        .build()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    let conn = db.connect()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    conn.execute(
        "CREATE TABLE test (id INTEGER PRIMARY KEY)",
        (),
    ).await
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    Ok(JsValue::from_str("Database created successfully"))
}
```

### Edge Computing with WASM
```rust
// Cloudflare Workers example
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // libSQL runs in WASM on edge
    let db = libsql::Builder::new_in_memory()
        .build()
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;
    
    // Process request with local database
    let conn = db.connect().map_err(|e| Error::RustError(e.to_string()))?;
    
    // ... handle request
    
    Response::ok("Success")
}
```

## Encryption at Rest

libSQL supports transparent encryption of database files.

### Enabling Encryption
```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database is encrypted with provided key
    let db = Builder::new_local("encrypted.db")
        .encryption_key("your-secret-key-at-least-32-characters-long")
        .build()
        .await?;
    
    let conn = db.connect()?;
    
    // All data is transparently encrypted
    conn.execute(
        "CREATE TABLE secrets (data TEXT)",
        (),
    ).await?;
    
    conn.execute(
        "INSERT INTO secrets (data) VALUES ('sensitive information')",
        (),
    ).await?;
    
    Ok(())
}
```

### Encryption Details
- Algorithm: AES-256-GCM
- Key length: 32 bytes (256 bits) minimum
- Transparent to application code
- All pages encrypted individually
- Key required to open database

### Key Management
```rust
// Load key from environment
let key = std::env::var("DATABASE_KEY")
    .expect("DATABASE_KEY must be set");

let db = Builder::new_local("secure.db")
    .encryption_key(&key)
    .build()
    .await?;
```

## Replication Protocol

libSQL supports master-replica replication for high availability and read scaling.

### Embedded Replicas
```rust
// Replica syncs from remote master
let db = Builder::new_sync(
    "local-replica.db",                    // Local path
    "libsql://mydb-org.turso.io",         // Remote URL
    "auth-token"                           // Authentication
)
.build()
.await?;

// Reads are local (fast)
let rows = conn.query("SELECT * FROM users", ()).await?;

// Writes are forwarded to master
conn.execute("INSERT INTO users (name) VALUES ('Alice')", ()).await?;
```

### Replication Modes
```rust
// Async replication (default)
let db = Builder::new_sync("local.db", remote_url, token)
    .build()
    .await?;

// Sync replication (wait for confirmation)
let db = Builder::new_sync("local.db", remote_url, token)
    .sync_mode(SyncMode::Sync)  // Wait for replica sync
    .build()
    .await?;
```

### Conflict Resolution
```rust
// Handle replication conflicts
let db = Builder::new_sync("local.db", remote_url, token)
    .on_conflict(|conflict| {
        match conflict {
            Conflict::LocalWins => Resolution::KeepLocal,
            Conflict::RemoteWins => Resolution::AcceptRemote,
            Conflict::Merge => Resolution::Custom(merge_values),
        }
    })
    .build()
    .await?;
```

## Advanced Configuration

### Performance Tuning
```rust
let db = Builder::new_local("data.db")
    // Cache size (pages)
    .cache_size(10000)
    
    // Page size
    .page_size(4096)
    
    // Journal mode
    .journal_mode(JournalMode::WAL)
    
    // Synchronous mode
    .synchronous(SynchronousMode::Normal)
    
    // Temp store
    .temp_store(TempStore::Memory)
    
    .build()
    .await?;
```

### Connection Limits
```rust
let db = Builder::new_local("data.db")
    .max_connections(50)           // Connection pool size
    .connection_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(3600))
    .build()
    .await?;
```

### WAL Configuration
```rust
let db = Builder::new_local("data.db")
    .journal_mode(JournalMode::WAL)
    .wal_autocheckpoint(1000)      // Checkpoint every 1000 pages
    .wal_checkpoint_threshold(10000)
    .build()
    .await?;
```

## Backup and Recovery

### Online Backup
```rust
// Backup while database is running
let source = Builder::new_local("production.db").build().await?;
let backup = Builder::new_local("backup.db").build().await?;

source.backup(&backup).await?;
```

### Incremental Backup
```rust
// Backup only changed pages
source.incremental_backup(&backup, last_checkpoint).await?;
```

### Point-in-Time Recovery
```rust
// Restore to specific WAL frame
conn.restore_to_frame(frame_number).await?;
```

## Monitoring and Metrics

### Connection Metrics
```rust
// Get pool statistics
let stats = db.pool_stats();
println!("Connections: {}/{}", stats.active, stats.max);
println!("Idle: {}", stats.idle);
println!("Queued: {}", stats.queued);
```

### Query Performance
```rust
// Enable query logging
let db = Builder::new_local("data.db")
    .log_queries(true)
    .log_slow_queries(Duration::from_millis(100))
    .build()
    .await?;
```

### Custom Metrics
```rust
use prometheus::{Counter, Histogram};

let query_counter = Counter::new("db_queries_total", "Total queries").unwrap();
let query_duration = Histogram::new("db_query_duration_seconds").unwrap();

// Instrument queries
let timer = query_duration.start_timer();
let result = conn.query("SELECT * FROM users", ()).await;
query_counter.inc();
timer.stop_and_record();
```

## Next Steps

- **MCP Server**: [07-mcp-server.md](./07-mcp-server.md)
- **Turso Cloud**: [../turso-cloud/01-overview.md](../turso-cloud/01-overview.md)
- **AgentFS**: [../agentfs/01-overview.md](../agentfs/01-overview.md)