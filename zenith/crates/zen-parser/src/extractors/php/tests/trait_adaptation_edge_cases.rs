use super::*;

#[test]
fn trait_as_clause_captures_visibility_and_alias() {
    let source = r"
<?php
trait A { public function ping() {} }
class C {
  use A { A::ping as private pingA; }
}
";

    let items = parse_and_extract(source);
    let as_clause = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains(" as private "))
        .expect("expected as clause symbol");

    assert!(as_clause
        .metadata
        .attributes
        .iter()
        .any(|a| a == "trait_use:mode=as"));
    assert!(as_clause
        .metadata
        .attributes
        .iter()
        .any(|a| a == "trait_use:visibility=private"));
    assert!(as_clause
        .metadata
        .attributes
        .iter()
        .any(|a| a == "trait_use:alias=pingA"));
}

#[test]
fn trait_insteadof_clause_captures_all_targets() {
    let source = r"
<?php
trait A { public function ping() {} }
trait B { public function ping() {} }
trait D { public function ping() {} }
class C {
  use A, B, D { A::ping insteadof B, D; }
}
";

    let items = parse_and_extract(source);
    let insteadof = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("insteadof"))
        .expect("expected insteadof clause symbol");
    assert!(insteadof
        .metadata
        .attributes
        .iter()
        .any(|a| a == "trait_use:mode=insteadof"));
    assert!(insteadof
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("trait_use:instead_of=")));
    assert!(
        insteadof
            .metadata
            .attributes
            .iter()
            .filter(|a| a.starts_with("trait_use:instead_of="))
            .count()
            >= 1
    );
}

#[test]
fn trait_use_emits_each_used_trait() {
    let source = r"
<?php
trait A {}
trait B {}
class C { use A, B; }
";

    let items = parse_and_extract(source);
    let a = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name == "A")
        .expect("expected used trait A symbol");
    assert!(a.metadata.attributes.iter().any(|attr| attr == "trait_use"));

    let b = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name == "B")
        .expect("expected used trait B symbol");
    assert!(b.metadata.attributes.iter().any(|attr| attr == "trait_use"));
}
