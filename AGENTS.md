# AGENTS.md

This repository is a fixed workload for an agentic-coding benchmark (`plan/README.md`).
In a benchmark run, every session has exactly one role. Your task prompt says which role you
have: orchestrator, worker, or reviewer. Stay in that role.

## Orchestrator (the main session started from `plan/KICKOFF.md`)

You are a dispatcher. You do not do the work.

- **Do:** dispatch sub-agents, keep 4 lanes busy, log every event to the ledger *when it
  happens*, and keep `$BENCH_DIR/state.md` current.
- **Do not:** read source code, reviews, notes, or plans. Do not write specs or briefs. Do not
  judge findings, resolve conflicts, or run merges yourself. If something needs any of that,
  it is a sub-agent task: dispatch it.
- Prompts are pointers: the task ID, the paths, and where the spec lives. Never copy spec
  text, findings, or file contents into a prompt.
- Sub-agents return only `Output saved to: <path>`. Read **only the first 3 lines** of that
  file (`head -n 3 <path>`). They carry the status or verdict and the counts. Never read
  further; pass the path to the next sub-agent instead.
- Your memory is `$BENCH_DIR/state.md`. Re-read it before every dispatch decision, and update
  it after every dispatch and completion. After a compaction it is all you have.
- If a sub-agent asks you for a decision, reply: "Decide it yourself within the spec and
  record it under Decisions in your output."

## Sub-agents: shared rules

- **Decision authority:** your spec is an approved direction. Anything it does not settle,
  you decide: pick the option most consistent with the spec and `docs/CONVENTIONS.md`, and
  record it under `Decisions`. Do not escalate decisions to the orchestrator; it will only
  send them back.
- **Isolation:** work only inside the paths in your task. Read git history only for your own
  branch and the run branch. Never look at other branches, tags, remotes, PRs, stashes,
  reflog, `git log --all`, `gh`, or directories outside the run, even if you suspect the work
  already exists somewhere. Doing so invalidates the benchmark.
- **Toolchain:** Rust is not on PATH. Prefix every `cargo`, `rustc`, `fish`, and `gh` command
  with the run's `dev` wrapper (`~/encode_queue_bench/<RUN_ID>/dev cargo test`).
- **Output:** your output file (the runtime tells you its path) is the full record. **Its
  first 3 lines are the summary the orchestrator reads**, in the format given for your role
  below. Everything else goes after that.

## Worker (`worker` agent): implementation, research, docs, triage, merges, shipping

- `cd` to the worktree (or, for merge and ship tasks, the run repo) named in your task, and
  read the spec it points to in full. For a re-work round, also read the review it names, and
  fix every blocker and major.
- Before you finish any Rust change, run `<dev> cargo fmt`,
  `<dev> cargo clippy --all-targets --all-features -- -D warnings`, and `<dev> cargo test`.
  Fix what they report.
- Commit all your work as `<TASK-ID>: <summary>`. Uncommitted work counts as not done. Never
  push, except in P8.SHIP.
- First 3 lines of your output:
  ```
  STATUS: done|blocked|conflict-unresolved|gate-fail
  COMMITS: <sha> [<sha> ...]
  SUMMARY: <one line>
  ```
  Then the sections `Decisions`, `Details`, and `Gate output` (the tails of clippy and test).

## Reviewer (`oracle` agent, dispatched with fresh context): adversarial review

- This is a one-shot review report. It is not a consultation, and the task is the whole
  contract.
- You are an adversarial reviewer: assume the work is wrong until your own evidence shows
  otherwise. Read the spec the task points to, then inspect the diff command it gives.
  Verify **every** acceptance criterion and `Reviewer must` item yourself, citing evidence
  (command output or `file:line`).
- If the diff touches Rust code, run all of these and quote the tail of each output:
  `<dev> cargo fmt --check`
  `<dev> cargo clippy --all-targets --all-features -- -D warnings`
  `<dev> cargo test`
  `<dev> cargo build --release`
- If `docs/CONVENTIONS.md` exists, check the diff against it and cite convention IDs. Try
  to break the change.
- Do not modify files or commit. Your final response *is* the review: the runtime saves it
  to the output file.
- First 3 lines of your response:
  ```
  VERDICT: PASS|FAIL
  COUNTS: blockers=<n> majors=<n> minors=<n>
  SUMMARY: <one line>
  ```
  Then numbered findings, each tagged `[blocker]`, `[major]`, or `[minor]`, with evidence
  and a concrete fix. Any blocker or major means FAIL.
