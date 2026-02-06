# Community Extensions Reference

> Complete catalog of community extensions for DuckDB v1.4.4+

## By Category

### 🌐 Web & HTTP

| Extension | Description | Install |
|-----------|-------------|---------|
| `airport` | Arrow Flight support for querying remote Arrow Flight servers | `INSTALL airport FROM community` |
| `cache_httpfs` | Read-cached filesystem for httpfs | `INSTALL cache_httpfs FROM community` |
| `crawler` | SQL-native web crawler with HTML extraction | `INSTALL crawler FROM community` |
| `curl_httpfs` | httpfs with connection pool, HTTP/2, async IO | `INSTALL curl_httpfs FROM community` |
| `http_client` | HTTP Client Extension | `INSTALL http_client FROM community` |
| `http_request` | HTTP client with GET/POST/PUT/PATCH/DELETE | `INSTALL http_request FROM community` |
| `httpserver` | DuckDB HTTP API Server Extension | `INSTALL httpserver FROM community` |
| `web_archive` | Query Common Crawl and Wayback Machine CDX APIs | `INSTALL web_archive FROM community` |
| `webbed` | XML/HTML processing with XPath extraction | `INSTALL webbed FROM community` |
| `webdavfs` | Read/write files over WebDAV | `INSTALL webdavfs FROM community` |

### 🔐 Cryptography & Security

| Extension | Description | Install |
|-----------|-------------|---------|
| `crypto` | Cryptographic hash functions and HMAC | `INSTALL crypto FROM community` |
| `hashfuncs` | Non-cryptographic hashes (xxHash, rapidhash, MurmurHash3) | `INSTALL hashfuncs FROM community` |
| `boilstream` | Secure remote secrets with OPAQUE PAKE, MFA | `INSTALL boilstream FROM community` |

### 📊 Data Formats & File I/O

| Extension | Description | Install |
|-----------|-------------|---------|
| `anndata` | Read AnnData (.h5ad) for single-cell genomics | `INSTALL anndata FROM community` |
| `h5db` | Read HDF5 datasets and attributes | `INSTALL h5db FROM community` |
| `h3` | Hierarchical hexagonal geospatial indexing | `INSTALL h3 FROM community` |
| `json_schema` | Validate JSON with JSON schemas | `INSTALL json_schema FROM community` |
| `jsonata` | JSONata expression language for JSON querying | `INSTALL jsonata FROM community` |
| `lance` | Query Lance datasets | `INSTALL lance FROM community` |
| `markdown` | Read/write Markdown files with block-level parsing | `INSTALL markdown FROM community` |
| `nanoarrow` | Apache Arrow IPC format consumption/production | `INSTALL nanoarrow FROM community` |
| `pst` | Read Microsoft PST files (emails, contacts, appointments) | `INSTALL pst FROM community` |
| `rusty_sheet` | Excel/WPS/OpenDocument Spreadsheets reader | `INSTALL rusty_sheet FROM community` |
| `yaml` | Read YAML files with native type support | `INSTALL yaml FROM community` |
| `zipfs` | Read files within zip archives | `INSTALL zipfs FROM community` |

### 🗄️ Database Connectors

| Extension | Description | Install |
|-----------|-------------|---------|
| `bigquery` | Query Google BigQuery datasets | `INSTALL bigquery FROM community` |
| `cassandra` | Connect to Apache Cassandra, ScyllaDB, DataStax Astra | `INSTALL cassandra FROM community` |
| `elasticsearch` | Query Elasticsearch indices | `INSTALL elasticsearch FROM community` |
| `mongo` | Query MongoDB collections with SQL | `INSTALL mongo FROM community` |
| `mssql` | Connect to Microsoft SQL Server via TDS | `INSTALL mssql FROM community` |
| `msolap` | Connect to SQL Server Analysis Services (SSAS) | `INSTALL msolap FROM community` |
| `nanodbc` | Connect to any ODBC-compatible database | `INSTALL nanodbc FROM community` |
| `nats_js` | Query NATS JetStream message streams | `INSTALL nats_js FROM community` |
| `redis` | Redis-compatible client | `INSTALL redis FROM community` |
| `snowflake` | Query Snowflake databases | `INSTALL snowflake FROM community` |
| `sqlite` | Read/write SQLite (also a core extension) | `INSTALL sqlite FROM core` |

### 📈 Analytics & Statistics

