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

    #[test]
    fn display_matches_snake_case() {
        assert_eq!(SymbolKind::Function.to_string(), "function");
        assert_eq!(SymbolKind::Method.to_string(), "method");
        assert_eq!(SymbolKind::Constructor.to_string(), "constructor");
        assert_eq!(SymbolKind::Field.to_string(), "field");
        assert_eq!(SymbolKind::Property.to_string(), "property");
        assert_eq!(SymbolKind::Event.to_string(), "event");
        assert_eq!(SymbolKind::Indexer.to_string(), "indexer");
    }

    #[test]
    fn serde_roundtrip_for_new_variants() {
        let variants = [
            SymbolKind::Constructor,
            SymbolKind::Field,
            SymbolKind::Property,
            SymbolKind::Event,
            SymbolKind::Indexer,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize kind");
            let parsed: SymbolKind = serde_json::from_str(&json).expect("deserialize kind");
            assert_eq!(parsed, variant);
        }
    }
}
