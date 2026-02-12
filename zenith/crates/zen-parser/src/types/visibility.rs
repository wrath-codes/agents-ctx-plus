use serde::{Deserialize, Serialize};

/// Symbol visibility level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    PublicCrate,
    Private,
    Export,
    Protected,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Public => "public",
            Self::PublicCrate => "pub(crate)",
            Self::Private => "private",
            Self::Export => "export",
            Self::Protected => "protected",
        };
        write!(f, "{s}")
    }
}
