# Task DAG

Task IDs are stable across runs. Do not rename them, because the judge compares them
one-to-one. Tasks that the run itself generates (P5 work packages, P7 fixes) use the
prefixes given below.

## Owner preferences (non-negotiable; override any article)

These are binding inputs to P2 and to every later review.

1. **Hexagonal architecture**: domain, ports (traits), adapters (inbound CLI, outbound
   file system/process), and an application/service layer. Dependencies point inward.
   Domain imports no adapter, clap, or `std::process` code. Layout follows the hexagonal
   guide's file patterns.
2. **Static dispatch via enums, not dynamic dispatch.** No `Box<dyn Port>` or `&dyn Port`
   for ports. When a port has multiple implementations, define an enum whose variants hold
   them, implement the port trait on the enum, and generate the delegating `match` arms
   with a `macro_rules!` macro. Use plain generics only where one concrete type is chosen
   at compile time.
3. **`thiserror` only.** No `anyhow`, not even in `main`. Each port and adapter boundary
   has its own error enum. `main` maps the top-level error to a message and an exit code.
4. **Parse, don't validate.** Raw input (CLI strings, file lines, env vars) is converted
   at the boundary into domain newtypes through `TryFrom`/`FromStr`. Once constructed,
   a domain value is valid by construction.
5. **Private fields.** Domain types expose constructors and accessors, never `pub` fields.
6. No `unwrap`/`expect`/`panic!`/`unreachable!` outside tests.

## Current behaviour contract (the refactor must preserve it)

- Queue file: `$HOME/.config/encode_queue/commands.txt`. The directory is created if
  missing.
- `add <CMD>`: appends `<CMD>\n` to the queue.
- `ls`: prints the queue file contents.
- `start`: loop as follows. Read the first line. If it is non-blank, run it with
  `fish -c <line>` and inherit stdio. **Re-read the file after the command finishes**
  (it may have been edited meanwhile) and rewrite it without the first line. Stop when
  the file is empty.
- `edit`: opens the queue in `$EDITOR`, default `nvim`.

Known bugs that P3 should flag and P5 may fix (record the choice in `decisions.md`): `ls`
errors if the file does not exist yet; the exit status of a command is ignored.

---

## P0 — Setup (orchestrator, serial)

1. **Preflight**, from your session's workspace:
   `plan/scripts/preflight.sh <RUN_ID>`. It checks that ninfer was restarted recently, creates the
   isolated clone `REPO`, the metrics dir `BENCH_DIR`, and the toolchain wrapper `DEV`, and it
   starts the background collector. If it exits non-zero, **stop and report its output**.
   Do not retry, work around it, or set override variables.
2. Confirm that `subagent({ action: "list" })` shows `bench-worker` and `bench-reviewer`.
   If either is missing, stop and report (pi project trust was probably declined).
3. `$L phase_start P0`, where `L` is `REPO/plan/scripts/ledger.sh` from here on.
4. In `REPO`: `git switch -c bench/<RUN_ID> bench-base-v1`, then `mkdir -p ~/eq-wt/<RUN_ID>`.
5. Record `DEV cargo --version`, `DEV rustc --version`, `DEV which fish`,
   `DEV gh auth status`, and `git rev-parse HEAD` with `$L note P0 msg=...`.
6. `DEV cargo build` in `REPO` must succeed. Then `$L phase_end P0`.

Every later phase starts with `$L phase_start <P>` and ends with `$L phase_end <P>`
once all of its tasks, including fixes, are merged or dropped. Phases never overlap.

## P1 — Research: pull the guides' source markdown (fan-out ×4)

The guides' source is published as markdown in `github.com/howtocodeit/article-md`. Every
run reads it at the same pinned commit, so all runs get byte-identical input.
`SRC = https://raw.githubusercontent.com/howtocodeit/article-md/b166e26aac8feeff87afa4b6268a4fcf18b90622/guides`

