# Refactor Plan — `encode_queue` (P4)

Decomposes the single-file `src/main.rs` CLI into the Target layout prescribed by
`docs/CONVENTIONS.md` (section **Target layout**), closing the 25 violations found
by the four P3 audits (`docs/audit/architecture.md`, `errors.md`, `types.md`,
`dispatch-testing.md`).

This is a **docs-only** plan. It assigns every P3 violation to a work package (WP)
or to **Deferred**, and fixes the file-ownership and dependency contract that the
implementing workers must respect. `src/`, `Cargo.toml`, and all code are out of
scope here.

---

## Behaviour contract (inherited by every WP)

WP01 through the integration WP (WP10) **must preserve this behaviour exactly**.
Only P5/P6 may change the specific bug behaviours named below.

> `add <CMD>` appends `<CMD>\n` to the queue file
> (`$HOME/.config/encode_queue/commands.txt`, dir created if missing); `ls` prints
> the queue file contents; `start` reads the first line, if non-blank runs it with
> `fish -c <line>` inheriting stdio, then re-reads the file (it may have been
> edited meanwhile) and rewrites it without the first line, repeating until the
> file is empty; `edit` opens the queue in `$EDITOR` (default `nvim`).

Concrete observable points that must hold (from the V-ARCH-05 empirical notes in
`docs/audit/architecture.md`):

- `add` writes the raw `<CMD>` verbatim followed by a newline and exits 0, with
  **no trim and no blank rejection**: an empty or whitespace-only command is
  accepted exactly as today (the blank/whitespace line is enqueued verbatim).
  `QueueEntry` is introduced but its constructor is **lenient** (stores the raw
  line; it does not reject blank input), so this is preserved (see V-TY-01/
  V-TY-08). The audits' suggested *strict* behaviour — trim + reject blank
  (`from_str("   ")` → `Empty`, so `add` of a blank command exits 2) — is **not**
  applied in this refactor and is owned by **P5/P6** (see Deferred).
- `ls` prints the queue file **verbatim**, including the blank/whitespace-only
  lines that `add` can create. `read_all` (WP04) parses each line through the
  lenient `QueueEntry` (every line, blank included, is a valid entry) and prints
  it back unchanged.
- `start` on a **missing or empty** file: empty file → exit 0, no output.
  (Missing-file `ls`/`start` currently exits 1 with an I/O error — that missing-file
  *exit code* is preserved here and its change is owned by **P6**, see Deferred.)
- `start` currently **discards the `ExitStatus`** of each `fish -c` child (V-ERR-09)
  and removes the first line regardless of exit status. The refactor **must keep
  dropping the exit status** and removing the line unconditionally — surfacing a
  non-zero exit is **P5**'s decision (see Deferred).
- `edit` opens `$EDITOR` (default `nvim`) on the queue path.

Where a P3 violation's *suggested fix* would change one of these observable
behaviours, the WP **implements the structural fix** (newtypes, ports, error
enums, layout) but **keeps the observable behaviour identical**, and the
behaviour change itself is deferred to P5/P6. A concrete instance of this rule is
the `QueueEntry` whitespace/blank validation: the structural fix (a `QueueEntry`
newtype with a canonical fallible constructor, `FromStr`, and boundary parsing in
`add`/`ls`/`start`) is implemented in WP02/WP04/WP08/WP09, but the constructor is
kept **lenient** (accepts blank, stores the raw line verbatim; `start` keeps its
current trim/blank-skip) so `add`/`ls`/`start` are byte-for-byte unchanged, and
the strict trim + `Empty` rejection is **deferred** (see Deferred).

---

## WP definitions

### WP01 — Scaffold (serial)

- **scope.** Create the Target-layout tree as compilable scaffolding and add the
  `thiserror` dependency, while leaving the current single-file behaviour fully
  intact. `main.rs` keeps its existing implementation (inline subcommand match +
  the `process_queue`/`open_editor` free functions) and merely gains `mod`
  declarations for the new modules, so `add`/`ls`/`start`/`edit` behave
  byte-for-byte as today. The new modules are declared but not yet wired into
  `main`; they must compile. Domain newtypes are created as **shells** (private
  field + basic `as_str`/`as_path`/`Display` + a constructor) so downstream WPs
  can be written against them; error enums are **skeletons** (enum shells with
  `thiserror` derive, variants completed in WP03); port traits are defined with
  their final signatures.
