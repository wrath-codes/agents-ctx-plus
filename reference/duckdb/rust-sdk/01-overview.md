# DuckDB Rust SDK

> Ergonomic Rust wrapper for DuckDB — `duckdb` crate on crates.io

## Overview

The `duckdb` crate provides a Rust interface to DuckDB with:
- Type-safe SQL bindings (rusqlite-inspired API)
- Zero-copy Arrow integration
- Polars DataFrame support
- Extension loading
- Appender API for bulk inserts

```
┌─────────────────────────────────────────┐
│           Your Rust Application         │
├─────────────────────────────────────────┤
│              duckdb crate               │
│         (ergonomic Rust API)            │
├─────────────────────────────────────────┤
│         libduckdb-sys crate             │
│      (FFI bindings to C API)            │
├─────────────────────────────────────────┤
│         libduckdb (C library)           │
│         (DuckDB embedded engine)        │
└─────────────────────────────────────────┘
```

## Installation

### Cargo.toml

```toml
[dependencies]
# Recommended: bundled compiles DuckDB from source
duckdb = { version = "1.4.4", features = ["bundled"] }

# With Arrow support
duckdb = { version = "1.4.4", features = ["bundled", "vtab-arrow"] }

# With Polars support
duckdb = { version = "1.4.4", features = ["bundled", "polars"] }

# Full featured
duckdb = { version = "1.4.4", features = ["extensions-full", "modern-full"] }
```

### From Git (Development)

```toml
[dependencies]
duckdb = { git = "https://github.com/duckdb/duckdb-rs", branch = "main", features = ["bundled"] }
```

## Quick Start

```rust
use duckdb::{params, Connection, Result};

#[derive(Debug)]
struct User {
    id: i32,
    name: String,
}

fn main() -> Result<()> {
    // In-memory database
    let conn = Connection::open_in_memory()?;
    
    // Create table
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
        [],
    )?;
    
    // Insert with parameters
    conn.execute(
        "INSERT INTO users (id, name) VALUES (?, ?)",
        params![1, "Alice"],
    )?;
    
    // Batch insert
    conn.execute_batch(r#"
        INSERT INTO users VALUES (2, 'Bob');
        INSERT INTO users VALUES (3, 'Carol');
    "#)?;
    
    // Query with mapping
    let mut stmt = conn.prepare("SELECT id, name FROM users")?;
    let user_iter = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    
    for user in user_iter {
        println!("{:?}", user?);
    }
    
    Ok(())
}
```

## Connection Types

### In-Memory

```rust
// Fast, ephemeral, no persistence
let conn = Connection::open_in_memory()?;

// Multiple connections to same in-memory DB (via ATTACH not supported)
// Each Connection::open_in_memory() is isolated
```

### File-based

```rust
// Persistent database
let conn = Connection::open("mydb.duckdb")?;

// Create if not exists, open otherwise
let conn = Connection::open("new_or_existing.duckdb")?;

// Custom path
let conn = Connection::open("/path/to/db.duckdb")?;
```

### With Configuration

```rust
use duckdb::Config;

let config = Config::default()
    .access_mode(duckdb::AccessMode::ReadWrite)?
    .threads(4)?
    .memory_limit("1GB")?;

let conn = Connection::open_with_flags("mydb.duckdb", config)?;
```

## Query Patterns

### Execute (No Results)

```rust
// DDL, INSERT without RETURNING
conn.execute(
    "CREATE TABLE t (a INTEGER)",
    [],
)?;

let rows_affected = conn.execute(
    "INSERT INTO t VALUES (1), (2), (3)",
    [],
)?;
println!("Inserted {} rows", rows_affected);
```

### Prepared Statements

```rust
let mut stmt = conn.prepare("INSERT INTO t VALUES (?)")?;

for i in 0..100 {
    stmt.execute(params![i])?;
}

// Drop explicitly or let it go out of scope
// stmt.finalize()?;  // Optional
```

### Query with Mapping

