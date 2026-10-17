# AGENTS.md

This repository is a fixed workload for an agentic-coding benchmark (`plan/README.md`).
In a benchmark run, every session has exactly one role, which its task prompt names:
orchestrator, worker, merger, or reviewer. Stay in that role.

Only the operator's global pi-subagents profiles are used: `worker` (worker and merger roles)
and `reviewer` (reviewer role). No other agents exist for this benchmark.

## Orchestrator (the main session started from `plan/KICKOFF.md`)

You are a dispatcher, and only a dispatcher. Every task is done by a sub-agent.

You may run only these things yourself:
1. `subagent` calls: dispatches, plus one `action: "list"` check in P0;
2. `plan/scripts/ledger.sh`;
3. reading and rewriting `$BENCH_DIR/state.md`;
4. `head -n 3 <output file>` on a sub-agent's output.

Nothing else. No git, no cargo, no reading code, specs, plans, notes, or reviews, no writing
briefs, no judging findings, no merging, no creating or removing worktrees. If something needs
doing, it is a task: dispatch the right sub-agent.

- Prompts are pointers: the role, the task ID, the paths, and where the spec lives. Never copy
  spec text, findings, or file contents into a prompt.
- Each sub-agent returns `Output saved to: <path>`. Read only its first 3 lines.
- `$BENCH_DIR/state.md` is your memory. Re-read it before every decision, and rewrite it after
  every dispatch and completion. After a compaction it is all you have.
- If a sub-agent asks you for a decision, reply: "Decide it yourself within the spec and
  record it under Decisions in your output."

## Sub-agents: shared rules

- **Decision authority:** your spec is an approved direction. Anything it does not settle, you
  decide: choose the option most consistent with the spec and `docs/CONVENTIONS.md`, and
  record it under `Decisions`. Do not escalate decisions to the orchestrator.
- **Isolation:** work only inside the paths in your task. Read git history only for your own
  task branch and the run branch. Never look at other branches, tags, remotes, PRs, stashes,
  reflog, `git log --all`, `gh`, or directories outside the run, even if you suspect the work
  already exists somewhere. Doing so invalidates the benchmark.
- **Toolchain:** Rust is not on PATH. Prefix every `cargo`, `rustc`, `fish`, and `gh` command
  with the run's `dev` wrapper (`~/encode_queue_bench/<RUN_ID>/dev cargo test`).
- **Output:** write your output file (its path is in your task). **Its first 3 lines are the
  summary the orchestrator reads**, in your role's format below. Everything else follows.
- **Git locks:** if a git command fails on a `.lock` file because a sibling is using the repo,
  wait 2 seconds and retry, up to 5 times.

## Worker (`worker`): implementation, research, docs, planning, triage, setup

- **Round 1:** create your worktree first, with
  `git -C <REPO> worktree add -b task/<RUN_ID>/<T> <WT_ROOT>/<T> bench/<RUN_ID>`.
  Re-work and rebase rounds reuse it. Then `cd` into it, and read the spec your task points
  to in full. For a re-work round, also read the review it names, and fix every P0 and P1.
- Before you finish any Rust change, run `<dev> cargo fmt`,
  `<dev> cargo clippy --all-targets --all-features -- -D warnings`, and `<dev> cargo test`.
  Fix what they report.
- Commit all your work as `<T>: <summary>`. Uncommitted work counts as not done. Never push.
- First 3 lines of your output:
  ```
  STATUS: done|blocked
  COMMITS: <sha> [<sha> ...]
  SUMMARY: <one line>
  ```
  Then the sections `Decisions`, `Details`, and `Gate output` (the tails of clippy and test).

## Merger (`worker`, role "merger"): merges, rebases, shipping

Follow the spec your task points to (in `plan/ORCHESTRATOR.md`). The output format is the
worker's, with `STATUS: done|blocked|conflict-unresolved|gate-fail`.

## Reviewer (`reviewer`): adversarial review

- Assume the work is wrong until your own evidence shows otherwise. Read the spec your task
  points to in full. Verify **every** acceptance criterion and `Reviewer must` item yourself,
  by running things, and cite command output or `file:line`.
- **Your own worktree:** first run
  `git -C <REPO> worktree add --detach <WT_ROOT>/<T>-review-r<k> <ref from your task>`,
  then review inside it. When you are done, remove it with
  `git -C <REPO> worktree remove --force <that path>`. Never commit, switch branches, or push.
- For a task's diff, use `git diff $(git merge-base bench/<RUN_ID> HEAD)..HEAD`.
- Whenever Rust code is in scope, run all of these and quote the tail of each:
  `<dev> cargo fmt --check`,
  `<dev> cargo clippy --all-targets --all-features -- -D warnings`,
  `<dev> cargo test`, and `<dev> cargo build --release`.
  Then exercise the built binary black-box against the spec, using a temp
  `ENCODE_QUEUE_DIR` once that variable exists.
- **Stress what can race or fail.** Use distinct commands, so a wrong deletion is visible.
  Fire parallel bursts (100+ `add`s while `start` drains; 10+ simultaneous `start`s, then check
  that each command ran exactly once). Kill commands with signals. Test inputs with no
  trailing newline, blank lines, and unicode.
- Check against `docs/CONVENTIONS.md` when it exists, citing convention IDs.
- First 3 lines of your output:
  ```
  VERDICT: PASS|FAIL
  COUNTS: p0=<n> p1=<n> p2=<n>
  SUMMARY: <one line>
  ```
  Then your normal review (Correct / Finding P0·P1·P2 with location, evidence, and smallest
  fix / Merge verdict). Any P0 or P1 means `VERDICT: FAIL`.
