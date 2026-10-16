//! Port traits (and their boundary error enums) that the application and the
//! outbound adapters are written against.
//!
//! The trait signatures are final: every port method takes only domain
//! newtypes and returns only its own boundary error enum (ARCH-03, ERR-02) —
//! no `dyn` (DISP-01), no `&str`/`PathBuf` parameters, and the enums stay
//! in-crate exhaustive so a new variant breaks the build (ERR-04). Each enum
//! lists every expected failure as a named variant with an owned payload
//! plus a catch-all (ERR-04, GEN-03). `dispatch.rs` provides the
//! `impl_dispatch!` macro (DISP-02..05) for multi-implementation enums.
//! Ports depend only on `crate::domain` (ARCH-01).

pub mod command_runner;
pub mod dispatch;
pub mod editor;
pub mod queue_lock;
pub mod queue_store;