```rust
let mut stmt = conn.prepare("SELECT id, name FROM users WHERE active = ?")?;

let users = stmt.query_map(params![true], |row| {
    Ok(User {
        id: row.get(0)?,
        name: row.get(1)?,
    })
})?;

for user in users {
    println!("{:?}", user?);
}
```

### Single Value

```rust
let count: i64 = conn.query_row(
    "SELECT COUNT(*) FROM users",
    [],
    |row| row.get(0),
)?;

let name: Option<String> = conn.query_row(
    "SELECT name FROM users WHERE id = ?",
    params![999],
    |row| row.get(0),
).optional()?;  // Returns None if no rows
```

### Batch Execution

```rust
conn.execute_batch(r#"
    CREATE TABLE a (i INTEGER);
    CREATE TABLE b (i INTEGER);
    INSERT INTO a VALUES (1), (2);
    INSERT INTO b SELECT * FROM a;
"#)?;
```

## Type Mapping

### Rust to DuckDB (ToSql)

| Rust Type | DuckDB Type |
|-----------|-------------|
| `bool` | BOOLEAN |
| `i8` `i16` `i32` `i64` | TINYINT, SMALLINT, INTEGER, BIGINT |
| `u8` `u16` `u32` `u64` | UTINYINT, USMALLINT, UINTEGER, UBIGINT |
| `f32` `f64` | FLOAT, DOUBLE |
| `String` `&str` | VARCHAR |
| `Vec<u8>` | BLOB |
| `Option<T>` | Nullable T |
| `chrono::NaiveDate` | DATE |
| `chrono::NaiveTime` | TIME |
| `chrono::NaiveDateTime` | TIMESTAMP |
| `Vec<T>` (when T: ToSql) | ARRAY |

### DuckDB to Rust (FromSql)

| DuckDB Type | Rust Type |
|-------------|-----------|
| BOOLEAN | `bool` |
| TINYINT | `i8` |
| SMALLINT | `i16` |
| INTEGER | `i32` |
| BIGINT | `i64` |
| UTINYINT | `u8` |
| USMALLINT | `u16` |
| UINTEGER | `u32` |
| UBIGINT | `u64` |
| FLOAT | `f32` |
| DOUBLE | `f64` |
| VARCHAR | `String` |
| BLOB | `Vec<u8>` |
| DATE | `chrono::NaiveDate` |
| TIME | `chrono::NaiveTime` |
| TIMESTAMP | `chrono::NaiveDateTime` |
| ARRAY | `Vec<T>` |
| LIST | `Vec<T>` |
| STRUCT | Custom struct (via FromSql) |

## Arrow Integration

### Query to Arrow

```rust
use duckdb::arrow::record_batch::RecordBatch;

let batches: Vec<RecordBatch> = conn.query_arrow(
    "SELECT * FROM large_table",
    [],
)?;

for batch in batches {
    println!("Batch with {} rows", batch.num_rows());
    // Process with arrow ecosystem
}
```

### Arrow to DuckDB

```rust
use duckdb::arrow::record_batch::RecordBatch;

// Create table from Arrow batches
let batches: Vec<RecordBatch> = // ... from parquet, etc.

// Register as view
conn.register_arrow("my_view", &batches)?;

// Query it
let result = conn.query_arrow("SELECT * FROM my_view WHERE ...", [])?;
```

### Streaming Results

```rust
let stream = conn.query_arrow_stream(
    "SELECT * FROM very_large_table",
    [],
)?;

for batch in stream {
    let batch = batch?;
    // Process each batch as it arrives
}
```

## Polars Integration

### Query to Polars DataFrame

```rust
#[cfg(feature = "polars")]
use duckdb::Polars;

let df = conn.query_polars(
    "SELECT * FROM users",
    [],
)?;

println!("{:?}", df);
```

### Polars to DuckDB

```rust
use polars::prelude::*;

let df = df! {
    "id" => [1, 2, 3],
    "name" => ["a", "b", "c"],
}?;

conn.register_polars("my_data", &df)?;

// Query the registered DataFrame
let result = conn.query_arrow("SELECT * FROM my_data WHERE id > 1", [])?;
```

