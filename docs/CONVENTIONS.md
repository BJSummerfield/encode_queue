# Conventions

This document is the binding set of conventions for `encode_queue`, the tiny CLI
that manages a queue file at `$HOME/.config/encode_queue/commands.txt`
(`add <CMD>`, `ls`, `start` — runs queued lines via `fish -c` — and `edit` —
`$EDITOR`, default `nvim`).

It synthesizes four research digests under `docs/research/` —
[error handling](research/error-handling/), [hexagonal
architecture](research/hexagonal-architecture/), [newtypes](research/newtypes/),
and [`macro_rules!`](research/macro-rules/) — with the owner preferences below.
**Owner preferences are non-negotiable and win every conflict** (see
[Resolved conflicts](#resolved-conflicts)).

The repo today is a single-file CLI in `src/main.rs`; the examples below quote
that code as the canonical "bad" case, and [Target layout](#target-layout)
describes where the tree should end up.

## Owner preferences (non-negotiable)

| # | Preference |
|---|------------|
| OP1 | Hexagonal architecture: domain, ports (traits), adapters (inbound CLI, outbound file system/process), and an application/service layer. Dependencies point inward. Domain imports no adapter, clap, or `std::process` code. Layout follows the hexagonal guide's file patterns. |
| OP2 | Static dispatch via enums, not dynamic dispatch. No `Box<dyn Port>` or `&dyn Port` for ports. When a port has multiple implementations, define an enum whose variants hold them, implement the port trait on the enum, and generate the delegating `match` arms with a `macro_rules!` macro. Plain generics only where one concrete type is chosen at compile time. |
| OP3 | `thiserror` only. No `anyhow`, not even in `main`. Each port and adapter boundary has its own error enum. `main` maps the top-level error to a message and an exit code. |
| OP4 | Parse, don't validate. Raw input (CLI strings, file lines, env vars) is converted at the boundary into domain newtypes through `TryFrom`/`FromStr`. Once constructed, a domain value is valid by construction. |
| OP5 | Private fields. Domain types expose constructors and accessors, never `pub` fields. |
| OP6 | No `unwrap`/`expect`/`panic!`/`unreachable!` outside tests. |

## Citation key

- `OP<n>` — owner preference *n* above.
- `hex-R<n>:S<m>` — rule **R**\<n\> in `docs/research/hexagonal-architecture/notes.md`, demonstrated by snippet **S**\<m\> in its `snippets.md`.
- `err-<n>:S<m>` — rule *n* in `docs/research/error-handling/notes.md`, snippet S\<m\>.
- `new-R<n>:S<m>` — rule **R**\<n\> in `docs/research/newtypes/notes.md`, snippet S\<m\>.
- `mac-<n>:S<m>` — rule *n* in `docs/research/macro-rules/notes.md`, snippet S\<m\>.

Every convention below has: an ID, a rule, a one-line rationale, a good/bad
example in this repo's domain, a **How to check** line, and its source(s).

---

## Architecture & layout

### ARCH-01 — Four layers; dependencies point inward

**Rule.** Code is split into `domain` (types + invariants), `ports` (traits),
`inbound`/`outbound` (adapters), and the service/application layer
(`domain/queue/service`). Dependencies point strictly inward: adapters import
ports + domain; the service imports ports; ports import only domain; domain
imports none of the others and no third-party crates. Domain and ports never
import `clap`, `std::process`, `std::fs`, or any adapter module.

**Why.** An HTTP handler "shouldn't even know what a database is"
(hex-R3:S5) — our service shouldn't know that commands run through `fish` or
that the queue lives in a file at all.

**Good.**
```rust
// src/domain/queue/service.rs — imports only crate::domain and crate::ports
pub struct QueueService<R: QueueStore, C: CommandRunner> {
    store: R,
    runner: C,
}

impl<R: QueueStore, C: CommandRunner> QueueService<R, C> {
    pub fn add(&self, entry: &QueueEntry) -> Result<(), QueueError> {
        self.store.append(entry)?;   // no fs, no clap, no process here
        Ok(())
    }
}
```

**Bad.** (current `src/main.rs`)
```rust
use clap::{Arg, ArgAction, Command};
use std::{env, fs, process::Command as SystemCommand};
// …one file where clap parsing, fs::read_to_string(queue_path) and
// SystemCommand::new("fish") all sit next to each other.
```

**How to check.** `grep -rnE "clap|std::process|std::fs" src/domain src/ports` → empty.

**Source.** OP1; hex-R2:S16, hex-R17:S29.

### ARCH-02 — `main` is bootstrap only

**Rule.** `main()` does exactly: read config from env, construct the concrete
outbound adapters, wire them into the service, call the inbound CLI adapter's
`run`, and map the top-level error to a message + exit code. No loops, no file
I/O, no command parsing, no process spawning.

**Why.** "The less code you put in `main`, the smaller your testing dead zone"
(hex-R17:S28) — `main` composes unmockable dependencies.

**Good.**
```rust
fn main() -> ExitCode {
    let config = Config::from_env();
    let store = FileQueueStore::new(config.queue_path.clone());
    let runner = FishRunner::new();
    let service = QueueService::new(store, runner);
    let cli = Cli::new(service);          // inbound adapter
    cli.run()                             // -> message + ExitCode
}
```

**Bad.** (current `src/main.rs`) `main` matches on clap subcommands itself and
does `fs::create_dir_all`, `writeln!(file, …)`, `process_queue(&queue_path)`,
`open_editor(…)` inline.

**How to check.** Manual: `fn main` is under ~40 lines and contains no
`fs::`, no `Command::new`, no `read_line`.

**Source.** OP1; hex-R17:S28.

### ARCH-03 — Adapters wrap third-party types and expose only what the application needs

**Rule.** `clap`'s `ArgMatches`, `std::process::Command`, and `OpenOptions`
details exist only inside `inbound/` and `outbound/`. Port signatures use only
domain types and boundary error enums.

**Why.** "Wrap external libraries and expose only the functionality your
application requires" (hex-R9:S8).

**Good.**
```rust
// src/outbound/fish.rs
pub fn run(&self, entry: &QueueEntry) -> Result<(), RunError> {
    SystemCommand::new("fish").arg("-c").arg(entry.as_str())
        .stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .spawn().map_err(RunError::Spawn)?   // std::io::Error stops here
        .wait().map_err(RunError::Wait)
}
```

**Bad.** (current `src/main.rs`) the CLI layer hands a raw `String` through
`sub_matches.get_one::<String>("COMMAND").unwrap()` straight into
`writeln!(file, "{}", command)`, and `process_queue` returns bare
`io::Result<()>`, so callers see OS-level detail instead of queue-level
failures.

**How to check.** `grep -rnE "clap|std::process|OpenOptions" src/ports src/domain` → empty.

**Source.** OP1; hex-R9:S8.

### ARCH-04 — Translate at the edges

**Rule.** The inbound CLI adapter converts raw CLI strings into domain newtypes
before calling the service; outbound adapters convert domain types to raw text
only when writing the queue file. Business code never sees raw `&str` input.

**Why.** Transport and domain types "can now change independently … no longer
spread through your program like wildfire" (hex-R13:S18/S20).

**Good.**
```rust
// src/inbound/cli.rs
let entry = QueueEntry::from_str(raw_arg).map_err(CliError::BadCommand)?;
service.add(&entry)
```

**Bad.** (current `src/main.rs`) `writeln!(file, "{}", command)` appends the
raw CLI string, and `process_queue` later has to repair the mess with
`first_line.trim()` / `first_line.trim().is_empty()` at use time.

**How to check.** Manual: every `QueueService` method takes only newtypes;
`grep -n "pub fn" src/domain/queue/service.rs` shows no `&str`/`String`
parameters.

**Source.** OP1, OP4; hex-R13:S18/S20.

---

## Errors

### ERR-01 — `thiserror` only; no `anyhow`, not even in `main`

**Rule.** Every error type in the repo derives `thiserror::Error`. `anyhow`
does not appear in `Cargo.toml` or `src/`, including in `main`.
`Box<dyn Error>` is not a public error type.

**Why.** `Box<dyn Error>` is "god-awful at providing structured data for an
error handler to act on" (err-4), and OP3 is explicit: `thiserror` only.

**Good.**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("failed to spawn `fish -c`: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("`fish -c` exited unsuccessfully: {code}")]
    Waited { code: i32 },
}
```

**Bad.** `fn main() -> anyhow::Result<()>` — which is exactly what the
hexagonal guide's `main` does (hex-R17:S28); see Resolved conflicts.

**How to check.** `grep -rn "anyhow" Cargo.toml src` → empty;
`grep -rn "dyn Error" src` → empty.

**Source.** OP3; err-8:S18/S19, err-4.

### ERR-02 — One error enum per boundary; the service error composes relevant ones

**Rule.** Each port has its own error enum (`QueueStoreError`, `RunError`,
`EditorError`); each adapter translates the failures it knows about into those
variants. The service-level `QueueError` composes only the errors relevant to
its operations via `#[from]`. No single module-wide `CliError` that every
function returns.

**Why.** "Design error types that prioritize relevant information. Minimize
unrelated noise" (err-7:S15), and "each port and adapter boundary has its own
error enum" (OP3).

**Good.**
```rust
#[derive(Debug, Error)]
pub enum QueueError {
    #[error(transparent)]
    Store(#[from] QueueStoreError),
    #[error(transparent)]
    Run(#[from] RunError),
    #[error("command rejected by the domain: {0}")]
    Invalid(#[from] QueueEntryError),
}
```

**Bad.** (current `src/main.rs`) everything returns `io::Result<()>`, so
"queue file missing", "permission denied" and "`fish` failed to spawn" are
indistinguishable to the caller.

**How to check.** Manual: exactly one error enum per file in `src/ports/` and
`src/outbound/`; `QueueError`'s variants number no more than the boundaries it
composes.

**Source.** OP3; err-7:S15, err-8:S19.

### ERR-03 — `main` maps the top-level error to a message and an exit code

**Rule.** `main` is the only place that translates the top-level error into a
human message and a process exit code. Usage/parse failures → 2; domain
violations (e.g. an empty command) and expected I/O failures → 1; success → 0.
What the user sees is built from error *variants*, never by leaking a wrapped
library error's `Display` verbatim as policy.

**Why.** "Always separate your public errors from their domain
representations" (hex-R14:S19) — raw domain errors "leak private details".

**Good.**
```rust
match cli.run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(CliError::BadCommand(e)) => { eprintln!("invalid command: {e}"); ExitCode::from(2) }
    Err(e)                       => { eprintln!("error: {e}");       ExitCode::from(1) }
}
```

**Bad.** (current `src/main.rs`) `fn main() -> io::Result<()>` lets the
`Result`-to-exit-code machinery print raw OS messages, and
`env::var("HOME").expect("HOME environment variable not set")` turns a missing
env var into a panic with a backtrace instead of a message + exit code.

**How to check.** Manual: `fn main` returns `std::process::ExitCode` (or
`std::process::exit`) and contains no `?` propagating a `Result` return.

**Source.** OP3; hex-R14:S19.

### ERR-04 — Codify every expected failure with a catch-all for the rest; never `non_exhaustive`

**Rule.** Port error enums list every failure the caller must handle
(`QueueStoreError::{Io, QueueFileMissing, …}`), plus a final catch-all variant
for anything unclassifiable (e.g. `RunError::Other(std::io::Error)`). Adapters
map every unknown underlying failure into the catch-all instead of panicking.
Application-internal error enums are **not** marked `non_exhaustive`: a new
variant must break the build so it gets handled.

**Why.** Callers of a port get "a complete description of every error scenario
it's expected to handle, and the compiler will make sure that it does"
(hex-R8:S9); "don't panic in response to unexpected errors" (hex-R15:S9).

**Good.**
```rust
// src/outbound/fish.rs — every spawn/wait failure becomes a named variant
.spawn().map_err(RunError::Spawn)?
.wait().map_err(RunError::Wait)?
// unexpected vendor condition → RunError::Other(std::io::Error)
```

**Bad.** (current `src/main.rs`) `open_editor` does
`SystemCommand::new(editor).arg(file_path).status().expect("Failed to open
editor")` — any spawn failure panics; and `ls`'s bare
`fs::read_to_string(&queue_path)?` conflate file-missing with any other I/O
error.

**How to check.** `grep -rn "non_exhaustive" src` → empty; manual: every
`impl <Port> for <Adapter>` maps every fallible call into a named variant.

**Source.** hex-R8:S9, hex-R15:S9; conflicts with err-10:S21/S23 — see Resolved
conflicts §4.

### ERR-05 — Error message strings are not the API

**Rule.** Callers react to error *variants*; nothing in `src/` branches on or
parses the `Display` text of another type's error.

**Why.** Hyrum's Law — Go's `MaxBytesError` message "cannot be changed" because
real programs depend on it (err-6:S10).

**Good.**
```rust
match store.read_all() {
    Err(QueueStoreError::QueueFileMissing(p)) => /* `ls`: empty queue, exit 0 */,
    Err(e) => return Err(QueueError::Store(e)),
}
```

**Bad.**
```rust
if err.to_string().contains("No such file") { /* pretend the queue is empty */ }
```

**How to check.** `grep -rnE "to_string\(\)\.(contains|starts_with)|e\.to_string\(\) ==" src` → empty.

**Source.** err-6:S10/S12.

---

## Types & parsing

### TY-01 — Parse, don't validate

**Rule.** Raw input — the `add` argument, lines read from `commands.txt`, the
`HOME`/`EDITOR` env vars — is converted into domain newtypes at the boundary
through `TryFrom`/`FromStr`. Once constructed, a domain value is valid by
construction; domain and service code never re-check it.

**Why.** "Investing extra time upfront to design datatypes that are always
valid" (new-R2:S8) makes the business functions "all business logic".

**Good.**
```rust
// src/domain/queue/mod.rs
impl FromStr for QueueEntry {
    type Err = QueueEntryError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() { return Err(QueueEntryError::Empty); }
        Ok(Self(trimmed.to_owned()))
    }
}
// `start` can now trust that any QueueEntry it receives is non-blank.
```

**Bad.** (current `src/main.rs`) validation is buried mid-orchestration:
`if !first_line.trim().is_empty() { /* spawn */ }` inside `process_queue`,
with the raw `String` as the working type.

**How to check.** Manual: no `if … .is_empty()` / `if … .trim()` guards in
`src/domain/` or the service; every domain constructor returns `Result`.

**Source.** OP4; new-R2:S8, hex-R5:S6.

### TY-02 — Distinct concepts get distinct types

**Rule.** `QueueEntry` (one queued command line), `QueuePath` (the
`commands.txt` location), and `Editor` (the editor binary) are separate
newtypes. The domain and port APIs take newtypes, never raw `&str`, `String`,
or `PathBuf`.

**Why.** With two `&str` parameters the caller *can* swap them (new-R1:S5/S2) —
e.g. run the editor's name as a queued command, or open the queue path in
`fish -c`.

**Good.**
```rust
pub trait CommandRunner {
    fn run(&self, entry: &QueueEntry) -> Result<(), RunError>;
}
pub trait EditorLauncher {
    fn open(&self, at: &QueuePath) -> Result<(), EditorError>;
}
// You cannot pass `Editor` where `QueueEntry` is expected.
```

**Bad.** (current `src/main.rs`) `fn open_editor(file_path: &str)` plus
`SystemCommand::new(editor).arg(file_path)` — every knob is a bare string, and
nothing stops `start` from being handed the editor name.

**How to check.** `grep -rnE "fn [a-z_0-9]+\((self, )*(&str|String|&Path|PathBuf)" src/domain src/ports` → empty.

**Source.** OP4; new-R1:S5/S2.

### TY-03 — Private fields; constructors and accessors only

**Rule.** Every domain newtype wraps a private field. It exposes its
validating constructor and accessors (`as_str`, `as_path`, `Display`); it has
no `pub` field and no `Default` unless a sensible default actually exists
(one does not for `QueueEntry` or `QueuePath`).

**Why.** With a `pub` field "there's absolutely nothing to stop you" building
`QueueEntry(raw_path_string)` (new-R3:S8; anti-pattern new A2:S6/S7).

**Good.**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry(String);
impl QueueEntry {
    pub fn as_str(&self) -> &str { &self.0 }
}
impl Display for QueueEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}
```

**Bad.**
```rust
pub struct QueueEntry(pub String);
// anyone can now write QueueEntry(String::new()) — an invalid, empty entry —
// and the invariant is gone.
```

**How to check.** `grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/domain` → empty (no `pub` fields).

**Source.** OP5; new-R3:S8, new A2:S6/S7.

### TY-04 — `From` for infallible, `TryFrom` for fallible; one canonical constructor

**Rule.** The validating constructor is the single source of truth; every
`TryFrom`/`From` impl is a thin delegation to it (`FromStr` delegates to
`new`, `TryFrom<&str>` delegates to `FromStr`). No second constructor with
independent validation logic for the same purpose.

**Why.** "A newtype's constructor serves as its source of truth – never have
multiple constructors for the same use case" (new-R9:S16); `From`/`TryFrom`
are the unsurprising bridge to "Other People's Code" (new-R8:S15/S16).

**Good.**
```rust
impl TryFrom<&str> for QueueEntry {
    type Error = QueueEntryError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)   // one line, same rules
    }
}
```

**Bad.** A `new(s: &str)` *and* a `try_from(s: &str)` each with their own
trim/empty checks that can drift apart (new A7).

**How to check.** Manual: every `TryFrom`/`From` impl in `src/domain/` has a
one-line body calling the canonical constructor.

**Source.** OP4; new-R8:S15/S16, new-R9:S16.

### TY-05 — No `Deref` (and no mismatched `Borrow`) on domain newtypes

**Rule.** Domain newtypes do not implement `Deref`. They expose `Display`
(whose blanket `ToString` gives `to_string` for free — never define an
inherent `to_string`) and named accessors. `Borrow<T>` is only acceptable if the
newtype hashes/compares *identically* to `T`.

**Why.** `Deref` "bolts every `&self` method of the wrapped type onto your
newtype" (new A4:S19/S20), and a `Borrow` whose hash/eq disagree with the
wrapped type is "unofficially unsafe" (new A5:S24/S25).

**Good.** `impl Display for QueueEntry` + `QueueEntry::as_str`; `Display`
provides `to_string` automatically.

**Bad.**
```rust
impl Deref for QueueEntry { type Target = str; /* … */ }
// now entry.len(), entry.is_empty(), entry.bytes() all leak through —
// including is_empty() on a type that *cannot* be empty.
// Or: fn to_string(&self) -> String { … }  // shadows Display's — clippy
// flags it (inherent_to_string_shadow_display, new A6:S17).
```

**How to check.** `grep -rn "impl Deref\|impl Borrow" src` → empty; clippy lint
`inherent_to_string_shadow_display` stays clean.

**Source.** new A4:S19/S20, new A5:S24/S25, new A6:S17.

### TY-06 — Derive the standard traits on domain types

**Rule.** Domain newtypes and error enums derive `Debug, Clone, PartialEq, Eq`
plus `Hash`/`PartialOrd`/`Ord` where meaningful. Don't derive `Copy` (our
wrappers hold `String`/`PathBuf`) or `Default` (no sensible default exists).

**Why.** Deriving is cheap and — for a library — required, because of the
Orphan Rule your users can't add these traits themselves (new-R5:S11).

**Good.**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueuePath(PathBuf);
```

