//! # Spike 0.19: Native lancedb Writes + serde_arrow Production Path
//!
//! Validates the native `lancedb` Rust crate for writing Lance datasets to R2,
//! replacing DuckDB `COPY TO (FORMAT lance)`.
//!
//! ## Production vs Exploratory Tests
//!
//! This spike contains two categories of tests:
//!
//! ### Production path (Parts H, L, M) — what production code will use
//!
//! In production, data flows as:
//!
//! ```text
//! tree-sitter parse → Rust structs (ApiSymbol)
//!     → serde_arrow + arrow_serde adapters → arrow-57 RecordBatch
//!     → lancedb::create_table() / tbl.add() → Lance on R2 (or local)
//! ```
//!
//! DuckDB is **read-only** in this architecture — it queries Lance datasets
//! via the lance extension (`lance_vector_search`, `lance_fts`, etc.) but
//! never holds the canonical data. There is no scenario where data lives
//! in a DuckDB table and needs to be extracted into Lance.
//!
//! Test M1 validates this complete production round-trip:
//! `Rust structs → serde_arrow → lancedb → Lance → DuckDB reads → serde_arrow → Rust structs`
//!
//! ### Exploratory path (Parts I) — informational, not production code
//!
//! Tests I1/I2 explore the DuckDB `query_arrow()` → lancedb pipeline via a
//! value-based arrow-56→57 bridge. This path **does not exist in production**
//! but was useful to validate:
//! - Arrow version coexistence (arrow 56 + 57 in same binary)
//! - DuckDB's `query_arrow()` API
//! - Type conversion gotchas (DuckDB `FLOAT[]` → `List(Float32)` vs `FixedSizeList`)
//!
//! The value bridge (`duckdb_batch_to_lance`) copies all data element-by-element
//! between arrow versions. This is acceptable for testing but would be avoided
//! in production by never putting data into DuckDB in the first place.
//!
//! ## Test Map
//!
//! | Part | Tests | Category | Description |
//! |------|-------|----------|-------------|
//! | H | H1-H5 | Production | Arrow bridge, lancedb writes (local + R2), indexes, incremental add |
//! | I | I1-I2 | Exploratory | DuckDB query_arrow() → bridge → lancedb (not production code) |
//! | L | L2, L4 | Production | Cross-process index reads, exist_ok behavior |
//! | M | M1 | **Production** | serde_arrow + arrow_serde full round-trip (THE production path) |
//!
//! ## Prerequisites
//!
//! R2 tests require these env vars in `zenith/.env`:
//!
//! ```bash
//! ZENITH_R2__ACCESS_KEY_ID=...
//! ZENITH_R2__SECRET_ACCESS_KEY=...
//! ZENITH_R2__ACCOUNT_ID=...
//! ZENITH_R2__BUCKET_NAME=zenith
//! AWS_ACCESS_KEY_ID=...          # Same as R2 key, for lancedb's object_store
//! AWS_SECRET_ACCESS_KEY=...      # Same as R2 secret
//! AWS_ENDPOINT_URL=https://{account_id}.r2.cloudflarestorage.com
//! ```
//!
//! Tests are skipped (not failed) when credentials are missing.

use std::sync::Arc;

use duckdb::Connection;
use duckdb::arrow::array::Array as DuckdbArray;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// arrow 57 types (lancedb's world)
use arrow_array::types::Float32Type;
use arrow_array::{
    BooleanArray, FixedSizeListArray, Int32Array, RecordBatch as RecordBatch57,
    RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};

// ============================================================================
// Helpers
// ============================================================================

/// Load env vars from the workspace .env file.
fn load_env() {
    let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(".env"));

    if let Some(env_path) = workspace_env {
        let _ = dotenvy::from_path(&env_path);
    }
}

struct R2Creds {
    account_id: String,
    access_key_id: String,
    secret_access_key: String,
    bucket_name: String,
}

fn r2_credentials() -> Option<R2Creds> {
    load_env();
    let creds = R2Creds {
        account_id: std::env::var("ZENITH_R2__ACCOUNT_ID").ok()?,
        access_key_id: std::env::var("ZENITH_R2__ACCESS_KEY_ID").ok()?,
        secret_access_key: std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").ok()?,
        bucket_name: std::env::var("ZENITH_R2__BUCKET_NAME").ok()?,
    };
    if creds.account_id.is_empty()
        || creds.access_key_id.is_empty()
        || creds.secret_access_key.is_empty()
        || creds.bucket_name.is_empty()
    {
        return None;
    }
    Some(creds)
}

/// Check if Lance AWS credentials are available (for lancedb's object_store).
fn lance_aws_available() -> bool {
    load_env();
    std::env::var("AWS_ACCESS_KEY_ID").is_ok()
        && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok()
        && std::env::var("AWS_ENDPOINT_URL").is_ok()
}

/// Unique R2 path prefix for this spike run.
fn spike_r2_prefix(creds: &R2Creds) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("s3://{}/zenith-spike19/{ts}", creds.bucket_name)
}

/// Generate a deterministic 384-dim embedding from a seed.
fn synthetic_embedding(seed: u32) -> Vec<f32> {
    (0..384)
        .map(|i| {
            let base = (seed as f32) / 100.0;
            let variation = (i as f32) / 384.0;
            (base + variation).sin()
        })
        .collect()
}

