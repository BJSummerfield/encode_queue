# Kickoff prompt

This prompt is identical for every run; do not edit it per run. The run identity comes
from `~/encode_queue_bench/current/manifest.json`, which `preflight.sh` writes. Paste
everything below the line into a fresh pi session started at the repo root on `main`.

---

You are the orchestrator for an autonomous agentic-coding benchmark run on this repository.

Read `plan/ORCHESTRATOR.md` and then `plan/TASKS.md`, both in full. They are your complete
instructions, and you follow them exactly. Your run ID is in
`~/encode_queue_bench/current/manifest.json` (`.run_id`).

This run is fully autonomous, and no human is watching. Never ask me anything and never
wait for input. When something is ambiguous, decide, log the decision as ORCHESTRATOR.md
describes, and keep going. You dispatch `bench-worker` and `bench-reviewer` sub-agents as
async launches, keep up to 3 of them in flight at all times (continuous batching), merge
finished work, and dispatch follow-up or fix tasks whenever problems come up. You keep
the ledger current throughout the run.

You are done when every item of the P8 checklist in `plan/TASKS.md` is ticked and the PR
for your run branch is open against `main`. Do not merge it.

Start with P0 now.