**Bad.** A hand-rolled `PartialEq` with no `Debug` (error messages become
`<opaque>` in `thiserror` output), or `#[derive(Default)]` on `QueuePath`
which invents a meaningless path.

**How to check.** Manual: review the derive list on each type in `src/domain/`
against this rule.

**Source.** new-R5:S11.

---

## Dispatch & macros

### DISP-01 — Static dispatch; no `dyn` ports

**Rule.** No `Box<dyn Port>`, `&dyn Port`, or `Arc<dyn Port>` anywhere. Where a
port has multiple implementations (e.g. `FishRunner` and `BashRunner`), define
an enum whose variants hold them, implement the port trait on the enum, and
let `impl_dispatch!` (DISP-02) generate the delegating match. Plain generics
are used only where one concrete type is chosen at compile time (the service's
`R: QueueStore` / `C: CommandRunner` parameters, which each call site
instantiates with one type — including mocks in tests).

**Why.** OP2 is non-negotiable; an enum + trait impl keeps dispatch
compile-time and the match exhaustive (the hexagonal guide instead threads
multi-parameter generics — `Service<R, M, N>` — which OP2 supersedes; see
Resolved conflicts §2).

**Good.**
```rust
pub enum ShellRunner {
    Fish(FishRunner),
    Bash(BashRunner),
}
impl_dispatch!(CommandRunner for ShellRunner { Fish, Bash });
// dispatch is static, exhaustive, and the match arms don't exist by hand.
```