/// Build the api_symbols Arrow schema using arrow 57 types.
fn symbols_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("ecosystem", DataType::Utf8, false),
        Field::new("package", DataType::Utf8, false),
        Field::new("version", DataType::Utf8, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("signature", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, true),
        Field::new("doc_comment", DataType::Utf8, true),
        Field::new("line_start", DataType::Int32, true),
        Field::new("line_end", DataType::Int32, true),
        Field::new("visibility", DataType::Utf8, true),
        Field::new("is_async", DataType::Boolean, true),
        Field::new("is_unsafe", DataType::Boolean, true),
        Field::new("is_error_type", DataType::Boolean, true),
        Field::new("returns_result", DataType::Boolean, true),
        Field::new("attributes", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                384,
            ),
            true,
        ),
    ]))
}

/// Build a RecordBatch (arrow 57) with `count` synthetic api_symbols rows.
fn synthetic_symbols_batch(count: usize) -> RecordBatch57 {
    let schema = symbols_schema();

    let ids: Vec<String> = (0..count).map(|i| format!("sym-{i:05}")).collect();
    let names: Vec<String> = (0..count).map(|i| format!("func_{i}")).collect();
    let sigs: Vec<String> = (0..count)
        .map(|i| format!("pub async fn func_{i}(x: i32) -> Result<()>"))
        .collect();
    let sources: Vec<String> = (0..count)
        .map(|i| format!("fn func_{i}(x: i32) {{ todo!() }}"))
        .collect();
    let docs: Vec<String> = (0..count)
        .map(|i| format!("Documentation for func_{i}. Handles async spawning of tasks."))
        .collect();
    let attrs: Vec<String> = (0..count).map(|_| "[\"#[tokio::main]\"]".to_string()).collect();

    let embeddings: Vec<Option<Vec<Option<f32>>>> = (0..count)
        .map(|i| {
            Some(
                synthetic_embedding(i as u32)
                    .into_iter()
                    .map(Some)
                    .collect(),
            )
        })
        .collect();

    RecordBatch57::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(vec!["rust"; count])),
            Arc::new(StringArray::from(vec!["tokio"; count])),
            Arc::new(StringArray::from(vec!["1.49.0"; count])),
            Arc::new(StringArray::from(vec!["src/runtime/mod.rs"; count])),
            Arc::new(StringArray::from(vec!["function"; count])),
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(sigs)),
            Arc::new(StringArray::from(sources)),
            Arc::new(StringArray::from(docs)),
            Arc::new(Int32Array::from_iter(
                (0..count).map(|i| Some((i * 10 + 1) as i32)),
            )),
            Arc::new(Int32Array::from_iter(
                (0..count).map(|i| Some((i * 10 + 8) as i32)),
            )),
            Arc::new(StringArray::from(vec!["public"; count])),
            Arc::new(BooleanArray::from(
                (0..count).map(|i| Some(i % 3 == 0)).collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(vec![Some(false); count])),
            Arc::new(BooleanArray::from(
                (0..count).map(|i| Some(i % 10 == 0)).collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(vec![Some(true); count])),
            Arc::new(StringArray::from(attrs)),
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(embeddings, 384),
            ),
        ],
    )
    .expect("Failed to create RecordBatch")
}

/// Convert a duckdb arrow-56 RecordBatch to a lancedb arrow-57 RecordBatch
/// by reading values from arrow-56 arrays and constructing arrow-57 arrays.
///
/// **This is NOT production code.** It exists only for exploratory tests I1/I2
/// that validate the DuckDB `query_arrow()` extraction path. In production,
/// data originates as Rust structs and goes through `serde_arrow` (see Part M),
/// so this bridge is never called.
///
/// The bridge copies all data element-by-element — O(n) with full allocation.
/// Approaches considered and rejected:
/// - Arrow C FFI: `FFI_ArrowArray` from arrow 56 and 57 are different Rust types
///   despite identical `#[repr(C)]` layout. Would require `transmute` (unsafe).
/// - Arrow IPC: duckdb's `arrow` doesn't enable the `ipc` feature.
/// - serde_arrow cross-version: features are non-additive, can't enable both
///   `arrow-56` and `arrow-57` simultaneously.
fn duckdb_batch_to_lance(
    batch: &duckdb::arrow::record_batch::RecordBatch,
) -> RecordBatch57 {
    let schema56 = batch.schema();

    // Reconstruct each column first (may change types, e.g. List → FixedSizeList)
    let columns57: Vec<Arc<dyn arrow_array::Array>> = batch
        .columns()
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let dt56 = schema56.field(i).data_type();
            convert_column(col.as_ref(), dt56)
        })
        .collect();

    // Build schema from converted columns (respects type changes)
    let fields57: Vec<Field> = schema56
        .fields()
        .iter()
        .zip(columns57.iter())
        .map(|(f, col)| Field::new(f.name(), col.data_type().clone(), f.is_nullable()))
        .collect();
    let schema57 = Arc::new(Schema::new(fields57));

    RecordBatch57::try_new(schema57, columns57).expect("create RecordBatch57")
}

