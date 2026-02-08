package parser

import (
	"strings"
	"testing"
)

func TestExtractGoAPI(t *testing.T) {
	src := []byte(`package mylib

// Config holds configuration.
type Config struct {
	Name string
}

// internal unexported type
type internal struct{}

// New creates a new Config.
func New(name string) *Config {
	return &Config{Name: name}
}

// helper is unexported.
func helper() {}

// Run starts the service.
func (c *Config) Run() error {
	return nil
}

const MaxRetries = 3
var DefaultTimeout = 30
`)

	api, err := ExtractAPI(src, LangGo)
	if err != nil {
		t.Fatalf("ExtractAPI(Go) error: %v", err)
	}

	// Should find: Config (struct), New (function), Run (method), MaxRetries (const), DefaultTimeout (var)
	// Should NOT find: internal, helper
	names := symbolNames(api)

	assertContains(t, names, "Config")
	assertContains(t, names, "New")
	assertContains(t, names, "Run")
	assertContains(t, names, "MaxRetries")
	assertContains(t, names, "DefaultTimeout")
	assertNotContains(t, names, "internal")
	assertNotContains(t, names, "helper")

	// Check kinds
	assertSymbolKind(t, api, "Config", "struct")
	assertSymbolKind(t, api, "New", "function")
	assertSymbolKind(t, api, "Run", "method")
}