**Bad.**
```rust
struct QueueService { runner: Box<dyn CommandRunner> }
// dynamic dispatch, heap allocation per service, no compile-time guarantee
// that every implementation stays in sync with the port.
```

**How to check.** `grep -rn "dyn " src` → empty.

**Source.** OP2; contrast hex-R2:S16, hex-R16:S27.

### DISP-02 — Delegating match arms are generated by a `macro_rules!` macro

**Rule.** Enum dispatch impls are written by a `macro_rules!` macro that
enumerates the variants: the macro has one rule per call form, expands to a
handful of fundamental calls (one match with one delegating arm per variant),
and keeps helper rules private (`@`-style internal rules or `#[doc(hidden)]`
helper macros).

**Why.** Rust has no variadic parameters, so "generate repeated boilerplate
with a helper macro that enumerates the cases, instead of writing them out by
hand" (mac-6:S16/S17) — the enumeration lives in exactly one place.

**Good.**
```rust
// src/ports/dispatch.rs
macro_rules! impl_dispatch {
    ($port:ident for $enum:ident { $($variant:ident),* $(,)? }) => {
        impl $port for $enum {
            fn run(&self, entry: &QueueEntry) -> Result<(), RunError> {
                match self {
                    $( Self::$variant(inner) => inner.run(entry), )*
                }
            }
        }
    };
}

// src/outbound/runners.rs
impl_dispatch!(CommandRunner for ShellRunner { Fish, Bash });
// expands to the exhaustive impl — adding a `Zsh(ZshRunner)` variant to the
// enum is a one-token change; the compiler catches a forgotten trait method.
```

