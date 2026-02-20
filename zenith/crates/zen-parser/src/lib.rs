#![allow(clippy::cast_possible_truncation)]
//! # zen-parser
//!
//! ast-grep-based source code parsing and API extraction for Zenith.
//!
//! Supports all 26 ast-grep built-in languages with tiered extraction:
//! - **Rich extractors** (Rust, Python, TypeScript/TSX/JS, Go, Elixir, C#, Haskell, Java, Lua, PHP, Ruby, JSON, YAML):
//!   full `ParsedItem` metadata with language-specific features
//! - **Generic extractor** (all other built-in languages):
//!   kind-based extraction capturing function/class/type definitions
//! - **Custom language lane** (Markdown via `tree-sitter-md`, TOML via `tree-sitter-toml-ng`, RST via `tree-sitter-rst`, Svelte via `tree-sitter-svelte-next`):
//!   parser-backed extraction using a custom ast-grep `Language`
//!
//! Symbol taxonomy is normalized across extractors:
//! - top-level callables use `Function`
//! - member callables use `Method` or `Constructor`
//! - member data uses `Field`/`Property`/`Event`/`Indexer`
//!
//! Member-level symbols should populate `SymbolMetadata::owner_name`,
//! `SymbolMetadata::owner_kind`, and `SymbolMetadata::is_static_member`.
//!
//! Primary API: [`extract_api()`] detects language from file path and dispatches
//! to the correct extractor. Regex fallback is planned but not yet implemented.

pub mod doc_chunker;
pub mod error;
pub mod extractors;
pub mod parser;
pub mod test_files;
pub mod types;

pub use error::ParserError;
pub use parser::{
    DetectedLanguage, MarkdownLang, RstLang, SvelteLang, TomlLang, detect_language,
    detect_language_ext, parse_markdown_source, parse_rst_source, parse_source,
    parse_svelte_source, parse_toml_source,
};
pub use test_files::{is_test_dir, is_test_file};
pub use types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use ast_grep_language::SupportLang;

/// Extract API symbols from source code for any supported language.
///
/// Detects the language from `file_path`, parses with ast-grep (or a custom
/// parser for Markdown/TOML/RST/Svelte/Text), and extracts symbols.
///
/// If ast-grep extraction yields zero items, logs a warning and returns an
/// empty `Vec`. Regex fallback is deferred to a future PR.
///
/// # Errors
///
/// Returns [`ParserError::UnsupportedLanguage`] if the file extension is not
/// recognized by any extractor.
///
/// # Examples
///
/// ```
/// use zen_parser::extract_api;
///
/// let items = extract_api("fn hello() {}", "src/main.rs").unwrap();
/// assert!(!items.is_empty());
/// ```
pub fn extract_api(source: &str, file_path: &str) -> Result<Vec<ParsedItem>, ParserError> {
    let lang = detect_language_ext(file_path)
        .ok_or_else(|| ParserError::UnsupportedLanguage(file_path.to_string()))?;

    let items = match lang {
        DetectedLanguage::Builtin(builtin) => extract_builtin(source, builtin)?,
        DetectedLanguage::Markdown => {
            let root = parse_markdown_source(source);
            extractors::markdown::extract(&root)?
        }
        DetectedLanguage::Toml => {
            let root = parse_toml_source(source);
            extractors::toml::extract(&root)?
        }
        DetectedLanguage::Rst => {
            let root = parse_rst_source(source);
            extractors::rst::extract(&root)?
        }
        DetectedLanguage::Svelte => {
            let root = parse_svelte_source(source);
            extractors::svelte::extract(&root)?
        }
        DetectedLanguage::Text => extractors::text::extract(source)?,
    };

    if items.is_empty() {
        tracing::debug!(
            file = file_path,
            "ast-grep extraction yielded 0 items; regex fallback not yet implemented"
        );
    }

    Ok(items)
}

/// Dispatch to the correct builtin language extractor.
///
/// Handles the three dispatcher signature families:
/// - `extract(root)` — most languages
/// - `extract(root, source)` — bash, c, cpp, rust
/// - `extract(root, lang)` — typescript, tsx
fn extract_builtin(source: &str, lang: SupportLang) -> Result<Vec<ParsedItem>, ParserError> {
    let root = parse_source(source, lang);
    match lang {
        SupportLang::Rust => extractors::rust::extract(&root, source),
        SupportLang::Python => extractors::python::extract(&root),
        SupportLang::TypeScript => extractors::typescript::extract(&root, lang),
        SupportLang::Tsx => extractors::tsx::extract(&root, lang),
        SupportLang::JavaScript => extractors::javascript::extract(&root),
        SupportLang::Go => extractors::go::extract(&root),
        SupportLang::Elixir => extractors::elixir::extract(&root),
        SupportLang::C => extractors::c::extract(&root, source),
        SupportLang::Cpp => extractors::cpp::extract(&root, source),
        SupportLang::CSharp => extractors::csharp::extract(&root),
        SupportLang::Css => extractors::css::extract(&root),
        SupportLang::Haskell => extractors::haskell::extract(&root),
        SupportLang::Html => extractors::html::extract(&root),
        SupportLang::Java => extractors::java::extract(&root),
        SupportLang::Json => extractors::json::extract(&root),
        SupportLang::Lua => extractors::lua::extract(&root),
        SupportLang::Php => extractors::php::extract(&root),
        SupportLang::Ruby => extractors::ruby::extract(&root),
        SupportLang::Bash => extractors::bash::extract(&root, source),
        SupportLang::Yaml => extractors::yaml::extract(&root),
        // Catch-all for any future SupportLang variants
        _ => Err(ParserError::UnsupportedLanguage(format!("{lang:?}"))),
    }
}

#[cfg(test)]
mod spike_ast_grep;

#[cfg(test)]
mod extract_api_tests {
    use super::*;

    #[test]
    fn rust_extraction() {
        let source = "fn hello() {}\nstruct Foo;\n";
        let items = extract_api(source, "src/main.rs").unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.name == "hello"));
    }

    #[test]
    fn python_extraction() {
        let source = "def greet(name):\n    return f'Hello {name}'\n";
        let items = extract_api(source, "app.py").unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.name == "greet"));
    }

    #[test]
    fn markdown_extraction() {
        let source = "# Title\n\nSome text.\n\n## Section\n\nMore text.\n";
        let items = extract_api(source, "README.md").unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.name == "Title"));
    }

    #[test]
    fn unsupported_extension_returns_error() {
        let result = extract_api("data", "file.xyz");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParserError::UnsupportedLanguage(_)
        ));
    }

    #[test]
    fn text_file_with_markdown_content() {
        // .txt with markdown content → text extractor detects MD and delegates
        let source = "# Title\n\nContent.\n\n## Section\n\nMore.\n";
        let items = extract_api(source, "llms.txt").unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.name == "Title"));
    }

    #[test]
    fn text_file_plain_content() {
        let source = "Just some plain text.\nWith no structure.\n";
        let items = extract_api(source, "notes.txt").unwrap();
        // Plain text with no headings — root item + paragraph items
        assert!(!items.is_empty());
    }
}
