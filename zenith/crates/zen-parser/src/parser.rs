//! ast-grep wrapper and language detection from file extensions.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;

mod markdown_lang;
pub use markdown_lang::MarkdownLang;
mod rst_lang;
pub use rst_lang::RstLang;
mod svelte_lang;
pub use svelte_lang::SvelteLang;
mod toml_lang;
pub use toml_lang::TomlLang;

/// The concrete AST tree type returned by `parse_source`.
pub type AstTree = ast_grep_core::AstGrep<StrDoc<SupportLang>>;

/// The concrete AST type returned by `parse_markdown_source`.
pub type MarkdownAstTree = ast_grep_core::AstGrep<StrDoc<MarkdownLang>>;

/// The concrete AST type returned by `parse_toml_source`.
pub type TomlAstTree = ast_grep_core::AstGrep<StrDoc<TomlLang>>;

/// The concrete AST type returned by `parse_rst_source`.
pub type RstAstTree = ast_grep_core::AstGrep<StrDoc<RstLang>>;

/// The concrete AST type returned by `parse_svelte_source`.
pub type SvelteAstTree = ast_grep_core::AstGrep<StrDoc<SvelteLang>>;

/// Extended language detection that includes custom languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedLanguage {
    Builtin(SupportLang),
    Markdown,
    Rst,
    Svelte,
    Toml,
}