| Extension | Description | Install |
|-----------|-------------|---------|
| `anofox_forecast` | Time series forecasting (ARIMA, SARIMA, ETS, TBATS) | `INSTALL anofox_forecast FROM community` |
| `anofox_statistics` | Statistical regression (OLS, Ridge, WLS) | `INSTALL anofox_statistics FROM community` |
| `datasketches` | Approximate distinct counts and quantiles | `INSTALL datasketches FROM community` |
| `quackstats` | Time series forecasting and statistics | `INSTALL quackstats FROM community` |
| `stochastic` | Statistical distribution functions | `INSTALL stochastic FROM community` |
| `system_stats` | System-level statistics monitoring | `INSTALL system_stats FROM community` |

### 🗺️ Geospatial

| Extension | Description | Install |
|-----------|-------------|---------|
| `a5` | Hierarchical pentagonal geospatial indexing | `INSTALL a5 FROM community` |
| `eeagrid` | EEA Reference Grid System support | `INSTALL eeagrid FROM community` |
| `geography` | Global spatial data processing on sphere | `INSTALL geography FROM community` |
| `h3` | Hierarchical hexagonal indexing (Uber H3) | `INSTALL h3 FROM community` |
| `lindel` | Z-Order, Hilbert and Morton curves | `INSTALL lindel FROM community` |
| `pdal` | Point cloud data manipulation | `INSTALL pdal FROM community` |
| `st_read_multi` | Read multiple geospatial files | `INSTALL st_read_multi FROM community` |

### 🤖 Machine Learning & AI

| Extension | Description | Install |
|-----------|-------------|---------|
| `faiss` | Access to FAISS indices for similarity search | `INSTALL faiss FROM community` |
| `flock` | LLM & RAG for analytics and semantic analysis | `INSTALL flock FROM community` |
| `infera` | In-database inference | `INSTALL infera FROM community` |
| `lsh` | Locality-sensitive hashing (LSH) | `INSTALL lsh FROM community` |
| `mlpack` | mlpack C++ machine learning library | `INSTALL mlpack FROM community` |
| `onager` | Graph data analytics | `INSTALL onager FROM community` |
| `open_prompt` | Interact with LLMs | `INSTALL open_prompt FROM community` |

### 🔧 Utilities & Tools

| Extension | Description | Install |
|-----------|-------------|---------|
| `bitfilters` | Probabilistic filters (quotient, XOR, binary fuse) | `INSTALL bitfilters FROM community` |
| `brew` | Homebrew casks, packages, formulas as tables | `INSTALL brew FROM community` |
| `bvh2sql` | BVH motion capture file parser | `INSTALL bvh2sql FROM community` |
| `chaos` | Throw DuckDB exceptions or raise signals (testing) | `INSTALL chaos FROM community` |
| `cronjob` | HTTP Cronjob Extension | `INSTALL cronjob FROM community` |
| `dns` | DNS lookups and reverse lookups | `INSTALL dns FROM community` |
| `duck_tails` | Git-aware data analysis, query git history | `INSTALL duck_tails FROM community` |
| `duckdb_mcp` | Model Context Protocol (MCP) for DuckDB | `INSTALL duckdb_mcp FROM community` |
| `duckherder` | Run DuckDB queries on remote server | `INSTALL duckherder FROM community` |
| `ducksync` | Query result caching between DuckDB and Snowflake | `INSTALL ducksync FROM community` |
| `erpl_web` | Connect to APIs via OData, GraphQL, REST | `INSTALL erpl_web FROM community` |
| `evalexpr_rhai` | Evaluate Rhai scripting language | `INSTALL evalexpr_rhai FROM community` |
| `fakeit` | Generate realistic fake/test data | `INSTALL fakeit FROM community` |
| `file_dialog` | Native file dialog chooser | `INSTALL file_dialog FROM community` |
| `fuzzycomplete` | Fuzzy matching based autocompletion | `INSTALL fuzzycomplete FROM community` |
| `gaggle` | Work with Kaggle datasets | `INSTALL gaggle FROM community` |
| `gsheets` | Read and write Google Sheets | `INSTALL gsheets FROM community` |
| `hostfs` | Navigate filesystem using SQL | `INSTALL hostfs FROM community` |
| `magic` | libmagic/file utilities | `INSTALL magic FROM community` |
| `marisa` | MARISA trie for fast string lookups | `INSTALL marisa FROM community` |
| `minijinja` | MiniJinja templating engine | `INSTALL minijinja FROM community` |
| `miniplot` | Interactive chart visualization | `INSTALL miniplot FROM community` |
| `monetary` | Currency-aware monetary values with exchange rates | `INSTALL monetary FROM community` |
| `netquack` | Parse and analyze domains, URIs, paths | `INSTALL netquack FROM community` |
| `observefs` | Filesystem IO observability | `INSTALL observefs FROM community` |
| `otlp` | Read OpenTelemetry metrics, logs, traces | `INSTALL otlp FROM community` |
| `parser_tools` | Parse SQL queries using DuckDB's native parser | `INSTALL parser_tools FROM community` |
| `prql` | PRQL (Pipelined Relational Query Language) | `INSTALL prql FROM community` |
| `psql` | PSQL piped SQL dialect | `INSTALL psql FROM community` |
| `quack` | Hello world demo extension | `INSTALL quack FROM community` |
| `quackstore` | Smart block-based caching for remote files | `INSTALL quackstore FROM community` |
| `quickjs` | QuickJS Runtime Extension | `INSTALL quickjs FROM community` |
| `radio` | Event buses (WebSocket, Redis pub/sub) | `INSTALL radio FROM community` |
| `rapidfuzz` | High-performance fuzzy string matching | `INSTALL rapidfuzz FROM community` |
| `rate_limit_fs` | Rate/burst limit on filesystem operations | `INSTALL rate_limit_fs FROM community` |
| `read_stat` | Read SAS, Stata, SPSS datasets | `INSTALL read_stat FROM community` |
| `scalarfs` | Virtual filesystems for scalars | `INSTALL scalarfs FROM community` |
| `shellfs` | Shell commands for input/output | `INSTALL shellfs FROM community` |
| `splink_udfs` | Record linkage functions (phonetic, address matching) | `INSTALL splink_udfs FROM community` |
| `sshfs` | Read/write files over SSH | `INSTALL sshfs FROM community` |
| `tera` | Tera templating engine | `INSTALL tera FROM community` |
| `textplot` | Text-based data visualization (ASCII charts) | `INSTALL textplot FROM community` |
| `tributary` | Apache Kafka interaction | `INSTALL tributary FROM community` |
| `tsid` | Time-Sortable ID generator | `INSTALL tsid FROM community` |
| `webmacro` | Load DuckDB Macros from the web | `INSTALL webmacro FROM community` |

