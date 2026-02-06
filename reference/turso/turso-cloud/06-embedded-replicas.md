# Embedded Replicas

## Overview

Embedded replicas are local copies of Turso Cloud databases that synchronize with the cloud. They provide ultra-low latency reads while maintaining data consistency.

## How Embedded Replicas Work

```
┌─────────────────────────────────────────────────────────────┐
│                   Embedded Replica Flow                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐         ┌──────────────────┐          │
│  │  Your Application│         │  Turso Cloud     │          │
│  │                  │         │                  │          │
│  │  ┌───────────┐  │         │  ┌────────────┐  │          │
│  │  │   Local   │  │◄───────►│  │  Primary   │  │          │
│  │  │  Replica  │  │  Sync   │  │  Database  │  │          │
│  │  │  (SQLite) │  │         │  │            │  │          │
│  │  └─────┬─────┘  │         │  └────────────┘  │          │
│  │        │        │         │                  │          │
│  │  ┌─────▼─────┐  │         │                  │          │
│  │  │  Reads:   │  │         │                  │          │
│  │  │  <1ms     │  │         │                  │          │
│  │  └───────────┘  │         │                  │          │
│  │        │        │         │                  │          │
│  │  ┌─────▼─────┐  │         │                  │          │
│  │  │  Writes:  │  │         │                  │          │
│  │  │  forwarded│  │────────►│                  │          │
│  │  └───────────┘  │  HTTP   │                  │          │
│  │                  │         │                  │          │
│  └─────────────────┘         └──────────────────┘          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Setting Up Embedded Replicas

### Basic Setup
```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create embedded replica
    let db = Builder::new_sync(
        "./local-replica.db",              // Local database path
        "libsql://mydb-org.turso.io",      // Remote database URL
        "your-auth-token"                   // Database token
    )
    .build()
    .await?;
    
    let conn = db.connect()?;
    
    // Reads are local (ultra-fast)
    let rows = conn.query("SELECT * FROM users", ()).await?;
    
    // Writes are forwarded to cloud
    conn.execute("INSERT INTO users (name) VALUES ('Alice')", ()).await?;
    
    Ok(())
}
```

### Configuration Options
```rust
let db = Builder::new_sync("./local.db", remote_url, token)
    // Sync interval (default: 5 seconds)
    .sync_interval(Duration::from_secs(5))
    
    // Sync on write (default: true)
    .sync_on_write(true)
    
    // Read your writes (default: true)
    .read_your_writes(true)
    
    // Encrypted local database
    .encryption_key("local-encryption-key")
    
    .build()
    .await?;
```

## Use Cases

### Local Development
```rust
// Development environment
let db = Builder::new_sync(
    "./dev-replica.db",
    "libsql://prod-db-org.turso.io",
    dev_token
)
.sync_on_write(false)  // Don't sync every write during dev
.build()
.await?;

// Work locally, sync when ready
db.sync().await?;  // Manual sync
```

### Edge Computing
```rust
// Edge function (e.g., Cloudflare Workers, Vercel Edge)
#[fetch]
async fn handle(req: Request) -> Result<Response> {
    // Local replica at edge
    let db = Builder::new_sync(
        "/tmp/edge-replica.db",
        "libsql://central-db.turso.io",
        token
    )
    .build()
    .await
    .map_err(|e| Error::RustError(e.to_string()))?;
    
    // Sub-millisecond reads
    let data = db.query("SELECT * FROM cache WHERE key = ?", [key]).await?;
    
    Response::from_json(&data)
}
```

### Offline-First Applications
```rust
// Mobile or desktop app
let db = Builder::new_sync(
    app_data_dir().join("local.db"),
    "libsql://cloud-db.turso.io",
    token
)
.build()
.await?;

// Works offline
// Changes queue locally
// Auto-sync when online
```

## Synchronization Strategies

### Continuous Sync
```rust
// Default behavior: sync every 5 seconds
let db = Builder::new_sync("./local.db", url, token)
    .sync_interval(Duration::from_secs(5))
    .build()
    .await?;

// Background sync thread automatically runs
```

### Manual Sync
```rust
// Disable automatic sync
let db = Builder::new_sync("./local.db", url, token)
    .sync_interval(None)  // Disable auto-sync
    .build()
    .await?;

// Sync when needed
db.sync().await?;

// Or sync specific tables
db.sync_table("users").await?;
```

### Sync on Write
```rust
// Ensure data is synced immediately after write
let db = Builder::new_sync("./local.db", url, token)
    .sync_on_write(true)
    .build()
    .await?;

conn.execute("INSERT INTO logs (msg) VALUES ('critical')", ()).await?;
// Automatically synced to cloud
```

## Conflict Resolution

### Last-Write-Wins (Default)
```rust
// Default conflict resolution
let db = Builder::new_sync("./local.db", url, token)
    .conflict_resolution(ConflictResolution::LastWriteWins)
    .build()
    .await?;
```

### Custom Resolution
```rust
use libsql::Conflict;

let db = Builder::new_sync("./local.db", url, token)
    .on_conflict(|conflict: Conflict| {
        match conflict.table() {
            "users" => Resolution::AcceptRemote,  // Cloud wins
            "logs" => Resolution::KeepLocal,      // Local wins  
            "config" => Resolution::Custom(merge_config),
            _ => Resolution::LastWriteWins,
        }
    })
    .build()
    .await?;
