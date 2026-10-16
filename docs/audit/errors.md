# Error-handling convention audit (ERR family + panic/unwrap rule)

**Task:** P3.A2 — audit of `dflash-k8v4-run1`
**Scope:** `docs/CONVENTIONS.md` conventions **ERR-01 … ERR-05** plus the
panic/unwrap rule **GEN-01** (owner preference **OP6**: no `unwrap`/`expect`/
`panic!`/`unreachable!` outside tests). All other convention families
(ARCH/TY/DISP/TST/GEN-02…04) are out of scope.
**Tree audited:** `src/main.rs` (116 lines, the entire `src/` tree),
`Cargo.toml` (deps: `clap = "4.5.4"` only).
**Nature:** docs-only. `src/` was not modified; P5 decides what to fix.

## Sweep commands and raw results

ERR-family How-to-check commands run from the repo root:

```console
$ grep -rn "anyhow" Cargo.toml src
(no output)
$ echo $?
1

$ grep -rn "dyn Error" src
(no output)
$ echo $?
1

$ grep -rn "non_exhaustive" src
(no output)
$ echo $?
1

$ grep -rnE "to_string\(\)\.(contains|starts_with)|e\.to_string\(\) ==" src
(no output)
$ echo $?
1
```

Manual-check evidence:

```console
$ grep -rn "enum" src
(no output)   # exit 1 — the codebase contains ZERO error enums
$ grep -n "fn main" src/main.rs
10:fn main() -> io::Result<()> {
$ grep -n "fn process_queue\|fn open_editor" src/main.rs
62:fn process_queue(queue_path: &Path) -> io::Result<()> {
107:fn open_editor(file_path: &str) -> std::io::Result<()> {
```

Test-aware unwrap/panic sweep. GEN-01's How-to-check prescribes the crate-root
attribute `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
clippy::panic, clippy::unreachable))]`; the current crate root has no such
attribute and `src/` is off-limits in this docs-only task, so the equivalent
lints were applied on the command line instead. The tree contains no
`#[cfg(test)]` code at all (`grep -rn "cfg(test)\|#\[test\]" src` → exit 1),
so for this tree the command-line form is exactly equivalent to the
test-aware `cfg_attr` form:

```console
$ cargo clippy --all-targets --all-features -- \
    -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::unreachable
error: used `expect()` on a `Result` value
  --> src/main.rs:31:20

error: used `unwrap()` on an `Option` value
  --> src/main.rs:39:27

error: used `unwrap()` on an `Option` value
  --> src/main.rs:54:25

error: usage of the `unreachable!` macro
  --> src/main.rs:56:14

error: used `expect()` on a `Result` value
   --> src/main.rs:110:5

error: could not compile `encode_queue` (bin "encode_queue") due to 5 previous errors
(clippy exit code: 101 — build failed)
```

Exactly five OP6 sites, matching GEN-01's own "Bad" enumeration of the current
`src/main.rs` ("five violations in one file").

## Violations

### V-ERR-01 — `expect` on `env::var("HOME")` panics with a backtrace

- **Convention:** GEN-01 (OP6)
- **Location:** `src/main.rs:31`
- **Evidence:**
  ```rust
  let home_dir = env::var("HOME").expect("HOME environment variable not set");
  ```
- **Severity:** `high` — a missing `HOME` (cron, container, trimmed env) turns
  into a panic with a backtrace instead of a message + exit code; GEN-01 says
  missing env vars must become a `ConfigError` variant.
- **Suggested fix:** `let home = env::var("HOME").map_err(ConfigError::HomeUnset)?;`
  with `ConfigError` a `thiserror` enum (per GEN-01's Good example and
  `config.rs` in the Target layout).

### V-ERR-02 — `unwrap` on the clap `COMMAND` value

- **Convention:** GEN-01 (OP6)
- **Location:** `src/main.rs:39`
- **Evidence:**
  ```rust
  let command = sub_matches.get_one::<String>("COMMAND").unwrap();
  ```
- **Severity:** `med` — `required(true)` on the `Arg` means clap currently
  guarantees presence, so it is unlikely to fire; still an absolute GEN-01
  violation and a panic landmine if the arg ever becomes optional.