**Bad.** Hand-writing the delegating `match` per trait method per enum and
forgetting a variant (silent drift, or a `todo!()` in the arm).

**How to check.** Manual: every multi-implementation enum has exactly one
`impl <Port> for <Enum>` and it comes from an `impl_dispatch!` call;
`grep -rn "todo!\|unimplemented!" src` → empty.

**Source.** OP2; mac-6:S16/S17, mac-1:S2, mac-5:S10/S11.

### DISP-03 — Macro hygiene: bind captured fragments once; qualify dependencies

**Rule.** Any macro that captures expression fragments binds them to locals
before using them. Any macro that is exported from the crate references its
dependencies through `$crate::` (fully qualified) or imports them inside the
rule body.

**Why.** Fragments "could be expensive function calls" that you must not
evaluate twice (mac-2:S3), and with `#[macro_export]` "there are no guarantees
about what is in scope at the call site" (mac-3:S4): "Use a fully qualified
path to declarative macro dependencies, or import them in the body of the
macro."

**Good.** (mac-2:S3 pattern)
```rust
match (&$left, &$right) { (left_val, right_val) => /* one evaluation each */ }
```

**Bad.** Substituting `$left` twice into the expansion (re-running a possibly
expensive expression), or an exported macro that calls bare
`panicking::assert_failed` relying on the caller's scope.