- **files owned.** `Cargo.toml`, `src/main.rs`, `src/config.rs`,
  `src/domain/queue/mod.rs`, `src/domain/queue/service.rs`, `src/ports/mod.rs`,
  `src/ports/queue_store.rs`, `src/ports/command_runner.rs`,
  `src/ports/editor.rs`, `src/ports/dispatch.rs`, `src/inbound/cli.rs`,
  `src/outbound/mod.rs`, `src/outbound/queue_file.rs`, `src/outbound/fish.rs`,
  `src/outbound/editor.rs`.
- **depends_on.** none.
- **violation IDs it closes.** V-ARCH-01 (the four-layer tree now exists;
  `domain`/`ports` import no `clap`/`std::process`/`std::fs`/`OpenOptions`),
  V-ARCH-03 (port signatures expose only domain newtypes + boundary error enums —
  the mechanism that confines third-party types to adapters; the How-to-check grep
  passes because `domain`/`ports` contain no third-party identifiers).
- **acceptance criteria.**
  - `cargo build` succeeds (scaffold-stage warnings from as-yet-unused scaffold
    items are tolerated; the `-D warnings` gate is a property of WP10, not WP01).
  - `add`/`ls`/`start`/`edit` are unchanged vs. the pre-refactor binary (smoke the
    four subcommands against an isolated `$HOME`).
  - The Target-layout files exist (`ls src src/domain/queue src/ports src/inbound
    src/outbound`).
  - ARCH-01 How-to-check: `grep -rnE "clap|std::process|std::fs" src/ | grep
    -E "^src/(domain|ports)/"` → empty (normal no-match exit 1).
  - ARCH-03 How-to-check: `grep -rnE "clap|std::process|OpenOptions" src/ | grep
    -E "^src/(domain|ports)/"` → empty.
  - `thiserror` appears in `Cargo.toml`; `grep -rn "anyhow" Cargo.toml src` →
    empty (ERR-01 stays satisfied).

### WP02 — Domain newtypes + constructor tests

