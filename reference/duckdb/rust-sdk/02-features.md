# DuckDB Rust SDK — Feature Flags

> Complete reference for `duckdb` crate Cargo features

## Feature Categories

```
┌─────────────────────────────────────────────┐
│              Feature Groups                 │
├─────────────────────────────────────────────┤
│ Build Config                                │
│   bundled, buildtime_bindgen                │
├─────────────────────────────────────────────┤
│ Data Formats                                │
│   json, parquet                             │
├─────────────────────────────────────────────┤
│ Virtual Tables                              │
│   vtab, vtab-arrow, vtab-excel, vscalar     │
├─────────────────────────────────────────────┤
│ Integration                                 │
│   polars, arrow integration                 │
├─────────────────────────────────────────────┤
│ Modern Rust                                 │
│   chrono, serde_json, url, r2d2, uuid       │
├─────────────────────────────────────────────┤
│ Convenience                                 │
│   vtab-full, extensions-full, modern-full │
├─────────────────────────────────────────────┤
│ Advanced                                    │
│   loadable-extension, appender-arrow          │
└─────────────────────────────────────────────┘
```

## Build Configuration

### `bundled` ⭐ Recommended

Compiles DuckDB C++ library from source and links statically.

```toml
[dependencies]
duckdb = { version = "1.4.4", features = ["bundled"] }
```

| Aspect | With `bundled` | Without `bundled` |
|--------|----------------|-------------------|
| DuckDB source | Embedded in crate | Must install separately |
| Build time | Longer (C++ compile) | Faster |
| Binary size | Larger | Smaller (if system DuckDB) |
| ICU extension | Must load at runtime | Built-in (if system has it) |
| Portability | Excellent | Requires system library |
| CI/CD | Just works | Needs libduckdb installed |

### `buildtime_bindgen`

Regenerate FFI bindings at build time instead of using pre-generated.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "buildtime_bindgen"] }
```

- Requires Clang/LLVM
- Slower builds
- Only needed for custom DuckDB versions

## Data Format Extensions

### `json`

Enable reading/writing JSON files. Requires `bundled`.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "json"] }
```

```rust
// Query JSON file
let batches = conn.query_arrow(
    "SELECT * FROM 'data.json'",
    [],
)?;
```

### `parquet`

Enable reading/writing Parquet files. Requires `bundled`.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "parquet"] }
```

```rust
// Read Parquet
let batches = conn.query_arrow(
    "SELECT * FROM 'data.parquet'",
    [],
)?;

// Write Parquet
conn.execute(
    "COPY (SELECT * FROM t) TO 'output.parquet'",
    [],
)?;
```

## Virtual Table Features

### `vtab`

Base support for creating custom table functions.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vtab"] }
```

```rust
use duckdb::vtab::{DataChunk, InitArgs, TableFunction};

// Create a custom table function
let func = TableFunction::new("my_func", my_bind, my_init, my_func);
conn.register_table_function(&func)?;
```

### `vtab-arrow`

Apache Arrow integration for virtual tables.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vtab-arrow"] }
```

```rust
// Return Arrow RecordBatch from table function
// Enables zero-copy data transfer
```

### `vtab-excel`

Read Excel files directly in SQL queries.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vtab-excel"] }
```

```rust
// Query Excel without explicit extension install
let batches = conn.query_arrow(
    "SELECT * FROM 'data.xlsx'",
    [],
)?;
```

### `vscalar`

Create custom scalar functions.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vscalar"] }
```

```rust
use duckdb::vscalar::ScalarFunction;

// Register Rust function as SQL function
let func = ScalarFunction::new("my_add", |args| {
    let a: i64 = args[0].get()?;
    let b: i64 = args[1].get()?;
    Ok(a + b)
});
conn.register_scalar_function(&func)?;

// Use in SQL
let result: i64 = conn.query_row("SELECT my_add(1, 2)", [], |r| r.get(0))?;
```

### `vscalar-arrow`

Arrow-optimized scalar functions.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vscalar-arrow"] }
```

Vectorized operations on Arrow arrays for better performance.

## Integration Features

### `polars`

Polars DataFrame integration.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "polars"] }
```

```rust
// Query to Polars DataFrame
let df = conn.query_polars("SELECT * FROM t", [])?;

// Register Polars DataFrame as table
conn.register_polars("my_df", &df)?;
let result = conn.query_arrow("SELECT * FROM my_df", [])?;
```

### Arrow Re-exports

The `arrow` crate is always re-exported via `duckdb::arrow`:

```rust
use duckdb::arrow::record_batch::RecordBatch;

// Available without explicit feature
let batches: Vec<RecordBatch> = conn.query_arrow("...", [])?;
```

## Modern Rust Features

### `chrono`

Chrono date/time types support.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "chrono"] }
```

```rust
use chrono::NaiveDate;

// NaiveDate automatically maps to DuckDB DATE
let date: NaiveDate = conn.query_row(
    "SELECT DATE '2024-01-01'",
    [],
    |r| r.get(0),
)?;
```