**How to check.** Manual at macro introduction: the expansion (`cargo expand`
or `rustc -Zunpretty=expanded`) shows each captured expression exactly once.

**Source.** mac-2:S3, mac-3:S4.

### DISP-04 — Edge cases get an explicit rule

**Rule.** The degenerate form of a macro (a zero-variant enum, a zero-argument
call) must have defined behavior via an explicit rule — either a sensible
expansion or a deliberate, named compile error — never an accidental
zero-repetition expansion with a cryptic error.

**Why.** `join!()` with no arguments gets "its own explicit rule" and is
treated as "a first-class, intentional branch, not an accident" (mac-14:S18).

**Good.**
```rust
macro_rules! impl_dispatch {
    // …normal rule…
    () => { compile_error!("impl_dispatch! needs at least one variant") };
}
```

**Bad.** `$( Self::$variant(inner) => …, )*` over an empty variant list
silently producing `match self {}` and a confusing "unmatched pattern" error.

**How to check.** Manual: invoke the macro with the empty form and confirm the
error message names the macro and the missing input.

**Source.** mac-14:S18.

### DISP-05 — Reduce every expansion to fundamental calls

**Rule.** A macro body expands to a handful of fundamental constructs (one
`match` with one method call per arm) — not to nested, generated control flow
of its own.