- **scope.** Finalize `src/domain/queue/mod.rs`: `QueueEntry` (canonical fallible
  constructor `new`, `FromStr` delegating to `new`, `TryFrom<&str>` as a one-line
  delegation to `from_str`, `as_str`, `Display`; a `QueueEntryError::Empty`
  variant is *declared* — it is CONVENTIONS.md's canonical error — but is
  **not produced** on the P5 path, see below), `QueuePath` (private `PathBuf`,
  `as_path`, `Display`, `Hash`), and `Editor` (private `String`, constructor
  applying the `nvim` default, `as_str`, `Display`). Private fields only; no
  `Deref`/`Borrow`; derive `Debug, Clone, PartialEq, Eq` (+ `Hash` where
  meaningful), no `Copy`/`Default`. **Lenient by design (behaviour
  preservation):** the constructor stores the raw line **verbatim** (no trim) and
  accepts empty/whitespace-only input, so `add`/`ls` stay byte-for-byte as today
  (Behaviour contract). The strict trim + `Empty` reject that CONVENTIONS.md's
  canonical `QueueEntry` shows is the **deferred** P5/P6 behaviour change (see
  Deferred), not applied here; the declared-but-unproduced `Empty` variant is
  retained (it is the domain's public error type for the deferred strict path).
  If the never-constructed variant trips a dead-code/clippy warning in this
  refactor, the WP02 `#[cfg(test)]` module keeps it referenced (the TST-02 suite
  covers the deferred `Empty` arm alongside the lenient `Ok` cases), so the
  `-D warnings` gate stays green.
  Add focused per-constructor tests (TST-02) matching the *current* behaviour:
  `QueueEntry::from_str("nix build .")` → `Ok` and `from_str("   ")` → `Ok`
  (blank is a valid entry, stored verbatim; the strict `Empty` reject is
  deferred).
- **files owned.** `src/domain/queue/mod.rs`.
- **depends_on.** WP01.
- **violation IDs it closes.** V-TY-06 (the three newtypes now exist), V-TY-07
  (canonical constructor + `FromStr`/`TryFrom` present), V-DT-03 (adds
  `#[cfg(test)]` constructor tests in `src/domain/`, satisfying TST-01/TST-02's
  domain-test How-to-check).
- **acceptance criteria.**
  - TY-03 How-to-check: `grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/ | grep
    -E "^src/domain/"` → empty.
  - TY-05 How-to-check: `grep -rn "impl Deref\|impl Borrow" src` → empty.
  - TY-02 How-to-check (scoped): `grep -rnE "fn [a-z_0-9]+ *\([^)]*:
    *(&str|String|&Path|PathBuf)\b" src/ | grep -E "^src/(domain|ports)/" | grep
    -vE "fn (from_str|try_from|from_utf8)\b"` → empty.
  - `cargo test` passes; `cargo clippy --all-targets --all-features -- -D
    warnings` clean for the domain module.
  - The `QueueEntry` constructor performs no trim/blank validation (lenient: it
    stores the line verbatim) — the strict `Empty` reject is deferred (see
    Deferred). The `start` blank-skip trim that lives in the service (WP08) is
    the *preserved* current behaviour, not new validation (TY-01).

### WP03 — Ports, error enums, dispatch macro

- **scope.** Finalize `src/ports/*`: the boundary error enums with their full,
  `thiserror`-derived, owned-payload variant sets plus a catch-all (ERR-02,
  ERR-04, GEN-03): `QueueStoreError::{QueueFileMissing(PathBuf), Io(std::io::Error)}`,
  `RunError::{Spawn(std::io::Error), Waited{code:i32}, Other(std::io::Error)}`,
  `EditorError::{Spawn(std::io::Error), Other(std::io::Error)}`. Confirm the
  `QueueStore`/`CommandRunner`/`EditorLauncher` trait signatures take only domain
  newtypes and return only the boundary enums (no `dyn`, no `&str`/`PathBuf`
  params on port methods, no `non_exhaustive`). Implement the `impl_dispatch!`
  macro in `ports/dispatch.rs` per DISP-02..05 (one rule per call form,
  `@`-private/doc-hidden helpers, an explicit zero-variant edge-case rule, and an
  expansion that reduces to a single `match` with one delegating arm per variant).
- **files owned.** `src/ports/mod.rs`, `src/ports/queue_store.rs`,
  `src/ports/command_runner.rs`, `src/ports/editor.rs`, `src/ports/dispatch.rs`.
- **depends_on.** WP01.
- **violation IDs it closes.** V-ERR-06 (a `thiserror` error enum per port
  boundary now exists beside each port) and reinforces V-ARCH-03.
- **acceptance criteria.**
  - ERR-01 How-to-check: `grep -rn "anyhow" Cargo.toml src` → empty; `grep -rn
    "dyn Error" src` → empty.
  - ERR-04: `grep -rn "non_exhaustive" src` → empty.
  - DISP-01: `grep -rn "dyn " src` → empty; DISP-02: `grep -rn
    "todo!\|unimplemented!" src` → empty.
  - Every port's How-to-check greps (ARCH-01/03) remain empty.
  - GEN-03: each enum's variant payloads are owned (`String`/`PathBuf`/
    `std::io::Error`), no borrowed references.
  - `cargo build` + `cargo clippy … -D warnings` clean.

### WP04 — Outbound: queue-file adapter

- **scope.** Implement `FileQueueStore` in `src/outbound/queue_file.rs` behind the
  `QueueStore` port: `read_all() -> Result<Vec<QueueEntry>, QueueStoreError>`
  (parse each line through `QueueEntry`'s lenient `FromStr` — every line,
  including blank/whitespace-only ones, parses to a valid entry and is kept —
  map a missing file to `QueueStoreError::QueueFileMissing(PathBuf)` and other I/O
  to `QueueStoreError::Io`),
  `append(&QueueEntry)`, and the rewrite-without-first-line used by `start`. The
  store owns its `QueuePath` (injected from config), so no raw `PathBuf` crosses
  the port. `OpenOptions`/`fs` details stay confined to this file (ARCH-03).
  **Preserved behaviour:** `ls`-on-missing-file still resolves to a non-success
  outcome (the `QueueFileMissing` variant → exit 1 at `main`); mapping it to
  an empty-queue/exit-0 is the **P6** change and is NOT done here. Blank and
  whitespace-only lines are parsed (lenient `FromStr`) and preserved, so `ls`
  prints them **verbatim** exactly as today (not omitted/trimmed; the strict
  reject is deferred, see Deferred).