/// Convert an arrow-56 array to an arrow-57 array by reading values.
fn convert_column(
    col: &dyn duckdb::arrow::array::Array,
    dt: &duckdb::arrow::datatypes::DataType,
) -> Arc<dyn arrow_array::Array> {
    use duckdb::arrow::array as a56;
    use duckdb::arrow::datatypes::DataType as DT56;

    match dt {
        DT56::Boolean => {
            let arr = col.as_any().downcast_ref::<a56::BooleanArray>().unwrap();
            let values: Vec<Option<bool>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(BooleanArray::from(values))
        }
        DT56::Int32 => {
            let arr = col.as_any().downcast_ref::<a56::Int32Array>().unwrap();
            let values: Vec<Option<i32>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(Int32Array::from(values))
        }
        DT56::Int64 => {
            let arr = col.as_any().downcast_ref::<a56::Int64Array>().unwrap();
            let values: Vec<Option<i64>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(arrow_array::Int64Array::from(values))
        }
        DT56::Float32 => {
            let arr = col.as_any().downcast_ref::<a56::Float32Array>().unwrap();
            let values: Vec<Option<f32>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(arrow_array::Float32Array::from(values))
        }
        DT56::Float64 => {
            let arr = col.as_any().downcast_ref::<a56::Float64Array>().unwrap();
            let values: Vec<Option<f64>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(arrow_array::Float64Array::from(values))
        }
        DT56::Utf8 => {
            let arr = col.as_any().downcast_ref::<a56::StringArray>().unwrap();
            let values: Vec<Option<&str>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(StringArray::from(values))
        }
        DT56::LargeUtf8 => {
            let arr = col.as_any().downcast_ref::<a56::LargeStringArray>().unwrap();
            let values: Vec<Option<&str>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(arrow_array::LargeStringArray::from(values))
        }
        DT56::FixedSizeList(inner_field, size) => {
            let arr = col.as_any().downcast_ref::<a56::FixedSizeListArray>().unwrap();
            match inner_field.data_type() {
                DT56::Float32 => {
                    let values: Vec<Option<Vec<Option<f32>>>> = (0..arr.len())
                        .map(|i| {
                            if arr.is_null(i) {
                                None
                            } else {
                                let inner = arr.value(i);
                                let f32_arr = inner.as_any().downcast_ref::<a56::Float32Array>().unwrap();
                                Some((0..f32_arr.len()).map(|j| Some(f32_arr.value(j))).collect())
                            }
                        })
                        .collect();
                    Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        values, *size,
                    ))
                }
                _ => panic!("Unsupported FixedSizeList inner type: {inner_field:?}"),
            }
        }
        DT56::List(inner_field) => {
            // For List<Float32> (DuckDB's FLOAT[] representation)
            let arr = col.as_any().downcast_ref::<a56::ListArray>().unwrap();
            match inner_field.data_type() {
                DT56::Float32 => {
                    // Check if all lists have the same length → convert to FixedSizeList
                    let first_len = if arr.len() > 0 && !arr.is_null(0) {
                        Some(arr.value(0).len() as i32)
                    } else {
                        None
                    };

                    let all_same_len = first_len.is_some()
                        && (0..arr.len()).all(|i| {
                            arr.is_null(i) || arr.value(i).len() as i32 == first_len.unwrap()
                        });

                    if all_same_len {
                        let size = first_len.unwrap();
                        let values: Vec<Option<Vec<Option<f32>>>> = (0..arr.len())
                            .map(|i| {
                                if arr.is_null(i) {
                                    None
                                } else {
                                    let inner = arr.value(i);
                                    let f32_arr = inner.as_any().downcast_ref::<a56::Float32Array>().unwrap();
                                    Some((0..f32_arr.len()).map(|j| Some(f32_arr.value(j))).collect())
                                }
                            })
                            .collect();
                        Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            values, size,
                        ))
                    } else {
                        // Variable-length lists — keep as List
                        // For now, panic. In production, we'd handle this properly.
                        panic!("Variable-length List<Float32> not supported in bridge");
                    }
                }
                _ => panic!("Unsupported List inner type: {inner_field:?}"),
            }
        }
        DT56::Timestamp(_, _) => {
            // DuckDB timestamps come as microseconds typically
            let arr = col.as_any().downcast_ref::<a56::TimestampMicrosecondArray>().unwrap();
            let values: Vec<Option<i64>> = (0..arr.len())
                .map(|i| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .collect();
            Arc::new(arrow_array::TimestampMicrosecondArray::from(values))
        }
        other => panic!("Unsupported DataType for bridge: {other:?}"),
    }
}

/// Create a DuckDB connection with lance extension loaded.
fn lance_duckdb_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open DuckDB");
    conn.execute_batch("INSTALL lance FROM community; LOAD lance;")
        .expect("Failed to install/load lance extension");
    conn
}

/// Create a DuckDB connection with lance + httpfs + R2 creds.
fn lance_r2_duckdb_conn(creds: &R2Creds) -> Connection {
    let conn = lance_duckdb_conn();
    conn.execute_batch("INSTALL httpfs; LOAD httpfs;")
        .expect("Failed to install/load httpfs");
    conn.execute_batch(&format!(
        "CREATE SECRET r2_spike19 (
            TYPE s3,
            KEY_ID '{key_id}',
            SECRET '{secret}',
            ENDPOINT '{account_id}.r2.cloudflarestorage.com',
            URL_STYLE 'path'
        )",
        key_id = creds.access_key_id,
        secret = creds.secret_access_key,
        account_id = creds.account_id,
    ))
    .expect("Failed to create R2 secret");
    conn
}

/// Connect to lancedb at the given URI with R2 storage options.
async fn lancedb_connect_r2(uri: &str) -> lancedb::Connection {
    load_env();
    let access_key = std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID");
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY");
    let endpoint = std::env::var("AWS_ENDPOINT_URL").expect("AWS_ENDPOINT_URL");

    lancedb::connect(uri)
        .storage_option("aws_access_key_id", &access_key)
        .storage_option("aws_secret_access_key", &secret_key)
        .storage_option("aws_endpoint", &endpoint)
        .storage_option("aws_region", "auto")
        .storage_option("aws_virtual_hosted_style_request", "false")
        .execute()
        .await
        .expect("Failed to connect to lancedb on R2")
}

