# Architecture Overview

## System Architecture

libSQL is architected as a modern, modular database engine that maintains SQLite compatibility while enabling advanced features.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│              (Your code using libSQL)                        │
├─────────────────────────────────────────────────────────────┤
│                    libSQL Interface                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   SQL API    │  │   Sync API   │  │   Vector API     │  │
│  │  (Standard)  │  │ (Replicate)  │  │ (Similarity)     │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    SQL Processing                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │    Parser    │  │   Planner    │  │   Optimizer      │  │
│  │  (SQLite)    │  │  (Cost-based)│  │  (Index/Rewrite) │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                   Virtual Machine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Bytecode   │  │   VDBE       │  │   Execution      │  │
│  │   Compiler   │  │  (Engine)    │  │   Context        │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Storage Engine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  B-Tree      │  │   Page       │  │    WAL           │  │
│  │  (Tables)    │  │   Cache      │  │  (Journal)       │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Async I/O Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   io_uring   │  │    Tokio     │  │   File System    │  │
│  │   (Linux)    │  │  (Runtime)   │  │   Abstraction    │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. SQL Interface Layer

Provides multiple APIs for different use cases:

#### Standard SQL API
```rust
// Synchronous-style API (async internally)
let conn = db.connect()?;
conn.execute("INSERT INTO users VALUES (?)", ["Alice"]).await?;
```

#### Synchronization API
```rust
// For embedded replicas
let db = Builder::new_sync("local.db", remote_url, token).build().await?;
```

#### Vector API
```rust
// Vector operations
let embedding: Vec<f32> = vec![0.1, 0.2, 0.3];
conn.execute(
    "INSERT INTO items (vec) VALUES (?)",
    [libsql::Value::Blob(embedding)],
).await?;
```

### 2. SQL Processing

#### Parser
- SQLite-compatible SQL parser
- Extended syntax for vectors
- Rust-based parser for safety

```rust
// SQL parsing happens here
let stmt = conn.prepare("SELECT * FROM users WHERE id = ?").await?;
```

#### Query Planner
- Cost-based optimization
- Index selection
- Join ordering

#### Optimizer
- Query rewriting
- Index usage optimization
- Vector index routing

### 3. Virtual Machine (VDBE)

The Virtual Database Engine executes compiled bytecode:

```
SQL Query → Parse Tree → Bytecode → VDBE → Results
```

Key features:
- Register-based VM
- Type-safe operations
- Async-aware execution

### 4. Storage Engine

#### B-Tree Structure
```
Database File Layout:
┌─────────────┬─────────────┬─────────────┬──────────┐
│   Header    │   Page 1    │   Page 2    │   ...    │
│   (100B)    │  (B-tree)   │  (B-tree)   │ (Pages)  │
└─────────────┴─────────────┴─────────────┴──────────┘
```

#### Page Cache
- In-memory page caching
- LRU eviction policy
- Dirty page tracking

#### Write-Ahead Logging (WAL)
```
┌─────────────────────────────────────┐
│         WAL Mode Operation          │
├─────────────────────────────────────┤
│  1. Write changes to WAL file       │
│  2. Update shared memory index      │
│  3. Checkpoint to main database     │
│  4. Truncate WAL                    │
└─────────────────────────────────────┘
```

### 5. Async I/O Layer

#### io_uring Integration (Linux)
```rust
// io_uring provides async system calls
let ring = IoUring::new(32)?;

// Submit async read operation
let read_op = opcode::Read::new(fd, buf.as_mut_ptr(), buf.len())
    .build()
    .user_data(0x01);

ring.submission().push(&read_op)?;
ring.submit_and_wait(1)?;
```

#### Tokio Runtime
```rust
// libSQL uses Tokio for async runtime
#[tokio::main]
async fn main() {
    let db = Builder::new_local("data.db").build().await?;
    // All operations are async
}
```

## Data Flow

### Read Path
```
1. SQL Query
   ↓
2. Parse → AST
   ↓
3. Plan → Execution Plan
   ↓
4. Optimize
   ↓
5. Compile → Bytecode
   ↓
6. Execute (VDBE)
   ↓
7. Fetch Pages (Cache or Disk)
   ↓
8. Return Results
```

### Write Path
```
1. SQL INSERT/UPDATE/DELETE
   ↓
2. Parse & Plan
   ↓
3. Acquire Locks (MVCC)
   ↓
4. Write to WAL
   ↓
5. Update Page Cache
   ↓
6. Notify CDC Listeners
   ↓
7. Return Success
   ↓
8. (Async) Checkpoint to DB
```

## Concurrency Model

### MVCC (Multi-Version Concurrency Control)
```rust
// Multiple readers, single writer
// BEGIN CONCURRENT allows multiple writers

// Connection 1
let tx1 = conn.transaction().await?;
tx1.execute("UPDATE users SET name = 'Alice' WHERE id = 1", ()).await?;

// Connection 2 - can read old version
let tx2 = conn2.transaction().await?;
let name: String = tx2.query_row("SELECT name FROM users WHERE id = 1", (), |row| {
    row.get(0)
}).await?;
// name is still old value until tx1 commits
```

### Locking Strategy
```
┌──────────────────────────────────────────┐
│           Lock Hierarchy                 │
├──────────────────────────────────────────┤
│  1. Database Lock (short duration)       │
│  2. WAL Write Lock (per transaction)     │
│  3. Page Locks (fine-grained)            │
│  4. Schema Locks (DDL operations)        │
└──────────────────────────────────────────┘
```

## Memory Management

### Page Cache
```rust
pub struct PageCache {
    cache: LruCache<PageNumber, Page>,
    max_size: usize,      // Configurable
    dirty_pages: HashSet<PageNumber>,
}
```

### Connection Pooling
```rust
// Automatic connection pooling
let db = Builder::new_local("data.db")
    .max_connections(10)  // Connection pool size
    .build().await?;
```

## Extension Architecture

### Virtual Tables
```rust
// Custom virtual table implementation
struct MyVirtualTable;

impl VTab for MyVirtualTable {
    fn open(&self) -> Result<VTabCursor> {
        // Custom data source
    }
}
```

### Custom Functions
```rust
// Register custom SQL functions
conn.create_function(
    "my_function",
    1,  // argc
    FunctionFlags::DETERMINISTIC,
    |ctx| {
        let arg = ctx.get::<String>(0)?;
        Ok(arg.to_uppercase())
    }
)?;
```

## Next Steps

- **Rust Implementation**: [rust-implementation.md](./rust-implementation.md)
- **SQLite Compatibility**: [sqlite-compatibility.md](./sqlite-compatibility.md)
- **Performance**: [performance-characteristics.md](./performance-characteristics.md)