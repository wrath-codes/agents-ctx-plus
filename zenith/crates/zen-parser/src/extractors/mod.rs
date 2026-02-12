//! Extraction orchestrator — two-tier fallback: ast-grep → regex.

pub mod c;
pub mod cpp;
pub mod dispatcher;
pub(crate) mod helpers;
pub mod tsx;

pub use dispatcher::bash;
pub use dispatcher::css;
pub use dispatcher::elixir;
pub use dispatcher::go;
pub use dispatcher::html;
pub use dispatcher::javascript;
pub use dispatcher::python;
pub use dispatcher::rust;
pub use dispatcher::typescript;