## Appender API (Bulk Insert)

```rust
// Faster than individual INSERTs
let mut appender = conn.appender("users")?;

// Append single row
appender.append_row(params![1, "Alice"])?;

// Append multiple rows
appender.append_rows([
    [2, "Bob"],
    [3, "Carol"],
    [4, "Dave"],
])?;

// Flush to disk
appender.flush()?;
```

## Transactions

```rust
// Manual transaction
let tx = conn.transaction()?;

tx.execute("INSERT INTO t VALUES (1)", [])?;
tx.execute("INSERT INTO t VALUES (2)", [])?;

// Commit or rollback
tx.commit()?;
// or tx.rollback()?;

// Transaction is also rolled back on Drop if not committed
```

## Error Handling

```rust
use duckdb::{Error, Result};

match conn.execute("INVALID SQL", []) {
    Err(Error::DuckDBFailure(err, msg)) => {
        eprintln!("DuckDB error {}: {:?}", err.extended_code, msg);
    }
    Err(Error::InvalidParameterCount(expected, got)) => {
        eprintln!("Expected {} params, got {}", expected, got);
    }
    Err(e) => eprintln!("Other error: {}", e),
    Ok(_) => println!("Success"),
}
```

## Extensions in Rust

```rust
// Install and load
conn.execute_batch(r#"
    INSTALL httpfs;
    LOAD httpfs;
    
    INSTALL spatial;
    LOAD spatial;
"#)?;

// Configure S3
conn.execute_batch(r#"
    CREATE SECRET s3 (
        TYPE S3,
        KEY_ID 'AKIA...',
        SECRET '...',
        REGION 'us-east-1'
    );
"#)?;

// Query S3 Parquet
let batches = conn.query_arrow(
    "SELECT * FROM 's3://bucket/data.parquet'",
    [],
)?;
```

## Async Support

DuckDB itself is synchronous, but works in async contexts:

```rust
use tokio::task;

async fn query_async(conn_str: &str, sql: &str) -> Result<Vec<RecordBatch>> {
    let sql = sql.to_string();
    
    // Run in blocking thread pool
    task::spawn_blocking(move || {
        let conn = Connection::open(conn_str)?;
        conn.query_arrow(&sql, [])
    })
    .await
    .expect("Join error")
}
```

## Resource Management

### RAII Pattern

```rust
{
    let conn = Connection::open_in_memory()?;
    // ... use conn
} // Connection closed automatically via Drop

// Explicit close
let conn = Connection::open_in_memory()?;
conn.close()?; // Returns Result to handle errors
```

### Interrupting Queries

```rust
let handle = conn.interrupt_handle();

// In another thread / async task
std::thread::spawn(move || {
    std::thread::sleep(Duration::from_secs(5));
    handle.interrupt();
});

// This will be interrupted after 5 seconds
conn.execute("SLOW QUERY", [])?; // Returns Error if interrupted
```

## Configuration Reference

```rust
use duckdb::Config;

let config = Config::default()
    // Access mode
    .access_mode(duckdb::AccessMode::ReadOnly)?  // or ReadWrite, Automatic
    
    // Threads
    .threads(8)?  // Max worker threads
    
    // Memory
    .memory_limit("2GB")?
    
    // Default ordering
    .default_order(duckdb::DefaultOrder::Desc)?
    .default_null_order(duckdb::DefaultNullOrder::NullsLast)?
    
    // Enable progress bar
    .enable_progress_bar(true)?
    
    // Profiling
    .enable_profiling(true)?
    .profiling_output("/tmp/profile.json")?;

let conn = Connection::open_with_flags("mydb.duckdb", config)?;
```

## Cross-References

- [Feature Flags](02-features.md) — Cargo feature reference
- [Core Extensions](../core-extensions/01-overview.md) — Extension documentation
- [Community Extensions](../community-extensions/01-overview.md) — Third-party extensions