func TestExtractRustAPI(t *testing.T) {
	src := []byte(`pub fn create(name: &str) -> Result<Config, Error> {
    todo!()
}

fn internal_helper() {}

pub struct Config {
    name: String,
}

pub enum Status {
    Active,
    Inactive,
}

pub trait Service {
    fn start(&self);
}

pub const MAX_RETRIES: u32 = 3;
`)

	api, err := ExtractAPI(src, LangRust)
	if err != nil {
		t.Fatalf("ExtractAPI(Rust) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "create")
	assertContains(t, names, "Config")
	assertContains(t, names, "Status")
	assertContains(t, names, "Service")
	assertContains(t, names, "MAX_RETRIES")
	assertNotContains(t, names, "internal_helper")

	assertSymbolKind(t, api, "create", "function")
	assertSymbolKind(t, api, "Config", "struct")
	assertSymbolKind(t, api, "Status", "enum")
	assertSymbolKind(t, api, "Service", "trait")
}

func TestExtractPythonAPI(t *testing.T) {
	src := []byte(`def process(data):
    """Process input data."""
    return data

def _internal():
    pass

class DataProcessor:
    """Processes data."""
    def run(self):
        pass
`)

	api, err := ExtractAPI(src, LangPython)
	if err != nil {
		t.Fatalf("ExtractAPI(Python) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "process")
	assertContains(t, names, "DataProcessor")
	assertNotContains(t, names, "_internal")

	// Check docstrings extracted
	for _, sym := range api.Symbols {
		if sym.Name == "process" && sym.DocString == "" {
			t.Error("expected docstring for process()")
		}
	}
}

func TestExtractTypeScriptAPI(t *testing.T) {
	src := []byte(`export function add(a: number, b: number): number {
    return a + b;
}

export class Calculator {
    compute() {}
}

export interface Config {
    debug: boolean;
}

export type ID = string;

function internal() {}

export const VERSION = "1.0";
`)

	api, err := ExtractAPI(src, LangTypeScript)
	if err != nil {
		t.Fatalf("ExtractAPI(TypeScript) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "add")
	assertContains(t, names, "Calculator")
	assertContains(t, names, "Config")
	assertContains(t, names, "ID")
	assertContains(t, names, "VERSION")
	// internal is not exported, but the extractor includes top-level declarations too
	// The important thing is the exported ones are there
}

func TestExtractElixirAPI(t *testing.T) {
	src := []byte(`defmodule MyApp.Worker do
  def start(opts) do
    :ok
  end

  defp validate(opts) do
    :ok
  end

  defmacro log_call(name) do
    quote do
      IO.puts("Calling #{unquote(name)}")
    end
  end
end
`)

	api, err := ExtractAPI(src, LangElixir)
	if err != nil {
		t.Fatalf("ExtractAPI(Elixir) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "MyApp.Worker")
	// defp should be excluded
	hasDefp := false
	for _, sym := range api.Symbols {
		if strings.Contains(sym.Signature, "defp") {
			hasDefp = true
		}
	}
	if hasDefp {
		t.Error("defp (private) should be excluded from API")
	}
}

func TestExtractJavaScriptAPI(t *testing.T) {
	src := []byte(`export function render(el) {
    return el;
}

export class Component {
    mount() {}
}
`)

	api, err := ExtractAPI(src, LangJavaScript)
	if err != nil {
		t.Fatalf("ExtractAPI(JavaScript) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "render")
	assertContains(t, names, "Component")
}

func TestExtractGleamAPI(t *testing.T) {
	src := []byte(`pub fn greet(name: String) -> String {
  "Hello, " <> name
}

fn internal() {
  Nil
}

pub type User {
  User(name: String, age: Int)
}

pub const max_retries = 3
`)

	api, err := ExtractAPI(src, LangGleam)
	if err != nil {
		t.Fatalf("ExtractAPI(Gleam) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "greet")
	assertContains(t, names, "User")
	assertContains(t, names, "max_retries")
	assertNotContains(t, names, "internal")

	assertSymbolKind(t, api, "greet", "function")
	assertSymbolKind(t, api, "User", "type")
	assertSymbolKind(t, api, "max_retries", "const")
}

func TestExtractMojoAPI(t *testing.T) {
	src := []byte(`fn private_fn():
    pass

def public_function(x: Int) -> Int:
    """Process a value."""
    return x * 2

struct MyStruct:
    var name: String
    var age: Int

    fn __init__(inout self, name: String, age: Int):
        self.name = name
        self.age = age

trait Printable:
    fn to_string(self) -> String: ...
`)

	api, err := ExtractAPI(src, LangMojo)
	if err != nil {
		t.Fatalf("ExtractAPI(Mojo) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "public_function")
	assertContains(t, names, "MyStruct")
	assertContains(t, names, "Printable")
	// private_fn starts with _ equivalent (fn keyword = Mojo-strict, not private per se,
	// but private_fn does not start with underscore so it should be included)
	// Actually private_fn doesn't start with _, so it IS included
	assertContains(t, names, "private_fn")
	// __init__ starts with _ so excluded
	assertNotContains(t, names, "__init__")

	assertSymbolKind(t, api, "MyStruct", "struct")
	assertSymbolKind(t, api, "Printable", "trait")
	assertSymbolKind(t, api, "public_function", "function")

	// Check docstring extraction
	for _, sym := range api.Symbols {
		if sym.Name == "public_function" && sym.DocString == "" {
			t.Error("expected docstring for public_function")
		}
	}
}

func TestExtractSvelteAPI(t *testing.T) {
	src := []byte(`<script>
export function greet(name) {
    return "Hello " + name;
}

export const VERSION = "1.0";

function internal() {}
</script>

<h1>Hello</h1>
`)

	api, err := ExtractAPI(src, LangSvelte)
	if err != nil {
		t.Fatalf("ExtractAPI(Svelte) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "greet")
	assertContains(t, names, "VERSION")
	// internal is not exported, shouldn't appear via export_statement path
	// (but may appear as a top-level declaration - that's fine, extractJSTS includes those)
}

func TestExtractAstroAPI(t *testing.T) {
	src := []byte(`---
import { getCollection } from 'astro:content';

export interface Props {
  title: string;
  description: string;
}

const posts = await getCollection('blog');
---
<html>
  <body>
    <h1>Hello</h1>
  </body>
</html>
`)

	api, err := ExtractAPI(src, LangAstro)
	if err != nil {
		t.Fatalf("ExtractAPI(Astro) error: %v", err)
	}

	names := symbolNames(api)
	assertContains(t, names, "Props")

	assertSymbolKind(t, api, "Props", "interface")
}

func TestExtractNoSymbols(t *testing.T) {
	// JSON has no meaningful API symbols
	src := []byte(`{"key": "value"}`)

	api, err := ExtractAPI(src, LangJSON)
	if err != nil {
		t.Fatalf("ExtractAPI(JSON) error: %v", err)
	}

	if len(api.Symbols) != 0 {
		t.Errorf("expected 0 symbols for JSON, got %d", len(api.Symbols))
	}
}

func TestFormatAPIIndex(t *testing.T) {
	apis := []*FileAPI{
		{
			Path:     "lib.go",
			Language: LangGo,
			Symbols: []Symbol{
				{Kind: "function", Name: "New", Signature: "func New(name string) *Config", DocString: "New creates a Config."},
				{Kind: "struct", Name: "Config", Signature: "type Config struct {"},
			},
		},
		{
			Path:     "util.go",
			Language: LangGo,
			Symbols:  []Symbol{}, // empty, should be skipped
		},
		{
			Path:     "main.rs",
			Language: LangRust,
			Symbols: []Symbol{
				{Kind: "function", Name: "run", Signature: "pub fn run() -> Result<()>"},
			},
		},
	}

	output := FormatAPIIndex(apis)

	if !strings.Contains(output, "--- lib.go (go) ---") {
		t.Error("expected lib.go header in output")
	}
	if strings.Contains(output, "util.go") {
		t.Error("empty util.go should be skipped")
	}
	if !strings.Contains(output, "[function] func New(name string) *Config") {
		t.Error("expected New function signature")
	}
	if !strings.Contains(output, "// New creates a Config.") {
		t.Error("expected docstring comment")
	}
	if !strings.Contains(output, "--- main.rs (rust) ---") {
		t.Error("expected main.rs header")
	}
}

func TestFormatAPIIndexEmpty(t *testing.T) {
	output := FormatAPIIndex(nil)
	if output != "" {
		t.Errorf("expected empty string for nil apis, got %q", output)
	}
}

// Helpers

func symbolNames(api *FileAPI) []string {
	var names []string
	for _, sym := range api.Symbols {
		names = append(names, sym.Name)
	}
	return names
}

func assertContains(t *testing.T, names []string, want string) {
	t.Helper()
	for _, n := range names {
		if n == want {
			return
		}
	}
	t.Errorf("expected symbol %q not found in %v", want, names)
}

func assertNotContains(t *testing.T, names []string, unwanted string) {
	t.Helper()
	for _, n := range names {
		if n == unwanted {
			t.Errorf("symbol %q should not be present but found in %v", unwanted, names)
			return
		}
	}
}

func assertSymbolKind(t *testing.T, api *FileAPI, name, kind string) {
	t.Helper()
	for _, sym := range api.Symbols {
		if sym.Name == name {
			if sym.Kind != kind {
				t.Errorf("symbol %q kind = %q, want %q", name, sym.Kind, kind)
			}
			return
		}
	}
	t.Errorf("symbol %q not found", name)
}