| ID | `<slug>` | Source file | Article URL (for attribution) | Code blocks | sha256 (first 12) |
|----|---------|-------------|-------------------------------|-------------|-------------------|
| P1.R1 | `error-handling` | `$SRC/the-definitive-guide-to-error-handling-in-rust.md` | https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling | 23 | `9cae1a8243c4` |
| P1.R2 | `hexagonal-architecture` | `$SRC/master-hexagonal-architecture-in-rust.md` | https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust | 29 | `c2a76a836078` |
| P1.R3 | `newtypes` | `$SRC/the-ultimate-guide-to-rust-newtypes.md` | https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes | 31 | `e0862cad1d42` |
| P1.R4 | `macro-rules` | `$SRC/writing-production-rust-macros-with-macro_rules.md` | https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules | 27 | `1eb369476fb9` |

Worker spec (same for each):

1. **Download the raw markdown with `curl`, and read the whole file yourself.** Do not use
   a web-search, fetch, or summarizing tool, and do not fetch anything else from GitHub or
   the website. Run
   `curl -sfL <source file URL> -o $BENCH_DIR/research/raw/<slug>.md`.
   Check its `sha256sum` against the table, and stop with status `blocked` if it differs.
   Write `docs/research/<slug>/SOURCE.md` with the source URL, the article URL, the full
   sha256, and the fetch time. The raw file stays out of the repo.
2. Extract **every** fenced code block (```` ``` ```` … ```` ``` ````) verbatim into
   `docs/research/<slug>/snippets.md`. Number them `S1..Sn` in file order, and state `n` at
   the top. Above each block, put the nearest preceding `#`/`##`/`###` heading, the
   fence's info string (including any `path=...` attribute), and the attribution line
   `Source: <article URL>`. The license requires that link next to every copied example.
   Write any extraction script in `$BENCH_DIR/scratch/`, not in the repo.
3. Write `docs/research/<slug>/notes.md`: the prescriptive rules the article teaches,
   **derived from the code and the surrounding prose**. Each rule gets a short statement in
   your own words, the article's reasoning, and at least one snippet ID that demonstrates it.
   Also list the anti-patterns the article warns against, with the snippet that shows each
   one where there is one. Quote at most 100 words of prose at a time, always with the article
   link (license terms). Put the article link at the top of the file.

Reviewer must:
- Download the source file independently and confirm its sha256 matches the table.
- Independently count the fenced blocks in the raw markdown, and confirm the count equals
  both `n` and the table.
- Pick 5 random snippets and diff each one byte-for-byte against the raw markdown.
- Confirm every snippet has the attribution line, and no prose quote exceeds 100 words.
- Confirm every rule cites an existing snippet ID that actually supports it, and that no
  rule invents crates, APIs, or advice that are not in the article.
- Name at least two prescriptive points from the article that the notes missed, or state
  that you searched for them and found none.

## P2 — Conventions synthesis (fan-in ×1), depends on P1.*

**P2.C1** writes `docs/CONVENTIONS.md`:
- Sections: Architecture & layout, Errors, Types & parsing, Dispatch & macros, Testing,
  General.
- Each convention has an ID (`ARCH-01`, `ERR-03`, …), a rule, a one-line rationale, a
  good/bad example **in this repo's domain** (queue entries, commands, the queue file), a
  **How to check** line (a grep, a clippy lint, or a specific manual check), and a source
  (an owner preference number or a snippet ID such as `R2:S14`).
- A **Resolved conflicts** section lists every place an article disagrees with the owner
  preferences and states which one wins. Owner preferences always win.
- A **Target layout** section gives the concrete `src/` tree this repo should end up with.

Reviewer must: check that all six owner preferences are encoded; spot-check 8 source
citations against `snippets.md`; reject vague rules ("write clean code") and rules without
a working How to check.

## P3 — Audit (fan-out ×4), depends on P2

Every auditor reads `docs/CONVENTIONS.md` and the **entire** `src/` plus `Cargo.toml`, but
reports only its own convention families.

