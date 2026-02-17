//! Indexing pipeline: walk → parse → embed → store.
//!
//! Orchestrates the end-to-end indexing of a local directory into the local DuckDB cache.
//! The pipeline:
//! 1. Walk files (using `zen-search::walk::build_walker`)
//! 2. Parse each file with `zen-parser::extract_api`
//! 3. Chunk documentation files with `zen-parser::chunk_document`
//! 4. Generate embeddings with `zen-embeddings::EmbeddingEngine` (batch)
//! 5. Store symbols, doc chunks, and source files in DuckDB via `ZenLake` and `SourceFileStore`
//! 6. Register the package in `indexed_packages` and mark `source_cached = TRUE`
//!
//! The pipeline is invoked by the CLI `zen index` command (to be implemented in Phase 5).

use std::path::Path;

use serde_json;
use zen_embeddings::EmbeddingEngine;
use zen_lake::{
    ApiSymbolRow, DocChunkRow, LakeError, ZenLake,
    source_files::{SourceFile, SourceFileStore},
};
use zen_parser::doc_chunker::chunk_document;
use zen_parser::types::{SymbolKind, SymbolMetadata, Visibility};
use zen_parser::{DetectedLanguage, ParsedItem, detect_language_ext, extract_api};

/// Indexing pipeline for a single package.
pub struct IndexingPipeline {
    lake: ZenLake,
    source_store: SourceFileStore,
}

/// Result of an indexing run.
pub struct IndexResult {
    pub ecosystem: String,
    pub package: String,
    pub version: String,
    pub file_count: i32,
    pub symbol_count: i32,
    pub doc_chunk_count: i32,
    pub source_file_count: i32,
}

impl IndexingPipeline {
    /// Create a new pipeline with a lake and source file store.
    pub fn new(lake: ZenLake, source_store: SourceFileStore) -> Self {
        Self { lake, source_store }
    }

    /// Index a local directory (already cloned/extracted).
    ///
    /// # Arguments
    ///
    /// - `dir`: Root directory of the package source.
    /// - `ecosystem`, `package`, `version`: Package identity.
    /// - `embedder`: Embedding engine (caller manages its lifecycle).
    /// - `skip_tests`: When true, test files and directories are skipped.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` on storage failures or embedding failures.
    pub fn index_directory(
        &self,
        dir: &Path,
        ecosystem: &str,
        package: &str,
        version: &str,
        embedder: &mut EmbeddingEngine,
        skip_tests: bool,
    ) -> Result<IndexResult, LakeError> {
        let mut symbols = Vec::new();
        let mut doc_chunks = Vec::new();
        let mut source_files = Vec::new();
        let mut file_count = 0i32;

        // Step 1+2: Walk and parse
        let walker = zen_search::walk::build_walker(
            dir,
            zen_search::walk::WalkMode::Raw,
            skip_tests,
            None,
            None,
        );

        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();
            let rel_path = path.strip_prefix(dir).unwrap_or(path);
            let rel_path_str = rel_path.to_string_lossy().to_string();

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue, // Skip binary/unreadable files
            };

            // Detect language for SourceFile.language
            let lang = detect_language_ext(&rel_path_str);
            let lang_str = lang.as_ref().map(|l| match l {
                DetectedLanguage::Builtin(builtin) => format!("{builtin:?}").to_lowercase(),
                DetectedLanguage::Markdown => "markdown".to_string(),
                DetectedLanguage::Rst => "rst".to_string(),
                DetectedLanguage::Svelte => "svelte".to_string(),
                DetectedLanguage::Toml => "toml".to_string(),
                DetectedLanguage::Text => "text".to_string(),
            });

            // Compute size and line count before moving content
            let size_bytes = content.len() as i32;
            let line_count = content.lines().count() as i32;

            // Extract API symbols for source files with a recognized language
            // (must happen BEFORE content is moved into SourceFile)
            if let Some(_lang) = lang {
                // extract_api returns symbols for all supported languages
                let items = extract_api(&content, &rel_path_str).unwrap_or_default();

                for item in &items {
                    symbols.push(parsed_item_to_row(
                        item,
                        ecosystem,
                        package,
                        version,
                        &rel_path_str,
                    ));
                }
                file_count += 1;
            }

            // Check if documentation file (must happen BEFORE content is moved)
            if is_doc_file(&rel_path_str) {
                let chunks = chunk_document(&content, &rel_path_str);
                for chunk in chunks {
                    doc_chunks.push((
                        chunk,
                        ecosystem.to_string(),
                        package.to_string(),
                        version.to_string(),
                    ));
                }
            }

