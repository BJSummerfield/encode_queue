//! `FileQueueStore` outbound adapter (the queue file boundary).
//!
//! Scaffold (WP01): the module exists so the Target layout compiles. The
//! `FileQueueStore` implementing `QueueStore` (read/append/rewrite, with
//! `OpenOptions`/`fs` confined here — ARCH-03) is implemented in WP04.
