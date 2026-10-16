//! Static-dispatch helper for multi-implementation port enums (DISP-02..05).
//!
//! Scaffold (WP01): the module exists so the Target layout compiles. The
//! `impl_dispatch!` macro itself (one rule per call form, `@`-private helpers,
//! an explicit zero-variant edge-case rule, and an expansion that reduces to a
//! single `match`) is implemented in WP03.
