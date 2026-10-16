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

## Isolation

Each run happens in a fresh single-branch clone at `~/eq-runs/<run-id>`, which preflight
creates. The only refs in it are `main` and `bench-base-v1`, so agents can't stumble onto
`design-file-improvement-orchestrator` (an earlier hexagonal refactor of this repo) or
another run's `bench/*` branch. Its fetch refspec is limited to `main`. The orchestrator
rules, the kickoff prompt, and both agent definitions also forbid reading other refs,
remotes, PRs, or directories. Never start a run from a paseo worktree of
`~/projects/encode_queue`, because worktrees share every branch of the main repo.

## Runbook

### 1. Reset ninfer (on the GPU host, not in this container)

ninfer runs in the `local-llm` nixos-container. Its request log is at host path
`/var/lib/local-llm/ninfer/requests.jsonl`. The ops service keeps the last 50,000
requests in memory, so you must stop **both** units, rotate the log, and start them again:

```sh
RUN=mtp-run1   # the run you are about to do
sudo nixos-container run local-llm -- systemctl stop ninfer-metrics ninfer
sudo mv /var/lib/local-llm/ninfer/requests.jsonl /var/lib/local-llm/ninfer/requests.pre-$RUN.jsonl
# set the run's config: NINFER_EXTRA_ARGS=... in /var/lib/local-llm/ninfer.env (or your usual way)
sudo nixos-container run local-llm -- systemctl start ninfer ninfer-metrics
```

Wait until `curl -s https://llm.mist-gamma.ts.net:8443/ops/health` shows the engine `ok`.

### 2. Preflight (in this container, from any up-to-date checkout of `main`)

```sh
plan/scripts/preflight.sh <run-id>
```

Preflight:
- refuses to run if the ops counters weren't reset (`ALLOW_STALE_LOG=1` overrides this,
  and the manifest records that you did);
- creates the isolated clone `~/eq-runs/<run-id>` and checks it is exactly `bench-base-v1`;
- creates `~/encode_queue_bench/<run-id>/` and points `current` at it, which is how the
  identical kickoff prompt knows which run it is;
- records the server config in `manifest.json` and prints `spec=… kv=…`. **Check that it
  matches the run table.**

### 3. Start the collector (own tmux pane, leave it running)

```sh
~/eq-runs/<run-id>/plan/scripts/sample_server.sh
```

### 4. Kick off

```sh
cd ~/eq-runs/<run-id> && pi
```

Accept project trust when pi asks; without it the bench agents don't load. Optionally
check that `bench-worker` and `bench-reviewer` appear in `subagent({ action: "list" })`.
Paste the prompt from `KICKOFF.md`, then leave it alone.

### 5. Postflight (after the PR is open)

Stop the collector with Ctrl-C, then run `~/eq-runs/<run-id>/plan/scripts/finalize.sh`
so `report.md` includes the last requests. Move on to the next run starting at step 1.

### After all four runs

Give the judge the four PRs, the four `~/encode_queue_bench/<id>/` dirs, and
`plan/EVALUATION.md`.