| ID | Families | Output |
|----|----------|--------|
| P3.A1 | ARCH | `docs/audit/architecture.md` |
| P3.A2 | ERR (plus the panic/unwrap rule) | `docs/audit/errors.md` |
| P3.A3 | Types & parsing, private fields | `docs/audit/types.md` |
| P3.A4 | Dispatch & macros, Testing, General | `docs/audit/dispatch-testing.md` |

Each violation gets: ID `V-<area>-NN`, convention ID, `file:line`, quoted evidence,
severity (`high`/`med`/`low`), and suggested fix. End the file with a "Clean" list of
conventions checked and found satisfied.

Reviewer must: open every cited `file:line` and confirm the quote and the violation. Reject
false positives. Run its own sweep of the How-to-check commands for those families and
report any missed violations.

## P4 — Refactor plan (fan-in ×1), depends on P3.*

**P4.P1** writes `docs/refactor/PLAN.md`, which decomposes the refactor into work packages
`WP01..WPnn`. Each WP has scope, **files owned**, `depends_on`, the violation IDs it closes,
and acceptance criteria. Hard constraints:
- Every violation ID from P3 is either closed by a WP or listed under "Deferred" with a
  reason.
- `WP01` is a serial scaffold. It creates the target module layout, error enum skeletons,
  and port traits, and the existing behaviour must still work after it.
- After `WP01`, at least 3 WPs can run in parallel, and **no two WPs that can run
  concurrently own the same file**.
- The dependency graph is acyclic and ends with `WPnn = integration` (wiring in `main`,
  dead code removal).
- Each WP is small enough for one worker session.

Reviewer must: check violation coverage mechanically, check file-ownership disjointness for
every concurrent pair, check the DAG for cycles, and confirm the plan reaches the
CONVENTIONS target layout.

## P5 — Refactor execution (dynamic fan-out), depends on P4

- Run `P5.WP01`, then dispatch the remaining WPs following `PLAN.md`'s DAG, up to
  `MAX_IN_FLIGHT` children at once. Task IDs are `P5.WPnn`.
- Each WP reviewer also checks that the violation IDs the WP claims to close are
  actually closed, and that the behaviour contract still holds.
- **P5.Z — integration review** (fan-in, reviewer-only task on `RUN_BRANCH`). Re-run all
  P3 How-to-check commands across the whole tree and confirm every non-deferred violation
  is gone. Exercise the behaviour contract black-box with `HOME=$(mktemp -d)`: `add`,
  `ls`, and `start` (if `fish` is present; otherwise note that this is deferred to P6.F2).
  Failures become `P5.FIXnn` worker tasks with normal review.

## P6 — Features (serial harness, then fan-out ×4), depends on P5.Z

### P6.H — Test harness (serial)
- Support the env var `ENCODE_QUEUE_DIR`. When set, it overrides the queue directory.
  Parse it at the boundary into a domain path type.
- Add dev-dependencies `assert_cmd`, `predicates`, `tempfile`. Add `tests/cli.rs` with
  black-box tests for `add` and `ls` that use `ENCODE_QUEUE_DIR`.
- Make `ls` on a missing queue print nothing and exit 0.

### Fan-out (all depend on P6.H; each must add black-box tests in its own `tests/<feature>.rs`)

**P6.F1 — `rm <N>`**: removes the N-th entry (1-based). `N` is parsed into a domain
newtype; 0 and non-numeric values are rejected at parse time. Out of range: message to
stderr, exit code 2, queue unchanged. Add `ls -n/--numbered`, which prints `N: <cmd>` per
line. Plain `ls` output stays unchanged.

**P6.F2 — Selectable shell**: the env var `ENCODE_QUEUE_SHELL` accepts `fish|bash|sh`
(default `fish`). Any other value exits with code 2 and a message listing the valid
values. Implement it as a shell-runner **enum** that implements the command-runner port,
with delegation generated by a `macro_rules!` macro (owner preference 2). Tests use `sh`.

