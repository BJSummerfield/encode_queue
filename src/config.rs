//! Environment configuration (`HOME`, `EDITOR`) → domain newtypes.
//!
//! Scaffold (WP01): the module exists so the Target layout compiles.
//! `Config::from_env` (single origin of the queue `QueuePath` and the `Editor`,
//! with a `ConfigError` for a missing `HOME` — never a panic) is implemented in
//! WP07.
