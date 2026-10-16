# known_issues.md — encode_queue, run dflash-k8v4-run1

Residual findings accepted by the orchestrator (adjudication ledger:
`plan/LEADER.md`). None is a blocker or major; each was explicitly
reviewed and accepted.

## API shape (observations, no action)

1. **`QueueService::start(&self, stop_on_error: bool)` takes a raw
   `bool` mode flag** (P7.L3 finding 10). CONVENTIONS.md has no newtype
   convention for mode flags (the u32/u64 data-primitive rule is the
   binding class), so this is a documented observation, not a violation.
   A `StopOnError`-style flag newtype could be introduced if the owner
   wants it.

2. **`ShellError` is a struct (private `input` + `input()` accessor),
   not a named-variant enum, and duplicates the
   `ConfigError::ShellInvalid(String)` concept** (P7.L3 finding 7,
   partially fixed by P7.FIX04). The OP5/TY-03 pub-field violation was
   the binding defect and is resolved; the enum-shape preference and the
   config↔domain duplication were adjudicated as not-worth-a-fix
   (single construction site, boundary translation retained in
   `ConfigError`).

3. **Inherent `QueueEntryIndex::from_str` with no `impl FromStr`**
   (P7.L1 finding 7). Single source of truth is preserved (TY-04 intent);
   an inherent method named `from_str` on a type without the trait is a
   mild reader trap. Optional: rename to `parse` or add a delegating
   `impl FromStr`.

4. **`Copy` derivations on `QueueEntryIndex` (wraps u32) and domain
   `Shell` (fieldless enum)** (P7.L1 finding 8). Outside TY-06's stated
   scope ("our wrappers hold String/PathBuf") and explicitly adjudicated
   in the `QueueEntryIndex` module doc.

## Suppressions

5. **One suppression attribute in the tree, in `cfg_attr` form**:
   `#[cfg_attr(not(test), allow(dead_code))]` on the deferred
   `QueueEntryError::Empty` variant (`src/domain/queue/mod.rs`).
   Sanctioned by PLAN.md WP02/Deferred (V-TY-01/02/08 behaviour half);
   the variant is genuinely unconstructible on the lenient path. The
   GEN-04 line-based grep (`grep -rn "#\[allow" src`) is empty by design.

## Behaviour / diagnostics

6. **`ENCODE_QUEUE_DIR` pointing at a file (or a path whose parent is a
   file) yields a generic EISDIR error message, exit 1** (P6.L2 minor,
   accepted no-fix). A `QueueDirNotADirectory` variant would be
   friendlier; the failure mode is bounded (exit 1, clear I/O error
   chain) and the config contract only promises a directory override.

7. **`ls` prints a single trailing newline for an empty/missing queue
   file** (P6.F3 nit, accepted). Carried over from the legacy
   `cat file; echo` behaviour the PLAN.md contract pins; documented in
   README (Behaviour section).

8. **`fn main` is 59 lines vs the ~40-line soft budget** (P7.L1 finding 2
   context, accepted). ARCH-02's hard checks (no `fs::`, no
   `Command::new`, no `read_line` in `main`) all pass; the line count is
   inflated by the behaviour-contract-required exit-code mapping arms
   (0/1/2/3) and their parity comments.

## Test coverage (accepted nits)

9. **`add` store-error propagation is pinned by unit test only**
   (P5.WP08 r2 nit, accepted): a black-box `add` failure path needs a
   read-only directory sandbox, which black-box env setup (env_clear +
   pinned HOME) cannot express portably.

10. **`tests/lock.rs` `HeldMock` uses a bare `Mutex<...>`**
    (P5.WP09 r2 nit, accepted): `poisoning` could be a problem if a
    holder panics mid-test; the lock test harness controls its own
    mocks and no holder panics.

11. **Single-argument `pin` calls** (e.g.
    `pin(config.queue_path().as_path())`) read as the `pin` newtype
    constructor; the P5.WP06 review r1 requested the literal
    `pin(PathBuf::from(...))` form for call-site clarity (r1 nit;
    adjudicated cosmetic — `QueuePath::as_path` is `&Path`, so the
    coercion is real and unambiguous).