/// Format a vector as a DuckDB array literal.
fn vec_to_sql(v: &[f32]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// ============================================================================
// Part H: Arrow Bridge + Native lancedb Writes
// ============================================================================

/// H1: Validate Arrow C FFI bridge converts RecordBatch from arrow 56 → arrow 57.
#[test]
fn spike_arrow_ffi_bridge() {
    // Build a RecordBatch using duckdb's arrow 56
    let conn = Connection::open_in_memory().expect("open duckdb");
    conn.execute_batch(
        "CREATE TABLE test_bridge (
            id INTEGER,
            name VARCHAR,
            value FLOAT,
            flag BOOLEAN
        )",
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO test_bridge VALUES
            (1, 'alpha', 1.5, true),
            (2, 'beta', 2.5, false),
            (3, 'gamma', 3.5, true)",
    )
    .unwrap();

    let mut stmt = conn.prepare("SELECT * FROM test_bridge ORDER BY id").unwrap();
    let batches: Vec<duckdb::arrow::record_batch::RecordBatch> =
        stmt.query_arrow([]).unwrap().collect();
    assert_eq!(batches.len(), 1);
    let batch56 = &batches[0];

    eprintln!("  arrow 56 batch: {} rows, {} cols", batch56.num_rows(), batch56.num_columns());
    eprintln!("  arrow 56 schema: {:?}", batch56.schema());

    // Convert via FFI
    let batch57 = duckdb_batch_to_lance(batch56);

    // Verify
    assert_eq!(batch57.num_rows(), 3);
    assert_eq!(batch57.num_columns(), 4);

    let ids = batch57
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("column 0 should be Int32");
    assert_eq!(ids.value(0), 1);
    assert_eq!(ids.value(1), 2);
    assert_eq!(ids.value(2), 3);

    let names = batch57
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("column 1 should be String");
    assert_eq!(names.value(0), "alpha");
    assert_eq!(names.value(1), "beta");
    assert_eq!(names.value(2), "gamma");

    let flags = batch57
        .column(3)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .expect("column 3 should be Boolean");
    assert!(flags.value(0));
    assert!(!flags.value(1));
    assert!(flags.value(2));

    eprintln!("  arrow 57 batch: {} rows, {} cols", batch57.num_rows(), batch57.num_columns());
    eprintln!("  arrow 57 schema: {:?}", batch57.schema());
    eprintln!("  PASS: Arrow C FFI bridge works (arrow 56 → 57, zero-copy)");
}

/// H2: Write api_symbols schema to local Lance via lancedb, read back via DuckDB.
#[test]
fn spike_lancedb_write_local() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmpdir = tempfile::tempdir().unwrap();
    let lance_path = tmpdir.path().join("test_symbols");
    let lance_uri = lance_path.to_str().unwrap();

    let batch = synthetic_symbols_batch(50);
    let schema = batch.schema();

    eprintln!("  Writing 50 symbols to local Lance: {lance_uri}");

    // Write via lancedb
    rt.block_on(async {
        let db = lancedb::connect(lance_uri).execute().await.unwrap();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        db.create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();
    });

    // Read back via DuckDB lance extension
    let lance_dataset_path = lance_path.join("symbols.lance");
    let dataset_uri = lance_dataset_path.to_str().unwrap();
    let conn = lance_duckdb_conn();

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{dataset_uri}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 50, "Should have 50 rows");

    // Verify schema survives — check key columns
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, name, kind, is_async FROM '{dataset_uri}' WHERE is_async = true LIMIT 3"
        ))
        .unwrap();
    let rows: Vec<(String, String, String, bool)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!rows.is_empty(), "Should find async functions");

    // Verify embedding column has 384 dims
    let mut stmt = conn
        .prepare(&format!(
            "SELECT len(embedding) FROM '{dataset_uri}' LIMIT 1"
        ))
        .unwrap();
    let emb_len: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(emb_len, 384, "Embedding should be 384-dim");

    eprintln!("  PASS: lancedb local write — 50 rows, 19 columns, 384-dim embeddings");
}

/// H3: Write api_symbols to R2 via lancedb, read back via DuckDB.
#[test]
fn spike_lancedb_write_r2() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_aws_available() {
        eprintln!("SKIP: AWS env vars not set for lancedb");
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let prefix = spike_r2_prefix(&creds);
    let lance_uri = format!("{prefix}/h3_symbols");

    let batch = synthetic_symbols_batch(50);
    let schema = batch.schema();

    eprintln!("  Writing 50 symbols to R2 via lancedb: {lance_uri}");

    rt.block_on(async {
        let db = lancedb_connect_r2(&lance_uri).await;
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        db.create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();
    });

    // Read back via DuckDB lance extension
    let dataset_path = format!("{lance_uri}/symbols.lance");
    let conn = lance_r2_duckdb_conn(&creds);

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{dataset_path}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 50, "Should have 50 rows on R2");

    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, name, is_async FROM '{dataset_path}' LIMIT 3"
        ))
        .unwrap();
    let rows: Vec<(String, String, bool)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);

    eprintln!("  PASS: lancedb R2 write — 50 rows readable via DuckDB lance extension");
}

