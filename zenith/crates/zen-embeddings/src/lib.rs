//! # zen-embeddings
//!
//! Local embedding generation for Zenith using fastembed (ONNX runtime).
//!
//! Generates 384-dimensional vectors for text content (API symbols, doc chunks,
//! search queries) without requiring any external API keys.
//!
//! ## Model
//!
//! Uses [`AllMiniLML6V2`](fastembed::EmbeddingModel::AllMiniLML6V2) (sentence-transformers/all-MiniLM-L6-v2):
//! - 384-dimensional output vectors
//! - Mean pooling (no query/passage prefix needed)
//! - ~80MB model size, cached at `~/.zenith/cache/fastembed/`
//!
//! ## Async usage
//!
//! The fastembed ONNX runtime is synchronous. When calling from async code,
//! wrap calls in [`tokio::task::spawn_blocking`]:
//!
//! ```ignore
//! let embeddings = tokio::task::spawn_blocking(move || {
//!     engine.embed_batch(texts)
//! }).await??;
//! ```

pub mod error;

pub use error::EmbeddingError;
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

/// Local embedding engine backed by fastembed (ONNX runtime).
///
/// Wraps the `AllMiniLML6V2` model to produce 384-dimensional float vectors.
/// Model files are downloaded on first use and cached at `~/.zenith/cache/fastembed/`.
///
/// # Thread safety
///
/// [`TextEmbedding::embed`] requires `&mut self`. To use from multiple threads,
/// wrap in a `Mutex` or create one engine per thread. From async code, prefer
/// [`tokio::task::spawn_blocking`] with a moved engine.
pub struct EmbeddingEngine {
    model: TextEmbedding,
}

impl EmbeddingEngine {
    /// Create a new embedding engine with the `AllMiniLML6V2` model.
    ///
    /// Downloads the model on first run (~80MB) to `~/.zenith/cache/fastembed/`.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddingError::InitFailed`] if model download or ONNX initialization fails.
    pub fn new() -> Result<Self, EmbeddingError> {
        let cache_dir = dirs::home_dir().map_or_else(
            || std::path::PathBuf::from(".fastembed_cache"),
            |h| h.join(".zenith").join("cache").join("fastembed"),
        );

        let model = TextEmbedding::try_new(
            TextInitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_cache_dir(cache_dir)
                .with_show_download_progress(true),
        )
        .map_err(|e| EmbeddingError::InitFailed(e.to_string()))?;

        Ok(Self { model })
    }

    /// Embed a batch of texts. Returns one 384-dim vector per input.
    ///
    /// # Arguments
    ///
    /// * `texts` — The texts to embed. Each text is embedded independently.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddingError::EmbedFailed`] if the ONNX inference fails.
    pub fn embed_batch(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.model
            .embed(texts, None)
            .map_err(|e| EmbeddingError::EmbedFailed(e.to_string()))
    }

    /// Embed a single text. Returns a 384-dim vector.
    ///
    /// Convenience wrapper around [`Self::embed_batch`].
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddingError::EmbedFailed`] if inference fails, or
    /// [`EmbeddingError::EmptyResult`] if the model returns no embeddings.
    pub fn embed_single(&mut self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut results = self.embed_batch(vec![text.to_string()])?;
        results.pop().ok_or(EmbeddingError::EmptyResult)
    }

    /// Embedding vector dimensionality (always 384 for `AllMiniLML6V2`).
    #[must_use]
    pub const fn dimension() -> usize {
        384
    }
}

#[cfg(test)]
mod spike_fastembed;

#[cfg(test)]
mod tests {
    use super::*;

    /// Cosine similarity between two vectors.
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

    #[test]
    fn engine_initializes() {
        let engine = EmbeddingEngine::new();
        assert!(
            engine.is_ok(),
            "engine should initialize: {:?}",
            engine.err()
        );
    }

    #[test]
    fn single_embed_384_dims() {
        let mut engine = EmbeddingEngine::new().expect("engine should init");
        let embedding = engine
            .embed_single("Rust is a systems programming language")
            .expect("embed should succeed");

        assert_eq!(embedding.len(), 384, "embedding should have 384 dimensions");

        for (i, val) in embedding.iter().enumerate() {
            assert!(val.is_finite(), "dimension {i} should be a finite float");
        }
    }

    #[test]
    fn batch_embed_correct_count() {
        let mut engine = EmbeddingEngine::new().expect("engine should init");
        let texts = vec![
            "tokio::spawn creates a new async task".to_string(),
            "fn main() is the entry point".to_string(),
            "impl Iterator for MyStruct".to_string(),
        ];

        let embeddings = engine
            .embed_batch(texts)
            .expect("batch embed should succeed");

        assert_eq!(embeddings.len(), 3, "should return one embedding per input");
        for (i, emb) in embeddings.iter().enumerate() {
            assert_eq!(emb.len(), 384, "embedding {i} should have 384 dimensions");
        }
    }

    #[test]
    fn cosine_similarity_clustering() {
        let mut engine = EmbeddingEngine::new().expect("engine should init");

        let emb_async = engine
            .embed_single("spawn a new async task in tokio")
            .expect("embed A");
        let emb_similar = engine
            .embed_single("create a new asynchronous task")
            .expect("embed B");
        let emb_unrelated = engine
            .embed_single("chocolate cake recipe with buttercream")
            .expect("embed C");

        let sim_related = cosine_similarity(&emb_async, &emb_similar);
        let sim_unrelated = cosine_similarity(&emb_async, &emb_unrelated);

        assert!(
            sim_related > sim_unrelated,
            "related texts ({sim_related:.4}) should have higher similarity than unrelated ({sim_unrelated:.4})"
        );
        assert!(
            sim_related > 0.5,
            "related texts should have >0.5 similarity, got {sim_related:.4}"
        );
    }

    #[test]
    fn determinism() {
        let mut engine = EmbeddingEngine::new().expect("engine should init");
        let text = "pub async fn connect(addr: SocketAddr) -> io::Result<TcpStream>";

        let emb1 = engine.embed_single(text).expect("first embed");
        let emb2 = engine.embed_single(text).expect("second embed");

        assert_eq!(emb1, emb2, "same text should produce identical embeddings");
    }

    #[test]
    fn empty_text_no_panic() {
        let mut engine = EmbeddingEngine::new().expect("engine should init");
        let embedding = engine
            .embed_single("")
            .expect("empty text should not panic");

        assert_eq!(
            embedding.len(),
            384,
            "empty text should still produce 384-dim vector"
        );
    }

    #[test]
    fn dimension_constant() {
        assert_eq!(EmbeddingEngine::dimension(), 384);
    }
}
