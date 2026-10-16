//! Inbound CLI adapter (ARCH-04, ERR-03).
//!
//! Scaffold (WP01): the module exists so the Target layout compiles. The clap
//! `Cli` (subcommand parsing, boundary conversion of the `add` argument into a
//! `QueueEntry`, dispatch to the service) and its `CliError` are implemented in
//! WP09.
