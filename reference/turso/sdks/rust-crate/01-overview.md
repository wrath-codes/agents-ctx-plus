# Rust Crate

## Overview

The `turso` crate provides Rust bindings for Turso Database, Turso Cloud, and AgentFS. It offers a type-safe, async-first API for building applications with Turso.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
turso = "0.4"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use turso::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local database
    let db = Builder::new_local("./data.db").build().await?;
    
    // Remote database
    let db = Builder::new_remote(
        "libsql://mydb-org.turso.io",
        "your-auth-token"
    ).build().await?;
    
    let conn = db.connect()?;
    
    // Execute SQL
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
        ()
    ).await?;
    
    conn.execute(
        "INSERT INTO users (name) VALUES (?)",
        ["Alice"]
    ).await?;
    
    // Query
    let mut rows = conn.query("SELECT * FROM users", ()).await?;
    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        println!("{}: {}", id, name);
    }
    
    Ok(())
}
```

## Database Types

### Local Database

```rust
use turso::Builder;

// In-memory database
let db = Builder::new_in_memory().build().await?;

// File-based database
let db = Builder::new_local("./mydb.db").build().await?;

// With options
let db = Builder::new_local("./mydb.db")
    .encryption_key("secret-key")
    .cache_size(10000)
    .build().await?;
```

### Remote Database

```rust
// Turso Cloud database
let db = Builder::new_remote(
    "libsql://mydb-org.turso.io",
    "auth-token"
).build().await?;

// With custom HTTP client
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()?;

let db = Builder::new_remote_with_client(
    "libsql://mydb-org.turso.io",
    "auth-token",
    client
).build().await?;
```

### Sync/Embedded Replica

```rust
// Local replica with cloud sync
let db = Builder::new_sync(
    "./local-replica.db",
    "libsql://mydb-org.turso.io",
    "auth-token"
)
.sync_interval(Duration::from_secs(5))
.build().await?;
```

## Connection Management

### Connection Pool

```rust
// Automatic connection pooling
let db = Builder::new_local("./mydb.db")
    .max_connections(10)
    .connection_timeout(Duration::from_secs(30))
    .build().await?;

// Get connection from pool
let conn = db.connect()?;
```

### Transactions

```rust
let conn = db.connect()?;

// Standard transaction
let tx = conn.transaction().await?;
tx.execute("INSERT INTO users (name) VALUES (?)", ["Alice"]).await?;
tx.commit().await?;

// Concurrent transaction (MVCC)
let tx = conn.transaction_with_behavior(
    TransactionBehavior::Concurrent
).await?;
tx.execute("INSERT INTO users (name) VALUES (?)", ["Bob"]).await?;
tx.commit().await?;
```

## Query Operations

### Prepared Statements

```rust
let conn = db.connect()?;

// Prepare once
let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?").await?;

// Execute multiple times
for id in 1..=100 {
    let mut rows = stmt.query([id]).await?;
    while let Some(row) = rows.next().await? {
        let name: String = row.get(1)?;
        println!("User {}: {}", id, name);
    }
}
```

### Batch Operations

```rust
let conn = db.connect()?;

// Batch insert
let tx = conn.transaction().await?;
for i in 0..1000 {
    tx.execute(
        "INSERT INTO logs (msg) VALUES (?)",
        [format!("Log entry {}", i)]
    ).await?;
}
tx.commit().await?;
```

### Streaming Results

```rust
let conn = db.connect()?;

// Stream large result sets
let mut rows = conn.query("SELECT * FROM large_table", ()).await?;
while let Some(row) = rows.next().await? {
    let data: String = row.get(0)?;
    process_row(data).await;
}
```

## Vector Operations

### Storing Vectors

```rust
use turso::Builder;

let db = Builder::new_local("vectors.db").build().await?;
let conn = db.connect()?;

// Create table with vector column
conn.execute(
    "CREATE TABLE documents (
        id INTEGER PRIMARY KEY,
        content TEXT,
        embedding F32_BLOB(384)
    )",
    ()
).await?;

// Insert vector
let embedding: Vec<f32> = vec![0.1, 0.2, 0.3, /* ... */];
conn.execute(
    "INSERT INTO documents (content, embedding) VALUES (?, ?)",
    ("Hello world", embedding)
).await?;
```

### Vector Search

```rust
// Search for similar documents
let query_vector: Vec<f32> = generate_embedding("search query");