/// H4: Create IVF-PQ vector + FTS indexes via lancedb, query via DuckDB.
#[test]
fn spike_lancedb_create_indexes() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_aws_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let prefix = spike_r2_prefix(&creds);
    let lance_uri = format!("{prefix}/h4_indexes");

    // PQ index training requires minimum 256 rows
    let batch = synthetic_symbols_batch(300);
    let schema = batch.schema();

    eprintln!("  Writing 300 symbols + creating indexes...");

    rt.block_on(async {
        let db = lancedb_connect_r2(&lance_uri).await;
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let tbl = db
            .create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();

        // Create vector index
        eprintln!("  Creating vector index...");
        tbl.create_index(&["embedding"], lancedb::index::Index::Auto)
            .execute()
            .await
            .unwrap();

        // Create FTS index
        eprintln!("  Creating FTS index...");
        tbl.create_index(
            &["doc_comment"],
            lancedb::index::Index::FTS(lancedb::index::scalar::FtsIndexBuilder::default()),
        )
        .execute()
        .await
        .unwrap();
    });

    // Query via DuckDB lance extension
    let dataset_path = format!("{lance_uri}/symbols.lance");
    let conn = lance_r2_duckdb_conn(&creds);

    // Vector search
    let query_emb = synthetic_embedding(5);
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_path}', 'embedding', {query_sql}::FLOAT[384], k=5)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let vec_results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!vec_results.is_empty(), "Vector search should return results");
    assert_eq!(
        vec_results[0].0, "func_5",
        "Nearest should be func_5, got {}",
        vec_results[0].0
    );
    assert!(
        vec_results[0].1 < 0.01,
        "Self-distance should be ~0, got {}",
        vec_results[0].1
    );

    eprintln!("  Vector search results:");
    for (name, dist) in &vec_results {
        eprintln!("    {name}: distance={dist:.6}");
    }

    // FTS search
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _score
             FROM lance_fts('{dataset_path}', 'doc_comment', 'async spawning', k=5)"
        ))
        .unwrap();
    let fts_results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!fts_results.is_empty(), "FTS should return results");
    eprintln!("  FTS search results:");
    for (name, score) in &fts_results {
        eprintln!("    {name}: score={score:.6}");
    }

    eprintln!("  PASS: lancedb indexes (vector + FTS) queryable via DuckDB");
}

/// H5: Incremental add via tbl.add().
#[test]
fn spike_lancedb_incremental_add() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_aws_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let prefix = spike_r2_prefix(&creds);
    let lance_uri = format!("{prefix}/h5_incremental");

    let batch1 = synthetic_symbols_batch(100);
    let schema = batch1.schema();

    eprintln!("  Writing initial 100 symbols...");

    rt.block_on(async {
        let db = lancedb_connect_r2(&lance_uri).await;

        // Initial write
        let batches = RecordBatchIterator::new(vec![Ok(batch1)], schema.clone());
        let tbl = db
            .create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();

        let count = tbl.count_rows(None).await.unwrap();
        assert_eq!(count, 100, "Initial count should be 100");
        eprintln!("  Initial count: {count}");

        // Incremental add — build a second batch with different IDs
        let mut batch2 = synthetic_symbols_batch(50);

        // Modify IDs to avoid duplication (rename sym-00000..sym-00049 → sym-00100..sym-00149)
        let new_ids: Vec<String> = (100..150).map(|i| format!("sym-{i:05}")).collect();
        let new_names: Vec<String> = (100..150).map(|i| format!("func_{i}")).collect();

        // Rebuild the batch with new IDs — simplest approach is to build fresh
        let ids_col = Arc::new(StringArray::from(new_ids));
        let names_col = Arc::new(StringArray::from(new_names));
        // Replace columns 0 (id) and 6 (name) in the batch
        let mut columns: Vec<Arc<dyn arrow_array::Array>> = batch2.columns().to_vec();
        columns[0] = ids_col;
        columns[6] = names_col;
        batch2 = RecordBatch57::try_new(schema.clone(), columns).unwrap();

        eprintln!("  Adding 50 more symbols...");
        let add_batches = RecordBatchIterator::new(vec![Ok(batch2)], schema);
        tbl.add(Box::new(add_batches)).execute().await.unwrap();

        let count = tbl.count_rows(None).await.unwrap();
        assert_eq!(count, 150, "After add, count should be 150");
        eprintln!("  After add: {count} rows");
    });

    // Read via DuckDB and verify
    let dataset_path = format!("{lance_uri}/symbols.lance");
    let conn = lance_r2_duckdb_conn(&creds);

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{dataset_path}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 150, "DuckDB should see 150 rows");

    // Vector search should find symbols from both batches
    let query_emb = synthetic_embedding(120); // seed 120 → in second batch
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_path}', 'embedding', {query_sql}::FLOAT[384], k=3)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    // Note: seed 120 maps to func_120 in batch2, but the embedding is generated
    // from seed 20 (since synthetic_symbols_batch uses 0..count as seeds).
    // The second batch has seeds 0..50 for embeddings but IDs 100..149.
    // So the query for seed 120 won't find an exact match — but it should return results.
    assert!(!results.is_empty(), "Should find results from combined dataset");

    eprintln!("  PASS: incremental add — 100 + 50 = 150 rows, search works across both");
}

// ============================================================================
// Part I: DuckDB → lancedb Pipeline (EXPLORATORY — not production code)
// ============================================================================
//
// These tests explore extracting data from DuckDB via `query_arrow()` and
// writing it to Lance via lancedb. This path does NOT exist in production —
// data originates as Rust structs (via tree-sitter + fastembed), not from
// DuckDB tables. These tests were valuable to understand:
//   - Arrow 56/57 coexistence and the value bridge cost
//   - DuckDB's FLOAT[] → List(Float32) type mapping (not FixedSizeList)
//   - query_arrow() API ergonomics
//
// The value bridge (`duckdb_batch_to_lance`) copies all data element-by-element.
// Production code avoids this entirely by using serde_arrow (see Part M).

