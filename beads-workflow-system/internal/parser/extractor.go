package parser

import (
	"fmt"
	"strings"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
)

// Symbol represents an extracted public API symbol.
type Symbol struct {
	Kind      string `json:"kind"` // "function", "type", "struct", "trait", "module", "class", "const", "interface", "enum"
	Name      string `json:"name"`
	Signature string `json:"signature"` // full signature line (e.g. "func Foo(x int) error")
	DocString string `json:"doc,omitempty"`
	Line      uint   `json:"line"`
}

// FileAPI represents the extracted public API of a single file.
type FileAPI struct {
	Path     string   `json:"path"`
	Language Language `json:"language"`
	Symbols  []Symbol `json:"symbols"`
}

// ExtractAPI parses source and extracts public symbols.
func ExtractAPI(source []byte, lang Language) (*FileAPI, error) {
	p := New()
	defer p.Close()

	tree, err := p.Parse(source, lang)
	if err != nil {
		return nil, err
	}
	defer tree.Close()

	api := &FileAPI{Language: lang}
	root := tree.RootNode()

	switch lang {
	case LangGo:
		extractGo(root, source, api)
	case LangRust:
		extractRust(root, source, api)
	case LangPython:
		extractPython(root, source, api)
	case LangJavaScript, LangTypeScript, LangTSX:
		extractJSTS(root, source, api)
	case LangElixir:
		extractElixir(root, source, api)
	case LangZig:
		extractZig(root, source, api)
	case LangSvelte:
		extractSvelte(root, source, api)
	case LangGleam:
		extractGleam(root, source, api)
	case LangMojo:
		extractMojo(root, source, api)
	case LangAstro:
		extractAstro(root, source, api)
	default:
		// For non-code languages (JSON, CSS, TOML, Markdown), just return empty.
		return api, nil
	}

	return api, nil
}

// --- Go extractor ---

