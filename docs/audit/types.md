# Audit: Types & parsing (TY family) and private fields (OP5)

**Task:** P3.A3 (round 1) — run `dflash-k8v4-run1`
**Scope:** `docs/CONVENTIONS.md` TY-01…TY-06 plus the private-fields convention (OP5,
codified as TY-03). All commands were run in the repo root against the current
single-file tree (`src/main.rs`, 116 lines; `Cargo.toml` depends only on `clap`).
**Tree state audited:** the Target layout does not exist yet — there is no
`src/domain/`, no `src/ports/`, and no domain newtypes at all. Every finding
below is therefore rooted in `src/main.rs`.

## Sweep (TY-family How-to-check commands, quoted)

Run verbatim from the repo root:

```console
$ grep -rnE "fn [a-z_0-9]+ *\([^)]*: *(&str|String|&Path|PathBuf)\b" src/
src/main.rs:62:fn process_queue(queue_path: &Path) -> io::Result<()> {
src/main.rs:107:fn open_editor(file_path: &str) -> std::io::Result<()> {
# stage-1 exit: 0

# TY-02 full 3-stage check, as written in CONVENTIONS.md:
$ grep -rnE "fn [a-z_0-9]+ *\([^)]*: *(&str|String|&Path|PathBuf)\b" src/ \
    | grep -E "^src/(domain|ports)/" | grep -vE "fn (from_str|try_from|from_utf8)\b"
# (no output) exit: 1

$ grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/ | grep -E "^src/domain/"
# (no output) exit: 1
$ grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/          # unscoped control run
# (no output) exit: 1

$ grep -rn "impl Deref\|impl Borrow" src
# (no output) exit: 1

$ grep -rnE "impl (FromStr|TryFrom|From|FromUtf8) for" src
# (no output) exit: 1
$ grep -rnE "struct (QueueEntry|QueuePath|Editor)\b" src
# (no output) exit: 1
$ grep -rn "derive" src
# (no output) exit: 1

# TY-01 manual check — validation guards and raw working types:
$ grep -rnE "if .*is_empty\(\)" src/
src/main.rs:71:        if !first_line.trim().is_empty() {
# exit: 0
$ grep -rn "trim()" src/
src/main.rs:71:        if !first_line.trim().is_empty() {
src/main.rs:74:                .arg(first_line.trim())
# exit: 0
$ grep -rn "env::var" src/
src/main.rs:31:    let home_dir = env::var("HOME").expect("HOME environment variable not set");
src/main.rs:108:    let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());
```

Note on the 3-stage check: stage 2 scopes to `src/(domain|ports)/`, which does
not exist yet, so the scoped pipeline exits 1 (the convention's normal
"no matches" result). Per CONVENTIONS.md the line-based sweep is a spot check
"alongside a manual scan of port signatures"; since **no** domain/ports layer
exists, the business logic in `main.rs` *is* the domain API today, so stage 1
of the sweep (unscoped, quoted above) is the operative evidence for
V-TY-04/V-TY-05.

## Violations

### V-TY-01 — TY-01 (parse, don't validate): `add` argument crosses the boundary as a raw `String`
- **Convention:** TY-01 (OP4)
- **Location:** `src/main.rs:39`, `src/main.rs:44`
- **Evidence:**
  ```rust
  let command = sub_matches.get_one::<String>("COMMAND").unwrap();   // :39
  ...
  writeln!(file, "{}", command)?;                                    // :44
  ```
  The raw clap `String` is written verbatim into `commands.txt` with no
  conversion to a domain newtype and no validation whatsoever (an empty or
  whitespace-only command can be enqueued). This is the exact "bad" shape the
  convention quotes for the current file.
- **Severity:** high
- **Suggested fix:** introduce `QueueEntry` (TY-02/TY-03) with a canonical
  fallible constructor and `impl FromStr for QueueEntry` (trim + reject
  blank, returning `QueueEntryError`); at the inbound boundary do
  `QueueEntry::from_str(raw).map_err(CliError::BadCommand)?` and write
  `entry.as_str()` (ARCH-04 good example).

