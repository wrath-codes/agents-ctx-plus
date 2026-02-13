//! ast-grep wrapper and language detection from file extensions.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;

/// The concrete AST tree type returned by `parse_source`.
pub type AstTree = ast_grep_core::AstGrep<StrDoc<SupportLang>>;

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

/// Parse source code into an ast-grep tree for the given language.
#[must_use]
pub fn parse_source(source: &str, lang: SupportLang) -> AstTree {
    use ast_grep_language::LanguageExt;
    lang.ast_grep(source)
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
    fn detect_unknown_returns_none() {
        assert_eq!(detect_language("data.csv"), None);
        assert_eq!(detect_language("readme"), None);
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
}