func extractGo(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		switch kind {
		case "function_declaration":
			name := childByField(node, "name", source)
			if isExported(name) {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "function",
					Name:      name,
					Signature: nodeText(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "method_declaration":
			name := childByField(node, "name", source)
			if isExported(name) {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "method",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "type_declaration":
			// Walk type specs inside
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				if child.Kind() == "type_spec" {
					name := childByField(child, "name", source)
					if isExported(name) {
						typeKind := "type"
						typeNode := childByField2(child, "type")
						if typeNode != nil {
							switch typeNode.Kind() {
							case "struct_type":
								typeKind = "struct"
							case "interface_type":
								typeKind = "interface"
							}
						}
						api.Symbols = append(api.Symbols, Symbol{
							Kind:      typeKind,
							Name:      name,
							Signature: signatureLine(child, source),
							DocString: precedingComment(node, source),
							Line:      uint(node.StartPosition().Row + 1),
						})
					}
				}
			}

		case "const_declaration", "var_declaration":
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				if child.Kind() == "const_spec" || child.Kind() == "var_spec" {
					name := childByField(child, "name", source)
					if isExported(name) {
						api.Symbols = append(api.Symbols, Symbol{
							Kind:      strings.TrimSuffix(kind, "_declaration"),
							Name:      name,
							Signature: signatureLine(child, source),
							Line:      uint(child.StartPosition().Row + 1),
						})
					}
				}
			}
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// --- Rust extractor ---

func extractRust(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		isPub := false
		for i := uint(0); i < node.ChildCount(); i++ {
			if node.Child(i).Kind() == "visibility_modifier" {
				isPub = true
				break
			}
		}

		if isPub {
			switch kind {
			case "function_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "function",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "struct_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "struct",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "enum_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "enum",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "trait_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "trait",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "type_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "type",
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "const_item", "static_item":
				name := childByField(node, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "const",
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// --- Python extractor ---

func extractPython(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		switch kind {
		case "function_definition":
			name := childByField(node, "name", source)
			if !strings.HasPrefix(name, "_") {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "function",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: extractDocstring(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "class_definition":
			name := childByField(node, "name", source)
			if !strings.HasPrefix(name, "_") {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "class",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: extractDocstring(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "decorated_definition":
			// Look inside for the actual function/class
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				if child.Kind() == "function_definition" || child.Kind() == "class_definition" {
					name := childByField(child, "name", source)
					if !strings.HasPrefix(name, "_") {
						api.Symbols = append(api.Symbols, Symbol{
							Kind:      strings.TrimSuffix(child.Kind(), "_definition"),
							Name:      name,
							Signature: signatureLine(child, source),
							DocString: extractDocstring(child, source),
							Line:      uint(child.StartPosition().Row + 1),
						})
					}
				}
			}
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// --- JavaScript / TypeScript extractor ---

func extractJSTS(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		switch kind {
		case "export_statement":
			// Walk children of export for the actual declaration
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				extractJSTSDeclaration(child, source, api)
			}

		case "function_declaration", "class_declaration", "lexical_declaration",
			"variable_declaration", "interface_declaration", "type_alias_declaration",
			"enum_declaration":
			extractJSTSDeclaration(node, source, api)
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

func extractJSTSDeclaration(node *tree_sitter.Node, source []byte, api *FileAPI) {
	kind := node.Kind()
	switch kind {
	case "function_declaration":
		name := childByField(node, "name", source)
		api.Symbols = append(api.Symbols, Symbol{
			Kind:      "function",
			Name:      name,
			Signature: signatureLine(node, source),
			Line:      uint(node.StartPosition().Row + 1),
		})

	case "class_declaration":
		name := childByField(node, "name", source)
		api.Symbols = append(api.Symbols, Symbol{
			Kind:      "class",
			Name:      name,
			Signature: signatureLine(node, source),
			Line:      uint(node.StartPosition().Row + 1),
		})

	case "interface_declaration":
		name := childByField(node, "name", source)
		api.Symbols = append(api.Symbols, Symbol{
			Kind:      "interface",
			Name:      name,
			Signature: signatureLine(node, source),
			Line:      uint(node.StartPosition().Row + 1),
		})

	case "type_alias_declaration":
		name := childByField(node, "name", source)
		api.Symbols = append(api.Symbols, Symbol{
			Kind:      "type",
			Name:      name,
			Signature: signatureLine(node, source),
			Line:      uint(node.StartPosition().Row + 1),
		})

	case "enum_declaration":
		name := childByField(node, "name", source)
		api.Symbols = append(api.Symbols, Symbol{
			Kind:      "enum",
			Name:      name,
			Signature: signatureLine(node, source),
			Line:      uint(node.StartPosition().Row + 1),
		})

	case "lexical_declaration", "variable_declaration":
		// const/let/var -- extract the declarator names
		for i := uint(0); i < node.ChildCount(); i++ {
			child := node.Child(i)
			if child.Kind() == "variable_declarator" {
				name := childByField(child, "name", source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "const",
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}
		}
	}
}

// --- Elixir extractor ---

func extractElixir(root *tree_sitter.Node, source []byte, api *FileAPI) {
	walkAll(root, func(node *tree_sitter.Node) {
		kind := node.Kind()
		if kind == "call" {
			fnName := ""
			if node.ChildCount() > 0 {
				fnName = nodeText(node.Child(0), source)
			}
			switch fnName {
			case "def", "defp", "defmacro", "defmodule":
				symKind := "function"
				if fnName == "defmodule" {
					symKind = "module"
				}
				// Second child is usually the function name/signature
				name := fnName
				if node.ChildCount() > 1 {
					args := node.Child(1)
					name = signatureLine(args, source)
				}
				if fnName != "defp" { // defp is private
					api.Symbols = append(api.Symbols, Symbol{
						Kind:      symKind,
						Name:      name,
						Signature: signatureLine(node, source),
						DocString: precedingComment(node, source),
						Line:      uint(node.StartPosition().Row + 1),
					})
				}
			}
		}
	})
}

// --- Zig extractor ---

func extractZig(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		if kind == "TopLevelDecl" || kind == "FnProto" || kind == "VarDecl" {
			// Check if pub
			text := nodeText(node, source)
			if strings.HasPrefix(text, "pub ") {
				name := childByField(node, "name", source)
				symKind := "function"
				if strings.Contains(kind, "Var") {
					symKind = "const"
				}
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      symKind,
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// --- Svelte extractor ---
// Svelte files are HTML-like with <script> blocks containing JS/TS.
// We re-parse the script content with the TypeScript grammar for proper extraction.

func extractSvelte(root *tree_sitter.Node, source []byte, api *FileAPI) {
	walkAll(root, func(node *tree_sitter.Node) {
		if node.Kind() == "script_element" {
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				if child.Kind() == "raw_text" {
					lineOffset := child.StartPosition().Row
					extractEmbeddedJSTS(nodeText(child, source), lineOffset, api)
				}
			}
		}
	})
}

// --- Astro extractor ---
// Astro files have a frontmatter block (between --- fences) containing JS/TS.
// We re-parse the frontmatter content with the TypeScript grammar.

func extractAstro(root *tree_sitter.Node, source []byte, api *FileAPI) {
	walkAll(root, func(node *tree_sitter.Node) {
		if node.Kind() == "frontmatter" {
			for i := uint(0); i < node.ChildCount(); i++ {
				child := node.Child(i)
				if child.Kind() == "frontmatter_js_block" {
					lineOffset := child.StartPosition().Row
					extractEmbeddedJSTS(nodeText(child, source), lineOffset, api)
				}
			}
		}
	})
}

// extractEmbeddedJSTS re-parses a JS/TS code block with the TypeScript grammar
// and extracts symbols. lineOffset is added to reported line numbers so they
// map back to the original file.
func extractEmbeddedJSTS(code string, lineOffset uint, api *FileAPI) {
	src := []byte(code)
	p := New()
	defer p.Close()

	tree, err := p.Parse(src, LangTypeScript)
	if err != nil {
		return
	}
	defer tree.Close()

	// Extract into a temporary FileAPI, then adjust line numbers.
	tmp := &FileAPI{Language: LangTypeScript}
	extractJSTS(tree.RootNode(), src, tmp)

	for _, sym := range tmp.Symbols {
		sym.Line += lineOffset
		api.Symbols = append(api.Symbols, sym)
	}
}

// --- Gleam extractor ---
// Gleam has `function`, `type_definition`, and `constant` at the top level.
// Public items have a `visibility_modifier` child with text "pub".

func extractGleam(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		isPub := false
		for i := uint(0); i < node.ChildCount(); i++ {
			if node.Child(i).Kind() == "visibility_modifier" {
				isPub = true
				break
			}
		}

		if isPub {
			switch kind {
			case "function":
				name := gleamChildIdentifier(node, source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "function",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "type_definition":
				// The type name is inside a `type_name` child → `type_identifier`
				name := ""
				for i := uint(0); i < node.ChildCount(); i++ {
					child := node.Child(i)
					if child.Kind() == "type_name" {
						for j := uint(0); j < child.ChildCount(); j++ {
							gc := child.Child(j)
							if gc.Kind() == "type_identifier" {
								name = nodeText(gc, source)
								break
							}
						}
						break
					}
				}
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "type",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: precedingComment(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})

			case "constant":
				name := gleamChildIdentifier(node, source)
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "const",
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// gleamChildIdentifier finds the first `identifier` child of a node.
func gleamChildIdentifier(node *tree_sitter.Node, source []byte) string {
	for i := uint(0); i < node.ChildCount(); i++ {
		child := node.Child(i)
		if child.Kind() == "identifier" {
			return nodeText(child, source)
		}
	}
	return ""
}

// --- Mojo extractor ---
// Mojo uses Python-like AST: `function_definition`, `class_definition` (for struct),
// `trait_definition`. Functions starting with `_` are treated as private.
// `fn` is Mojo-strict, `def` is Python-compatible; both appear as `function_definition`.
// `struct` appears as `class_definition` with first child "struct".

func extractMojo(root *tree_sitter.Node, source []byte, api *FileAPI) {
	cursor := root.Walk()
	defer cursor.Close()

	if !cursor.GotoFirstChild() {
		return
	}
	for {
		node := cursor.Node()
		kind := node.Kind()

		switch kind {
		case "function_definition":
			name := childByField(node, "name", source)
			if name == "" {
				// Fallback: find first identifier child
				for i := uint(0); i < node.ChildCount(); i++ {
					if node.Child(i).Kind() == "identifier" {
						name = nodeText(node.Child(i), source)
						break
					}
				}
			}
			if !strings.HasPrefix(name, "_") {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      "function",
					Name:      name,
					Signature: signatureLine(node, source),
					DocString: extractMojoDocstring(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "class_definition":
			// Mojo struct appears as class_definition with first child "struct"
			name := childByField(node, "name", source)
			if name == "" {
				for i := uint(0); i < node.ChildCount(); i++ {
					if node.Child(i).Kind() == "identifier" {
						name = nodeText(node.Child(i), source)
						break
					}
				}
			}
			symKind := "class"
			if node.ChildCount() > 0 && node.Child(0).Kind() == "struct" {
				symKind = "struct"
			}
			if !strings.HasPrefix(name, "_") {
				api.Symbols = append(api.Symbols, Symbol{
					Kind:      symKind,
					Name:      name,
					Signature: signatureLine(node, source),
					Line:      uint(node.StartPosition().Row + 1),
				})
			}

		case "trait_definition":
			name := childByField(node, "name", source)
			if name == "" {
				for i := uint(0); i < node.ChildCount(); i++ {
					if node.Child(i).Kind() == "identifier" {
						name = nodeText(node.Child(i), source)
						break
					}
				}
			}
			api.Symbols = append(api.Symbols, Symbol{
				Kind:      "trait",
				Name:      name,
				Signature: signatureLine(node, source),
				Line:      uint(node.StartPosition().Row + 1),
			})
		}

		if !cursor.GotoNextSibling() {
			break
		}
	}
}

// extractMojoDocstring extracts a Mojo/Python docstring from a function body.
func extractMojoDocstring(node *tree_sitter.Node, source []byte) string {
	// Look for a `block` child containing a string as first statement
	for i := uint(0); i < node.ChildCount(); i++ {
		child := node.Child(i)
		if child.Kind() == "block" && child.ChildCount() > 0 {
			first := child.Child(0)
			if first.Kind() == "string" {
				text := nodeText(first, source)
				// Strip triple quotes
				text = strings.TrimPrefix(text, `"""`)
				text = strings.TrimSuffix(text, `"""`)
				text = strings.TrimPrefix(text, `'''`)
				text = strings.TrimSuffix(text, `'''`)
				return strings.TrimSpace(text)
			}
			// Also check expression_statement wrapping a string
			if first.Kind() == "expression_statement" && first.ChildCount() > 0 {
				expr := first.Child(0)
				if expr.Kind() == "string" || expr.Kind() == "concatenated_string" {
					text := nodeText(expr, source)
					text = strings.Trim(text, "\"'")
					return strings.TrimSpace(text)
				}
			}
		}
	}
	return ""
}

// --- Helpers ---

// childByField returns the text of the first child with the given field name.
func childByField(node *tree_sitter.Node, field string, source []byte) string {
	child := node.ChildByFieldName(field)
	if child == nil {
		return ""
	}
	return nodeText(child, source)
}

// childByField2 returns the node of the first child with the given field name.
func childByField2(node *tree_sitter.Node, field string) *tree_sitter.Node {
	return node.ChildByFieldName(field)
}

// nodeText returns the source text of a node.
func nodeText(node *tree_sitter.Node, source []byte) string {
	start := node.StartByte()
	end := node.EndByte()
	if int(end) > len(source) {
		end = uint(len(source))
	}
	return string(source[start:end])
}

// signatureLine returns just the first line of a node's text (the signature).
func signatureLine(node *tree_sitter.Node, source []byte) string {
	text := nodeText(node, source)
	if idx := strings.Index(text, "\n"); idx > 0 {
		return strings.TrimSpace(text[:idx])
	}
	return strings.TrimSpace(text)
}

// precedingComment returns the comment text immediately before a node, if any.
func precedingComment(node *tree_sitter.Node, source []byte) string {
	prev := node.PrevSibling()
	if prev == nil {
		return ""
	}
	if prev.Kind() == "comment" || prev.Kind() == "line_comment" || prev.Kind() == "doc_comment" {
		return strings.TrimSpace(nodeText(prev, source))
	}
	return ""
}

// extractDocstring extracts a Python docstring from a function/class body.
func extractDocstring(node *tree_sitter.Node, source []byte) string {
	body := node.ChildByFieldName("body")
	if body == nil || body.ChildCount() == 0 {
		return ""
	}
	first := body.Child(0)
	if first.Kind() == "expression_statement" && first.ChildCount() > 0 {
		expr := first.Child(0)
		if expr.Kind() == "string" || expr.Kind() == "concatenated_string" {
			text := nodeText(expr, source)
			// Strip triple quotes
			text = strings.Trim(text, "\"'")
			return strings.TrimSpace(text)
		}
	}
	return ""
}

// isExported checks if a Go identifier is exported (starts with uppercase).
func isExported(name string) bool {
	if name == "" {
		return false
	}
	return name[0] >= 'A' && name[0] <= 'Z'
}

// walkAll walks all nodes in the tree depth-first and calls fn for each.
func walkAll(root *tree_sitter.Node, fn func(*tree_sitter.Node)) {
	cursor := root.Walk()
	defer cursor.Close()

	var walk func()
	walk = func() {
		fn(cursor.Node())
		if cursor.GotoFirstChild() {
			for {
				walk()
				if !cursor.GotoNextSibling() {
					break
				}
			}
			cursor.GotoParent()
		}
	}
	walk()
}

// FormatAPIIndex formats a FileAPI into a compressed text index suitable
// for LLM consumption. Follows AGENTS.md style: signatures only, no bodies.
func FormatAPIIndex(apis []*FileAPI) string {
	var b strings.Builder
	for _, api := range apis {
		if len(api.Symbols) == 0 {
			continue
		}
		if api.Path != "" {
			fmt.Fprintf(&b, "--- %s (%s) ---\n", api.Path, api.Language)
		} else {
			fmt.Fprintf(&b, "--- %s ---\n", api.Language)
		}
		for _, sym := range api.Symbols {
			if sym.DocString != "" {
				fmt.Fprintf(&b, "  // %s\n", truncate(sym.DocString, 120))
			}
			fmt.Fprintf(&b, "  [%s] %s\n", sym.Kind, sym.Signature)
		}
		b.WriteByte('\n')
	}
	return b.String()
}

func truncate(s string, maxLen int) string {
	// Collapse to single line first
	s = strings.ReplaceAll(s, "\n", " ")
	if len(s) > maxLen {
		return s[:maxLen-3] + "..."
	}
	return s
}
