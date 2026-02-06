# Secondary Core Extensions

> Best-effort support tier — specialized functionality for databases, formats, and analytics

## spatial

Geospatial data types, functions, and indexes.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No (explicit required) |
| Tier | Secondary |

### Installation

```sql
INSTALL spatial;
LOAD spatial;
```

### Usage

```sql
-- Create point
SELECT ST_Point(1.0, 2.0);

-- Parse WKT
SELECT ST_GeomFromText('POINT(1 2)');

-- GeoJSON
SELECT ST_GeomFromGeoJSON('{"type":"Point","coordinates":[1,2]}');

-- Distance
SELECT ST_Distance(
    ST_Point(0, 0),
    ST_Point(3, 4)
); -- 5.0

-- Within
SELECT * FROM cities 
WHERE ST_Within(geom, ST_GeomFromText('POLYGON(...)'));

-- Read GeoParquet
SELECT * FROM 'data.geoparquet';

-- Read shapefile
SELECT * FROM st_read('data.shp');
```

### Rust

```rust
conn.execute_batch(r#"
    INSTALL spatial;
    LOAD spatial;
"#)?;
```

## iceberg

Apache Iceberg lakehouse table format.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL iceberg;
LOAD iceberg;
```

### Usage

```sql
-- Attach Iceberg catalog
ATTACH 's3://bucket/warehouse' AS warehouse (TYPE ICEBERG);

-- Query table
SELECT * FROM warehouse.schema.table;

-- Time travel
SELECT * FROM warehouse.schema.table 
FOR SYSTEM_TIME AS OF '2024-01-01';

-- Iceberg REST catalog
CREATE SECRET iceberg_token (
    TYPE HTTP,
    BEARER_TOKEN 'token...'
);

ATTACH 'https://rest-catalog.example.com/' AS rest_cat (
    TYPE ICEBERG,
    REST_CATALOG_MODE 'SNOWFLAKE'
);
```

## delta

Delta Lake table format.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL delta;
LOAD delta;
```

### Usage

```sql
-- Read Delta table
SELECT * FROM delta_scan('s3://bucket/delta-table');

-- Attach Delta table
CREATE VIEW my_delta AS 
SELECT * FROM delta_scan('path/to/delta');

-- Time travel
SELECT * FROM delta_scan('s3://bucket/table', 
    version => 5);
```

## aws

AWS SDK integration for advanced S3 features.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL aws;
LOAD aws;
```

### Usage

```sql
-- Load credentials from AWS profile
CALL load_aws_credentials('default');

-- Use with httpfs
SELECT * FROM 's3://bucket/data.parquet';

-- S3 Express One Zone
CREATE SECRET s3_express (
    TYPE S3,
    KEY_ID '...',
    SECRET '...',
    REGION 'us-east-1',
    S3_EXPRESS true
);
```

## azure

Azure Blob Storage filesystem.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL azure;
LOAD azure;
```

### Usage

```sql
-- Create Azure secret
CREATE SECRET azure_secret (
    TYPE AZURE,
    CONNECTION_STRING 'DefaultEndpointsProtocol=...'
);

-- Query Azure Blob
SELECT * FROM 'azure://container/data.parquet';
```

## mysql

Read and write MySQL databases.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Aliases | `mysql_scanner` |
| Tier | Secondary |

### Installation

```sql
INSTALL mysql;
LOAD mysql;
```

### Usage

```sql
-- Attach MySQL database
ATTACH 'host=localhost user=root password=... database=mydb' 
AS mysqldb (TYPE MYSQL);

-- Query table
SELECT * FROM mysqldb.users;

-- Copy to MySQL
INSERT INTO mysqldb.users 
SELECT * FROM local_users;

-- Direct query without ATTACH
SELECT * FROM mysql_query('host=...', 'SELECT * FROM users');
```

## postgres

Read and write PostgreSQL databases.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Aliases | `postgres_scanner` |
| Tier | Secondary |

### Installation

```sql
INSTALL postgres;
LOAD postgres;
```

### Usage