- **files owned.** `src/outbound/queue_file.rs`.
- **depends_on.** WP01, WP02, WP03.
- **violation IDs it closes.** V-ERR-08 (`ls`'s conflated `read_to_string` is
  replaced by a named `QueueStoreError` variant at the boundary), V-TY-08
  (`ls` reads/prints parsed `QueueEntry`s via `read_all()`, not a raw `String`),
  and realizes V-ARCH-03/V-ARCH-04 for the file boundary.
- **acceptance criteria.**
  - `grep -nE "OpenOptions|std::fs|fs::" src/ports/ src/domain/` → empty (the I/O
    is confined to `outbound/`).
  - A missing queue file yields `QueueStoreError::QueueFileMissing`, not a leaked
    `std::io::Error`.
  - Adapter is exercised with a temp dir (TST-01); no test touches
    `$HOME/.config/encode_queue`.
  - `cargo test` passes.

### WP05 — Outbound: fish runner

- **scope.** Implement `FishRunner` in `src/outbound/fish.rs` behind the
  `CommandRunner` port: `run(&QueueEntry) -> Result<(), RunError>` that runs
  `fish -c <entry.as_str()>` with inherited stdio, mapping `spawn` failures to
  `RunError::Spawn(std::io::Error)` and wait I/O failures to the catch-all
  `RunError::Other`. **Preserved behaviour (V-ERR-09, P5-deferred):** the
  `ExitStatus` of the child is **discarded** and the line is not treated as
  failed on a non-zero exit — `run` returns `Ok(())` after `wait()` regardless of
  the exit status, matching the current `start` semantics.
- **files owned.** `src/outbound/fish.rs`.
- **depends_on.** WP01, WP02, WP03.
- **violation IDs it closes.** Realizes V-ARCH-03/V-ARCH-01 for the process
  boundary (`std::process::Command` confined to `outbound/fish.rs`).
- **acceptance criteria.**
  - `grep -n "std::process" src/ports/ src/domain/ src/inbound/` → empty.
  - A non-zero `fish -c` exit does **not** surface an error here (V-ERR-09
    behaviour preserved; `wait` maps only the I/O `Error`).
  - `cargo clippy … -D warnings` clean (no `unwrap`/`expect`).

### WP06 — Outbound: editor launcher

- **scope.** Implement `EditorProcess` in `src/outbound/editor.rs` behind the
  `EditorLauncher` port: `open(&QueuePath) -> Result<(), EditorError>` that
  launches `$EDITOR` (default `nvim`) on the queue path, mapping a spawn/exit
  failure to a named `EditorError` variant (no `expect`). Takes the `QueuePath`
  newtype (never a raw `&str`), so `to_str().unwrap()` is unnecessary.
- **files owned.** `src/outbound/editor.rs`.
- **depends_on.** WP01, WP02, WP03.
- **violation IDs it closes.** V-ERR-05 (`status().expect("Failed to open editor")`
  becomes `EditorError`), V-TY-05 (`open_editor(file_path: &str)` is replaced by
  `EditorLauncher::open(&QueuePath)`), and realizes V-ARCH-03 for the editor
  boundary.
- **acceptance criteria.**
  - A bad/missing `$EDITOR` produces an `EditorError` (message + non-zero exit at
    `main`), not a panic/backtrace.
  - `grep -nE "fn open *\([^)]*: *&str" src/` → empty (no raw-`&str` port/adapter
    signature).
  - Adapter tested with a fake process (e.g. `$EDITOR=true`); `cargo test` passes.

### WP07 — Config

