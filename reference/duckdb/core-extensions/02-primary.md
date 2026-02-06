# Primary Core Extensions

> Built-in and autoloadable extensions with community support coverage

## json

Read, write, and query JSON data.

| Property | Value |
|----------|-------|
| Built-in | ✅ Yes |
| Autoload | ✅ Yes |
| Tier | Primary |

### Usage

```sql
-- Read JSON file
SELECT * FROM 'data.json';

-- Read JSON array
SELECT * FROM read_json_auto('array.json');

-- Extract nested fields
SELECT json_extract(data, '$.name') FROM users;

-- Create JSON
SELECT to_json({'name': 'Alice', 'age': 30});

-- Aggregate to JSON array
SELECT json_group_array(name) FROM users;
```

### Rust
```rust
// JSON support included with bundled feature
// Enable explicit: features = ["json", "bundled"]
```

## parquet

Read and write Apache Parquet columnar format.

| Property | Value |
|----------|-------|
| Built-in | ✅ Yes |
| Autoload | ✅ Yes |
| Tier | Primary |

### Usage

```sql
-- Read Parquet
SELECT * FROM 'data.parquet';

-- Write Parquet
COPY (SELECT * FROM users) TO 'users.parquet';

-- Parquet metadata
SELECT * FROM parquet_metadata('data.parquet');

-- Parquet schema
SELECT * FROM parquet_schema('data.parquet');

-- Filter pushdown (auto)
SELECT * FROM 'data.parquet' WHERE id > 100;

-- Partitioned write
COPY (SELECT * FROM events) 
TO 'events/' 
(FORMAT PARQUET, PARTITION_BY (year, month));
```

### Rust
```rust
// Parquet support: features = ["parquet", "bundled"]
// Arrow integration for zero-copy
use duckdb::arrow::record_batch::RecordBatch;

let batches: Vec<RecordBatch> = conn.query_arrow(
    "SELECT * FROM 'data.parquet'",
    []
)?;
```

## icu

International Components for Unicode — time zones, collations, date formatting.

| Property | Value |
|----------|-------|
| Built-in | Partial (without bundled) |
| Autoload | ✅ Yes |
| Tier | Primary |

### Usage

```sql
-- Install if using bundled feature
INSTALL icu;
LOAD icu;

-- Time zone conversion
SELECT '2024-01-01'::TIMESTAMPTZ AT TIME ZONE 'Europe/Amsterdam';

-- Current time in zone
SELECT now() AT TIME ZONE 'America/New_York';

-- Collation
SELECT * FROM names ORDER BY name COLLATE "de_DE";

-- Interval arithmetic
SELECT now() - INTERVAL '1 day';
```

### Rust Note

With `bundled` feature, ICU is excluded due to size limits. Load at runtime:

```rust
conn.execute_batch("INSTALL icu; LOAD icu;")?;
```

Without `bundled`, ICU is built-in.

## httpfs

HTTP(S) and S3 filesystem abstraction. The most important extension for cloud workflows.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ✅ Yes |
| Tier | Primary |
| Aliases | `http`, `https`, `s3` |

### Usage — HTTP

```sql
INSTALL httpfs;
LOAD httpfs;

-- Read CSV from URL
SELECT * FROM 'https://example.com/data.csv';

-- Read Parquet from URL
SELECT * FROM 'https://example.com/data.parquet';

-- Read JSON from URL
SELECT * FROM read_json_auto('https://api.example.com/data');
```

### Usage — S3

```sql
-- S3 without credentials (public buckets)
SELECT * FROM 's3://bucket/data.parquet';

-- With credentials via secrets
CREATE SECRET my_s3 (
    TYPE S3,
    KEY_ID 'AKIA...',
    SECRET '...',
    REGION 'us-east-1'
);

-- Query S3
SELECT * FROM 's3://mybucket/data/*.parquet';

-- Write to S3
COPY (SELECT * FROM results) 
TO 's3://mybucket/results.parquet';
```

### S3-compatible Services

```sql
-- Cloudflare R2
CREATE SECRET r2 (
    TYPE S3,
    KEY_ID '...',
    SECRET '...',
    ENDPOINT 'https://<account>.r2.cloudflarestorage.com',
    URL_STYLE 'path'
);

-- Google Cloud Storage (GCS)
CREATE SECRET gcs (
    TYPE GCS,
    HMAC_KEY_ID '...',
    HMAC_SECRET '...'
);

-- MinIO / LocalStack
CREATE SECRET local (
    TYPE S3,
    KEY_ID 'minio',
    SECRET 'minio123',
    ENDPOINT 'http://localhost:9000',
    URL_STYLE 'path',
    USE_SSL false
);
```

### Rust

```rust
conn.execute_batch(r#"
    INSTALL httpfs;
    LOAD httpfs;
    
    CREATE SECRET s3 (
        TYPE S3,
        KEY_ID 'AKIA...',
        SECRET '...',
        REGION 'us-east-1'
    );
"#)?;

// Query remote Parquet
let batches = conn.query_arrow(
    "SELECT * FROM 's3://bucket/data.parquet'",
    []
)?;
```

### Environment Authentication

DuckDB auto-detects standard AWS credential sources:

1. Environment: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`
2. `~/.aws/credentials`
3. IAM role (EC2/ECS/Lambda)

```sql
-- Use environment credentials
CREATE SECRET env_s3 (TYPE S3);
```

## Feature Comparison

```bash
# Quick reference — extension capabilities

Extension │ Read  │ Write │ Cloud │ Local │ Auto
──────────┼───────┼───────┼───────┼───────┼───────
json      │ JSON  │ JSON  │ ✅    │ ✅    │ ✅
parquet   │ PQT   │ PQT   │ ✅    │ ✅    │ ✅
icu       │ —     │ —     │ —     │ —     │ ✅
httpfs    │ *     │ *     │ ✅    │ —     │ ✅

* httpfs enables cloud read/write for json/parquet/csv
```

## When to Use

| Scenario | Extension |
|----------|-----------|
| Process local JSON | `json` (auto) |
| Process local Parquet | `parquet` (auto) |
| Time zone math | `icu` (auto or explicit) |
| S3/HTTP data | `httpfs` (auto on first use) |
| Cloud analytics | `httpfs` + `parquet` |
| ETL pipeline | `httpfs` + `parquet` + `json` |

## Quick Commands

```bash
# Install all primary extensions
duckdb -c "INSTALL httpfs; INSTALL icu"

# Check loaded extensions
duckdb -c "SELECT extension_name FROM duckdb_extensions() WHERE loaded"

# Verify autoload works (no explicit LOAD needed)
duckdb -c "SELECT * FROM 'https://raw.githubusercontent.com/duckdb/duckdb-web/main/data/weather.csv'"
```