- **Suggested fix:**
  `let Some(cmd) = matches.get_one::<String>("COMMAND") else { return Err(CliError::MissingArgument("COMMAND".into())); };`
  (GEN-01's Good example), or fold the `TryFrom<&str>` conversion into
  `CliError::BadCommand` per ERR-02/ARCH-04.

### V-ERR-03 — `unwrap` on `queue_path.to_str()` in the `edit` arm

- **Convention:** GEN-01 (OP6)
- **Location:** `src/main.rs:54`
- **Evidence:**
  ```rust
  open_editor(queue_path.to_str().unwrap())?;
  ```
- **Severity:** `low` — only fires on a non-UTF-8 `HOME`-derived path, but it
  is a banned `unwrap` in production code.
- **Suggested fix:** map `Option<&str>` to an error variant
  (e.g. `QueuePathNonUtf8(PathBuf)`) and propagate with `?`; or, once
  `QueuePath` exists (TY-02), hand the newtype to the `EditorLauncher` port
  instead of a raw `&str`.

### V-ERR-04 — `unreachable!()` after the subcommand match

- **Convention:** GEN-01 (OP6)
- **Location:** `src/main.rs:56`
- **Evidence:**
  ```rust
  _ => unreachable!(),
  ```
- **Severity:** `low` — with `subcommand_required(true)` and a closed set of
  four subcommands the arm is unreachable in practice, but GEN-01 bans the
  macro outright outside tests.
- **Suggested fix:** make the match exhaustive over the real variants only, or
  map the unexpected branch to a `CliError` variant instead of panicking.

### V-ERR-05 — `expect` on the editor's `status()`; any spawn failure panics

- **Convention:** GEN-01 (OP6), ERR-04
- **Location:** `src/main.rs:110–113`
- **Evidence:**
  ```rust
  SystemCommand::new(editor)
      .arg(file_path)
      .status()
      .expect("Failed to open editor");
  ```
- **Severity:** `high` — this is ERR-04's named Bad case: "any spawn failure
  panics". If `$EDITOR` names a nonexistent binary, the user gets a backtrace
  instead of an `EditorError` message.
- **Suggested fix:** translate into the port enum:
  `.status().map_err(EditorError::Spawn)?` with a catch-all variant for
  unclassifiable failures (ERR-04).

### V-ERR-06 — No error enum per boundary; everything is bare `io::Result<()>`

- **Convention:** ERR-02 (and OP3)
- **Location:** `src/main.rs:10`, `src/main.rs:62`, `src/main.rs:107`
  (repo-wide: `grep -rn "enum" src` → no output, exit 1)
- **Evidence:**
  ```rust
  fn main() -> io::Result<()> { … }
  fn process_queue(queue_path: &Path) -> io::Result<()> { … }
  fn open_editor(file_path: &str) -> std::io::Result<()> { … }
  ```
- **Severity:** `high` — this is ERR-02's exact Bad case: "everything returns
  `io::Result<()>`, so 'queue file missing', 'permission denied' and 'fish
  failed to spawn' are indistinguishable to the caller". No `QueueStoreError`,
  `RunError`, `EditorError`, `QueueError`, or `CliError` exists.
- **Suggested fix:** introduce the per-port error enums in `src/ports/*`
  (Target layout: `queue_store.rs` → `QueueStoreError`,
  `command_runner.rs` → `RunError`, `editor.rs` → `EditorError`) plus
  `QueueEntryError` in the domain, and compose them in `QueueError` via
  `#[from]` (ERR-02's Good example).

### V-ERR-07 — `main` returns `io::Result<()>` and propagates raw OS errors with `?`

- **Convention:** ERR-03
- **Location:** `src/main.rs:10`, with `?` at `src/main.rs:35`,
  `src/main.rs:47`, `src/main.rs:50`, `src/main.rs:54`
- **Evidence:**
  ```rust
  fn main() -> io::Result<()> {
  …
      fs::create_dir_all(queue_dir)?;
  …
          process_queue(&queue_path)?;
  …
          let contents = fs::read_to_string(&queue_path)?;
  ```
- **Severity:** `high` — ERR-03's How-to-check ("`fn main` returns
  `std::process::ExitCode` … and contains no `?` propagating a `Result`
  return") fails on both counts. The `Result`-to-`ExitCode` machinery prints
  the raw `io::Error` `Display` verbatim as policy, and usage failures (exit
  2) cannot be distinguished from I/O failures (exit 1).
- **Suggested fix:** `fn main() -> ExitCode` that matches the top-level
  `CliError`/`QueueError` variants and maps them to a message + exit code
  (ERR-03's Good example; usage/parse → 2, domain + expected I/O → 1).

### V-ERR-08 — Expected I/O failures not codified as variants (conflated `ls` read)

- **Convention:** ERR-04
- **Location:** `src/main.rs:50` (and generally `src/main.rs:62–105`,
  `src/main.rs:107–116`)
- **Evidence:**
  ```rust
  let contents = fs::read_to_string(&queue_path)?;
  ```
- **Severity:** `med` — this is ERR-04's named Bad case: "`ls`'s bare
  `fs::read_to_string(&queue_path)?` conflates file-missing with any other
  I/O error", so `ls` cannot implement "queue file missing → empty queue,
  exit 0" (ERR-05's Good example / TST). Likewise every fallible call in
  `process_queue`/`open_editor` (`File::open` at lines 64/82/99, `spawn` at
  line 77, `status` at line 112) maps straight to a raw `std::io::Error`
  instead of a named variant or catch-all.
- **Suggested fix:** a `QueueStoreError::{Io, QueueFileMissing(PathBuf), …}`
  enum with a catch-all, with the file adapter matching on the `io::Error`
  kind and translating into named variants (ERR-04's Good example).

### V-ERR-09 — KNOWN BUG: `start` drops the exit status of every `fish -c` command

- **Convention:** ERR-04 (supporting: ERR-02)
- **Location:** `src/main.rs:79` (`child.wait()?`), with the consequence at
  `src/main.rs:86–95`
- **Evidence:**
  ```rust
  let mut child = SystemCommand::new("fish")
      .arg("-c")
      .arg(first_line.trim())
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .spawn()?;                       // src/main.rs:72–77

  child.wait()?;                       // src/main.rs:79  <-- ExitStatus dropped here
  ```
  ```rust
  if all_lines.len() > 1 {
      let remaining_lines = &all_lines[1..];   // src/main.rs:86–87: first line
      …                                            // removed unconditionally
      for line in remaining_lines {
          writeln!(file, "{}", line)?;           // src/main.rs:93–95: rewritten
      }
  }
  ```
- **Severity:** `high` — `Command::wait()` returns
  `io::Result<std::process::ExitStatus>`; the `?` at line 79 propagates only
  the I/O `Error` and the `Ok(ExitStatus)` payload is discarded. A queued
  command exiting non-zero is treated as successful: the failure is never
  surfaced through any error type, the queue keeps processing the remaining
  lines, and the line is removed from `commands.txt` (lines 86–95) as if it
  had succeeded. The whole `start` run reports success (exit 0) even when
  every queued command failed. This is the failure mode ERR-04 exists to
  codify (`RunError::Waited { code: i32 }` / catch-all) and there is no
  `RunError` enum at all (V-ERR-06). **P5 decides whether to fix; P3 only
  flags.**
- **Suggested fix (for P5's consideration):**
  `let status = child.wait().map_err(RunError::Wait)?;` then
  `if !status.success() { return Err(RunError::Waited { code: … }); }` (map the
  absent-code case to the catch-all instead of `unwrap_or`, per ERR-04)
  and only remove the queue line on success (per ERR-02/ERR-04 and TST-01's
  `start_removes_entry_after_successful_run`).

## Clean

Conventions in the audited family that were checked and found satisfied:

- **ERR-01 — `thiserror` only; no `anyhow`, no `dyn Error`.** SATISFIED
  (vacuously: no error types exist yet, and no forbidden ones do). Evidence:
  `grep -rn "anyhow" Cargo.toml src` → no output, exit 1; `grep -rn "dyn
  Error" src` → no output, exit 1; `Cargo.toml` lists only `clap = "4.5.4"`.
- **ERR-05 — error message strings are not the API.** SATISFIED. Evidence:
  `grep -rnE "to_string\(\)\.(contains|starts_with)|e\.to_string\(\) =="
  src` → no output, exit 1; no call site branches on any error's `Display`
  text.

Not clean (violated — see findings above): ERR-02 (V-ERR-06, V-ERR-09),
ERR-03 (V-ERR-07), ERR-04 (V-ERR-05, V-ERR-08, V-ERR-09), GEN-01
(V-ERR-01…V-ERR-05). Note for ERR-04's second half ("never
`non_exhaustive`"): `grep -rn "non_exhaustive" src` → no output, exit 1, so
the never-`non_exhaustive` requirement is met; the violation is the
missing-variants half of the rule.