```

### Conflict Callback
```rust
let db = Builder::new_sync("./local.db", url, token)
    .on_conflict(|conflict| {
        // Log conflict for review
        log::warn!("Conflict in table {}: {:?}", 
            conflict.table(), 
            conflict.row()
        );
        
        Resolution::LastWriteWins
    })
    .build()
    .await?;
```

## Advanced Features

### Read-Your-Writes Consistency
```rust
// Ensure you can read your own writes immediately
let db = Builder::new_sync("./local.db", url, token)
    .read_your_writes(true)
    .build()
    .await?;

conn.execute("INSERT INTO users (name) VALUES ('Alice')", ()).await?;

// This read will include the new row
let rows = conn.query("SELECT * FROM users WHERE name = 'Alice'", ()).await?;
```

### Selective Sync
```rust
// Only sync specific tables
let db = Builder::new_sync("./local.db", url, token)
    .sync_tables(vec!["users", "products"])
    .skip_tables(vec!["logs", "temp_data"])
    .build()
    .await?;
```

### Encrypted Local Replica
```rust
// Encrypt local replica database
let db = Builder::new_sync("./local.db", url, token)
    .encryption_key("32-byte-encryption-key-here")
    .build()
    .await?;

// Local data is encrypted at rest
// Cloud data uses Turso encryption
```

### Multiple Replicas
```rust
// Create multiple local replicas for different purposes
let cache_db = Builder::new_sync("./cache.db", url, token)
    .sync_interval(Duration::from_secs(1))  // Fast sync
    .sync_tables(vec!["cache"])
    .build()
    .await?;

let analytics_db = Builder::new_sync("./analytics.db", url, token)
    .sync_interval(Duration::from_secs(60))  // Hourly sync
    .sync_tables(vec!["events", "metrics"])
    .build()
    .await?;
```

## Performance Characteristics

### Read Performance
```
┌────────────────────────────────────────────┐
│         Embedded Replica Reads             │
├────────────────────────────────────────────┤
│ Cold start (first read)    │ 5-50 ms       │
│ Warm read (cached)         │ 0.1-1 ms      │
│ Concurrent reads           │ 10,000+ QPS   │
│ Sequential scan            │ 1M rows/sec   │
└────────────────────────────────────────────┘
```

### Sync Performance
```
┌────────────────────────────────────────────┐
│         Sync Characteristics               │
├────────────────────────────────────────────┤
│ Small changes (< 1KB)      │ < 100 ms      │
│ Medium changes (1MB)       │ 1-5 sec       │
│ Large changes (100MB)      │ 30-120 sec    │
│ Sync frequency             │ Configurable  │
└────────────────────────────────────────────┘
```

### Write Performance
```
┌────────────────────────────────────────────┐
│         Write Performance                  │
├────────────────────────────────────────────┤
│ Local write (async)        │ < 1 ms        │
│ Local write (sync)         │ 50-200 ms     │
│ Remote write               │ 50-500 ms     │
│ Batch writes (1000 rows)   │ 1-5 sec       │
└────────────────────────────────────────────┘
```

## Best Practices

### Production Deployment
```rust
// Recommended production configuration
let db = Builder::new_sync("./replica.db", url, token)
    .sync_interval(Duration::from_secs(5))
    .sync_on_write(true)
    .read_your_writes(true)
    .encryption_key(&std::env::var("LOCAL_DB_KEY")?)
    .conflict_resolution(ConflictResolution::LastWriteWins)
    .build()
    .await?;
```

### Development Setup
```rust
// Faster sync for development
let db = Builder::new_sync("./dev-replica.db", url, token)
    .sync_interval(Duration::from_secs(1))  // Fast feedback
    .sync_on_write(false)  // Don't slow down development
    .build()
    .await?;

// Manual sync when needed
db.sync().await?;
```

### Error Handling
```rust
match db.sync().await {
    Ok(_) => println!("Sync successful"),
    Err(libsql::Error::SyncConflict(conflicts)) => {
        println!("Conflicts: {:?}", conflicts);
        // Handle conflicts
    }
    Err(e) => {
        println!("Sync error: {}", e);
        // Queue for retry
    }
}
```

## CLI Commands

```bash
# Create database with embedded replica support
turso db create mydb --enable-embedded-replicas

# Get replica URL
turso db show mydb --embedded-replica-url

# Check sync status
turso db show mydb --sync-status

# Force sync
turso db sync mydb

# Reset local replica (re-download from cloud)
turso db reset-replica mydb
```

## Troubleshooting

### Sync Issues
```bash
# Check sync status
turso db show mydb --sync-status --verbose

# Reset if corrupted
turso db reset-replica mydb

# Check network connectivity
curl -I https://mydb-org.turso.io/health
```

### Performance Problems
```rust
// Enable sync logging
let db = Builder::new_sync("./local.db", url, token)
    .sync_logging(true)
    .build()
    .await?;

// Monitor sync metrics
let stats = db.sync_stats();
println!("Last sync: {:?}", stats.last_sync);
println!("Sync duration: {:?}", stats.duration);
println!("Rows synced: {}", stats.rows_synced);
```

## Next Steps

- **Branching**: [07-branching.md](./07-branching.md)
- **Advanced Features**: [08-advanced-features.md](./08-advanced-features.md)
- **Platform API**: [09-platform-api.md](./09-platform-api.md)