- **scope.** Implement `src/config.rs`: `Config::from_env() -> Result<Config,
  ConfigError>` that reads `HOME` (a missing `HOME` becomes `ConfigError::HomeUnset`,
  never a panic), joins the fixed `.config/encode_queue/commands.txt` suffix into a
  `QueuePath`, and parses `EDITOR` into an `Editor` newtype (default `nvim`). This
  is the single origin of the queue path, so `main` no longer computes or threads
  a raw `PathBuf` (V-ARCH-05).
- **files owned.** `src/config.rs`.
- **depends_on.** WP01 (relies on the `QueuePath`/`Editor` newtype shells from the
  scaffold).
- **violation IDs it closes.** V-ERR-01 (`env::var("HOME").expect(…)` →
  `ConfigError::HomeUnset`), V-TY-03 (raw `HOME`/`EDITOR` env vars converted to
  domain newtypes at the boundary), V-ARCH-05 (queue path owned in `config.rs` as
  a `QueuePath`, no longer computed/threaded in `main`).
- **acceptance criteria.**
  - `grep -rn "expect(" src/config.rs` → empty; missing `HOME` is a `Result`
    error, not a panic.
  - `grep -nE "env::var" src/main.rs` → empty after WP10 wires config in; the
    only `env::var` calls live in `config.rs`.
  - `Config::from_env` is unit-tested with controlled env; `cargo test` passes.

### WP08 — Service layer + mock tests

- **scope.** Implement `QueueService` in `src/domain/queue/service.rs`
  (`QueueService<R: QueueStore, C: CommandRunner, E: EditorLauncher>`) with
  **note on generic arity:** the service takes a **third** port generic
  `E: EditorLauncher` beyond CONVENTIONS.md's canonical `QueueService<R, C>`
  shorthand, because TST-01 requires in-crate mock implementations of **all
  three** ports (`QueueStore`/`CommandRunner`/`EditorLauncher`), which the
  two-generic `<R, C>` form cannot inject; this deviation from the canonical
  arity is intentional.
  `add`/`ls`/`start`/`edit` methods that take and return only domain newtypes and
  the `QueueError` enum (which composes `QueueStoreError`/`RunError`/
  `EditorError`/`QueueEntryError` via `#[from]`, never duplicating them). `start`
  reads through the store, and for a non-blank first line (decided by
  `entry.as_str().trim().is_empty()`, as today) calls `runner.run` with the
  trimmed line, then re-reads (it may have been edited) and rewrites the
  remaining lines without the first line, repeating until empty — **preserving**
  the discard-exit-status behaviour (WP05), the unconditional line removal, and
  the current **trim-before-run / blank-skip** semantics (the `start` behaviour
  is byte-for-byte unchanged; moving validation to a strict "trust non-blank"
  `QueueEntry` is the deferred V-TY-02 behaviour change, see Deferred).
  No `&str`/`String` parameters on any service method (all values are newtypes;
  the blank-skip/trim operate on `QueueEntry`'s `as_str`, not a raw file
  `String`). Add `#[cfg(test)]` mock-port tests
  (TST-01) covering the add/start mapping, e.g.
  `start_removes_entry_after_successful_run`.
- **files owned.** `src/domain/queue/service.rs`.
- **depends_on.** WP02, WP03.
- **violation IDs it closes.** V-ARCH-04 (business code sees only newtypes;
  `grep -n "pub fn" src/domain/queue/service.rs` shows no `&str`/`String`
  params), V-TY-02 (file-line handling now operates on `QueueEntry`, not a raw
  `String` read straight from the file; the current trim/blank-skip is preserved
  and its removal is deferred — see Deferred), V-TY-04
  (`process_queue(&Path)` is replaced by a service method that never names the
  path), V-ERR-06 (part: `QueueError` composes the boundary enums via `#[from]`),
  V-DT-03 (adds `#[cfg(test)]` mock tests in `src/domain/`).
- **acceptance criteria.**
  - ARCH-01/ARCH-04 How-to-check greps stay empty; service imports only
    `crate::domain` and `crate::ports` (no `clap`/`std::process`/`std::fs`).
  - TST-01 How-to-check: `grep -rn "#\[cfg(test)\]" src/ | grep -E "^src/domain/"`
    is non-empty, and `grep -rnE "\.config/encode_queue|fish" src/ | grep
    -E "^src/domain/"` → empty (mock tests touch no real path / spawn no `fish`).
  - `cargo test` passes; the service tests use in-crate mocks, no `$HOME` access.