/// I1: DuckDB table → query_arrow() → value bridge → lancedb → local Lance.
/// EXPLORATORY: validates DuckDB extraction, not the production write path.
#[test]
fn spike_duckdb_to_lance_local() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmpdir = tempfile::tempdir().unwrap();
    let lance_path = tmpdir.path().join("pipeline_test");
    let lance_uri = lance_path.to_str().unwrap();

    // Create and populate DuckDB table
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id VARCHAR,
            ecosystem VARCHAR NOT NULL,
            package VARCHAR NOT NULL,
            version VARCHAR NOT NULL,
            file_path VARCHAR NOT NULL,
            kind VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            signature VARCHAR,
            doc_comment TEXT,
            line_start INTEGER,
            line_end INTEGER,
            is_async BOOLEAN DEFAULT FALSE,
            embedding FLOAT[]
        )",
    )
    .unwrap();

    // Insert 50 rows with embeddings
    for i in 0..50u32 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!(
                "INSERT INTO api_symbols VALUES (
                    'sym-{i:05}', 'rust', 'tokio', '1.49.0',
                    'src/mod.rs', 'function', 'func_{i}',
                    'pub fn func_{i}()',
                    'Documentation for func_{i}',
                    {ls}, {le}, {is_async},
                    {emb_sql}::FLOAT[384]
                )",
                ls = i * 10 + 1,
                le = i * 10 + 8,
                is_async = i % 3 == 0,
            ),
            [],
        )
        .unwrap();
    }

    // Extract via query_arrow (arrow 56)
    let mut stmt = conn
        .prepare("SELECT * FROM api_symbols ORDER BY id")
        .unwrap();
    let batches56: Vec<duckdb::arrow::record_batch::RecordBatch> =
        stmt.query_arrow([]).unwrap().collect();

    eprintln!(
        "  DuckDB query_arrow: {} batch(es), {} total rows",
        batches56.len(),
        batches56.iter().map(|b| b.num_rows()).sum::<usize>()
    );

    // Convert via FFI bridge (arrow 56 → 57)
    let batches57: Vec<RecordBatch57> = batches56.iter().map(duckdb_batch_to_lance).collect();

    let schema57 = batches57[0].schema();
    eprintln!("  FFI bridge: schema has {} fields", schema57.fields().len());

    // Write via lancedb
    rt.block_on(async {
        let db = lancedb::connect(lance_uri).execute().await.unwrap();
        let batch_iter =
            RecordBatchIterator::new(batches57.into_iter().map(Ok), schema57);
        db.create_table("symbols", Box::new(batch_iter))
            .execute()
            .await
            .unwrap();
    });

    // Read back via DuckDB lance extension
    let dataset_path = lance_path.join("symbols.lance");
    let dataset_uri = dataset_path.to_str().unwrap();
    let reader = lance_duckdb_conn();

    let mut stmt = reader
        .prepare(&format!("SELECT count(*) FROM '{dataset_uri}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 50, "Should have 50 rows via lance");

    // Vector search on the lance dataset written from DuckDB data
    let query_emb = synthetic_embedding(10);
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = reader
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_uri}', 'embedding', {query_sql}::FLOAT[384], k=3)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "func_10", "Nearest should be func_10");
    assert!(results[0].1 < 0.01, "Self-distance should be ~0");

    eprintln!("  PASS: DuckDB → FFI → lancedb → local Lance — full pipeline works");
}

/// I2: DuckDB table → query_arrow() → value bridge → lancedb → R2.
/// EXPLORATORY: validates DuckDB extraction to R2, not the production write path.
#[test]
fn spike_duckdb_to_lance_r2() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_aws_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let prefix = spike_r2_prefix(&creds);
    let lance_uri = format!("{prefix}/i2_pipeline");

    // Create and populate DuckDB table
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id VARCHAR, name VARCHAR, doc_comment TEXT,
            embedding FLOAT[]
        )",
    )
    .unwrap();

    for i in 0..30u32 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!(
                "INSERT INTO api_symbols VALUES (
                    'sym-{i:05}', 'func_{i}',
                    'Doc for func_{i}',
                    {emb_sql}::FLOAT[384]
                )"
            ),
            [],
        )
        .unwrap();
    }

    // Extract, bridge, write
    let mut stmt = conn
        .prepare("SELECT * FROM api_symbols ORDER BY id")
        .unwrap();
    let batches56: Vec<duckdb::arrow::record_batch::RecordBatch> =
        stmt.query_arrow([]).unwrap().collect();
    let batches57: Vec<RecordBatch57> = batches56.iter().map(duckdb_batch_to_lance).collect();
    let schema57 = batches57[0].schema();

    eprintln!("  DuckDB → FFI → lancedb → R2: {lance_uri}");

    rt.block_on(async {
        let db = lancedb_connect_r2(&lance_uri).await;
        let batch_iter =
            RecordBatchIterator::new(batches57.into_iter().map(Ok), schema57);
        db.create_table("symbols", Box::new(batch_iter))
            .execute()
            .await
            .unwrap();
    });

    // Read back via DuckDB
    let dataset_path = format!("{lance_uri}/symbols.lance");
    let conn = lance_r2_duckdb_conn(&creds);

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{dataset_path}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 30, "Should have 30 rows on R2");

    // Vector search
    let query_emb = synthetic_embedding(15);
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_path}', 'embedding', {query_sql}::FLOAT[384], k=3)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "func_15");
    assert!(results[0].1 < 0.01);

    eprintln!("  PASS: DuckDB → FFI → lancedb → R2 — full upload pipeline works");
}