### V-TY-02 — TY-01 (parse, don't validate): file-line validation buried mid-orchestration on raw `String`
- **Convention:** TY-01 (OP4)
- **Location:** `src/main.rs:66–74`
- **Evidence:**
  ```rust
  let mut first_line = String::new();                                  // :66
  if reader.read_line(&mut first_line)? == 0 { break; }                // :67
  ...
  if !first_line.trim().is_empty() {                                   // :71
      let mut child = SystemCommand::new("fish")
          .arg("-c")
          .arg(first_line.trim())                                      // :74
  ```
  The raw `String` is the working type, and the validity check
  (`if … .trim().is_empty()`) sits inside the orchestration loop exactly where
  TY-01's "bad" example says it must not be; the *same* `trim()` is computed
  twice (guard at :71, value at :74), which a by-construction value would
  eliminate.
- **Severity:** high
- **Suggested fix:** parse at the read site: `QueueEntry::from_str(&line)`;
  treat the `Empty` error as "skip the line" and hand the *parsed*
  `&QueueEntry` to a `CommandRunner` port (`runner.run(&entry)`), so the
  domain can trust it is non-blank.

### V-TY-03 — TY-01 (parse, don't validate): raw `HOME`/`EDITOR` env vars used unconverted
- **Convention:** TY-01 (OP4)
- **Location:** `src/main.rs:31`, `src/main.rs:108–110`
- **Evidence:**
  ```rust
  let home_dir = env::var("HOME").expect("HOME environment variable not set");  // :31
  ...
  let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());       // :108
  SystemCommand::new(editor)                                                    // :110
  ```
  TY-01 explicitly names "the `HOME`/`EDITOR` env vars" as raw input that must
  be converted at the boundary; here both cross straight into `PathBuf`
  arithmetic (:32) and `SystemCommand::new` with the default `nvim` inlined as
  a raw string literal.
- **Severity:** med
- **Suggested fix:** `Config::from_env()` that yields newtypes: `HOME` joined
  with the fixed queue suffix into a `QueuePath` (with a `ConfigError::HomeUnset`
  variant instead of the `.expect`), and `EDITOR` parsed into an `Editor`
  newtype whose constructor applies the `nvim` default.

### V-TY-04 — TY-02 (distinct concepts get distinct types): business function takes raw `&Path`
- **Convention:** TY-02 (OP4)
- **Location:** `src/main.rs:62`
- **Evidence:**
  ```rust
  fn process_queue(queue_path: &Path) -> io::Result<()> {   // :62
  ```
  Flagged by the TY-02 sweep stage 1 (quoted above). The queue location — one
  of the three concepts the convention demands a distinct newtype for — is a
  bare `&Path` that any caller can point at any file.
- **Severity:** med
- **Suggested fix:** `QueuePath(PathBuf)` newtype; the store port signature
  `fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError>` keeps
  `QueuePath` inside the adapter, and `process_queue` becomes a service method
  that never names the path.

### V-TY-05 — TY-02 (distinct concepts get distinct types): business function takes raw `&str`
- **Convention:** TY-02 (OP4)
- **Location:** `src/main.rs:107`
- **Evidence:**
  ```rust
  fn open_editor(file_path: &str) -> std::io::Result<()> {   // :107
  ```
  Flagged by the TY-02 sweep stage 1 (quoted above). This is the precise
  signature the TY-02 "bad" example quotes from this file; nothing stops
  `start` from being handed the editor name, or `edit` from opening a queued
  command.
- **Severity:** high
- **Suggested fix:** `pub trait EditorLauncher { fn open(&self, at: &QueuePath) -> Result<(), EditorError>; }`
  (TY-02 good example) with `EditorProcess` as the outbound adapter; the
  `Editor` binary name also gets its own newtype so it cannot be passed where
  a `QueueEntry` or `QueuePath` is expected.

