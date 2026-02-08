//! # Spike 0.6: fastembed Local Embedding Validation
//!
//! Validates that the `fastembed` crate (v5.8) compiles and generates embeddings for
//! zenith's indexing and search needs:
//!
//! - **Default model**: `BGESmallENV15` (384-dim, ~100MB, fastembed's default)
//! - **Design model**: `AllMiniLML6V2` (384-dim, ~80MB, specified in `05-crate-designs.md`)
//! - **Single embedding**: `model.embed(vec![text], None)` → 384-dim `Vec<f32>`
//! - **Batch embedding**: Multiple texts → correct count, all 384 dims
//! - **Determinism**: Same text → identical vector on repeated calls
//! - **Semantic similarity**: Similar texts cluster, dissimilar texts don't
//! - **Query/passage prefixes**: BGE models benefit from `"query: "` / `"passage: "` prefixes
//! - **Edge cases**: Empty strings, single words, very long text
//! - **Batch size control**: Explicit `Some(64)` produces same results as default
//! - **(Future)** Sparse embeddings, reranking, and quantized models validated separately
//!   to avoid downloading 3 extra models (~500MB+) during the core spike
//!
//! ## Validates
//!
//! Embeddings generate locally with 384 dimensions — blocks Phase 3.
//!
//! ## Model Choice: BGESmallENV15 vs AllMiniLML6V2
//!
//! fastembed's default is `BGESmallENV15` (BAAI/bge-small-en-v1.5) which uses CLS pooling
//! and benefits from `"query: "` / `"passage: "` prefixes. The crate design
//! (`05-crate-designs.md`) specifies `AllMiniLML6V2` (sentence-transformers/all-MiniLM-L6-v2)
//! which uses Mean pooling and does NOT use prefixes. Both are 384-dim.
//!
//! This spike validates both to compare:
//! - BGESmallENV15: Better retrieval quality on benchmarks (MTEB ~60.1 avg)
//! - AllMiniLML6V2: Slightly faster, smaller (~80MB vs ~100MB), simpler (no prefixes)
//!
//! The design can be revisited after this spike confirms both work.
//!
//! ## Async Strategy
//!
//! fastembed is synchronous — the ONNX runtime and Rayon handle parallelism internally.
//! No async API is provided. When calling from zenith's tokio-based pipeline, use
//! `tokio::task::spawn_blocking` (same pattern as DuckDB, see spike 0.4).
//!
//! ## First Run
//!
//! First run downloads models to `~/.zenith/cache/fastembed/`.
//! BGESmallENV15 is ~100MB, AllMiniLML6V2 is ~80MB. Progress bars are shown by default.
//!
//! ## API Notes (v5.8)
//!
//! - `TextEmbedding::try_new()` accepts `TextInitOptions` (aliased as deprecated `InitOptions`)
//! - `model.embed(texts, batch_size)` takes `&mut self` (not `&self`)
//! - `embed()` accepts `impl AsRef<[S]>` where `S: AsRef<str>` — works with `Vec<&str>`,
//!   `Vec<String>`, `&[&str]`, etc.
//! - `batch_size: Option<usize>` — `None` defaults to 256
//! - Returns `Result<Vec<Embedding>>` where `Embedding = Vec<f32>`
//! - `SparseTextEmbedding::embed()` returns `Vec<SparseEmbedding>` with `.indices` and `.values`
//! - `TextRerank::rerank()` takes `&mut self` and returns `Vec<RerankResult>` with `.score`,
//!   `.index`, and `.document: Option<String>`
//! - Quantized models with dynamic quantization (like `AllMiniLML6V2Q`) cannot use batching
//!   smaller than the total input size — fastembed enforces this at runtime.
//!
//! ## Model Caching
//!
//! fastembed's default cache dir is `.fastembed_cache` (relative to CWD), which is
//! unpredictable for tests run from different directories. All spike tests use a stable
//! cache at `~/.zenith/cache/fastembed/` via `TextInitOptions::with_cache_dir()`. Models
//! are downloaded once and reused across all subsequent runs.
//!
//! To pre-warm the cache (avoid download during test runs), run:
//! ```bash
//! cargo test -p zen-embeddings spike_fastembed::spike_default_model_loads -- --nocapture
//! cargo test -p zen-embeddings spike_fastembed::spike_allminilm_model_loads -- --nocapture
//! ```

