// Package parser provides tree-sitter based source code parsing and
// public API extraction. It supports 11 languages and produces compressed
// API indices suitable for feeding to LLMs.
package parser

import (
	"fmt"
	"path/filepath"
	"strings"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"

	tree_sitter_svelte "github.com/tree-sitter-grammars/tree-sitter-svelte/bindings/go"
	tree_sitter_toml "github.com/tree-sitter-grammars/tree-sitter-toml/bindings/go"
	tree_sitter_zig "github.com/tree-sitter-grammars/tree-sitter-zig/bindings/go"
	tree_sitter_css "github.com/tree-sitter/tree-sitter-css/bindings/go"
	tree_sitter_elixir "github.com/tree-sitter/tree-sitter-elixir/bindings/go"
	tree_sitter_go "github.com/tree-sitter/tree-sitter-go/bindings/go"
	tree_sitter_javascript "github.com/tree-sitter/tree-sitter-javascript/bindings/go"
	tree_sitter_json "github.com/tree-sitter/tree-sitter-json/bindings/go"
	tree_sitter_python "github.com/tree-sitter/tree-sitter-python/bindings/go"
	tree_sitter_rust "github.com/tree-sitter/tree-sitter-rust/bindings/go"
	tree_sitter_typescript "github.com/tree-sitter/tree-sitter-typescript/bindings/go"

	// Local bindings (regenerated with tree-sitter CLI v0.26.5 for ABI 15)
	tree_sitter_astro "github.com/your-org/beads-workflow-system/internal/parser/grammars/astro"
	tree_sitter_gleam "github.com/your-org/beads-workflow-system/internal/parser/grammars/gleam"
	tree_sitter_markdown "github.com/your-org/beads-workflow-system/internal/parser/grammars/markdown"
	tree_sitter_mojo "github.com/your-org/beads-workflow-system/internal/parser/grammars/mojo"
)

// Language identifies a supported programming language.
type Language string

const (
	LangGo         Language = "go"
	LangRust       Language = "rust"
	LangJavaScript Language = "javascript"
	LangTypeScript Language = "typescript"
	LangTSX        Language = "tsx"
	LangPython     Language = "python"
	LangElixir     Language = "elixir"
	LangZig        Language = "zig"
	LangJSON       Language = "json"
	LangCSS        Language = "css"
	LangTOML       Language = "toml"
	LangSvelte     Language = "svelte"
	LangAstro      Language = "astro"
	LangGleam      Language = "gleam"
	LangMarkdown   Language = "markdown"
	LangMojo       Language = "mojo"
)

// NOTE: The following languages were requested but are not yet supported:
//   - Gleam: tree-sitter-gleam Go bindings have broken CGo include paths
//   - Astro: tree-sitter-astro (ABI 14) incompatible with go-tree-sitter v0.25 (ABI 15)
//   - Mojo: tree-sitter-mojo module path conflicts with tree-sitter-python + ABI mismatch
//   - Markdown: tree-sitter-markdown has no Go bindings (only Node/Python/Rust/Swift)
//   - Tailwind: no tree-sitter parser exists at all
//
// Astro/Svelte files with embedded JS/TS can be partially parsed.
// When these grammars are regenerated for ABI 15, they can be added.

// languageEntry maps a Language to its tree-sitter grammar.
type languageEntry struct {
	tsLang     *tree_sitter.Language
	extensions []string // file extensions (without dot)
}

var languages = map[Language]languageEntry{
	LangGo:         {tree_sitter.NewLanguage(tree_sitter_go.Language()), []string{"go"}},
	LangRust:       {tree_sitter.NewLanguage(tree_sitter_rust.Language()), []string{"rs"}},
	LangJavaScript: {tree_sitter.NewLanguage(tree_sitter_javascript.Language()), []string{"js", "mjs", "cjs"}},
	LangTypeScript: {tree_sitter.NewLanguage(tree_sitter_typescript.LanguageTypescript()), []string{"ts"}},
	LangTSX:        {tree_sitter.NewLanguage(tree_sitter_typescript.LanguageTSX()), []string{"tsx"}},
	LangPython:     {tree_sitter.NewLanguage(tree_sitter_python.Language()), []string{"py"}},
	LangElixir:     {tree_sitter.NewLanguage(tree_sitter_elixir.Language()), []string{"ex", "exs"}},
	LangZig:        {tree_sitter.NewLanguage(tree_sitter_zig.Language()), []string{"zig"}},
	LangJSON:       {tree_sitter.NewLanguage(tree_sitter_json.Language()), []string{"json"}},
	LangCSS:        {tree_sitter.NewLanguage(tree_sitter_css.Language()), []string{"css"}},
	LangTOML:       {tree_sitter.NewLanguage(tree_sitter_toml.Language()), []string{"toml"}},
	LangSvelte:     {tree_sitter.NewLanguage(tree_sitter_svelte.Language()), []string{"svelte"}},
	LangAstro:      {tree_sitter.NewLanguage(tree_sitter_astro.Language()), []string{"astro"}},
	LangGleam:      {tree_sitter.NewLanguage(tree_sitter_gleam.Language()), []string{"gleam"}},
	LangMarkdown:   {tree_sitter.NewLanguage(tree_sitter_markdown.Language()), []string{"md", "markdown"}},
	LangMojo:       {tree_sitter.NewLanguage(tree_sitter_mojo.Language()), []string{"mojo", "🔥"}},
}