**Why.** "Reducing complexity to a handful of fundamental function calls is a
hallmark of a well-written macro" (mac-5:S26).

**Good.** `impl_dispatch!` expanding to exactly one `impl` block whose bodies
are single `match`es (DISP-02's expansion).

**Bad.** A dispatch macro that additionally generates loop wrappers,
intermediate `Option` plumbing, or re-invokes itself — behavior that no
reader can verify by reading one expansion.

**How to check.** `cargo expand` the macro; the expansion is readable in one
pass and contains no generated logic beyond the match arms.

**Source.** mac-5:S26.

---

## Testing

### TST-01 — Test the service and adapters by injecting mock ports

**Rule.** `QueueService` behavior is unit-tested with in-crate `#[cfg(test)]`
mock implementations of `QueueStore` / `CommandRunner` / `EditorLauncher`;
no test touches `$HOME/.config/encode_queue`. File-adapter tests use temp
directories; runner/editor adapter tests use fake processes (e.g.
`$EDITOR=true`).

**Why.** Because handlers depend only on the port, a mock "can stand in for a
real database, letting you test the handler's mapping of repository results"
(hex-R12:S21/S22) — "impossible with the hard-wired version" (R1/R2).

**Good.**
```rust
#[cfg(test)]
mod tests {
    struct MockStore { lines: std::sync::Arc<std::sync::Mutex<Vec<QueueEntry>>> }
    impl QueueStore for MockStore { /* canned results, no real file */ }

    #[test]
    fn start_removes_entry_after_successful_run() {
        // build QueueService<MockStore, RecordingRunner>, run one entry,
        // assert the mock store's lines shrank by exactly one.
    }
}
```

**Bad.** Testing `start` by writing a real `commands.txt` under `$HOME` (or
the current code, which is untestable: `process_queue` hard-codes the path and
spawns `fish`).

**How to check.** `grep -rn "#\[cfg(test)\]" src/domain/queue` is non-empty, and
no test contains `.config/encode_queue` or spawns `fish`.

**Source.** OP1; hex-R12:S21/S22.

### TST-02 — Each validating constructor gets focused success/failure tests

**Rule.** Every `new`/`FromStr` in the domain has direct unit tests for the
accept and reject cases (`QueueEntry::from_str("   ")` → `Empty`;
`QueueEntry::from_str("nix build .")` → `Ok`).

**Why.** Moving validation into constructors is what makes tests "focused" —
the article's `create_user` no longer calls `validate_email`, and its tests
shrink accordingly (new-R2:S9); the sprawl they remove is the combinatorial
matrix of new A1:S4.

**Good.**
```rust
#[test]
fn from_str_rejects_blank_lines() {
    assert_eq!(QueueEntry::from_str("   "), Err(QueueEntryError::Empty));
}
```

**Bad.** One giant integration test exercising `add` + validity + file I/O +
`fish` spawn, so every failure mode is tested in permutations (new A1:S4).

**How to check.** Manual: for each parse-error variant, there is a test that
asserts it specifically (not via `Display` text — ERR-05).

**Source.** new-R2:S8/S9, new A1:S4.

### TST-03 — `unwrap`/`expect` are acceptable in tests

**Rule.** `unwrap`/`expect` are permitted inside `#[cfg(test)]` code (asserting
a known-good fixture is the point), and prohibited everywhere else (GEN-01).

**Why.** OP6 scopes the ban to non-test code; the research's own test snippets
rely on it (hex-R12:S22 uses `.unwrap()` throughout its handler tests).

**Good.** `let entry = QueueEntry::from_str("nix build").unwrap();` inside a
`#[test]` building a fixture.

**Bad.** The same call in `inbound/cli.rs` (violates GEN-01).

**How to check.** The lint setup in GEN-01; `grep -rnE "\.unwrap\(\)|\.expect\(" src | grep -v "#\[cfg(test)\]"` → only test modules.

**Source.** OP6; hex-R12:S22.

---

## General

### GEN-01 — No `unwrap`/`expect`/`panic!`/`unreachable!` outside tests

**Rule.** Non-test code never calls `unwrap`, `expect`, `panic!`, or
`unreachable!`. Unexpected I/O becomes a catch-all error variant (ERR-04);
missing env vars become a `ConfigError` variant.

**Why.** OP6 is non-negotiable, and panicking "poisons held mutexes" and
"Other Rust devs won't expect you to panic" (hex-R15:S9).

**Good.**
```rust
let home = std::env::var("HOME").map_err(ConfigError::HomeUnset)?;
let Some(cmd) = matches.get_one::<String>("COMMAND") else {
    return Err(CliError::MissingArgument("COMMAND".into()));
};
```

**Bad.** (current `src/main.rs`, four violations in one file)
`env::var("HOME").expect(…)`, `get_one::<String>(…).unwrap()`,
`_ => unreachable!()` after the subcommand match, and
`status().expect("Failed to open editor")`.

**How to check.** Crate root: `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`
plus `grep -rnE "\.unwrap\(\)|\.expect\(|panic!|unreachable!" src/ | grep -v "cfg(test)"` → empty.

**Source.** OP6; hex-R15:S9.

### GEN-02 — Synchronous ports; no async/`Arc` supertrait stack

**Rule.** Port methods are plain synchronous functions returning `Result`.
Port traits carry no `impl Future<…> + Send` return types, no mandated
`Clone + Send + Sync + 'static` supertraits, and the codebase has no `tokio`
or `Arc`-shared state. (The article derives that whole stack from a
multi-threaded web server; see Resolved conflicts §5.)

**Why.** A single-threaded CLI gets nothing from async bounds and pays for
them in every mock (hex-R12:S21 shows the `Arc<Mutex<Result<…>>>` mock weight
that `Clone` + non-`Clone` error types force).

**Good.**
```rust
pub trait QueueStore: Clone + 'static {
    fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError>;
    fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError>;
}
```

**Bad.**
```rust
pub trait CommandRunner: Clone + Send + Sync + 'static {
    fn run(&self, entry: &QueueEntry)
        -> impl Future<Output = Result<(), RunError>> + Send;
}
```
for a `fish -c` CLI.

**How to check.** `grep -rnE "tokio|async fn|impl Future" src` → empty.

**Source.** OP1 (CLI shape); adapted from hex-R10:S14/S15, hex-R11:S16.

### GEN-03 — Error types are `'static` with owned data

**Rule.** Error variants carry owned payloads (`String`, `PathBuf`,
`std::io::Error`), never borrowed references.

**Why.** Errors are handled long after the producing code returns, "sometimes
on a different thread"; a borrowed field makes the error
`QuestionablyStatic` and un-downcastable (err-1:S2/S3).

**Good.** `#[error("queue file not found at {0}")] QueueFileMissing(PathBuf)`.

**Bad.** `QueueFileMissing(&'a Path)` — the error only compiles when the path
happens to outlive the call site.

**How to check.** Manual: `grep -rnE "impl(Debug|Display|Error)" -A 15 src/ports` shows no `&` payloads in variant fields.

**Source.** err-1:S2/S3.

### GEN-04 — Clippy-clean on every change

**Rule.** `cargo fmt` is run and `cargo clippy --all-targets --all-features -- -D warnings`
passes on every change; `cargo test` passes. Lints like
`inherent_to_string_shadow_display` (TY-05) and `unwrap_used`/`expect_used`
(GEN-01) are enforced, not silenced.

**Why.** A two-crate-free CLI should carry zero `#[allow]` debt; suppression
comments hide exactly the violations the other conventions forbid.

**Good.** `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`
at the crate root; no `#[allow]` in `src/`.

**Bad.** `#[allow(clippy::unwrap_used)]` sprinkled over `open_editor` to keep
the current `.expect(…)` compiling.

**How to check.** `cargo clippy --all-targets --all-features -- -D warnings` exits clean;
`grep -rn "#\[allow" src` → empty.

**Source.** repo toolchain; supports OP6 and TY-05.

---

## Resolved conflicts

Owner preferences always win. Every place an article disagrees with an owner
preference (or with another article where the outcome affects this repo):

1. **`anyhow` everywhere vs `thiserror` only.** The hexagonal guide's catch-all
   error wraps `anyhow::Error` (hex-R8:S9), its adapters "may return
   `anyhow::Result` freely" outside traits (hex-R9:S8), and its `main` returns
   `anyhow::Result<()>` (hex-R17:S28); the error-handling article presents
   "a helper like `anyhow`" as the fix for `Box<dyn Error>` (err-3) and says
   `thiserror` and `anyhow` "can coexist" (err-8). **OP3 wins:** no `anyhow`
   anywhere, not even in `main`; catch-alls wrap concrete types such as
   `std::io::Error` (ERR-01, ERR-04).

2. **Multi-parameter generics vs enum dispatch.** The hexagonal guide threads
   every port as a generic parameter — `AppState<AR: AuthorRepository>`,
   `Service<R, M, N>` (hex-R2:S16, hex-R11:S16/S25, hex-R16:S27) — so the
   assembly of implementations is the generic instantiation. **OP2 wins:**
   when a port has several implementations, an enum holds them and the macro
   generates the delegating `match`; generics are reserved for the single
   concrete type chosen at compile time (DISP-01, DISP-02).

3. **`unwrap` in production code.** The guide's canonical `Service`
   implementation calls `result.as_ref().unwrap()` (hex-R16:S27), and the
   current `src/main.rs` uses `expect`/`unwrap`/`unreachable!` four times.
   **OP6 wins:** the service propagates the error (e.g. via
   `match`/`as_ref().ok_or_else`) instead of panicking; `main` returns an
   `ExitCode` (GEN-01).

4. **`non_exhaustive` error enums (article vs article).** The error-handling
   article recommends marking public error enums `non_exhaustive` to reserve
   the right to grow (err-10:S21/S23); the hexagonal guide explicitly advises
   *not* marking application-internal error enums `non_exhaustive`, because
   "you *want* new variants to break the build so they get handled",
   reserving `non_exhaustive` for libraries (hex-R8). **The hexagonal guide
   wins here:** this repo is an application, so error enums stay exhaustive
   and unmarked (ERR-04).

5. **Async/`Arc`/`Send+Sync+Clone` supertrait stack.** The guide mandates
   `Clone + Send + Sync + 'static` ports with `impl Future<…> + Send` methods
   and `Arc`-shared state (hex-R10:S14/S15, hex-R11:S16) because its handlers
   run on a multi-threaded web server. **OP1's shape wins:** `encode_queue`
   is a single-threaded synchronous CLI, so ports are plain sync methods and
   no `Arc` is needed; the supertrait stack is dropped (GEN-02).

6. **`Box<dyn Error>` as the escape hatch.** The error-handling article blesses
   `Box<dyn Error>` for "telling a human something went wrong" (err-4).
   **OP3 wins:** our boundaries use structured `thiserror` enums with
   concrete catch-all payloads, so there is no dynamic error in the public
   surface at all (ERR-01).

---

## Target layout

The concrete `src/` tree this repo should end up with. It follows the
hexagonal guide's file patterns (`config`, `domain/<name>/service`,
`inbound/<adapter>`, `outbound/<adapter>` — hex-R17:S29) plus a dedicated
`ports/` layer, as OP1 lists ports (traits) as their own layer; the guide
embeds port traits in the domain modules, but OP1's explicit layering wins the
layout.

