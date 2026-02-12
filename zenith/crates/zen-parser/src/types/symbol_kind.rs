use serde::{Deserialize, Serialize};

/// The kind of extracted symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Constructor,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    TypeAlias,
    Const,
    Static,
    Field,
    Property,
    Event,
    Indexer,
    Macro,
    Module,
    Union,
    Component,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Constructor => "constructor",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Class => "class",
            Self::TypeAlias => "type_alias",
            Self::Const => "const",
            Self::Static => "static",
            Self::Field => "field",
            Self::Property => "property",
            Self::Event => "event",
            Self::Indexer => "indexer",
            Self::Macro => "macro",
            Self::Module => "module",
            Self::Union => "union",
            Self::Component => "component",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolKind;

    const KIND_CASES: &[(SymbolKind, &str)] = &[
        (SymbolKind::Function, "function"),
        (SymbolKind::Method, "method"),
        (SymbolKind::Constructor, "constructor"),
        (SymbolKind::Struct, "struct"),
        (SymbolKind::Enum, "enum"),
        (SymbolKind::Trait, "trait"),
        (SymbolKind::Interface, "interface"),
        (SymbolKind::Class, "class"),
        (SymbolKind::TypeAlias, "type_alias"),
        (SymbolKind::Const, "const"),
        (SymbolKind::Static, "static"),
        (SymbolKind::Field, "field"),
        (SymbolKind::Property, "property"),
        (SymbolKind::Event, "event"),
        (SymbolKind::Indexer, "indexer"),
        (SymbolKind::Macro, "macro"),
        (SymbolKind::Module, "module"),
        (SymbolKind::Union, "union"),
        (SymbolKind::Component, "component"),
    ];

    #[test]
    fn display_matches_snake_case() {
        for (kind, expected) in KIND_CASES {
            assert_eq!(kind.to_string(), *expected);
        }
    }

    #[test]
    fn serde_roundtrip_for_all_variants() {
        for (kind, expected) in KIND_CASES {
            let json = serde_json::to_string(kind).expect("serialize kind");
            assert_eq!(json, format!("\"{expected}\""));
            let parsed: SymbolKind = serde_json::from_str(&json).expect("deserialize kind");
            assert_eq!(parsed, *kind);
        }
    }
}
