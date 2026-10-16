//! Static dispatch for multi-implementation port enums (DISP-01..05).
//!
//! `impl_dispatch!` implements a single-method port on a dispatcher enum,
//! one match arm per variant. There is exactly one macro for this purpose
//! and it has one rule per call form (DISP-02); the call forms pick the arm
//! shape:
//!
//! **Expression-arm form** — the port's parameter is bound under the
//! caller-spelled name (the `entry;` line), then one
//! `Variant => <expression>` per variant follows. The expressions are
//! caller-supplied and resolve at the call site (DISP-03 — the macro
//! definition gains no cross-layer references); spelling the parameter name
//! at the call site is required because macro_rules hygiene keeps
//! macro-defined locals invisible to caller-supplied tokens. This is the
//! production shape:
//!
//! ```ignore
//! impl_dispatch!(CommandRunner for ShellRunner {
//!     entry;
//!     Fish => run_with(Shell::Fish.program(), entry),
//!     Bash => run_with(Shell::Bash.program(), entry),
//!     Sh   => run_with(Shell::Sh.program(), entry),
//! });
//! ```
//!
//! **Wrapper-arm form** — one `Variant` per variant: each variant holds the
//! concrete implementation and the arm delegates to its `run` (the
//! parameter is bound as `entry` by the macro itself):
//!
//! ```ignore
//! impl_dispatch!(CommandRunner for ShellRunner { Fish, Bash });
//! ```
//!
//! The expansion is a single `impl` block whose method body is one `match`
//! with one arm per variant (DISP-02, DISP-05): adding an implementation is
//! a one-line change, and the compiler keeps the match exhaustive, so a
//! forgotten variant cannot silently drift.
//!
//! Hygiene (DISP-03): the macro is exported and qualifies its dependencies
//! (`QueueEntry`, `RunError`) through fully qualified `$crate::` paths in
//! the expansion, so it does not depend on the call site's scope; each
//! caller-supplied expression is substituted exactly once.
//!
//! Edge case (DISP-04): the zero-variant form has its own explicit rule
//! that raises a named `compile_error!`, rather than an accidental empty
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

    // Expression-arm form: the port's parameter is bound under the
    // caller-spelled name (`$entry;`), then one `Variant => <expression>`
    // per variant (DISP-02, DISP-05). Each expression is caller-supplied
    // and resolves at the call site (DISP-03 — the macro definition gains
    // no cross-layer references); the spelled name is used because
    // macro_rules hygiene keeps macro-defined locals invisible to
    // caller-supplied tokens (tokens of one invocation share a hygiene
    // context, so the caller's references to the spelled name resolve).
    // Each captured expression is substituted exactly once, and the
    // compiler keeps the match exhaustive over the arm list.
    ($port:ident for $enum:ident {
        $entry:ident ;
        $( $variant:ident => $body:expr ),* $(,)?
    }) => {
        impl $port for $enum {
            fn run(
                &self,
                $entry: &$crate::domain::queue::QueueEntry,
            ) -> Result<(), $crate::ports::command_runner::RunError> {
                match self {
                    $( Self::$variant => $body, )*
                }
            }
        }
    };

    // Wrapper-arm form: one `Variant` per variant; the variant holds the
    // concrete implementation and the arm delegates to its `run` (DISP-02,
    // DISP-05). The parameter is bound as `entry` by the macro itself, so
    // this form needs no caller-spelled name. The compiler keeps the match
    // exhaustive over the variant list.
    ($port:ident for $enum:ident { $( $variant:ident ),* $(,)? }) => {
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

    /// Wrapper-arm form (kept from the original grammar): variants hold a
    /// concrete implementation and the macro delegates to its `run`.
    enum ShellRunner {
        Alpha(Alpha),
        Beta(Beta),
    }

    impl_dispatch!(CommandRunner for ShellRunner { Alpha, Beta });

    /// Expression-arm helper: proves the caller-supplied expressions run and
    /// resolve at the call site (each names this helper and the spelled
    /// `entry` parameter). Errors with `Waited { code: 1 }` when the entry
    /// text equals the variant's tag.
    fn expr_arm(tag: &str, entry: &QueueEntry) -> Result<(), RunError> {
        if entry.as_str() == tag {
            Err(RunError::Waited { code: 1 })
        } else {
            Ok(())
        }
    }

    /// Expression-arm form (the production shape): the caller spells the
    /// parameter name (`entry;`) and supplies one `=> <expression>` per
    /// variant; the expressions differ per variant, so arm selection is
    /// observable.
    enum ExprRunner {
        Alpha,
        Beta,
    }

    impl_dispatch!(CommandRunner for ExprRunner {
        entry;
        Alpha => expr_arm("alpha", entry),
        Beta => expr_arm("beta", entry),
    });

    #[test]
    fn each_variant_delegates_to_its_wrapped_runner() {
        let entry = QueueEntry::new("true").unwrap();
        assert!(ShellRunner::Alpha(Alpha).run(&entry).is_ok());
        assert!(ShellRunner::Beta(Beta).run(&entry).is_ok());
    }

    #[test]
    fn expression_arms_run_the_caller_supplied_expression_per_variant() {
        // The arm selected is the variant's own: its tag matches and errors,
        // the other tag does not.
        assert!(matches!(
            ExprRunner::Alpha
                .run(&QueueEntry::new("alpha").unwrap())
                .unwrap_err(),
            RunError::Waited { code: 1 }
        ));
        assert!(ExprRunner::Alpha
            .run(&QueueEntry::new("beta").unwrap())
            .is_ok());
        assert!(matches!(
            ExprRunner::Beta
                .run(&QueueEntry::new("beta").unwrap())
                .unwrap_err(),
            RunError::Waited { code: 1 }
        ));
        assert!(ExprRunner::Beta
            .run(&QueueEntry::new("alpha").unwrap())
            .is_ok());
    }
}