use std::path::PathBuf;

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

/// Stable cache directory for model files.
/// Uses `~/.zenith/cache/fastembed/` so models persist across builds and test runs
/// and stay out of the repository tree.
fn cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zenith")
        .join("cache")
        .join("fastembed")
}

/// Create TextInitOptions for a given model with stable cache dir.
fn init_opts(model: EmbeddingModel) -> TextInitOptions {
    TextInitOptions::new(model)
        .with_cache_dir(cache_dir())
        .with_show_download_progress(true)
}

/// Create TextInitOptions for the default model with stable cache dir.
fn init_opts_default() -> TextInitOptions {
    TextInitOptions::new(EmbeddingModel::BGESmallENV15)
        .with_cache_dir(cache_dir())
        .with_show_download_progress(true)
}

/// Cosine similarity between two vectors.
///
/// Used for semantic similarity assertions in tests. In production,
/// DuckDB's `array_cosine_similarity()` or Lance's vector search handles this.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same dimensionality");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── 1. Default model loads ──────────────────────────────────────────────────

/// Smoke test: fastembed's default model (BGESmallENV15) loads successfully.
#[test]
fn spike_default_model_loads() {
    let model = TextEmbedding::try_new(init_opts_default());
    assert!(
        model.is_ok(),
        "default model should load: {:?}",
        model.err()
    );
}

// ── 2. AllMiniLML6V2 loads ──────────────────────────────────────────────────

/// Load the model specified in the crate design (AllMiniLML6V2).
#[test]
fn spike_allminilm_model_loads() {
    let model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2));
    assert!(
        model.is_ok(),
        "AllMiniLML6V2 should load: {:?}",
        model.err()
    );
}

// ── 3. Single embed produces 384 dims ───────────────────────────────────────

/// Embed a single text and verify the output has exactly 384 dimensions.
#[test]
fn spike_single_embed_384_dims() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let embeddings = model
        .embed(vec!["Rust is a systems programming language"], None)
        .expect("embed should succeed");

    assert_eq!(embeddings.len(), 1, "should return exactly one embedding");
    assert_eq!(
        embeddings[0].len(),
        384,
        "embedding should have 384 dimensions"
    );

    // Verify values are finite floats
    for (i, val) in embeddings[0].iter().enumerate() {
        assert!(val.is_finite(), "dimension {i} should be a finite float");
    }
}

// ── 4. Batch embed correct count ────────────────────────────────────────────

/// Embed multiple texts and verify output count matches input count, all 384 dims.
#[test]
fn spike_batch_embed_correct_count() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let texts = vec![
        "tokio::spawn creates a new async task",
        "fn main() is the entry point",
        "impl Iterator for MyStruct",
        "use std::collections::HashMap",
        "async fn fetch_data() -> Result<Response>",
    ];

    let embeddings = model
        .embed(texts.clone(), None)
        .expect("batch embed should succeed");

    assert_eq!(
        embeddings.len(),
        texts.len(),
        "should return one embedding per input"
    );
    for (i, emb) in embeddings.iter().enumerate() {
        assert_eq!(emb.len(), 384, "embedding {i} should have 384 dimensions");
    }
}

// ── 5. Deterministic embedding ──────────────────────────────────────────────

/// Same text embedded twice should produce identical vectors.
#[test]
fn spike_deterministic_embedding() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let text = "pub async fn connect(addr: SocketAddr) -> io::Result<TcpStream>";

    let emb1 = model
        .embed(vec![text], None)
        .expect("first embed should succeed");
    let emb2 = model
        .embed(vec![text], None)
        .expect("second embed should succeed");

    assert_eq!(
        emb1[0], emb2[0],
        "same text should produce identical embeddings"
    );
}

// ── 6. Cosine similarity sanity ─────────────────────────────────────────────