### WP09 — Inbound: CLI adapter

- **scope.** Implement `src/inbound/cli.rs`: the clap `Cli` that parses the
  `add`/`start`/`ls`/`edit` subcommands, converts the raw `add` argument into a
  `QueueEntry` at the boundary (`QueueEntry::from_str(raw).map_err(CliError::
  BadCommand)`), and dispatches to the service. Because the constructor is
  **lenient** (see WP02 / Behaviour contract), `from_str` accepts blank/
  whitespace input, so `add "   "` enqueues the blank line verbatim and exits 0
  exactly as today; `CliError::BadCommand` is wired for the **deferred** strict
  path but is not triggered by any input the current binary accepts. `CliError`
  covers `BadCommand(QueueEntryError)`, `MissingArgument(String)`, and the
  service-level errors. The subcommand `match` is exhaustive over the real
  variants (no `unreachable!()`); the `COMMAND` value is taken with
  `let Some(cmd) = … else { … MissingArgument }` (no `unwrap`).
- **files owned.** `src/inbound/cli.rs`.
- **depends_on.** WP08 (needs the service), WP02 (needs `QueueEntry`).
- **violation IDs it closes.** V-TY-01 (the raw `add` argument is translated to a
  `QueueEntry` before calling the service), V-ERR-02 (`get_one::<String>(…).
  unwrap()` → `MissingArgument`), V-ERR-04 (`_ => unreachable!()` → exhaustive
  match / `CliError` variant).
- **acceptance criteria.**
  - `grep -rnE "unwrap\(\)|unreachable!|expect\(" src/inbound/` → empty.
  - `grep -rn "clap" src/ | grep -E "^src/(domain|ports)/"` → empty (domain and
    ports never import `clap`; satisfiable once **WP09 alone** has landed). The
    stronger form "clap appears only in `src/inbound/` (and `Cargo.toml`)" holds
    only after **WP10** rewrites `src/main.rs` (which still carries
    `use clap::…` until then) and is checked under WP10, not at WP09.
  - `cargo build` + `cargo clippy … -D warnings` clean.

### WP10 — Integration (wire `main`, crate-root lints, dead-code removal)

- **scope.** Rewrite `src/main.rs` to bootstrap only (ARCH-02): read config from
  env, construct the concrete `FileQueueStore`/`FishRunner`/`EditorProcess`, wire
  them into `QueueService`, construct the `Cli`, call `cli.run()`, and map the
  top-level error to a message + `ExitCode` (usage/parse → 2; domain + expected
  I/O → 1; success → 0). `fn main` returns `std::process::ExitCode`, contains no
  `?` propagating a `Result` and is under ~40 lines. Delete the now-dead
  `process_queue`/`open_editor` free functions and the old inline subcommand
  match (dead-code removal). Add the crate-root lint attributes for GEN-01/GEN-04/
  TY-05: `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::unreachable))]` and
  `#![deny(clippy::inherent_to_string_shadow_display)]`.
- **files owned.** `src/main.rs`.
- **depends_on.** WP04, WP05, WP06, WP07, WP08, WP09 (everything).
- **violation IDs it closes.** V-ARCH-02 (`main` is bootstrap-only), V-ERR-07
  (`main` returns `ExitCode` with no `?`), V-ERR-03 (`queue_path.to_str().unwrap()`
  is gone — the editor port takes a `QueuePath`), V-DT-01 (all five OP6
  `unwrap`/`expect`/`unreachable!` sites are removed and the crate-root deny now
  enforces the ban), V-DT-02 (crate-root lint attributes present).
