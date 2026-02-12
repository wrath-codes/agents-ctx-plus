//! Parser error types for zen-parser.

/// Errors that can occur during source code parsing and extraction.
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Parse failed for {language}: {message}")]
    ParseFailed { language: String, message: String },

    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
