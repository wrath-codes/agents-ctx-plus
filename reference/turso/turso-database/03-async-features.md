# Async Features and I/O

## Overview

libSQL is designed from the ground up for asynchronous I/O, enabling high-performance, non-blocking database operations essential for modern applications.

## Why Async Matters

### Traditional Blocking I/O
```rust
// Blocking - thread waits for disk
let file = File::open("data.db")?;
let mut buf = vec![0; 4096];
file.read(&mut buf)?; // Thread blocked here
// ... continue processing
```

### Async I/O with libSQL
```rust
// Non-blocking - thread can do other work
let db = Builder::new_local("data.db").build().await?;
let rows = conn.query("SELECT * FROM large_table", ()).await?;
// While waiting for disk I/O, thread handles other tasks
```

## io_uring Integration

### What is io_uring?
io_uring is a Linux kernel interface for efficient async I/O:
- Submit multiple operations at once
- No system call overhead per operation
- Completion notifications via ring buffer
- Zero-copy operations possible

### How libSQL Uses io_uring

```rust
// io_uring setup
let ring = io_uring::IoUring::new(256)?;

// Operation submission
let mut sq = ring.submission();
sq.push(&read_op)?;
sq.push(&write_op)?;
drop(sq);

// Batch submit
ring.submit_and_wait(2)?;

// Process completions
for cqe in ring.completion() {
    match cqe.user_data() {
        0x01 => handle_read_completion(cqe),
        0x02 => handle_write_completion(cqe),
        _ => {}
    }
}
```

### Performance Benefits

| Metric | Traditional | io_uring |
|--------|-------------|----------|
| Syscalls per op | 2 (submit + wait) | Batch submission |
| Context switches | High | Minimal |
| Throughput | Lower | 2-5x higher |
| Latency | Higher | Lower |

## Tokio Integration

### Runtime Setup
```rust
use tokio::runtime::Runtime;

// libSQL uses Tokio internally
#[tokio::main]
async fn main() -> Result<()> {
    let db = Builder::new_local("data.db").build().await?;
    // All operations are async
    Ok(())
}
```

### Connection Pool
```rust
// Automatic connection pooling with Tokio
let db = Builder::new_local("data.db")
    .max_connections(10)
    .build().await?;

// Connections are managed automatically
let conn1 = db.connect()?;
let conn2 = db.connect()?; // Returns immediately from pool
```

### Concurrent Operations
```rust
// Run multiple queries concurrently
let futures = vec![
    conn.query("SELECT * FROM users", ()),
    conn.query("SELECT * FROM orders", ()),
    conn.query("SELECT * FROM products", ()),
];

let results = futures::future::join_all(futures).await;
```

## Concurrent Writes with MVCC

### BEGIN CONCURRENT

Traditional SQLite allows only one writer at a time. libSQL extends this with MVCC:

```rust
// Multiple connections can write concurrently
let db = Builder::new_local("data.db").build().await?;

// Connection 1
let conn1 = db.connect()?;
let tx1 = conn1.transaction_with_behavior(TransactionBehavior::Concurrent).await?;

// Connection 2  
let conn2 = db.connect()?;
let tx2 = conn2.transaction_with_behavior(TransactionBehavior::Concurrent).await?;

// Both can write simultaneously
tx1.execute("INSERT INTO logs (msg) VALUES ('tx1')", ()).await?;
tx2.execute("INSERT INTO logs (msg) VALUES ('tx2')", ()).await?;

// Commits succeed independently
tx1.commit().await?;
tx2.commit().await?;
```

### Conflict Resolution
```rust
// If conflicts occur, later transaction fails
let tx1 = conn1.transaction_with_behavior(TransactionBehavior::Concurrent).await?;
let tx2 = conn2.transaction_with_behavior(TransactionBehavior::Concurrent).await?;

// Both try to update same row
tx1.execute("UPDATE users SET name = 'Alice' WHERE id = 1", ()).await?;
tx2.execute("UPDATE users SET name = 'Bob' WHERE id = 1", ()).await?;

tx1.commit().await?; // Succeeds
tx2.commit().await?; // Fails with SQLITE_BUSY_SNAPSHOT
```