### `serde_json`

JSON serialization support.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "serde_json"] }
```

```rust
use serde_json::Value;

// Query JSON columns to serde_json::Value
let json: Value = conn.query_row(
    "SELECT '{\"a\":1}'::JSON",
    [],
    |r| r.get(0),
)?;
```

### `url`

URL type support.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "url"] }
```

```rust
use url::Url;

// URL parsing integration
```

### `r2d2`

Connection pooling support.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "r2d2"] }
```

```rust
use duckdb::DuckdbConnectionManager;
use r2d2::Pool;

let manager = DuckdbConnectionManager::file("mydb.duckdb")?;
let pool = Pool::new(manager)?;

let conn = pool.get()?;
// Use conn...
```

### `uuid`

UUID type support.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "uuid"] }
```

```rust
use uuid::Uuid;

let id: Uuid = Uuid::new_v4();
conn.execute("INSERT INTO t (id) VALUES (?)", params![id])?;
```

## Convenience Combinations

### `vtab-full`

All virtual table features:
- `vtab-excel`
- `vtab-arrow`
- `appender-arrow`

```toml
duckdb = { version = "1.4.4", features = ["bundled", "vtab-full"] }
```

### `extensions-full`

All major extensions:
- `json`
- `parquet`
- `vtab-full`

```toml
duckdb = { version = "1.4.4", features = ["bundled", "extensions-full"] }
```

### `modern-full`

Modern Rust ecosystem:
- `chrono`
- `serde_json`
- `url`
- `r2d2`
- `uuid`
- `polars`

```toml
duckdb = { version = "1.4.4", features = ["bundled", "modern-full"] }
```

## Advanced Features

### `loadable-extension`

Experimental: Create loadable DuckDB extensions in Rust.

```toml
duckdb = { version = "1.4.4", features = ["loadable-extension"] }
```

Includes procedural macros for extension development.

```rust
#[duckdb::extension_entrypoint]
pub fn my_extension_init(conn: &mut duckdb::Connection) -> duckdb::Result<()> {
    // Register functions, types, etc.
    Ok(())
}
```

### `appender-arrow`

Efficient bulk insertion of Arrow data.

```toml
duckdb = { version = "1.4.4", features = ["bundled", "appender-arrow"] }
```

```rust
use duckdb::arrow::record_batch::RecordBatch;

let batch: RecordBatch = // ...
conn.append_arrow("my_table", &batch)?;
```

## Feature Decision Matrix

### Quick Start

| Profile | Features |
|---------|----------|
| Minimal | `bundled` |
| Standard | `bundled`, `json`, `parquet` |
| Data Science | `bundled`, `extensions-full`, `polars` |
| Full | `bundled`, `extensions-full`, `modern-full` |

### By Use Case

| Use Case | Required Features |
|----------|-----------------|
| Basic SQL queries | `bundled` |
| Cloud/S3 analytics | `bundled`, `json`, `parquet` |
| DataFrame workflows | `bundled`, `polars` |
| Excel processing | `bundled`, `vtab-excel` |
| Custom SQL functions | `bundled`, `vscalar` |
| Extension development | `bundled`, `loadable-extension` |
| Production web app | `bundled`, `r2d2`, `chrono`, `uuid` |
| ETL pipelines | `bundled`, `extensions-full`, `modern-full` |

## Feature Size Impact

Approximate binary size increases:

| Feature | Size Increase |
|---------|---------------|
| `bundled` | +20-30MB (DuckDB itself) |
| `json` | +1MB |
| `parquet` | +2MB |
| `polars` | +5-10MB (with dependencies) |
| `vtab-*` | +500KB each |
| `chrono` | +200KB |
| `r2d2` | +100KB |

## Build Time Optimization

### Release Profile

```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = true
```

### Feature Selection

```toml
# Only enable what you need
duckdb = { version = "1.4.4", features = [
    "bundled",
    "json",      # Only if reading JSON
    "parquet",   # Only if reading Parquet
] }
```

### Caching

```bash
# Enable sccache for faster rebuilds
export RUSTC_WRAPPER=sccache

# Or use cargo cache
cargo install cargo-cache
cargo cache --autoclean
```

## Version Compatibility

| duckdb crate | DuckDB C++ | libduckdb-sys |
|--------------|------------|---------------|
| 1.4.4 | v1.4.1 | 1.4.4 |
| 1.4.3 | v1.4.1 | 1.4.3 |
| 1.4.0 | v1.4.0 | 1.4.0 |
| 0.10.2 | v0.10.2 | 0.10.2 |

Extensions must match DuckDB version exactly.

## Cross-References

- [Rust SDK Overview](01-overview.md) — API reference
- [Core Extensions](../../core-extensions/01-overview.md) — Built-in extensions
- [Building from Source](https://github.com/duckdb/duckdb-rs/blob/main/CONTRIBUTING.md) — Development setup
