# Turso Database (libSQL) Overview

## What is libSQL?

libSQL is an open-source, SQLite-compatible database engine rewritten in Rust. It maintains full compatibility with SQLite while adding modern features essential for contemporary applications.

## Key Value Propositions

### 1. SQLite Compatibility
- Drop-in replacement for SQLite
- Supports all SQLite features and extensions
- Existing SQLite databases work without migration
- Compatible SQLite file format

### 2. Modern Rust Implementation
- Memory-safe implementation
- Better performance through Rust optimizations
- Easier to extend and maintain
- Strong type safety guarantees

### 3. Async-First Architecture
- Built on io_uring for Linux
- Non-blocking I/O operations
- Better concurrency handling
- Ideal for async/await patterns

## Core Features

### Vector Search (Built-in)
```sql
-- Define vector column
CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
);

-- Create vector index
CREATE INDEX idx_items_embedding ON items(libsql_vector_idx(embedding));

-- Search similar items
SELECT * FROM vector_top_k('idx_items_embedding', vector('[0.1, 0.2, ...]'), 5);
```

### Concurrent Writes
```sql
-- Multiple connections can write concurrently
BEGIN CONCURRENT;
INSERT INTO users (name) VALUES ('Alice');
COMMIT;
```

### Change Data Capture
```rust
// Stream changes in real-time
let changes = conn.changes_stream();
while let Some(change) = changes.next().await {
    println!("Change: {:?}", change);
}
```

### WebAssembly Support
```rust
// Run libSQL in WASM environments
let conn = libsql::Builder::new_in_memory()
    .build()
    .await?;
```

## Architecture Highlights

### Three-Layer Design
```
┌─────────────────────────────────────┐
│         SQL Interface Layer         │
│    (SQLite compatible parser)       │
├─────────────────────────────────────┤
│         Virtual Machine             │
│    (Bytecode execution engine)      │
├─────────────────────────────────────┤
│         Storage Engine              │
│  (B-trees, pages, WAL, async I/O)   │
└─────────────────────────────────────┘
```

### Async I/O with io_uring
```rust
// Efficient async operations
let db = Builder::new_local("./data.db")
    .build()
    .await?;

// All operations are non-blocking
let conn = db.connect()?;
let rows = conn.query("SELECT * FROM users", ()).await?;
```

## Use Cases

### 1. Embedded Applications
- Mobile apps (iOS, Android)
- Desktop applications
- IoT devices
- Edge computing

### 2. Local-First Software
- Offline-capable applications
- Sync engines
- CRDT-based systems

### 3. Vector Databases
- RAG applications
- Semantic search
- Recommendation systems

### 4. Real-Time Systems
- CDC pipelines
- Event sourcing
- Stream processing

## Performance Benchmarks

### Read Performance
- Point queries: ~0.1ms
- Sequential scans: ~1M rows/sec
- Index lookups: ~0.05ms

### Write Performance
- Single-threaded: ~50K inserts/sec
- Concurrent (BEGIN CONCURRENT): ~200K inserts/sec
- Bulk import: ~500K rows/sec

### Vector Search
- 10K vectors: <1ms
- 100K vectors: <5ms
- 1M vectors: <50ms

## Comparison with SQLite

| Aspect | SQLite | libSQL |
|--------|---------|---------|
| Language | C | Rust |
| Async Support | No | Native |
| Vector Search | Extension | Built-in |
| Concurrent Writes | Limited | MVCC |
| Change Streaming | No | CDC |
| WASM | No | Yes |
| Encryption | Extension | Built-in |

## Getting Started

### Installation

```bash
# Cargo
cargo add libsql

# Or clone and build
git clone https://github.com/tursodatabase/libsql
cd libsql && cargo build --release
```

### Basic Usage

```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create or open database
    let db = Builder::new_local("./data.db").build().await?;
    let conn = db.connect()?;
    
    // Execute SQL
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)",
        (),
    ).await?;
    
    // Insert data
    conn.execute("INSERT INTO users (name) VALUES (?)", ["Alice"]).await?;
    
    // Query data
    let mut rows = conn.query("SELECT * FROM users", ()).await?;
    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        println!("{}: {}", id, name);
    }
    
    Ok(())
}
```

## Integration with Turso Cloud

libSQL powers Turso Cloud:
- **Local Development**: Use libSQL directly
- **Production**: Connect to Turso Cloud
- **Edge Deployment**: Embedded replicas sync with cloud

```rust
// Connect to Turso Cloud
let db = Builder::new_remote(
    "libsql://mydb-org.turso.io",
    "token_here"
).build().await?;
```

## Next Steps

- **Architecture**: [02-architecture/](./02-architecture/)
- **Async Features**: [03-async-features/](./03-async-features/)
- **Vector Search**: [04-vector-search/](./04-vector-search/)
- **Extensions**: [05-extensions/](./05-extensions/)