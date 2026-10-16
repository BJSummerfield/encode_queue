# Audit — Dispatch & macros (DISP), Testing (TST), General (GEN)

- **Task:** P3.A4 (round 1) — benchmark run `dflash-k8v4-run1`
- **Scope:** DISP (static dispatch via enums, no `Box<dyn Port>`/`&dyn Port`,
  `macro_rules!` delegation), TST, and GEN families only, per
  `docs/CONVENTIONS.md`.
- **Tree under audit:** pre-refactor single-file CLI. `src/` contains only
  `src/main.rs` (116 lines). There is no `src/domain/`, `src/ports/`,
  `src/inbound/`, or `src/outbound/` yet, and `Cargo.toml`'s only dependency is
  `clap = "4.5.4"` (no `thiserror`, no `tokio`).
- **Method:** the DISP/TST/GEN *How-to-check* commands in `docs/CONVENTIONS.md`
  were run against the tree. Each finding below quotes the command's exit
  status and/or output as proof.

## Findings

### V-DT-01 — Non-test code uses `unwrap`/`expect`/`unreachable!` (GEN-01)

- **Convention violated:** GEN-01 (No `unwrap`/`expect`/`panic!`/`unreachable!`
  outside tests; source OP6; hex-R15:S9).
- **Severity:** high.
- **Evidence.** `grep -rnE "unwrap\(\)|expect\(|panic!|unreachable!" src` exits
  0 and prints five non-test call sites in `src/main.rs`:

  - `src/main.rs:31` —
    `let home_dir = env::var("HOME").expect("HOME environment variable not set");`
    A missing `HOME` becomes a panic with a backtrace instead of a
    message + exit code (ERR-03).
  - `src/main.rs:39` —
    `let command = sub_matches.get_one::<String>("COMMAND").unwrap();`
  - `src/main.rs:54` —
    `open_editor(queue_path.to_str().unwrap())?;`
    (`.to_str()` only returns `None` on non-UTF-8 paths; `unwrap` panics there.)
  - `src/main.rs:56` —
    `_ => unreachable!(),`
    after the `matches.subcommand()` match; a clap input that reaches the
    fall-through arm panics.
  - `src/main.rs:113` —
    `.expect("Failed to open editor");`
    on the `SystemCommand::new(editor).arg(file_path).status()` chain; any
    editor spawn/exit failure panics.

  (The `env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string())` at
  `src/main.rs:108` is the correct non-panicking pattern and is *not* flagged.)

- **Suggested fix.** Map each fallible call to a named error variant instead of
  panicking: `env::var("HOME").map_err(ConfigError::HomeUnset)?`; use
  `let Some(cmd) = … else { return Err(CliError::MissingArgument(…)) };`; convert
  `to_str()` with `.map_err(…)`; and turn the `unreachable!()` arm and the
  editor `.status()` failure into `CliError`/`EditorError` variants.

### V-DT-02 — Crate root does not enforce the OP6 / TY-05 lints (GEN-04)

- **Convention violated:** GEN-04 (Clippy-clean on every change; the four OP6
  lints and `inherent_to_string_shadow_display` are enforced, not silenced).
- **Severity:** med.
- **Evidence.** `src/main.rs` is the crate root for this single-file crate and
  its line 1 is `use clap::{Arg, ArgAction, Command};` — there is no
  `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::unreachable))]` and no
  `#![deny(clippy::inherent_to_string_shadow_display)]`. Proof:

  ```
  $ grep -rn "cfg_attr(not(test)\|deny(clippy" src
  (no output; exit 1)
  ```

  With those deny attributes absent, the GEN-01 check's own How-to-check
  command passes vacuously — `cargo clippy --all-targets --all-features -- -D
  warnings` exits 0:

  ```
  $ /home/agent/encode_queue_bench/dflash-k8v4-run1/dev cargo clippy --all-targets --all-features -- -D warnings
      Checking encode_queue v0.1.0 (/home/agent/eq-wt/dflash-k8v4-run1/P3.A4)
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.49s
  (exit 0)
  ```

  so the five V-DT-01 `unwrap`/`expect`/`unreachable!` sites are not caught by
  the linter. `grep -rn "#\[allow" src` exits 1 (no suppressions present), so
  the "no `#[allow]`" half of GEN-04 holds — the violation is specifically the
  missing *enforcement* attributes.