**P6.F3 — Failure handling**: if a command exits non-zero, append
`<unix_ts>\t<exit_code>\t<command>\n` to `failed.txt` in the queue dir, remove the command
from the queue, and continue. With `start --stop-on-error`, stop after logging the failure
and exit 1. Without the flag, exit 0 even if commands failed. Tests set
`ENCODE_QUEUE_SHELL=sh` and use `false`/`true`. If F2 has not merged yet, this test
depends on it, so the orchestrator must order the merges F2 → F3.

**P6.F4 — Single-runner lock**: `start` creates `queue.lock` in the queue dir with
`create_new` and writes its PID into it. A second `start` while the lock is held exits with
code 3 and `queue already running (pid N)`. A stale lock (PID not alive, checked through
`/proc/<pid>`; Linux only, documented) is reclaimed. The lock is removed on normal exit.
Lock errors are their own `thiserror` variant.

### P6.Z — Feature integration review (fan-in)
Run the full test suite on `RUN_BRANCH`. Manually exercise all four features together
(for example, `rm` while `start` runs in another process). Confirm no feature broke
another. Failures become `P6.FIXnn`.

## P7 — Final adversarial review (fan-out ×3, then fix fan-out, then fan-in)

Three independent `bench-reviewer`s on `RUN_BRANCH`. Each writes the standard review file, `review/P7.Ln-r1.md`, which the orchestrator copies to `$BENCH_DIR/reviews/`:

| ID | Lens |
|----|------|
| P7.L1 | Conventions conformance: walk every convention ID in CONVENTIONS.md and mark pass/fail with evidence |
| P7.L2 | Correctness: try to break it with empty or blank lines, missing trailing newline, unicode, a very long queue, a concurrent `add` during `start`, a missing `$EDITOR`, and a nonexistent dir |
| P7.L3 | Architecture & API hygiene: dependency direction (grep `use` across layers), `pub` surface, dead code, README accuracy |

Then the orchestrator consolidates all blocker and major findings into `P7.FIXnn` tasks.
Fixes whose files are disjoint run in parallel. After the fixes merge, **P7.Z** re-runs
only the lenses that had findings. At most 2 fix cycles; anything left goes into
`known_issues.md`.

## P8 — Ship (orchestrator, serial)

- [ ] Dispatch `P8.D1` (worker plus review): README documents `rm`, `ls -n`,
      `ENCODE_QUEUE_DIR`, `ENCODE_QUEUE_SHELL`, `--stop-on-error`, the lock, and the new
      layout.
- [ ] All four cargo gates pass on `RUN_BRANCH` in `REPO` (via `DEV`).
- [ ] `$L phase_end P8`, then run `plan/scripts/finalize.sh`. It collects the remaining
      ninfer request records and writes `$BENCH_DIR/report.md`.
- [ ] Copy `report.md`, `ledger.jsonl`, `requests.jsonl`, `tiles.jsonl`, `server.jsonl`,
      `manifest.json`, `decisions.md`, `known_issues.md`, and `reviews/` from `$BENCH_DIR`
      into `.bench/<RUN_ID>/`. Write `.bench/<RUN_ID>/RUN_REPORT.md`: three paragraphs on
      what happened (the phases that went badly, escalations and drops, notable decisions),
      followed by the contents of `report.md`. Commit as `P8: run report and ledger`.
- [ ] `git push -u origin bench/<RUN_ID>`. Then, if `gh auth status` succeeds, run
      `gh pr create --base main --title "[bench <RUN_ID>] hexagonal refactor + queue features" --body-file .bench/<RUN_ID>/RUN_REPORT.md`.
      If gh is not authenticated, do not try to log in. Instead, `$L note P8
      msg="pr-manual"` and finish with the compare URL
      `https://github.com/BJSummerfield/encode_queue/compare/main...bench/<RUN_ID>`.
- [ ] Do **not** merge. Remove any remaining worktrees under `~/eq-wt/<RUN_ID>`. Stop.