            // Move content into SourceFile (after all borrows are done)
            source_files.push(SourceFile {
                ecosystem: ecosystem.to_string(),
                package: package.to_string(),
                version: version.to_string(),
                file_path: rel_path_str.clone(),
                content,
                language: lang_str,
                size_bytes,
                line_count,
            });
        }

        // Step 4: Generate embeddings (batch)
        let embed_texts: Vec<String> = symbols
            .iter()
            .map(|s| {
                format!(
                    "{} {} {}",
                    s.name,
                    s.signature.as_deref().unwrap_or(""),
                    s.doc_comment.as_deref().unwrap_or("")
                )
            })
            .collect();

        let symbol_embeddings = if !embed_texts.is_empty() {
            embedder
                .embed_batch(embed_texts)
                .map_err(|e| LakeError::Other(format!("Embedding failed: {e}")))?
        } else {
            Vec::new()
        };

        // Defensive check: ensure embedding count matches symbol count
        if symbol_embeddings.len() != symbols.len() {
            return Err(LakeError::Other(format!(
                "Embedding count mismatch: expected {}, got {}",
                symbols.len(),
                symbol_embeddings.len()
            )));
        }

        for (sym, emb) in symbols.iter_mut().zip(symbol_embeddings.into_iter()) {
            sym.embedding = emb;
        }

        let doc_embed_texts: Vec<String> = doc_chunks
            .iter()
            .map(|(c, _, _, _)| c.content.clone())
            .collect();

        let doc_embeddings = if !doc_embed_texts.is_empty() {
            embedder
                .embed_batch(doc_embed_texts)
                .map_err(|e| LakeError::Other(format!("Embedding failed: {e}")))?
        } else {
            Vec::new()
        };

        // Defensive check: ensure embedding count matches doc chunk count
        if doc_embeddings.len() != doc_chunks.len() {
            return Err(LakeError::Other(format!(
                "Doc embedding count mismatch: expected {}, got {}",
                doc_chunks.len(),
                doc_embeddings.len()
            )));
        }

        let doc_chunk_rows: Vec<DocChunkRow> = doc_chunks
            .into_iter()
            .zip(doc_embeddings.into_iter())
            .map(|((chunk, eco, pkg, ver), emb)| DocChunkRow {
                id: String::new(), // DuckDB will generate via md5()
                ecosystem: eco,
                package: pkg,
                version: ver,
                chunk_index: chunk.chunk_index as i32,
                title: chunk.title,
                content: chunk.content,
                source_file: Some(chunk.source_file),
                format: Some(chunk.format),
                embedding: emb,
            })
            .collect();

        let symbol_count = symbols.len() as i32;
        let doc_chunk_count = doc_chunk_rows.len() as i32;
        let source_file_count = source_files.len() as i32;

        // Step 5: Store in local DuckDB cache (temporary for Phase 3)
        self.lake.store_symbols(&symbols)?;
        self.lake.store_doc_chunks(&doc_chunk_rows)?;
        self.source_store.store_source_files(&source_files)?;

        // Step 6: Register package and mark source cached
        self.lake.register_package(
            ecosystem,
            package,
            version,
            None,
            None,
            None,
            None,
            file_count,
            symbol_count,
            doc_chunk_count,
        )?;
        self.lake.set_source_cached(ecosystem, package, version)?;

        Ok(IndexResult {
            ecosystem: ecosystem.to_string(),
            package: package.to_string(),
            version: version.to_string(),
            file_count,
            symbol_count,
            doc_chunk_count,
            source_file_count,
        })
    }
}

/// Convert a `ParsedItem` into an `ApiSymbolRow`.
///
/// The `id` field is left empty (`String::new()`) so that DuckDB generates a
/// deterministic 16-character hex ID via `substr(md5(concat(...)), 1, 16)`.
fn parsed_item_to_row(
    item: &ParsedItem,
    ecosystem: &str,
    package: &str,
    version: &str,
    file_path: &str,
) -> ApiSymbolRow {
    ApiSymbolRow {
        id: String::new(), // Server-side generation
        ecosystem: ecosystem.to_string(),
        package: package.to_string(),
        version: version.to_string(),
        file_path: file_path.to_string(),
        kind: item.kind.to_string(),
        name: item.name.clone(),
        signature: Some(item.signature.clone()),
        source: item.source.clone(),
        doc_comment: if item.doc_comment.is_empty() {
            None
        } else {
            Some(item.doc_comment.clone())
        },
        line_start: Some(item.start_line as i32),
        line_end: Some(item.end_line as i32),
        visibility: Some(item.visibility.to_string()),
        is_async: item.metadata.is_async,
        is_unsafe: item.metadata.is_unsafe,
        is_error_type: item.metadata.is_error_type,
        returns_result: item.metadata.returns_result,
        return_type: item.metadata.return_type.clone(),
        generics: item.metadata.generics.clone(),
        attributes: if item.metadata.attributes.is_empty() {
            None
        } else {
            // Serialize Vec<String> as JSON array string
            Some(serde_json::to_string(&item.metadata.attributes).unwrap_or_default())
        },
        metadata: Some(serde_json::to_string(&item.metadata).unwrap_or_default()),
        embedding: Vec::new(), // Filled after batch embedding
    }
}

