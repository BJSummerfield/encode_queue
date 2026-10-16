# ARCH convention audit — `dflash-k8v4-run1` (round 1)

Scope: **only** the `ARCH-01..04` family (architecture & layout — hexagonal
layering, dependency direction inward, module layout vs the Target layout).
`docs/CONVENTIONS.md` (985 lines) and the entire `src/` (`src/main.rs`, 116
lines) plus `Cargo.toml` were read in full. This is a docs-only audit; `src/`
was not modified.

## Sweep

The ARCH **How-to-check** commands were run against the tree as the sweep.

The two grep-based checks (ARCH-01, ARCH-03) anchor on `src/(domain|ports)/`
and therefore report "clean" (empty) **vacuously** — those directories do not
exist yet, so there is nothing inside them that could leak `clap`,
`std::process`, `std::fs`, or `OpenOptions`. The empty result is the expected
pre-layout "no matches" exit 1 called out in the convention text, **not** a
pass: it says nothing about the fact that the required layers are absent
entirely. That absence is itself the finding.

```
$ grep -rnE "clap|std::process|std::fs" src/ | grep -E "^src/(domain|ports)/"; echo "exit=$?"
# (no output)
exit=1

$ grep -rnE "clap|std::process|OpenOptions" src/ | grep -E "^src/(domain|ports)/"; echo "exit=$?"
# (no output)
exit=1

$ ls -R src/
src/:
main.rs

$ ls -d src/domain src/ports
ls: cannot access 'src/domain': No such file or directory
ls: cannot access 'src/ports': No such file or directory
exit=2
```

The manual checks (ARCH-02, ARCH-04) fail directly on the current `src/main.rs`.

---

## Violations

### V-ARCH-01 — Single-file CLI; the four-layer split does not exist
- **Convention:** ARCH-01 — Four layers; dependencies point inward
- **file:line:** `src/main.rs:1-116` (the whole file); layers absent at `src/`
- **Evidence (quoted):** the whole program is one file; the two How-to-check
  greps return empty and `ls -d src/domain src/ports` reports
  `No such file or directory` (exit 2). All three concerns the convention keeps
  apart sit side by side:
  - `src/main.rs:1` — `use clap::{Arg, ArgAction, Command};`
  - `src/main.rs:4` — `fs::{self, File, OpenOptions},`
  - `src/main.rs:7` — `process::{Command as SystemCommand, Stdio},`
  - clap parse at `src/main.rs:11` (`let matches = Command::new("Encode Queue")`),
    `fs::create_dir_all` at `src/main.rs:35`, `SystemCommand::new("fish")` at
    `src/main.rs:72` — exactly the "Bad" case quoted in ARCH-01.
- **Severity:** high
- **Suggested fix (layout only):** create the Target-layout tree —
  `src/domain/queue/{mod.rs,service.rs}`, `src/ports/{mod.rs,queue_store.rs,command_runner.rs,editor.rs}`,
  `src/inbound/cli.rs`, `src/outbound/{queue_file.rs,fish.rs,editor.rs}` — so
  that `domain`/`ports` import nothing and dependencies point inward.

### V-ARCH-02 — `main` is not bootstrap-only
- **Convention:** ARCH-02 — `main` is bootstrap only
- **file:line:** `src/main.rs:10-60`
- **Evidence (quoted):** `main` is 51 lines (over the ~40-line budget) and does
  everything the convention forbids inline. From the manual check: it matches
  subcommands itself, does file I/O, and spawns nothing *through a port*:
  - `src/main.rs:10` — `fn main() -> io::Result<()> {`
  - `src/main.rs:35` — `fs::create_dir_all(queue_dir)?;`
  - `src/main.rs:43` — `.open(&queue_path)?;` (in the `add` arm)
  - `src/main.rs:47` — `process_queue(&queue_path)?;`
  - `src/main.rs:50` — `let contents = fs::read_to_string(&queue_path)?;`
  - `src/main.rs:54` — `open_editor(queue_path.to_str().unwrap())?;`
- **Severity:** high
- **Suggested fix (layout only):** reduce `main` to config → construct concrete
  outbound adapters → wire the service → call the inbound CLI adapter's `run` →
  map the top-level error to a message + `ExitCode` (see ARCH-02 "Good" block).

