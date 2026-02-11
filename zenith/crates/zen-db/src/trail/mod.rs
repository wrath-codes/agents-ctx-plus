//! JSONL trail writer and replayer.
//!
//! The trail is the source of truth for all mutations. Per-session JSONL files
//! live in `.zenith/trail/` and the database is rebuildable from them.

pub mod replayer;
pub mod writer;
