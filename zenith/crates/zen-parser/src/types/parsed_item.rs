use serde::{Deserialize, Serialize};

use super::{SymbolKind, SymbolMetadata, Visibility};

/// A single extracted symbol from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedItem {
    pub kind: SymbolKind,
    pub name: String,
    pub signature: String,
    pub source: Option<String>,
    pub doc_comment: String,
    pub start_line: u32,
    pub end_line: u32,
    pub visibility: Visibility,
    pub metadata: SymbolMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: &[SymbolKind] = &[
        SymbolKind::Function,
        SymbolKind::Method,
        SymbolKind::Constructor,
        SymbolKind::Struct,
        SymbolKind::Enum,
        SymbolKind::Trait,
        SymbolKind::Interface,
        SymbolKind::Class,
        SymbolKind::TypeAlias,
        SymbolKind::Const,
        SymbolKind::Static,
        SymbolKind::Field,
        SymbolKind::Property,
        SymbolKind::Event,
        SymbolKind::Indexer,
        SymbolKind::Macro,
        SymbolKind::Module,
        SymbolKind::Union,
        SymbolKind::Component,
    ];

    #[test]
    fn parsed_item_serializes_kind_in_snake_case() {
        for kind in KINDS {
            let item = ParsedItem {
                kind: *kind,
                name: "sample".to_string(),
                signature: "sample()".to_string(),
                source: None,
                doc_comment: String::new(),
                start_line: 1,
                end_line: 1,
                visibility: Visibility::Public,
                metadata: SymbolMetadata::default(),
            };

            let value = serde_json::to_value(&item).expect("serialize parsed item");
            let kind_value = value
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .expect("kind should be serialized as string");
            assert_eq!(kind_value, kind.to_string());
        }
    }
}
