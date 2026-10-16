//! Port traits (and their boundary error enums) that the application and the
//! outbound adapters are written against.
//!
//! Scaffold (WP01): the trait signatures are final; the per-port error enums
//! are `thiserror` skeletons whose full variant sets are completed in WP03, and
//! `dispatch.rs` gains the `impl_dispatch!` macro in WP03. Ports depend only on
//! `crate::domain` (ARCH-01).

pub mod command_runner;
pub mod dispatch;
pub mod editor;
pub mod queue_store;
