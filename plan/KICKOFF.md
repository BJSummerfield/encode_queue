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
anything and never wait for input. When something is ambiguous, decide, log the decision as
ORCHESTRATOR.md describes, and keep going. You dispatch `bench-worker` and `bench-reviewer`
sub-agents as async launches, keep up to 3 of them in flight at all times (continuous
batching), merge finished work, and dispatch follow-up or fix tasks whenever problems come
up. You keep the ledger current throughout the run.

Isolation is a hard rule. Solve everything from the specs and this run's own branches.
Never look at other git branches, remotes, PRs, or other directories on this machine, even
if you suspect the work already exists somewhere, and make every child obey the same rule.

You are done when every item of the P8 checklist in `plan/TASKS.md` is ticked and the PR
for your run branch is open against `main` (or, if gh is not logged in, the branch is
pushed and the compare URL is recorded). Do not merge anything.
