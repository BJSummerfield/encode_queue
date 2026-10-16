//! Static dispatch for multi-implementation port enums (DISP-01..05).
//!
//! `impl_dispatch!` implements a single-method port on a dispatcher enum that
//! holds one concrete implementation per variant. The expansion is a single
//! `impl` block whose method body is one `match` with one delegating arm per
//! variant (DISP-02, DISP-05): adding an implementation is a one-token
//! change, and the compiler keeps the match exhaustive, so a forgotten
//! variant cannot silently drift.
//!
//! ```ignore
//! impl_dispatch!(CommandRunner for ShellRunner { Fish, Bash });
//! ```
//!
//! Hygiene (DISP-03): the macro is exported and qualifies its dependencies
//! (`QueueEntry`, `RunError`) through fully qualified `$crate::` paths in the
//! expansion, so it does not depend on the call site's scope; each captured
//! fragment is substituted exactly once.
//!
//! Edge case (DISP-04): the zero-variant form has its own explicit rule that
//! raises a named `compile_error!`, rather than an accidental empty
//! `match`.
//!
//! The expansion hard-codes the single-method port signature
//! `fn run(&self, &QueueEntry) -> Result<(), RunError>` — the shape of
//! `CommandRunner`, the only port with multiple candidate implementations.
//! Multi-method ports (e.g. `QueueStore`) have no multi-implementation enum
//! in this code base and are not dispatched this way.

#[macro_export]
macro_rules! impl_dispatch {
    // Explicit zero-variant rule (DISP-04): a dispatcher enum with no
    // implementation is a deliberate, named compile error.
    ($port:ident for $enum:ident {}) => {
        compile_error!("impl_dispatch! needs at least one variant")
    };

    // Normal form: one rule per call form, expanding to a single `match` with
    // one delegating arm per variant (DISP-02, DISP-05).
    ($port:ident for $enum:ident { $($variant:ident),* $(,)? }) => {
        impl $port for $enum {
            fn run(
                &self,
                entry: &$crate::domain::queue::QueueEntry,
            ) -> Result<(), $crate::ports::command_runner::RunError> {
                match self {
                    $( Self::$variant(inner) => inner.run(entry), )*
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::domain::queue::QueueEntry;
    use crate::ports::command_runner::{CommandRunner, RunError};

    struct Alpha;

    impl CommandRunner for Alpha {
        fn run(&self, _entry: &QueueEntry) -> Result<(), RunError> {
            Ok(())
        }
    }

    struct Beta;

    impl CommandRunner for Beta {
        fn run(&self, _entry: &QueueEntry) -> Result<(), RunError> {
            Ok(())
        }
    }

    enum ShellRunner {
        Alpha(Alpha),
        Beta(Beta),
    }

    impl_dispatch!(CommandRunner for ShellRunner { Alpha, Beta });

    #[test]
    fn each_variant_delegates_to_its_wrapped_runner() {
        let entry = QueueEntry::new("true");
        assert!(ShellRunner::Alpha(Alpha).run(&entry).is_ok());
        assert!(ShellRunner::Beta(Beta).run(&entry).is_ok());
    }
}