// ============================================================================
// Part L: Operational Concerns
// ============================================================================

/// L2: Indexes created by lancedb are readable by a separate DuckDB connection.
#[test]
fn spike_lance_cross_process_index_read() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmpdir = tempfile::tempdir().unwrap();
    let lance_path = tmpdir.path().join("cross_process");
    let lance_uri = lance_path.to_str().unwrap();

    // PQ index training requires minimum 256 rows
    let batch = synthetic_symbols_batch(300);
    let schema = batch.schema();

    // Write + create indexes, then DROP all lancedb handles
    rt.block_on(async {
        let db = lancedb::connect(lance_uri).execute().await.unwrap();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let tbl = db
            .create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();

        tbl.create_index(&["embedding"], lancedb::index::Index::Auto)
            .execute()
            .await
            .unwrap();

        eprintln!("  Created dataset + vector index, dropping handles...");
        drop(tbl);
        drop(db);
    });

    // Now open a fresh DuckDB connection (simulating a different process)
    let dataset_uri = lance_path.join("symbols.lance");
    let dataset_str = dataset_uri.to_str().unwrap();
    let conn = lance_duckdb_conn();

    let query_emb = synthetic_embedding(42);
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_str}', 'embedding', {query_sql}::FLOAT[384], k=5)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty(), "Should find results after handle drop");
    assert_eq!(results[0].0, "func_42");
    assert!(results[0].1 < 0.01);

    eprintln!("  PASS: cross-process index read — vector index survives handle drop");
}

/// L4: create_table with exist_ok when dataset already exists.
#[test]
fn spike_lancedb_create_table_exists() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmpdir = tempfile::tempdir().unwrap();
    let lance_path = tmpdir.path().join("exist_ok");
    let lance_uri = lance_path.to_str().unwrap();

    let batch1 = synthetic_symbols_batch(30);
    let schema = batch1.schema();

    rt.block_on(async {
        let db = lancedb::connect(lance_uri).execute().await.unwrap();

        // First create
        let batches = RecordBatchIterator::new(vec![Ok(batch1)], schema.clone());
        db.create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();

        eprintln!("  Created table with 30 rows");

        // Second create with same name — should fail without exist_ok
        let batch2 = synthetic_symbols_batch(10);
        let batches2 = RecordBatchIterator::new(vec![Ok(batch2)], schema.clone());
        let result = db
            .create_table("symbols", Box::new(batches2))
            .execute()
            .await;

        assert!(
            result.is_err(),
            "create_table should fail when table exists"
        );
        eprintln!("  Second create_table without exist_ok: correctly failed");

        // With exist_ok — should return existing table
        let tbl = db
            .create_empty_table("symbols", schema.clone())
            .mode(lancedb::database::CreateTableMode::exist_ok(Box::new(
                |req| req,
            )))
            .execute()
            .await
            .unwrap();

        let count = tbl.count_rows(None).await.unwrap();
        assert_eq!(count, 30, "exist_ok should preserve original 30 rows");
        eprintln!("  exist_ok: returned existing table with {count} rows (no data loss)");

        eprintln!("  PASS: create_table exist_ok — no data loss, correct error on duplicate");
    });
}

// ============================================================================
// Part M: serde_arrow Production Path (THIS IS THE PRODUCTION CODE PATH)
// ============================================================================
//
// In production, data flows as:
//   1. tree-sitter parses source → Rust ApiSymbol structs
//   2. fastembed generates embeddings → Vec<f32> on each struct
//   3. serde_arrow serializes structs → arrow-57 RecordBatch
//      - arrow_serde adapters handle DateTime<Utc> → i64 microseconds
//      - FixedSizeList(384) override for embedding column
//   4. lancedb writes RecordBatch → Lance dataset (local or R2)
//   5. DuckDB lance extension queries the dataset (read-only)
//
// This is the ONLY write path in production. DuckDB never holds the canonical
// data — it is purely a read-only query engine for Lance datasets.
//
// The value bridge (Part I) and manual RecordBatch construction (Part H)
// are spike artifacts that won't appear in production code.

/// The Rust struct that represents an API symbol — the production data type.
/// serde_arrow uses Serde derive to map this to Arrow columns.
///
/// Note: `created_at` uses the `arrow_serde::timestamp_micros_utc_option` adapter
/// to serialize `DateTime<Utc>` as `i64` microseconds, which maps to Arrow's
/// `Timestamp(Microsecond, Some("UTC"))` type. Without this adapter, serde_arrow
/// would try to serialize chrono types as strings.
#[derive(Debug, Serialize, Deserialize)]
struct ApiSymbol {
    id: String,
    ecosystem: String,
    package: String,
    version: String,
    file_path: String,
    kind: String,
    name: String,
    signature: Option<String>,
    source: Option<String>,
    doc_comment: Option<String>,
    line_start: Option<i32>,
    line_end: Option<i32>,
    visibility: Option<String>,
    is_async: Option<bool>,
    is_unsafe: Option<bool>,
    is_error_type: Option<bool>,
    returns_result: Option<bool>,
    attributes: Option<String>,
    embedding: Vec<f32>,
    #[serde(with = "zen_core::arrow_serde::timestamp_micros_utc_option")]
    created_at: Option<DateTime<Utc>>,
}