- **acceptance criteria.**
  - ERR-03: `fn main` returns `ExitCode`, no `?`-propagated `Result` return.
  - V-ERR-03: `grep -rn "to_str().unwrap\|to_str().expect" src` → empty.
  - V-DT-01: `cargo clippy --all-targets --all-features -- -D warnings` fails on
    any `unwrap`/`expect`/`panic!`/`unreachable!` in non-test code (the attribute
    is present).
  - V-DT-02: `grep -rn "cfg_attr(not(test)\|deny(clippy" src` is non-empty;
    `grep -rn "#\[allow" src` → empty.
  - Behaviour contract holds end-to-end: smoke `add`/`ls`/`start`/`edit` against an
    isolated `$HOME` and confirm add-appends, ls-prints (including a blank/
    whitespace-only line enqueued by `add "   "`, printed verbatim — behaviour
    preserved, strict reject deferred), start-drains (dropping exit status per
    V-ERR-09), edit-opens.
  - `cargo fmt` applied; `cargo clippy --all-targets --all-features -- -D
    warnings` exits 0; `cargo test` passes.

---

## Dependency table

| WP  | depends_on                          | files owned                                                                                          | closes                                              |
|-----|-------------------------------------|------------------------------------------------------------------------------------------------------|-----------------------------------------------------|
| WP01 | — (serial scaffold)                | `Cargo.toml`, `src/main.rs`, `src/config.rs`, `src/domain/queue/{mod,service}.rs`, `src/ports/{mod,queue_store,command_runner,editor,dispatch}.rs`, `src/inbound/cli.rs`, `src/outbound/{mod,queue_file,fish,editor}.rs` | V-ARCH-01, V-ARCH-03 |
| WP02 | WP01                               | `src/domain/queue/mod.rs`                                                                            | V-TY-06, V-TY-07, V-DT-03 |
| WP03 | WP01                               | `src/ports/{mod,queue_store,command_runner,editor,dispatch}.rs`                                      | V-ERR-06, V-ARCH-03 |
| WP04 | WP01, WP02, WP03                    | `src/outbound/queue_file.rs`                                                                         | V-ERR-08, V-TY-08, V-ARCH-03, V-ARCH-04 |
| WP05 | WP01, WP02, WP03                    | `src/outbound/fish.rs`                                                                               | V-ARCH-03, V-ARCH-01 |
| WP06 | WP01, WP02, WP03                    | `src/outbound/editor.rs`                                                                             | V-ERR-05, V-TY-05, V-ARCH-03 |
| WP07 | WP01                               | `src/config.rs`                                                                                      | V-ERR-01, V-TY-03, V-ARCH-05 |
| WP08 | WP02, WP03                          | `src/domain/queue/service.rs`                                                                        | V-ARCH-04, V-TY-02, V-TY-04, V-ERR-06, V-DT-03 |
| WP09 | WP08, WP02                          | `src/inbound/cli.rs`                                                                                 | V-TY-01, V-ERR-02, V-ERR-04 |
| WP10 | WP04, WP05, WP06, WP07, WP08, WP09  | `src/main.rs`                                                                                        | V-ARCH-02, V-ERR-07, V-ERR-03, V-DT-01, V-DT-02 |

**Graph shape (acyclic).** WP01 → {WP02 ∥ WP03 ∥ WP07} → {WP04 ∥ WP05 ∥ WP06 ∥
WP08} → WP09 → WP10. WP10 (= `WPnn`) is the unique sink and is the integration
package (wiring `main`, dead-code removal, crate-root lints).

## API-freeze (makes parallel scheduling sound)

WP01 freezes the public API surface: the domain newtype constructors + accessors
+ `Display` (`QueueEntry::new`/`from_str`/`as_str`, `QueuePath` ctors/`as_path`,
`Editor` ctor/`as_str`) and every port trait signature. Downstream WPs code
against this frozen surface. A WP may refine a type's **internal** validation or
add tests, but must not change a public constructor/accessor/signature that a
concurrently-runnable WP depends on (this is what lets WP02 refine `QueueEntry`
while WP07 (`config.rs`) and WP03 (`ports/*`) build against the same `QueuePath`/
`Editor`/port API in parallel).

## Parallelism note

After **WP01**, the set **{WP02, WP03, WP07}** runs in parallel: all three depend
only on WP01, no dependency path exists between any pair, and they own disjoint
files (`src/domain/queue/mod.rs` / `src/ports/*.rs` / `src/config.rs`). After
that wave, **{WP04, WP05, WP06, WP08}** also run in parallel with disjoint
ownership (`outbound/queue_file.rs` / `outbound/fish.rs` / `outbound/editor.rs` /
`domain/queue/service.rs`). The three outbound adapters **{WP04, WP05, WP06}** are
a ≥3 set that a coordinator can schedule together immediately once WP02+WP03 are
done, with no two of them touching the same file. No two concurrently-runnable WPs
share a file (WP01's broad file set is safe because WP01 is the serial root and is
concurrent with nothing).

