//! # zen-db
//!
//! libSQL database operations for Zenith state management.
//!
//! Handles all relational state: research items, findings, hypotheses,
//! insights, tasks, sessions, audit trail, and entity links.
//! Uses libSQL embedded replicas with Turso Cloud sync on wrap-up.
//!
//! Uses the `libsql` crate (C SQLite fork, v0.9.29) — provides native FTS5,
//! stable API, and Turso Cloud embedded replica support.

#[cfg(test)]
mod spike_libsql;

#[cfg(test)]
mod spike_libsql_sync;

#[cfg(test)]
mod spike_studies;

#[cfg(test)]
mod spike_jsonl;

#[cfg(test)]
mod spike_clerk_auth;