### Best Practices
```rust
// 1. Keep concurrent transactions short
let tx = conn.transaction_with_behavior(TransactionBehavior::Concurrent).await?;
// Do work quickly...
tx.commit().await?;

// 2. Handle conflicts gracefully
match tx.commit().await {
    Ok(_) => println!("Committed successfully"),
    Err(libsql::Error::SqliteFailure(_, Some(msg))) if msg.contains("BUSY") => {
        // Retry with exponential backoff
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Retry logic...
    }
    Err(e) => return Err(e.into()),
}

// 3. Use appropriate isolation levels
let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).await?; // Pessimistic
let tx = conn.transaction_with_behavior(TransactionBehavior::Concurrent).await?; // Optimistic
```

## Async Patterns

### Streaming Results
```rust
// Process large result sets without loading into memory
let mut rows = conn.query("SELECT * FROM large_table", ()).await?;

while let Some(row) = rows.next().await? {
    let data: String = row.get(0)?;
    // Process each row as it arrives
    process_data(data).await;
}
```

### Pipelining
```rust
// Submit multiple operations without waiting
let mut futures = Vec::new();

for i in 0..1000 {
    let fut = conn.execute(
        "INSERT INTO logs (id, msg) VALUES (?, ?)",
        [i, format!("Log entry {}", i)],
    );
    futures.push(fut);
}

// Wait for all to complete
let results = futures::future::join_all(futures).await;
```

### Cancellation Safety
```rust
// Operations can be cancelled safely
let query = conn.query("SELECT * FROM users", ());

// Timeout after 5 seconds
match tokio::time::timeout(Duration::from_secs(5), query).await {
    Ok(Ok(rows)) => process_rows(rows).await,
    Ok(Err(e)) => println!("Query error: {}", e),
    Err(_) => println!("Query timed out"),
}
```

## Change Data Capture (CDC)

### Streaming Changes
```rust
// Stream database changes in real-time
let changes = conn.changes_stream();

while let Some(change) = changes.next().await {
    match change {
        Change::Insert { table, rowid, values } => {
            println!("INSERT into {}: rowid={}", table, rowid);
        }
        Change::Update { table, rowid, old_values, new_values } => {
            println!("UPDATE {}: rowid={}", table, rowid);
        }
        Change::Delete { table, rowid } => {
            println!("DELETE from {}: rowid={}", table, rowid);
        }
    }
}
```

### Use Cases for CDC
```rust
// 1. Real-time replication
let changes = conn.changes_stream();
while let Some(change) = changes.next().await {
    replicate_to_downstream(change).await?;
}

// 2. Cache invalidation
let changes = conn.changes_stream();
while let Some(change) = changes.next().await {
    if change.table() == "users" {
        invalidate_cache_entry(change.rowid()).await?;
    }
}

// 3. Audit logging
let changes = conn.changes_stream();
while let Some(change) = changes.next().await {
    audit_log.record(change).await?;
}
```

## Performance Tuning

### Async Runtime Configuration
```rust
// Customize Tokio runtime
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)  // Match CPU cores
    .max_blocking_threads(512)
    .enable_io()
    .enable_time()
    .build()?;

rt.block_on(async {
    let db = Builder::new_local("data.db").build().await?;
    // Your async code
})
```

### Connection Pool Tuning
```rust
let db = Builder::new_local("data.db")
    .max_connections(20)      // Max concurrent connections
    .connection_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .build().await?;
```

### Batch Operations
```rust
// Use transactions for batch operations
let tx = conn.transaction().await?;

for item in items {
    tx.execute("INSERT INTO data (value) VALUES (?)", [item]).await?;
}

tx.commit().await?;
```

## Platform Support

### Linux (io_uring)
```rust
// Full async support with io_uring
let db = Builder::new_local("data.db").build().await?;
// Uses io_uring for all I/O operations
```

### macOS/Windows
```rust
// Uses thread pool for I/O
let db = Builder::new_local("data.db").build().await?;
// Falls back to threaded I/O (still async API)
```

### WASM
```rust
// Browser/Node.js WASM target
let db = Builder::new_in_memory().build().await?;
// Uses JavaScript Promise-based I/O
```

## Next Steps

- **Vector Search**: [04-vector-search.md](./04-vector-search.md)
- **Extensions**: [05-extensions.md](./05-extensions.md)
- **Advanced Features**: [06-advanced-features.md](./06-advanced-features.md)