## Deferred

- **V-ERR-09** (`start` drops the exit status of every `fish -c` child — a queued
  command exiting non-zero is treated as successful and its line is removed
  anyway). Reason: this is an observed **bug behaviour** that the behaviour
  contract explicitly requires WP01..integration to preserve, and the P3
  audit flags it as "P5 decides whether to fix." The refactor implements the
  structural fix (a `CommandRunner` port with a `RunError` enum) but
  `FishRunner` (WP05) deliberately keeps discarding the `ExitStatus`. Surfacing a
  non-zero exit and only removing the line on success is **P5**'s change.
- **V-ARCH-05 (behaviour half)** — the *layout* half (queue path owned in
  `config.rs`/`FileQueueStore`, not threaded through `main`) is closed by WP07/
  WP04. The *observed behaviour* half — `ls`/`start` on a **missing** queue file
  currently print a raw `std::io::Error` and exit 1 — is preserved by the
  refactor (a `QueueStoreError::QueueFileMissing` → message + exit 1 at `main`),
  and the change to "missing → empty queue, exit 0" is owned by **P6** per the
  audit.
- **V-TY-01/V-TY-02/V-TY-08 (behaviour half) — strict whitespace/blank
  validation.** The audits' suggested fix for `QueueEntry` is trim + reject
  blank (`from_str("   ")` → `Empty`). Applied at the boundary it would change
  three observable behaviours the current binary has today: `add` of an
  empty/whitespace-only command (currently exit 0, blank line enqueued
  verbatim → would become exit 2, nothing enqueued), `ls` (currently prints
  blank/whitespace-only lines verbatim → would omit/trim them), and `start`
  (currently trims before `fish -c` and skips blank lines → would trust every
  entry as non-blank). This refactor implements the **structural** half
  (`QueueEntry` newtype + `FromStr` + boundary parsing in WP02/WP04/WP08/WP09)
  but keeps the constructor **lenient** (accepts blank, stores the raw line
  verbatim; `start` keeps trim/blank-skip), so all three behaviours are
  byte-for-byte preserved. The strict trim + `Empty` rejection is a behaviour
  change owned by **P5/P6** (consistent with the audits' suggested fixes and
  CONVENTIONS.md's canonical `QueueEntry`), landing when the queue-domain
  validation behaviour is adopted. The structural IDs V-TY-01 (WP09), V-TY-02
  (WP08), V-TY-08 (WP04) remain closed; only this behaviour half is deferred.

## Coverage check (all 25 P3 violations)

Closed by a WP: V-ARCH-01 (WP01), V-ARCH-02 (WP10), V-ARCH-03 (WP01/WP03/WP04/
WP05/WP06), V-ARCH-04 (WP08), V-ARCH-05 (WP07, layout half), V-ERR-01 (WP07),
V-ERR-02 (WP09), V-ERR-03 (WP10), V-ERR-04 (WP09), V-ERR-05 (WP06),
V-ERR-06 (WP03/WP08), V-ERR-07 (WP10), V-ERR-08 (WP04), V-TY-01 (WP09), V-TY-02 (WP08),
V-TY-03 (WP07), V-TY-04 (WP08), V-TY-05 (WP06), V-TY-06 (WP02), V-TY-07 (WP02),
V-TY-08 (WP04), V-DT-01 (WP10), V-DT-02 (WP10), V-DT-03 (WP02/WP08).
Deferred: V-ERR-09 (P5), V-ARCH-05 behaviour half (P6), V-TY-01/V-TY-02/V-TY-08
behaviour half (strict whitespace/blank validation; P5/P6).

Every violation ID from the P3 audits (V-ARCH-01..05, V-ERR-01..09, V-TY-01..08,
V-DT-01..03) is either closed by at least one WP or listed under Deferred. ✓