### ⛓️ Blockchain

| Extension | Description | Install |
|-----------|-------------|---------|
| `blockduck` | Live SQL queries on Blockchain | `INSTALL blockduck FROM community` |
| `duck_delta_share` | Delta Sharing protocol support | `INSTALL duck_delta_share FROM community` |
| `mooncake` | Read Iceberg tables written by Moonlink | `INSTALL mooncake FROM community` |

### 🎲 Specialized Data

| Extension | Description | Install |
|-----------|-------------|---------|
| `aixchess` | Query large chess game collections | `INSTALL aixchess FROM community` |
| `chess` | Parse and analyze chess games in PGN format | `INSTALL chess FROM community` |
| `cwiqduck` | CWIQ filesystem extension | `INSTALL cwiqduck FROM community` |
| `fire_duck_ext` | Query Google Cloud Firestore | `INSTALL fire_duck_ext FROM community` |
| `psyduck` | Pokemon data native in DuckDB | `INSTALL psyduck FROM community` |

### 🧪 Experimental/Development

| Extension | Description | Install |
|-----------|-------------|---------|
| `capi_quack` | C/C++ C API template demo | `INSTALL capi_quack FROM community` |
| `fivetran` | Fivetran community extension | `INSTALL fivetran FROM community` |
| `gcs` | Google Cloud Storage extension | `INSTALL gcs FROM community` |
| `lua` | Evaluate Lua scripts within queries | `INSTALL lua FROM community` |

## Quick Install Commands

```bash
# Analytics & ML essentials
duckdb -c "
    INSTALL crypto FROM community;
    INSTALL datasketches FROM community;
    INSTALL anofox_forecast FROM community;
    INSTALL faiss FROM community;
"

# Data format support
duckdb -c "
    INSTALL yaml FROM community;
    INSTALL jsonata FROM community;
    INSTALL lance FROM community;
    INSTALL zipfs FROM community;
"

# Database connectivity
duckdb -c "
    INSTALL bigquery FROM community;
    INSTALL mongo FROM community;
    INSTALL redis FROM community;
    INSTALL elasticsearch FROM community;
"

# Web & HTTP
duckdb -c "
    INSTALL http_client FROM community;
    INSTALL httpserver FROM community;
    INSTALL webbed FROM community;
"
```

## Extension Search

```bash
# Search extensions by keyword
search_extensions() {
    local keyword=$1
    curl -s https://community-extensions.duckdb.org/extensions.json | \
        jq -r --arg k "$keyword" '
            .extensions[] | 
            select(.name | contains($k)) |
            "\(.name): \(.description)"
        '
}

# Usage
search_extensions "http"
search_extensions "geo"
search_extensions "json"
```

## Extension Count

Total extensions listed: **130+** (for DuckDB v1.4.4)

```bash
# Count extensions
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq '.extensions | length'
```
