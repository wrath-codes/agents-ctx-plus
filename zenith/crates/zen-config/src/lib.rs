//! # zen-config
//!
//! Layered configuration loading for Zenith using figment.
//!
//! Configuration sources (in priority order):
//! 1. Environment variables (`ZEN_*`)
//! 2. Project-level `.zenith/config.toml`
//! 3. User-level `~/.config/zenith/config.toml`
//! 4. Built-in defaults
