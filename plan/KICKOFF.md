# Kickoff prompt

Start a new Paseo workspace off `main`, open a pi session in it, and paste everything
below the line. **Change only the `RUN_ID` line.** Everything else stays identical across
runs.

| Run order | RUN_ID |
|---|---|
| 1 | `mtp-run1` |
| 2 | `dflash-k8v4-run1` |
| 3 | `mtp-run2` |
| 4 | `dflash-k8v4-run2` |

---

RUN_ID=mtp-run1

You are the orchestrator for an autonomous agentic-coding benchmark run on this repository.

Read `plan/ORCHESTRATOR.md` and then `plan/TASKS.md`, both in full. They are your complete
instructions, and you follow them exactly. Start with P0, which runs preflight for the
RUN_ID above. If preflight fails, stop and report its error. That is the only time you
stop early.

After preflight passes, the run is fully autonomous and no human is watching. Never ask me
anything and never wait for input. Judgment calls belong to the sub-agents: they decide and
record their decisions in their reports. You keep going.

You are a dispatcher, not a worker (see `AGENTS.md`). You keep 4 lanes of `bench-worker`,
`bench-reviewer`, and `bench-merger` sub-agents busy at all times (continuous batching),
log every event to the ledger as it happens, and keep your state in `state.md`. Planning,
reviewing, triaging, merging, and conflict resolution are all sub-agent work: you dispatch
them and act only on the short summaries they return.

Isolation is a hard rule. Solve everything from the specs and this run's own branches.
Never look at other git branches, remotes, PRs, or other directories on this machine, even
if you suspect the work already exists somewhere, and make every child obey the same rule.

You are done when P8.SHIP (see `plan/TASKS.md`) has returned and you have logged it. That
means the run branch is pushed and the PR is open, or the compare URL is recorded if gh is not
logged in. Never merge anything into `main`.
