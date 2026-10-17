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
| `KICKOFF.md` | orchestrator | The prompt you paste. Only its `RUN_ID=` line changes between runs |
| `ORCHESTRATOR.md` | orchestrator | Continuous batching, git/worktrees, review loop, ledger |
| `TASKS.md` | everyone | Phase/task DAG, specs, acceptance criteria |
| `EVALUATION.md` | final judge (Opus/Fable) | Rubric for comparing the runs |
| `../AGENTS.md` | everyone | Role rules loaded into every session: the orchestrator only dispatches; `worker` (work, merges) and `reviewer` (reviews) follow a 3-line output contract |
| `scripts/preflight.sh` | orchestrator (P0) | Isolated clone, metrics dir, manifest, toolchain wrapper, starts the collector |
| `scripts/sample_server.sh` | preflight (background) | Background collector: KV gauges plus every ninfer request record |
| `scripts/ledger.sh` | orchestrator | Appends ledger events, each with a snapshot of ninfer gauges |
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

Every worker task gets its own git worktree, an adversarial reviewer, and a merger, all using
your global pi-subagents profiles: `worker` (work, merges, setup) and `reviewer` (reviews; your
override gives it bash, so it runs cargo and the binary). The
orchestrator is a pure dispatcher (see `../AGENTS.md`). It keeps 4 lanes in flight, matching
ninfer's `--max-concurrency 4`, and refills a lane whenever a sub-agent finishes. Its own
short turns briefly queue behind the lanes. Sub-agents write to an output file
(`outputMode: file-only`), and the orchestrator reads only its first 3 lines. No human gates.

## What gets measured

All of it lands in `~/encode_queue_bench/<run-id>/`. At P8 it is also committed to the
PR under `.bench/<run-id>/`.

| File | Source | Contents |
|------|--------|----------|
| `manifest.json` | preflight | run ID, base SHA, pi/rustc versions, **full ninfer config + argv** from `/ops/summary` |
| `ledger.jsonl` | orchestrator via `ledger.sh` | one line per phase/task/review/merge/gate event: wall clock, review verdicts and rounds, pi child usage (tokens, turns, tool calls, duration), plus a snapshot of ninfer gauges (running, waiting, paused, host KV occupancy) |
| `requests.jsonl` | `sample_server.sh` | every ninfer request record: prompt/cached/computed/output tokens, TTFT, queue, decode, accepted/drafted, preemptions, paused, replayed, finish |
| `server.jsonl` | `sample_server.sh` | running/waiting/paused and KV-tier occupancy every 5 s |
| `tiles.jsonl` | collector | `/ops/summary?window=15m` fetched every 15 min, back to back from preflight: spill pages, RAM restores, host→device KV traffic, preemptions, paused time |
| `report.md` | finalize | per-phase and per-task tables built from the above |

The ledger is self-contained for wall clock and pi-side token usage. ninfer's ops service
sums over its whole retained log (the last 50,000 requests, across restarts), so its
`*_total` counters are never used. Everything server-side is selected **by timestamp**:
request records completed after preflight, and 15-minute summary windows tiled back to
back from preflight to the end of the run. The logs on the server are never touched.

## Fairness controls (keep identical across runs)

- Same base tag, same pi + pi-subagents versions, same `~/.pi/agent/settings.json`
  (model `redtruck-wide/Qwen3.8-27B-NVFP4` for children, thinking levels, extensions).
- Same ninfer binary, model, `--max-concurrency 4`, `--max-context`, `--default-max-tokens`,
  and host context size. Only the speculative method and the KV dtype differ. The
  manifest records the full argv, so the judge can verify this.
- Restart ninfer right before every run, so each run starts with an empty RAM tier and a
  cold prefix cache. Preflight refuses to start if ninfer has been up more than 2 hours.
- No other inference load on the server during a run.
- Research inputs are identical across runs: P1 downloads the guides' source markdown from
  `howtocodeit/article-md` at a pinned commit, and checks each file's sha256.

## Isolation

Each run happens in a fresh single-branch clone at `~/eq-runs/<run-id>`, which preflight
creates. The only refs in the clone are `main` and `bench-base-v1`, so agents can't stumble onto
`design-file-improvement-orchestrator` (an earlier hexagonal refactor of this repo) or
another run's `bench/*` branch. Its fetch refspec is limited to `main`. The orchestrator
rules, the kickoff prompt, and both agent definitions also forbid reading other refs,
remotes, PRs, or directories. The Paseo workspace you start in is used only to launch
preflight. It shares every branch of `~/projects/encode_queue`, which is why
all work moves to the clone after P0.

## Runbook (Paseo)

For each run, in the order `mtp-run1`, `dflash-k8v4-run1`, `mtp-run2`, `dflash-k8v4-run2`:

**1. Restart ninfer** with the run's config (MTP, or DFlash + k8v4), your usual way. Logs
stay where they are.

**2. Start the run in Paseo.** Create a new workspace off `main`, open a pi session, and
paste `KICKOFF.md` with the `RUN_ID=` line set. Accept project trust if pi asks. That's
it. In P0 the orchestrator dispatches a setup `worker` that runs `plan/scripts/preflight.sh`, which:
- stops the run if ninfer has been up more than 2 hours (i.e. step 1 was skipped);
- creates the isolated clone `~/eq-runs/<run-id>` and checks it equals `bench-base-v1`;
- creates `~/encode_queue_bench/<run-id>/` with the manifest (including the server config)
  and a `dev` wrapper for the pinned Rust toolchain (this container has no Rust on PATH);
- starts the background collector, which records ninfer request data for the whole run.
  The ledger restarts it at every phase boundary if it has died, and P8's `finalize.sh`
  stops it.

**3. When it's done**, the PR is open (or the branch is pushed with a compare link, if `gh`
isn't logged in). Check that the `Server:` line of `~/encode_queue_bench/<run-id>/report.md`
shows the expected config, then move to the next run.

### After all four runs

Give the judge the four PRs, the four `~/encode_queue_bench/<id>/` dirs, and
`plan/EVALUATION.md`.
