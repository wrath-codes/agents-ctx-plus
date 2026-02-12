//! Extraction orchestrator — two-tier fallback: ast-grep → regex.

pub mod c;
pub mod cpp;
pub mod css;
pub mod dispatcher;
pub mod elixir;
pub mod go;
pub(crate) mod helpers;
pub mod javascript;
pub mod tsx;
pub mod typescript;

pub use dispatcher::bash;
pub use dispatcher::html;
pub use dispatcher::python;
pub use dispatcher::rust;