// DetectLanguage returns the Language for a file path based on extension.
// Returns ("", false) if unrecognised.
func DetectLanguage(path string) (Language, bool) {
	ext := strings.TrimPrefix(filepath.Ext(path), ".")
	ext = strings.ToLower(ext)
	for lang, entry := range languages {
		for _, e := range entry.extensions {
			if e == ext {
				return lang, true
			}
		}
	}
	return "", false
}

// SupportedLanguages returns all supported languages.
func SupportedLanguages() []Language {
	out := make([]Language, 0, len(languages))
	for lang := range languages {
		out = append(out, lang)
	}
	return out
}

// IsTestFile returns true if the filename matches common test file conventions
// across all supported languages.
func IsTestFile(name string) bool {
	lower := strings.ToLower(name)

	// Go: *_test.go
	if strings.HasSuffix(lower, "_test.go") {
		return true
	}
	// Rust: tests are usually in tests/ dir (handled at dir level),
	// but files named *_test.rs or *_tests.rs are tests too
	if strings.HasSuffix(lower, "_test.rs") || strings.HasSuffix(lower, "_tests.rs") {
		return true
	}
	// JS/TS: *.test.{js,ts,jsx,tsx}, *.spec.{js,ts,jsx,tsx}, *_test.{js,ts}
	for _, ext := range []string{".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs"} {
		base := strings.TrimSuffix(lower, ext)
		if base != lower {
			if strings.HasSuffix(base, ".test") || strings.HasSuffix(base, ".spec") || strings.HasSuffix(base, "_test") {
				return true
			}
		}
	}
	// Python: test_*.py, *_test.py
	if strings.HasSuffix(lower, ".py") {
		base := strings.TrimSuffix(lower, ".py")
		if strings.HasPrefix(base, "test_") || strings.HasSuffix(base, "_test") {
			return true
		}
	}
	// Elixir: *_test.exs
	if strings.HasSuffix(lower, "_test.exs") {
		return true
	}
	// Gleam: *_test.gleam
	if strings.HasSuffix(lower, "_test.gleam") {
		return true
	}
	// Zig: test_*.zig, *_test.zig
	if strings.HasSuffix(lower, ".zig") {
		base := strings.TrimSuffix(lower, ".zig")
		if strings.HasPrefix(base, "test_") || strings.HasSuffix(base, "_test") {
			return true
		}
	}
	// Mojo: test_*.mojo, *_test.mojo
	if strings.HasSuffix(lower, ".mojo") {
		base := strings.TrimSuffix(lower, ".mojo")
		if strings.HasPrefix(base, "test_") || strings.HasSuffix(base, "_test") {
			return true
		}
	}
	// Svelte/Astro: *.test.svelte, *.test.astro (vitest convention)
	if strings.HasSuffix(lower, ".test.svelte") || strings.HasSuffix(lower, ".test.astro") {
		return true
	}
	return false
}

// IsTestDir returns true if the directory name matches common test directory
// conventions across languages.
func IsTestDir(name string) bool {
	switch strings.ToLower(name) {
	case "test", "tests", "spec", "specs",
		"__tests__", "__mocks__", "__snapshots__",
		"test_helpers", "testing", "testdata", "testutil", "testutils",
		"fixtures", "e2e", "integration_tests", "unit_tests",
		"benches", "benchmarks", "examples":
		return true
	}
	return false
}

// Parser wraps a tree-sitter parser with language awareness.
type Parser struct {
	inner *tree_sitter.Parser
}

// New creates a new Parser.
func New() *Parser {
	return &Parser{inner: tree_sitter.NewParser()}
}

// Close releases resources.
func (p *Parser) Close() {
	p.inner.Close()
}

// Parse parses source code in the given language and returns the tree.
func (p *Parser) Parse(source []byte, lang Language) (*tree_sitter.Tree, error) {
	entry, ok := languages[lang]
	if !ok {
		return nil, fmt.Errorf("unsupported language: %s", lang)
	}
	if err := p.inner.SetLanguage(entry.tsLang); err != nil {
		return nil, fmt.Errorf("failed to set language %s: %w", lang, err)
	}
	tree := p.inner.Parse(source, nil)
	if tree == nil {
		return nil, fmt.Errorf("tree-sitter returned nil tree for language %s", lang)
	}
	return tree, nil
}

// ParseFile detects the language from the file path and parses the source.
func (p *Parser) ParseFile(source []byte, path string) (*tree_sitter.Tree, Language, error) {
	lang, ok := DetectLanguage(path)
	if !ok {
		return nil, "", fmt.Errorf("could not detect language for %s", path)
	}
	tree, err := p.Parse(source, lang)
	return tree, lang, err
}