/// Similar texts should have higher cosine similarity than dissimilar texts.
#[test]
fn spike_cosine_similarity_sanity() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let embeddings = model
        .embed(
            vec![
                "spawn a new async task in tokio", // A: async runtime concept
                "create a new asynchronous task",  // B: semantically similar to A
                "chocolate cake recipe with buttercream", // C: completely unrelated
            ],
            None,
        )
        .expect("embed should succeed");

    let sim_ab = cosine_similarity(&embeddings[0], &embeddings[1]);
    let sim_ac = cosine_similarity(&embeddings[0], &embeddings[2]);

    assert!(
        sim_ab > sim_ac,
        "similar texts (sim_ab={sim_ab:.4}) should have higher cosine similarity \
         than dissimilar texts (sim_ac={sim_ac:.4})"
    );

    // Similarity between related texts should be reasonably high
    assert!(
        sim_ab > 0.5,
        "semantically similar texts should have cosine similarity > 0.5, got {sim_ab:.4}"
    );

    // Similarity between unrelated texts should be low
    assert!(
        sim_ac < 0.5,
        "unrelated texts should have cosine similarity < 0.5, got {sim_ac:.4}"
    );
}

// ── 7. Query/passage prefix behavior ────────────────────────────────────────

/// BGE models benefit from query/passage prefixes. Verify they produce different
/// embeddings and that prefixed query-passage similarity differs from unprefixed.
#[test]
fn spike_query_passage_prefix() {
    let mut model = TextEmbedding::try_new(init_opts_default()).expect("BGE model should load");

    let embeddings = model
        .embed(
            vec![
                "query: connection pooling in reqwest",
                "passage: reqwest supports HTTP connection pooling for reuse",
                "connection pooling in reqwest", // no prefix
            ],
            None,
        )
        .expect("embed should succeed");

    let query_emb = &embeddings[0];
    let passage_emb = &embeddings[1];
    let plain_emb = &embeddings[2];

    // Prefixed and unprefixed should be different
    assert_ne!(
        query_emb, plain_emb,
        "query-prefixed embedding should differ from unprefixed"
    );

    // Query→passage similarity vs plain→passage similarity
    let sim_qp = cosine_similarity(query_emb, passage_emb);
    let sim_pp = cosine_similarity(plain_emb, passage_emb);

    // Both should be high (same semantic content), but potentially different
    assert!(
        sim_qp > 0.5,
        "query-passage similarity should be > 0.5, got {sim_qp:.4}"
    );
    assert!(
        sim_pp > 0.5,
        "plain-passage similarity should be > 0.5, got {sim_pp:.4}"
    );

    // NOTE: For BGE models, query prefix is designed for retrieval tasks.
    // The exact relationship between sim_qp and sim_pp is model-dependent.
    // We just verify both produce valid, different embeddings.
}

// ── 8. Empty and short inputs ───────────────────────────────────────────────

/// Edge cases: empty string, single character, single word should all produce
/// 384-dim vectors without panicking.
#[test]
fn spike_empty_and_short_inputs() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let embeddings = model
        .embed(vec!["", "x", "spawn"], None)
        .expect("edge-case embed should succeed");

    assert_eq!(embeddings.len(), 3);
    for (i, emb) in embeddings.iter().enumerate() {
        assert_eq!(
            emb.len(),
            384,
            "edge-case input {i} should still produce 384 dims"
        );
    }

    // Empty string should still produce a valid (non-NaN) vector
    for (i, val) in embeddings[0].iter().enumerate() {
        assert!(
            val.is_finite(),
            "empty string dim {i} should be finite, got {val}"
        );
    }
}

// ── 9. Long input ───────────────────────────────────────────────────────────

/// Very long text (well beyond 512 tokens) should still produce a 384-dim vector.
/// Models truncate internally to their max_length (512 tokens for MiniLM).
#[test]
fn spike_long_input() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    // Generate a ~2000-word text (well beyond 512 token limit)
    let long_text = "async fn process_request(req: Request) -> Result<Response> { ".repeat(200);

    let embeddings = model
        .embed(vec![long_text.as_str()], None)
        .expect("long input should not error — model truncates internally");

    assert_eq!(embeddings.len(), 1);
    assert_eq!(
        embeddings[0].len(),
        384,
        "long input should still produce 384 dims after truncation"
    );
}