/// Determine if a file path should be treated as documentation.
///
/// Matches common documentation filenames and paths: README, CHANGELOG, CONTRIBUTING,
/// files under docs/ or doc/, and .md/.rst extensions. .txt files are only treated
/// as docs if they're inside a docs/ or doc/ directory to avoid misclassifying
/// config files like requirements.txt.
fn is_doc_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md")
        || lower.ends_with(".rst")
        || (lower.ends_with(".txt")
            && (lower.starts_with("docs/")
                || lower.starts_with("doc/")
                || lower.contains("/docs/")
                || lower.contains("/doc/")))
        || lower.starts_with("readme")
        || lower.starts_with("changelog")
        || lower.starts_with("contributing")
        || lower.contains("/changelog")
        || lower.contains("/contributing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parsed_item_to_row_mapping() {
        let item = ParsedItem {
            kind: SymbolKind::Function,
            name: "test_fn".to_string(),
            signature: "pub fn test_fn()".to_string(),
            source: None,
            doc_comment: "Test function".to_string(),
            start_line: 1,
            end_line: 5,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                is_async: false,
                is_unsafe: false,
                ..Default::default()
            },
        };

        let row = parsed_item_to_row(&item, "rust", "test_pkg", "1.0.0", "src/lib.rs");

        assert_eq!(row.id, ""); // ID generated server-side
        assert_eq!(row.ecosystem, "rust");
        assert_eq!(row.package, "test_pkg");
        assert_eq!(row.version, "1.0.0");
        assert_eq!(row.file_path, "src/lib.rs");
        assert_eq!(row.kind, "function");
        assert_eq!(row.name, "test_fn");
        assert_eq!(row.signature, Some("pub fn test_fn()".to_string()));
        assert_eq!(row.doc_comment, Some("Test function".to_string()));
        assert_eq!(row.line_start, Some(1));
        assert_eq!(row.line_end, Some(5));
        assert_eq!(row.visibility, Some("public".to_string()));
        assert!(!row.is_async);
        assert!(!row.is_unsafe);
    }

    #[test]
    fn is_doc_file_detection() {
        assert!(is_doc_file("README.md"));
        assert!(is_doc_file("docs/guide.md"));
        assert!(is_doc_file("doc/api.rst"));
        assert!(is_doc_file("CHANGELOG.md"));
        assert!(is_doc_file("CONTRIBUTING.md"));
        assert!(is_doc_file("docs/tutorial.txt")); // .txt in docs/ is doc
        assert!(!is_doc_file("reference/tutorial.txt")); // .txt outside docs/ is NOT doc
        assert!(!is_doc_file("requirements.txt")); // config files not docs
        assert!(!is_doc_file("src/lib.rs"));
        assert!(!is_doc_file("Cargo.toml"));
    }

    #[test]
    fn pipeline_full_end_to_end() {
        // This test requires the fastembed model to be pre-cached at ~/.zenith/cache/fastembed/
        let tmp = TempDir::new().unwrap();

        // Create a simple Rust file and a README
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(
            tmp.path().join("src/lib.rs"),
            "/// My library\npub fn hello() -> &'static str { \"world\" }",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("README.md"),
            "# My Crate\n\nA simple library for greeting.",
        )
        .unwrap();

        let lake = ZenLake::open_in_memory().unwrap();
        let source_store = SourceFileStore::open_in_memory().unwrap();
        let pipeline = IndexingPipeline::new(lake, source_store);

        let mut embedder = match zen_embeddings::EmbeddingEngine::new() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Skipping full pipeline test: embedding engine not available: {e}");
                return;
            }
        };

        let result = pipeline
            .index_directory(
                tmp.path(),
                "rust",
                "test_crate",
                "0.1.0",
                &mut embedder,
                false,
            )
            .expect("indexing should succeed");

        // Should have extracted at least one symbol and one doc chunk
        assert!(result.symbol_count >= 1);
        assert!(result.doc_chunk_count >= 1);
        assert_eq!(result.file_count, 2); // src/lib.rs and README.md
        assert_eq!(result.source_file_count, 2);
        assert_eq!(result.ecosystem, "rust");
        assert_eq!(result.package, "test_crate");
        assert_eq!(result.version, "0.1.0");

        // source_cached should be true
        let cached: bool = pipeline
            .lake
            .conn()
            .query_row(
                "SELECT source_cached FROM indexed_packages WHERE ecosystem = 'rust' AND package = 'test_crate' AND version = '0.1.0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(cached);

        // Verify that symbols have 384-dim embeddings
        let dim: i64 = pipeline
            .lake
            .conn()
            .query_row(
                "SELECT array_length(embedding) FROM api_symbols LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dim, 384);

        // Verify doc chunks also have embeddings
        let doc_dim: i64 = pipeline
            .lake
            .conn()
            .query_row(
                "SELECT array_length(embedding) FROM doc_chunks LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(doc_dim, 384);

        // Verify source files stored in separate store
        let src_count: i64 = pipeline
            .source_store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM source_files WHERE ecosystem = 'rust' AND package = 'test_crate'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(src_count, 2);
    }
}