### V-TY-06 — TY-02 (distinct concepts get distinct types): the `QueueEntry`/`QueuePath`/`Editor` newtypes do not exist
- **Convention:** TY-02 (OP4)
- **Location:** `src/` (tree-wide)
- **Evidence:**
  ```console
  $ grep -rnE "struct (QueueEntry|QueuePath|Editor)\b" src
  # (no output) exit: 1
  ```
  The three distinct concepts of this domain are all bare `String`/`&str`/
  `PathBuf`/`&Path` today, so the swap hazard the convention warns about
  (new-R1) is live: `main` already passes `queue_path.to_str().unwrap()`
  (`src/main.rs:54`) into `open_editor(file_path: &str)` and `first_line.trim()`
  (`src/main.rs:74`) into `fish -c`.
- **Severity:** high
- **Suggested fix:** create `src/domain/queue/mod.rs` with the three newtypes
  per the Target layout, each per TY-03/TY-04/TY-06, and migrate the TY
  violations above so no raw text crosses a layer boundary.

### V-TY-07 — TY-04 (one canonical constructor): no `FromStr`/`TryFrom`/`From` impl exists anywhere
- **Convention:** TY-04 (OP4)
- **Location:** `src/` (tree-wide)
- **Evidence:**
  ```console
  $ grep -rnE "impl (FromStr|TryFrom|From|FromUtf8) for" src
  # (no output) exit: 1
  ```
  There is no canonical validating constructor to delegate to — the only
  "validation" in the crate is the ad-hoc `trim()`/`is_empty()` pair inside
  `process_queue` (V-TY-02), which is exactly the "validation buried
  mid-orchestration" anti-pattern TY-01/TY-04 exist to remove.
- **Severity:** med
- **Suggested fix:** a single canonical fallible constructor
  (`QueueEntry::new`/`from_str`) as the source of truth, with
  `TryFrom<&str>` as a one-line delegation (`Self::from_str(s)`, TY-04 good
  example); no second constructor with independent checks.

### V-TY-08 — TY-01 (parse, don't validate): `ls` reads and prints the queue file as a raw `String`
- **Convention:** TY-01 (OP4)
- **Location:** `src/main.rs:50–51`
- **Evidence:**
  ```rust
  let contents = fs::read_to_string(&queue_path)?;   // :50
  println!("{}", contents);                         // :51
  ```
  The lines read from `commands.txt` — named raw input in TY-01 — are never
  parsed into domain values on the `ls` path either; `ls` prints the raw file
  bytes (including any malformed line that V-TY-01 lets `add` write).
- **Severity:** low
- **Suggested fix:** `FileQueueStore::read_all() -> Result<Vec<QueueEntry>, QueueStoreError>`
  parsing each line through the same `FromStr` boundary; `ls` prints
  `entry.as_str()` per entry and maps `QueueFileMissing` to an empty queue.

## Clean

Conventions checked and satisfied (supporting evidence quoted from the sweep):

- **TY-03 / OP5 — private fields, no `pub` fields on domain types.** Satisfied
  (vacuously, no domain types exist yet; and there are no `pub` fields
  anywhere in the crate): `grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/ | grep
  -E "^src/domain/"` → no output, exit 1; the unscoped control run
  `grep -rnE "^\s*pub [a-z_][a-z0-9_]*\s*:" src/` is likewise empty (exit 1).
  No type bypasses the constructor rule because no type is visible.
- **TY-05 — no `Deref`/mismatched `Borrow` on domain newtypes.** Satisfied
  (vacuously): `grep -rn "impl Deref\|impl Borrow" src` → no output, exit 1;
  no newtypes exist on which the anti-pattern could be declared.
- **TY-06 — standard trait derives on domain types.** Satisfied (vacuously):
  `grep -rn "derive" src` → no output, exit 1; there are no domain types whose
  derive lists need review. The *absence* of the newtypes themselves is
  reported as a TY-02 violation (V-TY-06), not as a derive problem.