// ── 10. Batch size parameter ────────────────────────────────────────────────

/// Explicit batch_size=Some(2) should produce the same embeddings as None (default 256).
#[test]
fn spike_batch_size_parameter() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    let texts = vec![
        "tokio runtime",
        "async await",
        "thread pool",
        "connection pool",
    ];

    let emb_default = model
        .embed(texts.clone(), None)
        .expect("default batch should succeed");
    let emb_small_batch = model
        .embed(texts, Some(2))
        .expect("small batch should succeed");

    assert_eq!(emb_default.len(), emb_small_batch.len());
    for (i, (a, b)) in emb_default.iter().zip(emb_small_batch.iter()).enumerate() {
        assert_eq!(
            a, b,
            "embedding {i} should be identical regardless of batch size"
        );
    }
}

// ── 11. End-to-end embedding pipeline simulation ────────────────────────────

/// Simulate the zenith indexing flow: embed a mix of API signatures and doc chunks,
/// verify all vectors are 384-dim, and confirm code-related texts cluster together.
#[test]
fn spike_end_to_end_embedding_pipeline() {
    let mut model = TextEmbedding::try_new(init_opts(EmbeddingModel::AllMiniLML6V2))
        .expect("model should load");

    // Simulate API symbols (signatures + doc comments)
    let api_symbols = vec![
        "pub async fn spawn<T>(future: T) -> JoinHandle<T::Output> — Spawns a new asynchronous task",
        "pub fn block_on<F: Future>(future: F) -> F::Output — Runs a future to completion on the current thread",
        "pub struct Runtime — The Tokio runtime provides I/O, timer, and task scheduling",
        "pub fn new() -> Builder — Creates a new runtime builder with default configuration",
    ];

    // Simulate doc chunks (from README sections)
    let doc_chunks = vec![
        "Tokio is an asynchronous runtime for the Rust programming language. It provides the building blocks needed for writing networking applications.",
        "To spawn a task, use the tokio::spawn function. This function takes a future and returns a JoinHandle which can be used to await the task's result.",
        "The chocolate cake recipe requires three cups of flour, two cups of sugar, and four eggs. Preheat the oven to 350 degrees.",
    ];

    // Embed both together (as the pipeline would)
    let mut all_texts: Vec<&str> = Vec::new();
    all_texts.extend_from_slice(&api_symbols);
    all_texts.extend_from_slice(&doc_chunks);

    let embeddings = model
        .embed(all_texts.clone(), None)
        .expect("pipeline embed should succeed");

    // Verify dimensions
    assert_eq!(embeddings.len(), api_symbols.len() + doc_chunks.len());
    for (i, emb) in embeddings.iter().enumerate() {
        assert_eq!(
            emb.len(),
            384,
            "pipeline embedding {i} should have 384 dimensions"
        );
    }

    // Verify semantic clustering: tokio spawn signature should be more similar to
    // tokio spawn doc chunk than to the chocolate cake chunk.
    let spawn_sig = &embeddings[0]; // "pub async fn spawn..."
    let spawn_doc = &embeddings[5]; // "To spawn a task, use the tokio::spawn function..."
    let cake_doc = &embeddings[6]; // "The chocolate cake recipe..."

    let sim_spawn = cosine_similarity(spawn_sig, spawn_doc);
    let sim_cake = cosine_similarity(spawn_sig, cake_doc);

    assert!(
        sim_spawn > sim_cake,
        "spawn signature should be more similar to spawn docs ({sim_spawn:.4}) \
         than to cake recipe ({sim_cake:.4})"
    );

    // All tokio-related content should be somewhat similar
    let runtime_struct = &embeddings[2]; // "pub struct Runtime..."
    let runtime_doc = &embeddings[4]; // "Tokio is an asynchronous runtime..."
    let sim_runtime = cosine_similarity(runtime_struct, runtime_doc);
    assert!(
        sim_runtime > 0.4,
        "Runtime struct and Tokio runtime doc should have reasonable similarity, got {sim_runtime:.4}"
    );

    // FINDING: fastembed works well for API symbol → doc chunk matching.
    // Even raw function signatures with type parameters produce meaningful embeddings
    // that cluster with their documentation counterparts.
}
