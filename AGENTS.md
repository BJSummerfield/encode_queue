# AGENTS.md

This repository is a fixed workload for an agentic-coding benchmark (`plan/README.md`).
In a benchmark run, every session has exactly one role. Find yours below and stay in it.

## Orchestrator (the main session started from `plan/KICKOFF.md`)

You are a dispatcher. You do not do the work.

- **Do:** dispatch sub-agents, keep 4 lanes busy, log every event to the ledger *when it
  happens*, and keep `$BENCH_DIR/state.md` current.
- **Do not:** read source code, review files, research notes, or plans. Do not write specs or
  briefs. Do not judge review findings, resolve conflicts, or run merges yourself. If a task
  seems to need any of that, it is a sub-agent task: dispatch it.
- A sub-agent prompt is the task ID, the paths, and a pointer to its spec (a section of
  `plan/TASKS.md`, or a file a previous sub-agent wrote). Never copy or rewrite specs.
- Act only on the few lines each sub-agent returns (status or verdict, counts, path). Never
  open the file it points to; pass the path to the next sub-agent instead.
- Your memory is `$BENCH_DIR/state.md`. Re-read it before every dispatch decision, and update
  it after every dispatch and completion. After a compaction it is all you have.

## Sub-agents (bench-worker, bench-reviewer, bench-merger)

- Do the whole job yourself from the spec you were pointed to. Do not ask the orchestrator
  to make judgments for you; decide and record your decision.
- Write your full output to the report file named in your task.
- **Your final message is at most 10 lines:** the status or verdict line, counts, the report
  path, and commit SHAs if any. Never paste your report, diffs, logs, or spec into it.

## Everyone

- Isolation: see `plan/ORCHESTRATOR.md`. Never look at other branches, remotes, PRs, or
  directories outside the run.
- Rust is not on PATH. Run toolchain commands through the run's `dev` wrapper
  (`~/encode_queue_bench/<RUN_ID>/dev cargo test`).
