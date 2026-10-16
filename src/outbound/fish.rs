//! `FishRunner` outbound adapter (the process boundary).
//!
//! Scaffold (WP01): the module exists so the Target layout compiles. The
//! `FishRunner` implementing `CommandRunner` (`fish -c <entry>` with inherited
//! stdio, `std::process::Command` confined here — ARCH-03) is implemented in
//! WP05.
