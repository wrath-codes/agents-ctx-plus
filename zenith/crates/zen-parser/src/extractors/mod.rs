//! Extraction orchestrator — two-tier fallback: ast-grep → regex.

pub mod c;
pub mod cpp;
pub mod dispatcher;
pub mod go;
pub(crate) mod helpers;
pub mod javascript;
pub mod tsx;
pub mod typescript;

pub use dispatcher::bash;
pub use dispatcher::css;
pub use dispatcher::elixir;
pub use dispatcher::html;
pub use dispatcher::python;
pub use dispatcher::rust;