let mut rows = conn.query(
    "SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
     FROM documents
     ORDER BY distance
     LIMIT 5",
    [format!("{:?}", query_vector)]
).await?;

while let Some(row) = rows.next().await? {
    let content: String = row.get(0)?;
    let distance: f64 = row.get(1)?;
    println!("Content: {}, Distance: {}", content, distance);
}
```

### Vector Indexing

```rust
// Create vector index
conn.execute(
    "CREATE INDEX idx_embedding ON documents(
        libsql_vector_idx(embedding, 'metric=cosine')
    )",
    ()
).await?;

// Use indexed search
let mut rows = conn.query(
    "SELECT * FROM vector_top_k('idx_embedding', vector(?), 10)",
    [format!("{:?}", query_vector)]
).await?;
```

## Advanced Features

### Change Data Capture

```rust
// Stream database changes
let changes = conn.changes_stream();

while let Some(change) = changes.next().await {
    match change {
        Change::Insert { table, rowid, values } => {
            println!("INSERT: table={}, rowid={}", table, rowid);
        }
        Change::Update { table, rowid, old_values, new_values } => {
            println!("UPDATE: table={}, rowid={}", table, rowid);
        }
        Change::Delete { table, rowid } => {
            println!("DELETE: table={}, rowid={}", table, rowid);
        }
    }
}
```

### Encryption

```rust
// Encrypted database
let db = Builder::new_local("encrypted.db")
    .encryption_key("32-byte-key-minimum-required")
    .build().await?;
```

### WebAssembly

```rust
// Build for WASM target
#[cfg(target_arch = "wasm32")]
async fn init_db() -> Result<Database, Error> {
    let db = Builder::new_in_memory().build().await?;
    Ok(db)
}
```

## Error Handling

```rust
use turso::{Error, Result};

match conn.execute("INVALID SQL", ()).await {
    Ok(_) => { /* success */ },
    Err(Error::SqliteFailure(code, Some(msg))) => {
        eprintln!("SQL error {}: {}", code, msg);
    }
    Err(Error::ConnectionClosed) => {
        eprintln!("Connection closed");
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Type Mappings

| Rust Type | SQL Type |
|-----------|----------|
| `i8`, `i16`, `i32`, `i64` | INTEGER |
| `u8`, `u16`, `u32`, `u64` | INTEGER |
| `f32`, `f64` | REAL |
| `bool` | INTEGER (0 or 1) |
| `String` | TEXT |
| `Vec<u8>` | BLOB |
| `Vec<f32>` | F32_BLOB |
| `Option<T>` | NULLable |

## Best Practices

1. **Use connection pooling** for multi-threaded applications
2. **Prepare statements** for repeated queries
3. **Use transactions** for batch operations
4. **Handle errors** explicitly
5. **Enable WAL mode** for better concurrency
6. **Use appropriate types** for vector operations

## API Reference

See [docs.rs/turso](https://docs.rs/turso) for complete API documentation.

## Examples

### Basic CRUD

```rust
// Create
conn.execute(
    "INSERT INTO users (name, email) VALUES (?, ?)",
    ["Alice", "alice@example.com"]
).await?;

// Read
let user: (i64, String, String) = conn.query_row(
    "SELECT id, name, email FROM users WHERE id = ?",
    [1],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
).await?;

// Update
conn.execute(
    "UPDATE users SET name = ? WHERE id = ?",
    ["Alice Smith", 1]
).await?;

// Delete
conn.execute("DELETE FROM users WHERE id = ?", [1]).await?;
```

### Migration Example

```rust
async fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row(
        "PRAGMA user_version",
        (),
        |row| row.get(0)
    ).await?;
    
    match version {
        0 => {
            conn.execute(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
                ()
            ).await?;
            conn.execute("PRAGMA user_version = 1", ()).await?;
        }
        1 => {
            conn.execute(
                "ALTER TABLE users ADD COLUMN email TEXT",
                ()
            ).await?;
            conn.execute("PRAGMA user_version = 2", ()).await?;
        }
        _ => {}
    }
    
    Ok(())
}
```

## Next Steps

- [Go Binding](./go-binding.md)
- [JavaScript Binding](./javascript-binding.md)
- [Python Binding](./python-binding.md)