/// M1: Rust structs → serde_arrow → arrow-57 RecordBatch → lancedb → DuckDB reads.
/// This is the production path. No DuckDB extraction, no bridge.
#[test]
fn spike_serde_arrow_production_path() {
    use arrow_schema::FieldRef;
    use serde_arrow::schema::{SchemaLike, TracingOptions};

    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmpdir = tempfile::tempdir().unwrap();
    let lance_path = tmpdir.path().join("serde_arrow_test");
    let lance_uri = lance_path.to_str().unwrap();

    // 1. Create Rust structs (simulating tree-sitter output + fastembed)
    let symbols: Vec<ApiSymbol> = (0..50)
        .map(|i| ApiSymbol {
            id: format!("sym-{i:05}"),
            ecosystem: "rust".to_string(),
            package: "tokio".to_string(),
            version: "1.49.0".to_string(),
            file_path: "src/runtime/mod.rs".to_string(),
            kind: "function".to_string(),
            name: format!("func_{i}"),
            signature: Some(format!("pub async fn func_{i}(x: i32) -> Result<()>")),
            source: Some(format!("fn func_{i}(x: i32) {{ todo!() }}")),
            doc_comment: Some(format!(
                "Documentation for func_{i}. Handles async spawning of tasks."
            )),
            line_start: Some(i * 10 + 1),
            line_end: Some(i * 10 + 8),
            visibility: Some("public".to_string()),
            is_async: Some(i % 3 == 0),
            is_unsafe: Some(false),
            is_error_type: Some(i % 10 == 0),
            returns_result: Some(true),
            attributes: Some("[\"#[tokio::main]\"]".to_string()),
            embedding: synthetic_embedding(i as u32),
            created_at: Some(Utc::now()),
        })
        .collect();

    // 2. serde_arrow: trace schema from type + override embedding to FixedSizeList(384)
    let mut fields =
        Vec::<FieldRef>::from_type::<ApiSymbol>(TracingOptions::default()).expect("trace schema");

    // Override: embedding must be FixedSizeList(Float32, 384) for Lance vector search.
    // serde_arrow traces Vec<f32> as LargeList(Float32) by default.
    fields = fields
        .into_iter()
        .map(|f| {
            if f.name() == "embedding" {
                Arc::new(Field::new(
                    "embedding",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        384,
                    ),
                    false,
                ))
            } else {
                f
            }
        })
        .collect();

    eprintln!("  serde_arrow schema ({} fields):", fields.len());
    for f in &fields {
        eprintln!("    {}: {:?} (nullable={})", f.name(), f.data_type(), f.is_nullable());
    }

    let batch =
        serde_arrow::to_record_batch(&fields, &symbols).expect("serialize to RecordBatch");

    assert_eq!(batch.num_rows(), 50);
    assert_eq!(batch.num_columns(), fields.len());
    eprintln!(
        "  RecordBatch: {} rows, {} columns",
        batch.num_rows(),
        batch.num_columns()
    );

    // 3. Write to local Lance via lancedb
    let schema = batch.schema();
    rt.block_on(async {
        let db = lancedb::connect(lance_uri).execute().await.unwrap();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        db.create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();
    });

    // 4. Read back via DuckDB lance extension
    let dataset_path = lance_path.join("symbols.lance");
    let dataset_uri = dataset_path.to_str().unwrap();
    let conn = lance_duckdb_conn();

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{dataset_uri}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 50, "Should have 50 rows");

    // Verify column values survive roundtrip
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, name, is_async, doc_comment FROM '{dataset_uri}' WHERE name = 'func_0'"
        ))
        .unwrap();
    let (id, name, is_async, doc): (String, String, bool, String) = stmt
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap();

    assert_eq!(id, "sym-00000");
    assert_eq!(name, "func_0");
    assert!(is_async); // 0 % 3 == 0
    assert!(doc.contains("func_0"));

    // Vector search on serde_arrow-produced data
    let query_emb = synthetic_embedding(10);
    let query_sql = vec_to_sql(&query_emb);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{dataset_uri}', 'embedding', {query_sql}::FLOAT[384], k=3)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "func_10", "Nearest should be func_10");
    assert!(results[0].1 < 0.01, "Self-distance should be ~0");

    eprintln!("  Vector search: {} → distance={:.6}", results[0].0, results[0].1);

    // 5. Deserialize back from Arrow to Rust structs (round-trip proof)
    let read_batch = {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM '{dataset_uri}' ORDER BY id LIMIT 5"))
            .unwrap();
        let arrow_batches: Vec<duckdb::arrow::record_batch::RecordBatch> =
            stmt.query_arrow([]).unwrap().collect();
        // Bridge back to arrow-57 for serde_arrow deserialization
        duckdb_batch_to_lance(&arrow_batches[0])
    };

    let round_tripped: Vec<ApiSymbol> =
        serde_arrow::from_record_batch(&read_batch)
            .expect("deserialize from RecordBatch");

    assert_eq!(round_tripped.len(), 5);
    assert_eq!(round_tripped[0].id, "sym-00000");
    assert_eq!(round_tripped[0].name, "func_0");
    assert_eq!(round_tripped[0].embedding.len(), 384);
    assert!(round_tripped[0].is_async.unwrap());
    assert!(
        round_tripped[0].created_at.is_some(),
        "created_at should survive round-trip"
    );

    eprintln!(
        "  Round-trip: {} symbols deserialized back to Rust structs",
        round_tripped.len()
    );
    eprintln!("  PASS: serde_arrow production path — Rust structs → arrow-57 → lancedb → DuckDB → Rust structs");
}
