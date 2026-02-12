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