```
src/
├── main.rs                    # bootstrap only: config → adapters → service → CLI
│                              # (ARCH-02); maps CliError → message + ExitCode (ERR-03)
├── config.rs                  # env (HOME, EDITOR) → domain newtypes; ConfigError
├── domain/
│   └── queue/
│       ├── mod.rs             # QueueEntry, QueuePath, Editor newtypes (TY-01..TY-06);
│       │                      # QueueEntryError; zero third-party imports (ARCH-01)
│       └── service.rs         # application/service layer: QueueService<R, C> (ARCH-01);
│                              # QueueError composes boundary errors (ERR-02); tests w/ mocks (TST-01)
├── ports/                     # traits + per-port error enums (ARCH-01, ERR-02)
│   ├── mod.rs
│   ├── queue_store.rs         # trait QueueStore + QueueStoreError
│   ├── command_runner.rs      # trait CommandRunner + RunError
│   ├── editor.rs              # trait EditorLauncher + EditorError
│   └── dispatch.rs            # impl_dispatch! macro (DISP-02..DISP-05)
├── inbound/
│   └── cli.rs                 # clap adapter: subcommands → newtypes → service calls;
│                              # CliError (ARCH-04, ERR-03)
└── outbound/
    ├── mod.rs
    ├── queue_file.rs          # FileQueueStore: read/append/rewrite
    │                          # $HOME/.config/encode_queue/commands.txt
    ├── fish.rs                # FishRunner: fish -c <entry> (ARCH-03, ERR-04)
    └── editor.rs              # EditorProcess: $EDITOR (default nvim)
```

Dependency direction (all arrows point inward): `main` → `inbound`,
`outbound`, `config`; `inbound` → `service`, `ports`, `domain`;
`outbound` → `ports`, `domain`; `service` → `ports`, `domain`;
`ports` → `domain`; `domain` → (nothing).