/// Detect the programming language from a file path extension.
///
/// Returns `None` for unsupported or unrecognized extensions.
#[must_use]
pub fn detect_language(file_path: &str) -> Option<SupportLang> {
    let ext = file_path.rsplit('.').next()?;
    match ext {
        "rs" => Some(SupportLang::Rust),
        "py" => Some(SupportLang::Python),
        "ts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "go" => Some(SupportLang::Go),
        "ex" | "exs" => Some(SupportLang::Elixir),
        "c" | "h" => Some(SupportLang::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(SupportLang::Cpp),
        "cs" => Some(SupportLang::CSharp),
        "css" => Some(SupportLang::Css),
        "hs" => Some(SupportLang::Haskell),
        "tf" | "hcl" => Some(SupportLang::Hcl),
        "html" | "htm" => Some(SupportLang::Html),
        "java" => Some(SupportLang::Java),
        "json" => Some(SupportLang::Json),
        "kt" | "kts" => Some(SupportLang::Kotlin),
        "lua" => Some(SupportLang::Lua),
        "nix" => Some(SupportLang::Nix),
        "php" => Some(SupportLang::Php),
        "rb" => Some(SupportLang::Ruby),
        "scala" | "sc" => Some(SupportLang::Scala),
        "sol" => Some(SupportLang::Solidity),
        "swift" => Some(SupportLang::Swift),
        "sh" | "bash" | "zsh" => Some(SupportLang::Bash),
        "yaml" | "yml" => Some(SupportLang::Yaml),
        _ => None,
    }
}

/// Detect language including custom parser-backed extensions.
#[must_use]
pub fn detect_language_ext(file_path: &str) -> Option<DetectedLanguage> {
    let ext = file_path.rsplit('.').next()?;
    match ext {
        "md" | "markdown" => Some(DetectedLanguage::Markdown),
        "rst" | "rest" => Some(DetectedLanguage::Rst),
        "svelte" => Some(DetectedLanguage::Svelte),
        "toml" => Some(DetectedLanguage::Toml),
        _ => detect_language(file_path).map(DetectedLanguage::Builtin),
    }
}

/// Parse source code into an ast-grep tree for the given language.
#[must_use]
pub fn parse_source(source: &str, lang: SupportLang) -> AstTree {
    use ast_grep_language::LanguageExt;
    lang.ast_grep(source)
}

/// Parse markdown source using the custom `tree-sitter-md` language.
#[must_use]
pub fn parse_markdown_source(source: &str) -> MarkdownAstTree {
    use ast_grep_core::tree_sitter::LanguageExt;
    MarkdownLang.ast_grep(source)
}

/// Parse TOML source using the custom `tree-sitter-toml-ng` language.
#[must_use]
pub fn parse_toml_source(source: &str) -> TomlAstTree {
    use ast_grep_core::tree_sitter::LanguageExt;
    TomlLang.ast_grep(source)
}

/// Parse reStructuredText source using the custom `tree-sitter-rst` language.
#[must_use]
pub fn parse_rst_source(source: &str) -> RstAstTree {
    use ast_grep_core::tree_sitter::LanguageExt;
    RstLang.ast_grep(source)
}

/// Parse Svelte source using the custom `tree-sitter-svelte-next` language.
#[must_use]
pub fn parse_svelte_source(source: &str) -> SvelteAstTree {
    use ast_grep_core::tree_sitter::LanguageExt;
    SvelteLang.ast_grep(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language("src/main.rs"), Some(SupportLang::Rust));
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect_language("app.py"), Some(SupportLang::Python));
    }

    #[test]
    fn detect_typescript_variants() {
        assert_eq!(detect_language("index.ts"), Some(SupportLang::TypeScript));
        assert_eq!(detect_language("app.tsx"), Some(SupportLang::Tsx));
        assert_eq!(detect_language("util.js"), Some(SupportLang::JavaScript));
        assert_eq!(detect_language("util.mjs"), Some(SupportLang::JavaScript));
        assert_eq!(detect_language("util.cjs"), Some(SupportLang::JavaScript));
    }

    #[test]
    fn detect_go() {
        assert_eq!(detect_language("main.go"), Some(SupportLang::Go));
    }

    #[test]
    fn detect_elixir() {
        assert_eq!(detect_language("lib.ex"), Some(SupportLang::Elixir));
        assert_eq!(detect_language("test.exs"), Some(SupportLang::Elixir));
    }

    #[test]
    fn detect_cpp_variants() {
        assert_eq!(detect_language("main.cpp"), Some(SupportLang::Cpp));
        assert_eq!(detect_language("main.cc"), Some(SupportLang::Cpp));
        assert_eq!(detect_language("header.hpp"), Some(SupportLang::Cpp));
    }

    #[test]
    fn detect_csharp() {
        assert_eq!(detect_language("Program.cs"), Some(SupportLang::CSharp));
    }

    #[test]
    fn detect_haskell() {
        assert_eq!(detect_language("Main.hs"), Some(SupportLang::Haskell));
    }

    #[test]
    fn detect_java() {
        assert_eq!(detect_language("Main.java"), Some(SupportLang::Java));
    }

    #[test]
    fn detect_lua() {
        assert_eq!(detect_language("init.lua"), Some(SupportLang::Lua));
    }

    #[test]
    fn detect_php() {
        assert_eq!(detect_language("index.php"), Some(SupportLang::Php));
    }

    #[test]
    fn detect_ruby() {
        assert_eq!(detect_language("user.rb"), Some(SupportLang::Ruby));
    }

    #[test]
    fn detect_json() {
        assert_eq!(detect_language("config.json"), Some(SupportLang::Json));
    }

    #[test]
    fn detect_yaml() {
        assert_eq!(detect_language("config.yaml"), Some(SupportLang::Yaml));
        assert_eq!(detect_language("config.yml"), Some(SupportLang::Yaml));
    }

    #[test]
    fn detect_unknown_returns_none() {
        assert_eq!(detect_language("data.csv"), None);
        assert_eq!(detect_language("readme"), None);
    }

    #[test]
    fn detect_markdown_extended() {
        assert_eq!(
            detect_language_ext("docs/README.md"),
            Some(DetectedLanguage::Markdown)
        );
        assert_eq!(
            detect_language_ext("docs/spec.markdown"),
            Some(DetectedLanguage::Markdown)
        );
    }

    #[test]
    fn detect_toml_extended() {
        assert_eq!(
            detect_language_ext("config/settings.toml"),
            Some(DetectedLanguage::Toml)
        );
    }

    #[test]
    fn detect_rst_extended() {
        assert_eq!(
            detect_language_ext("docs/spec.rst"),
            Some(DetectedLanguage::Rst)
        );
        assert_eq!(
            detect_language_ext("docs/spec.rest"),
            Some(DetectedLanguage::Rst)
        );
    }

    #[test]
    fn detect_svelte_extended() {
        assert_eq!(
            detect_language_ext("web/App.svelte"),
            Some(DetectedLanguage::Svelte)
        );
    }

    #[test]
    fn detect_builtin_via_extended() {
        assert_eq!(
            detect_language_ext("src/main.rs"),
            Some(DetectedLanguage::Builtin(SupportLang::Rust))
        );
    }

    #[test]
    fn detect_nested_path() {
        assert_eq!(
            detect_language("src/parser/mod.rs"),
            Some(SupportLang::Rust)
        );
    }

    #[test]
    fn parse_source_produces_valid_tree() {
        let tree = parse_source("fn hello() {}", SupportLang::Rust);
        assert_eq!(tree.root().kind().as_ref(), "source_file");
    }

    #[test]
    fn parse_markdown_source_produces_document_root() {
        let tree = parse_markdown_source("# Title\n\nText\n");
        assert_eq!(tree.root().kind().as_ref(), "document");
    }

    #[test]
    fn parse_toml_source_produces_document_root() {
        let tree = parse_toml_source("title = \"Zen\"\n");
        assert_eq!(tree.root().kind().as_ref(), "document");
    }

    #[test]
    fn parse_rst_source_produces_document_root() {
        let tree = parse_rst_source("Title\n=====\n\nText\n");
        assert_eq!(tree.root().kind().as_ref(), "document");
    }

    #[test]
    fn parse_svelte_source_produces_document_root() {
        let tree = parse_svelte_source("<script>let n = 1;</script><h1>{n}</h1>");
        assert_eq!(tree.root().kind().as_ref(), "document");
    }
}
