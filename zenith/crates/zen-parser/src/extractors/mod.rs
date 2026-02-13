//! Extraction orchestrator — two-tier fallback: ast-grep → regex.

pub mod dispatcher;
pub(crate) mod helpers;
pub use dispatcher::bash;
pub use dispatcher::c;
pub use dispatcher::cpp;
pub use dispatcher::csharp;
pub use dispatcher::css;
pub use dispatcher::elixir;
pub use dispatcher::go;
pub use dispatcher::haskell;
pub use dispatcher::html;
pub use dispatcher::java;
pub use dispatcher::javascript;
pub use dispatcher::lua;
pub use dispatcher::python;
pub use dispatcher::rust;
pub use dispatcher::tsx;
pub use dispatcher::typescript;
