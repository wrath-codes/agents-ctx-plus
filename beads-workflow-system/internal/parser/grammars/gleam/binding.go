// Package gleam provides tree-sitter Gleam language support.
// Generated with tree-sitter CLI v0.26.5, ABI 15.
package gleam

// #cgo CFLAGS: -std=c11 -fPIC -I${SRCDIR}/src
// #include "src/parser.c"
// #include "src/scanner.c"
import "C"
import "unsafe"

func Language() unsafe.Pointer {
	return unsafe.Pointer(C.tree_sitter_gleam())
}
