# DuckDB Quick Start

Fastest way to get started with DuckDB in Rust.

## 1. Install

```bash
cargo add duckdb -F bundled
```

## 2. Basic Query

```rust
use duckdb::{Connection, Result};

fn main() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    
    conn.execute_batch(r#"
        CREATE TABLE users (id INTEGER, name TEXT);
        INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');
    "#)?;
    
    let mut stmt = conn.prepare("SELECT * FROM users")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
    })?;
    
    for row in rows {
        let (id, name) = row?;
        println!("{}: {}", id, name);
    }
    
    Ok(())
}
```

## 3. Load Extensions

```rust
conn.execute_batch(r#"
    INSTALL httpfs;
    LOAD httpfs;
")?;

// Query remote Parquet
let df = conn.query_arrow(
    "SELECT * FROM 'https://example.com/data.parquet' LIMIT 10",
    []
)?;
```

## 4. Key Commands

| Task | SQL | Rust |
|------|-----|------|
| Open file | `ATTACH 'db.duckdb'` | `Connection::open("db.duckdb")` |
| In-memory | — | `Connection::open_in_memory()` |
| Install ext | `INSTALL httpfs` | `conn.execute("INSTALL httpfs", [])` |
| List exts | `SELECT * FROM duckdb_extensions()` | same |
| Export Parquet | `COPY (SELECT ...) TO 'out.parquet'` | same |

## 5. Next Steps

- [Core Extensions](core-extensions/01-overview.md) — httpfs, json, parquet, spatial
- [Rust SDK](rust-sdk/01-overview.md) — Full API reference
- [Feature Flags](rust-sdk/02-features.md) — Enable vtab, polars, arrow
