use std::sync::Arc;

use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, FieldRef};
use chrono::Utc;
use serde::Serialize;
use serde_arrow::schema::{SchemaLike, TracingOptions};
use zen_config::R2Config;
use zen_core::enums::Visibility;

use crate::{ApiSymbolRow, DocChunkRow, LakeError, ZenLake};

#[derive(Debug, Clone, Serialize)]
pub struct R2WriteResult {
    pub symbols_lance_path: Option<String>,
    pub doc_chunks_lance_path: Option<String>,
    pub symbol_count: usize,
    pub doc_chunk_count: usize,
}

fn sanitize_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_underscore = false;

    for ch in input.chars() {
        let keep = ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_';
        if keep {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }

    let sanitized = out.trim_matches('_');
    if sanitized.is_empty() {
        return "_".to_string();
    }

    let mut capped = sanitized.to_string();
    if capped.len() > 128 {
        capped.truncate(128);
    }
    capped
}

fn symbols_dataset_root(
    r2: &R2Config,
    ecosystem: &str,
    package: &str,
    version: &str,
    visibility: Visibility,
) -> String {
    let ts = Utc::now().timestamp_millis();
    format!(
        "s3://{}/lance/{}/{}/{}/{}/symbols/{}",
        r2.bucket_name,
        visibility.as_str(),
        sanitize_segment(ecosystem),
        sanitize_segment(package),
        sanitize_segment(version),
        ts
    )
}

fn with_embedding_fixed_size_384(mut fields: Vec<FieldRef>) -> Vec<FieldRef> {
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
    fields
}

fn parse_embedding_sql(value: Option<String>) -> Result<Vec<f32>, LakeError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Ok(Vec::new());
    }
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let parsed: Vec<f32> = inner
        .split(',')
        .map(|part| {
            let token = part.trim();
            token.parse::<f32>().map_err(|error| {
                LakeError::Other(format!("invalid embedding float token '{token}': {error}"))
            })
        })
        .collect::<Result<_, _>>()?;

    if parsed.len() != 384 {
        return Err(LakeError::Other(format!(
            "invalid embedding vector length {}; expected 384 for FixedSizeList(384)",
            parsed.len()
        )));
    }

    Ok(parsed)
}

