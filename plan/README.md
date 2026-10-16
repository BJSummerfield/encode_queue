# Agentic Coding Benchmark — encode_queue

This directory defines a long-running, fully autonomous agentic coding workload for
**pi** against the local **ninfer** server (`Qwen3.8-27B-NVFP4`). The same plan runs
twice per server configuration. That shows the drift between two runs of one config as
well as the difference between configs.

| Run ID | Server config | Branch |
|--------|---------------|--------|
| `mtp-run1` | EAGLE/MTP speculative decoding | `bench/mtp-run1` |
| `dflash-k8v4-run1` | DFlash (`--spec dflash2`), `--kv-dtype k8v4` | `bench/dflash-k8v4-run1` |
| `mtp-run2` | EAGLE/MTP speculative decoding | `bench/mtp-run2` |
| `dflash-k8v4-run2` | DFlash (`--spec dflash2`), `--kv-dtype k8v4` | `bench/dflash-k8v4-run2` |

Alternate the configs as shown (MTP, DFlash, MTP, DFlash) so slow drift such as thermals
or upstream page changes doesn't line up with one config. Every run forks from the tag
`bench-base-v1`, the commit that adds this directory. Nothing in `plan/` or `.pi/`
changes during a run.

## Files

| File | Audience | Purpose |
|------|----------|---------|
| `README.md` | human | This runbook |
| `KICKOFF.md` | orchestrator | The prompt you paste. **Identical for every run.** |
| `ORCHESTRATOR.md` | orchestrator | Continuous batching, git/worktrees, review loop, ledger |
| `TASKS.md` | everyone | Phase/task DAG, specs, acceptance criteria |
| `EVALUATION.md` | final judge (Opus/Fable) | Rubric for comparing the runs |
| `../.pi/agents/bench-worker.md` | pi | Worker agent (shell + edit, commits in its worktree) |
| `../.pi/agents/bench-reviewer.md` | pi | Adversarial reviewer (shell, runs the cargo gates, read-only on code) |
| `scripts/preflight.sh` | human | Creates the run's metrics dir and manifest, checks the server is fresh |
| `scripts/sample_server.sh` | human | Background collector: KV gauges plus every ninfer request record |
| `scripts/ledger.sh` | orchestrator | Appends ledger events, each with a ninfer counter snapshot |
| `scripts/finalize.sh` | orchestrator (P8) | Final collection and `report.md` |
| `scripts/ledger_report.sh` | anyone | Ledger and request records → markdown tables |

## Workload shape

```
P0 setup ─► P1 research ×4 ─► P2 conventions ─► P3 audit ×4 ─► P4 refactor plan
   (1)        (fan-out)          (fan-in)          (fan-out)        (fan-in)
                                                                       │
   ┌───────────────────────────────────────────────────────────────────┘
   ▼
P5 refactor: scaffold ─► WPs ×N ─► integrate ─► P6 features: harness ─► F1..F4 ─► integrate
               (1)      (fan-out)   (fan-in)                    (1)    (fan-out)  (fan-in)
                                                                                     │
   ┌─────────────────────────────────────────────────────────────────────────────────┘
   ▼
P7 final review ×3 lenses ─► fix tasks ─► re-review ─► P8 ship (PR)
```

Every worker task gets its own git worktree and an adversarial reviewer. The orchestrator
keeps 3 children in flight (3 children + orchestrator = ninfer's `--max-concurrency 4`)
and refills a slot whenever a child finishes. No human gates.

## What gets measured

All of it lands in `~/encode_queue_bench/<run-id>/`. At P8 it is also committed to the
PR under `.bench/<run-id>/`.

| File | Source | Contents |
|------|--------|----------|
| `manifest.json` | preflight | run ID, base SHA, pi/rustc versions, **full ninfer config + argv** from `/ops/summary` |
| `ledger.jsonl` | orchestrator via `ledger.sh` | one line per phase/task/review/merge/gate event: wall clock, review verdicts and rounds, pi child usage (tokens, turns, tool calls, duration), plus a ninfer counter snapshot (spill pages, RAM restores, preemptions, replayed tokens, spec drafted/accepted, prefill/cached tokens) |
| `requests.jsonl` | `sample_server.sh` | every ninfer request record: prompt/cached/computed/output tokens, TTFT, queue, decode, accepted/drafted, preemptions, paused, replayed, finish |
| `server.jsonl` | `sample_server.sh` | running/waiting/paused and KV-tier occupancy every 5 s |
| `summary.json` | finalize | ninfer's own whole-run aggregates |
| `report.md` | finalize | per-phase and per-task tables built from the above |

The ledger is self-contained for wall clock and pi-side token usage. The server-side
numbers are only correct if ninfer's ops counters start from zero, which is why the request
log must be rotated at restart (step 1 below). The ops service sums over the last 50,000
logged requests, not over the current instance.

## Fairness controls (keep identical across runs)

- Same base tag, same pi + pi-subagents versions, same `~/.pi/agent/settings.json`
  (model `redtruck-wide/Qwen3.8-27B-NVFP4` for children, thinking levels, extensions).
- Same ninfer binary, model, `--max-concurrency 4`, `--max-context`, `--default-max-tokens`,
  and host context size. Only the speculative method and the KV dtype differ. The
  manifest records the full argv, so the judge can verify this.
- Restart ninfer and rotate its request log before every run, so each run starts with an
  empty RAM tier, a cold prefix cache, and zeroed ops counters.
- No other inference load on the server during a run.
- The research pages are scraped live. P1 records their sha256, so the judge can tell if
  they changed between runs.

## Runbook

### Once

pi must trust this project, or it won't load `.pi/agents/` and the custom agents will be
missing. Start pi in the repo once and accept project trust. Then confirm
`bench-worker` and `bench-reviewer` show up in `subagent({ action: "list" })`.

### Per run (`<id>` from the table)

1. **Restart ninfer** with the run's config, and **move `/var/lib/ninfer/requests.jsonl`
   aside** first (restart the ops service too if it caches the log). Wait until the model
   is loaded and idle.
2. **Preflight**, from the repo root on a clean `main`:
   ```sh
   git switch main && git pull --ff-only
   plan/scripts/preflight.sh <id>
   ```
   It refuses to run if the ops counters weren't reset (`ALLOW_STALE_LOG=1` overrides
   this). It prints the speculative method and KV dtype it sees, so check that they
   match the table.
3. **Start the collector** in its own tmux pane: `plan/scripts/sample_server.sh`.
4. **Kick off**: start a fresh `pi` session at the repo root and paste the prompt from
   `KICKOFF.md`. Then leave it alone.
5. **Postflight**, once the PR is open: stop the collector (Ctrl-C) and run
   `plan/scripts/finalize.sh` once more, so `report.md` includes the final requests.
   Then `git switch main` before the next run.

### After all four runs

Give the judge the four PRs, the four `~/encode_queue_bench/<id>/` dirs, and
`plan/EVALUATION.md`.
