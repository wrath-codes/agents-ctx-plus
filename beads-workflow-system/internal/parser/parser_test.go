package parser

import (
	"testing"
)

func TestDetectLanguage(t *testing.T) {
	tests := []struct {
		path string
		want Language
		ok   bool
	}{
		{"main.go", LangGo, true},
		{"lib.rs", LangRust, true},
		{"index.js", LangJavaScript, true},
		{"app.mjs", LangJavaScript, true},
		{"util.ts", LangTypeScript, true},
		{"component.tsx", LangTSX, true},
		{"script.py", LangPython, true},
		{"mix.exs", LangElixir, true},
		{"build.zig", LangZig, true},
		{"data.json", LangJSON, true},
		{"style.css", LangCSS, true},
		{"config.toml", LangTOML, true},
		{"App.svelte", LangSvelte, true},
		{"Page.astro", LangAstro, true},
		{"main.gleam", LangGleam, true},
		{"README.md", LangMarkdown, true},
		{"hello.mojo", LangMojo, true},
		// Unrecognised
		{"Makefile", "", false},
		{"foo.txt", "", false},
		{"", "", false},
	}

	for _, tt := range tests {
		got, ok := DetectLanguage(tt.path)
		if ok != tt.ok {
			t.Errorf("DetectLanguage(%q) ok = %v, want %v", tt.path, ok, tt.ok)
		}
		if got != tt.want {
			t.Errorf("DetectLanguage(%q) = %q, want %q", tt.path, got, tt.want)
		}
	}
}

func TestSupportedLanguages(t *testing.T) {
	langs := SupportedLanguages()
	if len(langs) != 16 {
		t.Errorf("SupportedLanguages() returned %d, want 16", len(langs))
	}
}

func TestParseGo(t *testing.T) {
	src := []byte(`package main

func main() {
	println("hello")
}
`)
	p := New()
	defer p.Close()

	tree, err := p.Parse(src, LangGo)
	if err != nil {
		t.Fatalf("Parse(Go) error: %v", err)
	}
	defer tree.Close()

	root := tree.RootNode()
	if root == nil {
		t.Fatal("root node is nil")
	}
	if root.Kind() != "source_file" {
		t.Errorf("root kind = %q, want source_file", root.Kind())
	}
}

func TestParseRust(t *testing.T) {
	src := []byte(`pub fn hello() -> String {
    String::from("hello")
}
`)
	p := New()
	defer p.Close()

	tree, err := p.Parse(src, LangRust)
	if err != nil {
		t.Fatalf("Parse(Rust) error: %v", err)
	}
	defer tree.Close()

	if tree.RootNode().Kind() != "source_file" {
		t.Errorf("root kind = %q, want source_file", tree.RootNode().Kind())
	}
}

func TestParsePython(t *testing.T) {
	src := []byte(`def greet(name):
    """Greet someone."""
    print(f"Hello, {name}!")
`)
	p := New()
	defer p.Close()

	tree, err := p.Parse(src, LangPython)
	if err != nil {
		t.Fatalf("Parse(Python) error: %v", err)
	}
	defer tree.Close()

	if tree.RootNode().Kind() != "module" {
		t.Errorf("root kind = %q, want module", tree.RootNode().Kind())
	}
}

func TestParseTypeScript(t *testing.T) {
	src := []byte(`export function add(a: number, b: number): number {
    return a + b;
}
`)
	p := New()
	defer p.Close()

	tree, err := p.Parse(src, LangTypeScript)
	if err != nil {
		t.Fatalf("Parse(TypeScript) error: %v", err)
	}
	defer tree.Close()

	if tree.RootNode().Kind() != "program" {
		t.Errorf("root kind = %q, want program", tree.RootNode().Kind())
	}
}

func TestParseFile(t *testing.T) {
	src := []byte(`package foo

func Bar() {}
`)
	p := New()
	defer p.Close()

	tree, lang, err := p.ParseFile(src, "foo.go")
	if err != nil {
		t.Fatalf("ParseFile error: %v", err)
	}
	defer tree.Close()

	if lang != LangGo {
		t.Errorf("ParseFile lang = %q, want go", lang)
	}
}

func TestParseUnsupportedLanguage(t *testing.T) {
	p := New()
	defer p.Close()

	_, err := p.Parse([]byte("hello"), Language("fortran"))
	if err == nil {
		t.Error("Parse(fortran) should have returned error")
	}
}

func TestParseFileUnknownExtension(t *testing.T) {
	p := New()
	defer p.Close()

	_, _, err := p.ParseFile([]byte("hello"), "foo.txt")
	if err == nil {
		t.Error("ParseFile(foo.txt) should have returned error")
	}
}

func TestIsTestFile(t *testing.T) {
	tests := []struct {
		name string
		want bool
	}{
		// Go
		{"handler_test.go", true},
		{"handler.go", false},
		// Rust
		{"parser_test.rs", true},
		{"parser_tests.rs", true},
		{"parser.rs", false},
		// JS/TS variants
		{"app.test.ts", true},
		{"app.spec.ts", true},
		{"app_test.ts", true},
		{"app.test.js", true},
		{"app.spec.js", true},
		{"app.test.tsx", true},
		{"app.spec.jsx", true},
		{"app.ts", false},
		{"app.js", false},
		// Python
		{"test_handler.py", true},
		{"handler_test.py", true},
		{"handler.py", false},
		// Elixir
		{"router_test.exs", true},
		{"router.ex", false},
		// Gleam
		{"parser_test.gleam", true},
		{"parser.gleam", false},
		// Zig
		{"test_alloc.zig", true},
		{"alloc_test.zig", true},
		{"alloc.zig", false},
		// Mojo
		{"test_math.mojo", true},
		{"math_test.mojo", true},
		{"math.mojo", false},
		// Svelte/Astro
		{"Button.test.svelte", true},
		{"Button.svelte", false},
		{"Page.test.astro", true},
		{"Page.astro", false},
		// Non-test
		{"README.md", false},
		{"config.toml", false},
		{"Makefile", false},
	}
	for _, tt := range tests {
		if got := IsTestFile(tt.name); got != tt.want {
			t.Errorf("IsTestFile(%q) = %v, want %v", tt.name, got, tt.want)
		}
	}
}

func TestIsTestDir(t *testing.T) {
	tests := []struct {
		name string
		want bool
	}{
		{"test", true},
		{"tests", true},
		{"spec", true},
		{"__tests__", true},
		{"__mocks__", true},
		{"testdata", true},
		{"fixtures", true},
		{"e2e", true},
		{"integration_tests", true},
		{"benches", true},
		{"examples", true},
		{"src", false},
		{"lib", false},
		{"internal", false},
		{"pkg", false},
		{"cmd", false},
	}
	for _, tt := range tests {
		if got := IsTestDir(tt.name); got != tt.want {
			t.Errorf("IsTestDir(%q) = %v, want %v", tt.name, got, tt.want)
		}
	}
}

// Test that all 16 languages can create a parser and parse empty source.
func TestAllLanguagesParse(t *testing.T) {
	for lang := range languages {
		t.Run(string(lang), func(t *testing.T) {
			p := New()
			defer p.Close()

			tree, err := p.Parse([]byte(""), lang)
			if err != nil {
				t.Fatalf("Parse(%s) error: %v", lang, err)
			}
			defer tree.Close()

			if tree.RootNode() == nil {
				t.Fatalf("Parse(%s) returned nil root", lang)
			}
		})
	}
}