- **Suggested fix.** Add at the top of the crate root
  `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::unreachable))]` and
  `#![deny(clippy::inherent_to_string_shadow_display)]`, which makes
  `cargo clippy … -D warnings` fail on any V-DT-01-style call.

### V-DT-03 — No tests at all (TST-01)

- **Convention violated:** TST-01 (Test the service and adapters by injecting
  mock ports; source OP1; hex-R12:S21/S22).
- **Severity:** low.
- **Evidence.** There is no test module anywhere in `src/`:

  ```
  $ grep -rn "#\[cfg(test)\]" src/
  (no output; exit 1)
  $ grep -rn "fn test\|mod tests\|#\[test\]" src/
  (no output; exit 1)
  $ cargo test
      Running unittests src/main.rs (…)
      running 0 tests
      test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (exit 0, 0 tests)
  ```

  TST-01's How-to-check — `grep -rn "#\[cfg(test)\]" src/ | grep -E "^src/domain/"`
  is non-empty — fails (empty, exit 1). Note the convention's own caveat,
  "(Applies once the Target layout exists.)": the target layout (`QueueService`,
  `QueueStore`, `CommandRunner`, `EditorLauncher` ports) does not exist yet, so
  this is a known gap to close during the refactor rather than a
  refactor-blocking defect.

- **Suggested fix.** When the layout lands, add `#[cfg(test)]` mock
  implementations of `QueueStore`/`CommandRunner`/`EditorLauncher` in
  `src/domain/queue/service.rs` and cover the service's add/start mapping with
  them; test the file/runner/editor adapters with temp dirs and fake
  processes (e.g. `$EDITOR=true`).

## Clean

Conventions checked and found satisfied (or not-yet-applicable and
vacuously satisfied given the single-file state), with supporting evidence:

- **DISP-01 — Static dispatch; no `dyn` ports.** `grep -rn "dyn " src` exits 1
  (no matches); the looser `grep -rn "dyn" src` also exits 1. No
  `Box<dyn …>`/`&dyn …`/`Arc<dyn …>` anywhere.
- **DISP-02 — Delegating match arms generated by a `macro_rules!` macro.**
  `grep -rn "todo!\|unimplemented!" src` exits 1. No multi-implementation port
  enums exist yet, so there are no hand-written delegating `match` arms and no
  `todo!()`/`unimplemented!()` stubs to drift — the rule is satisfied
  vacuously.
- **DISP-03 — Macro hygiene.** No `macro_rules!`/`#[macro_export]` exists
  (`grep -rn "macro_rules\|impl_dispatch\|macro_export" src` exits 1), so there
  is no fragment-capture or unqualified-dependency surface to violate.
- **DISP-04 — Edge cases get an explicit rule.** No macros exist (same empty
  grep as DISP-03), so no degenerate zero-variant form can be formed by
  accident.
- **DISP-05 — Reduce every expansion to fundamental calls.** No macros exist,
  so there is no expansion beyond fundamental calls to review
  (`cargo expand` is not applicable).
- **TST-02 — Each validating constructor gets focused success/failure
  tests.** The tree has no domain newtypes or validating constructors
  (`grep -rn "pub enum" src/` exits 1; there is no `QueueEntry::from_str`/`new`),
  so the conditional "each constructor has a test" rule is vacuously satisfied;
  the only validation is the inline `if !first_line.trim().is_empty()` guard at
  `src/main.rs:70`, a TY-01 concern outside this family.
- **TST-03 — `unwrap`/`expect` acceptable in tests.** No non-test
  `#[cfg(test)]` fixture exists to misuse `unwrap`/`expect` (see V-DT-03); the
  ban is scoped to non-test code via the GEN-01 lint, which is the subject of
  V-DT-02 rather than a TST-03 violation.
- **GEN-02 — Synchronous ports; no async/`Arc` supertrait stack.**
  `grep -rnE "tokio|async fn|impl Future" src` exits 1 (no matches). All
  functions are plain synchronous; no `tokio` dependency in `Cargo.toml`.
- **GEN-03 — Error types are `'static` with owned data.**
  `grep -rn "^pub enum" src/ | grep -E "^src/(ports|domain)/"` exits 1 and the
  broader `grep -rn "pub enum" src/` also exits 1 — no custom error enums exist
  yet (the code returns bare `std::io::Result`), so there is no borrowed-lifetime
  error payload to flag. (The absence of boundary error enums is an ERR-02
  concern outside this family.)