### V-ARCH-03 — Third-party / OS types are not wrapped at adapter boundaries
- **Convention:** ARCH-03 — Adapters wrap third-party types and expose only what the application needs
- **file:line:** `src/main.rs:39,40-44,62,72,107-113`
- **Evidence (quoted):** clap, `std::process::Command`, and `OpenOptions`
  details live in the top-level file rather than in `inbound/`/`outbound/`, and
  the helper functions expose bare `io::Result`:
  - `src/main.rs:39` — `let command = sub_matches.get_one::<String>("COMMAND").unwrap();`
    (raw `clap` `ArgMatches` handling)
  - `src/main.rs:40-43` — `let mut file = OpenOptions::new().create(true).append(true).open(&queue_path)?;`
  - `src/main.rs:62` — `fn process_queue(queue_path: &Path) -> io::Result<()> {`
  - `src/main.rs:72` — `let mut child = SystemCommand::new("fish")`
  - `src/main.rs:107` — `fn open_editor(file_path: &str) -> std::io::Result<()> {`
  - `src/main.rs:110` — `SystemCommand::new(editor)`
- **Severity:** high
- **Suggested fix (layout only):** move clap handling into `inbound/cli.rs`,
  `std::process::Command` into `outbound/fish.rs`/`outbound/editor.rs`, and
  `OpenOptions` into `outbound/queue_file.rs`; have ports expose only domain
  types + boundary error enums.

### V-ARCH-04 — No translation at the edges; raw `&str`/`String` is the working type
- **Convention:** ARCH-04 — Translate at the edges
- **file:line:** `src/main.rs:44,71,62,107`
- **Evidence (quoted):** the raw CLI string is appended verbatim with no
  newtype in between, and business logic later repairs it with `trim()`/
  `is_empty()`:
  - `src/main.rs:44` — `writeln!(file, "{}", command)?;` where `command` is the
    raw `&String` from `get_one::<String>` (line 39)
  - `src/main.rs:71` — `if !first_line.trim().is_empty() {`
  - `src/main.rs:62` — `fn process_queue(queue_path: &Path) -> io::Result<()> {`
    (business helper takes raw `&Path`)
  - `src/main.rs:107` — `fn open_editor(file_path: &str) -> std::io::Result<()> {`
    (business helper takes raw `&str`)
- **Severity:** high
- **Suggested fix (layout only):** convert the raw `add` argument to a
  `QueueEntry` newtype in `inbound/cli.rs` before calling the service; pass
  newtypes (`QueueEntry`, `QueuePath`) through service and port signatures.

### V-ARCH-05 — Queue path computed and passed around in `main` (known bug)
- **Convention:** ARCH-01 / ARCH-02 — the queue path does not live at a
  boundary/domain boundary as the Target layout requires (`config.rs` →
  `QueuePath`, held by `FileQueueStore` in `outbound/queue_file.rs`); instead it
  is computed in `main` and threaded into free functions.
- **file:line:** `src/main.rs:31-33,47,50,54`
- **Evidence (quoted):**
  - `src/main.rs:31` — `let home_dir = env::var("HOME").expect("HOME environment variable not set");`
  - `src/main.rs:32` — `let queue_dir = PathBuf::from(home_dir).join(".config/encode_queue");`
  - `src/main.rs:33` — `let queue_path = queue_dir.join("commands.txt");`
  - passed to free functions: `src/main.rs:47` `process_queue(&queue_path)?;`,
    `src/main.rs:50` `fs::read_to_string(&queue_path)?;`,
    `src/main.rs:54` `open_editor(queue_path.to_str().unwrap())?;`
- **Observed behaviour (empirically verified, `HOME` isolated to a temp dir):**
  - `ls` on a missing queue prints a raw `std::io::Error` to stderr and exits 1:
    ```
    $ encode_queue ls
    Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
    $ echo $?
    1
    ```
  - `start` on an **empty** queue (file present, zero bytes) prints nothing and
    exits 0:
    ```
    $ : > $HOME/.config/encode_queue/commands.txt
    $ encode_queue start
    $ echo $?
    0
    ```
    > Note: `start` on a *missing* file (dir created by the `create_dir_all`
    > at `src/main.rs:35`, file absent) currently also prints the raw `Error:
    > Os { code: 2, ... }` and exits 1 (see `Decisions`). The spec's
    > "missing *or* empty … exits 0" holds for the empty case.
- **Severity:** high
- **Suggested fix (layout only — no behaviour change; owner choice is P6):**
  own the queue path in `config.rs` as a `QueuePath` newtype and hold it inside
  `FileQueueStore` (outbound), so `main` no longer computes or passes a raw
  `PathBuf`; the missing/empty-queue distinction then becomes a `QueueStoreError`
  variant handled at the boundary rather than a leaked `std::io::Error`.

---

## Clean

None. All four ARCH conventions (`ARCH-01`, `ARCH-02`, `ARCH-03`, `ARCH-04`)
were checked and **found violated** (V-ARCH-01 … V-ARCH-04 above). The two
grep-based How-to-check commands returned empty, but that is vacuous — it
reflects the *absence* of `src/domain/` and `src/ports/`, not satisfaction of
the convention, so no ARCH convention can be listed as clean on the current
single-file tree.
