use super::*;

#[test]
fn embedded_struct_fields_and_interface_methods_are_tagged() {
    let source = r"package demo
type Logger struct{}
type Server struct {
    Logger
    Port int
}

type Reader interface {
    io.Reader
    Close() error
}
";

    let items = parse_and_extract(source);

    let embedded = items
        .iter()
        .find(|i| i.kind == SymbolKind::Field && i.name == "Server::Logger")
        .expect("expected embedded field item");
    assert!(
        embedded
            .metadata
            .attributes
            .iter()
            .any(|a| a == "go:embedded_field")
    );

    let close = items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "Reader::Close")
        .expect("expected interface method item");
    assert!(
        close
            .metadata
            .attributes
            .iter()
            .any(|a| a == "go:embedded_interface")
    );
}