impl ZenLake {
    fn query_symbols_for_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<Vec<ApiSymbolRow>, LakeError> {
        let mut stmt = self.conn().prepare(
            "SELECT
                id, ecosystem, package, version, file_path, kind, name,
                signature, source, doc_comment, line_start, line_end,
                visibility, is_async, is_unsafe, is_error_type, returns_result,
                return_type, generics, attributes, metadata, embedding::VARCHAR
             FROM api_symbols
             WHERE ecosystem = ? AND package = ? AND version = ?",
        )?;

        let mut rows = stmt.query(duckdb::params![ecosystem, package, version])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let embedding = parse_embedding_sql(row.get::<_, Option<String>>(21)?)?;
            out.push(ApiSymbolRow {
                id: row.get(0)?,
                ecosystem: row.get(1)?,
                package: row.get(2)?,
                version: row.get(3)?,
                file_path: row.get(4)?,
                kind: row.get(5)?,
                name: row.get(6)?,
                signature: row.get(7)?,
                source: row.get(8)?,
                doc_comment: row.get(9)?,
                line_start: row.get(10)?,
                line_end: row.get(11)?,
                visibility: row.get(12)?,
                is_async: row.get(13)?,
                is_unsafe: row.get(14)?,
                is_error_type: row.get(15)?,
                returns_result: row.get(16)?,
                return_type: row.get(17)?,
                generics: row.get(18)?,
                attributes: row.get(19)?,
                metadata: row.get(20)?,
                embedding,
            });
        }
        Ok(out)
    }

    fn query_doc_chunks_for_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<Vec<DocChunkRow>, LakeError> {
        let mut stmt = self.conn().prepare(
            "SELECT
                id, ecosystem, package, version, chunk_index,
                title, content, source_file, format, embedding::VARCHAR
             FROM doc_chunks
             WHERE ecosystem = ? AND package = ? AND version = ?",
        )?;

        let mut rows = stmt.query(duckdb::params![ecosystem, package, version])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let embedding = parse_embedding_sql(row.get::<_, Option<String>>(9)?)?;
            out.push(DocChunkRow {
                id: row.get(0)?,
                ecosystem: row.get(1)?,
                package: row.get(2)?,
                version: row.get(3)?,
                chunk_index: row.get(4)?,
                title: row.get(5)?,
                content: row.get(6)?,
                source_file: row.get(7)?,
                format: row.get(8)?,
                embedding,
            });
        }
        Ok(out)
    }

    async fn write_batch_to_r2(
        r2: &R2Config,
        dataset_root: &str,
        table_name: &str,
        batch: RecordBatch,
    ) -> Result<String, LakeError> {
        let endpoint = r2.endpoint_url();
        let db = lancedb::connect(dataset_root)
            .storage_option("aws_access_key_id", &r2.access_key_id)
            .storage_option("aws_secret_access_key", &r2.secret_access_key)
            .storage_option("aws_endpoint", &endpoint)
            .storage_option("aws_region", "auto")
            .storage_option("aws_virtual_hosted_style_request", "false")
            .execute()
            .await
            .map_err(|e| LakeError::Other(format!("lancedb connect failed: {e}")))?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let table = db
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .map_err(|e| LakeError::Other(format!("lancedb write failed: {e}")))?;

        let table_uri = table
            .uri()
            .await
            .map_err(|e| LakeError::Other(format!("lancedb table uri lookup failed: {e}")))?;
        let table_uri = table_uri
            .split_once('#')
            .map_or(table_uri.as_str(), |(prefix, _)| prefix)
            .trim_end_matches('?')
            .to_string();

        db.open_table(table_name).execute().await.map_err(|e| {
            LakeError::Other(format!(
                "lancedb post-write open_table verification failed for '{table_name}': {e}"
            ))
        })?;

        Ok(table_uri)
    }

    /// Export indexed package data to R2 as Lance datasets.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if local reads, Arrow conversion, or R2 writes fail.
    pub async fn write_to_r2(
        &self,
        r2: &R2Config,
        ecosystem: &str,
        package: &str,
        version: &str,
        visibility: Visibility,
    ) -> Result<R2WriteResult, LakeError> {
        if !r2.is_configured() {
            return Err(LakeError::Other(
                "R2 is not configured (set account_id/access_key_id/secret_access_key/bucket_name)"
                    .to_string(),
            ));
        }

        let symbols = self.query_symbols_for_package(ecosystem, package, version)?;
        let doc_chunks = self.query_doc_chunks_for_package(ecosystem, package, version)?;

        if symbols.is_empty() && doc_chunks.is_empty() {
            return Ok(R2WriteResult {
                symbols_lance_path: None,
                doc_chunks_lance_path: None,
                symbol_count: 0,
                doc_chunk_count: 0,
            });
        }

        let symbols_lance_path = if symbols.is_empty() {
            None
        } else {
            let fields = with_embedding_fixed_size_384(
                Vec::<FieldRef>::from_type::<ApiSymbolRow>(TracingOptions::default()).map_err(
                    |e| LakeError::Other(format!("serde_arrow schema trace failed: {e}")),
                )?,
            );
            let batch = serde_arrow::to_record_batch(&fields, &symbols).map_err(|e| {
                LakeError::Other(format!("serde_arrow symbol conversion failed: {e}"))
            })?;
            let root = symbols_dataset_root(r2, ecosystem, package, version, visibility);
            Some(Self::write_batch_to_r2(r2, &root, "symbols", batch).await?)
        };

        let doc_chunks_lance_path = None;

        Ok(R2WriteResult {
            symbols_lance_path,
            doc_chunks_lance_path,
            symbol_count: symbols.len(),
            doc_chunk_count: doc_chunks.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ZenLake;
    use zen_config::R2Config;

    #[tokio::test]
    async fn write_to_r2_empty_package_is_noop() {
        let lake = ZenLake::open_in_memory().unwrap();
        let r2 = R2Config {
            account_id: "acc".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            bucket_name: "zenith".to_string(),
            endpoint: "https://example.invalid".to_string(),
        };

        let result = lake
            .write_to_r2(&r2, "rust", "tokio", "1.40.0", Visibility::Public)
            .await
            .unwrap();

        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.doc_chunk_count, 0);
        assert!(result.symbols_lance_path.is_none());
        assert!(result.doc_chunks_lance_path.is_none());
    }

    #[test]
    fn r2_path_includes_visibility_prefix() {
        let r2 = R2Config {
            account_id: "acc".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            bucket_name: "zenith-bucket".to_string(),
            endpoint: String::new(),
        };

        let public_root = symbols_dataset_root(&r2, "rust", "tokio", "1.49.0", Visibility::Public);
        assert!(public_root.starts_with("s3://zenith-bucket/lance/public/"));

        let private_root =
            symbols_dataset_root(&r2, "rust", "tokio", "1.49.0", Visibility::Private);
        assert!(private_root.starts_with("s3://zenith-bucket/lance/private/"));

        let team_root = symbols_dataset_root(&r2, "rust", "tokio", "1.49.0", Visibility::Team);
        assert!(team_root.starts_with("s3://zenith-bucket/lance/team/"));
    }
}