```sql
-- Attach PostgreSQL database
ATTACH 'host=localhost user=postgres dbname=mydb' 
AS pgdb (TYPE POSTGRES);

-- Query table
SELECT * FROM pgdb.users;

-- Copy to PostgreSQL
INSERT INTO pgdb.users 
SELECT * FROM local_users;

-- Direct query
SELECT * FROM postgres_query('host=...', 'SELECT * FROM users');
```

## sqlite

Read and write SQLite databases.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Aliases | `sqlite_scanner`, `sqlite3` |
| Tier | Secondary |

### Installation

```sql
INSTALL sqlite;
LOAD sqlite;
```

### Usage

```sql
-- Attach SQLite database
ATTACH 'mydb.sqlite' AS sqlite_db (TYPE SQLITE);

-- Query table
SELECT * FROM sqlite_db.users;

-- Copy to SQLite
INSERT INTO sqlite_db.users 
SELECT * FROM local_users;
```

## excel

Read Excel files (.xlsx, .xls).

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL excel;
LOAD excel;
```

### Usage

```sql
-- Read Excel
SELECT * FROM 'data.xlsx';

-- Specific sheet
SELECT * FROM read_xlsx('data.xlsx', sheet='Sheet2');

-- Write Excel
COPY (SELECT * FROM users) TO 'users.xlsx';
```

## fts (Full Text Search)

Full-text search indexes on DuckDB tables.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL fts;
LOAD fts;
```

### Usage

```sql
-- Create FTS index
PRAGMA create_fts_index('documents', 'doc_id', 'title', 'content');

-- Search
SELECT * FROM documents 
WHERE doc_id IN (SELECT doc_id FROM fts_main_documents 
                 WHERE text MATCH 'search terms');

-- Drop index
PRAGMA drop_fts_index('documents');
```

## vss (Vector Similarity Search)

Approximate nearest neighbor search for embeddings.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL vss;
LOAD vss;
```

### Usage

```sql
-- Enable extension
LOAD vss;

-- Create table with vector
CREATE TABLE embeddings (
    id INTEGER,
    vec FLOAT[768]
);

-- Create HNSW index
CREATE INDEX idx ON embeddings 
USING HNSW (vec) WITH (metric = 'cosine');

-- Similarity search
SELECT * FROM embeddings 
ORDER BY array_distance(vec, [0.1, 0.2, ...]::FLOAT[768])
LIMIT 10;
```

## avro

Read Avro files.

| Property | Value |
|----------|-------|
| Built-in | ❌ No |
| Autoload | ❌ No |
| Tier | Secondary |

### Installation

```sql
INSTALL avro FROM core;
LOAD avro;
```

### Usage

```sql
SELECT * FROM 'data.avro';
```

## Quick Reference — Secondary Extensions

```bash
# Install all database connectors
duckdb -c "
    INSTALL mysql;
    INSTALL postgres;
    INSTALL sqlite;
"

# Install lakehouse formats
duckdb -c "
    INSTALL iceberg;
    INSTALL delta;
"

# Install cloud storage
duckdb -c "
    INSTALL aws;
    INSTALL azure;
    -- httpfs is autoloadable, no install needed
"

# List all extensions with their tier
duckdb -json -c "
    SELECT 
        extension_name,
        CASE extension_name
            WHEN 'json' THEN 'primary'
            WHEN 'parquet' THEN 'primary'
            WHEN 'icu' THEN 'primary'
            WHEN 'httpfs' THEN 'primary'
            ELSE 'secondary'
        END as tier
    FROM duckdb_extensions()
    WHERE installed
    ORDER BY tier, extension_name
" | jq -r '.[] | \"\(.tier) | \(.extension_name)\"'
```

## When to Use Secondary Extensions

| Use Case | Extension |
|----------|-----------|
| Geospatial analysis | `spatial` |
| Iceberg/Delta Lake | `iceberg`, `delta` |
| MySQL/PostgreSQL integration | `mysql`, `postgres` |
| SQLite migration | `sqlite` |
| Excel files | `excel` |
| Full-text search | `fts` |
| Vector search (embeddings) | `vss` |
| Avro format | `avro` |
