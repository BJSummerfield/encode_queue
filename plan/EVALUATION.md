# Evaluation rubric (final judge)

You compare four runs of the same autonomous agentic-coding plan, two per inference-server
configuration:

| Config | Runs |
|--------|------|
| MTP: EAGLE/MTP speculative decoding | `mtp-run1`, `mtp-run2` |
| DFlash: `dflash2` speculative decoding, KV `k8v4` | `dflash-k8v4-run1`, `dflash-k8v4-run2` |

Answer two questions. **Which configuration is better for this owner's agentic coding
workflow, and why?** And **how much do two runs of the same configuration drift apart?**
The drift sets the noise floor: a difference between configs counts only if it is clearly
larger than the drift within a config.

## Inputs

- Four PRs, from branches `bench/<run-id>`, all forked from `bench-base-v1`.
- For each run, `~/encode_queue_bench/<run-id>/` (also committed under `.bench/<run-id>/`
  in its PR):
  - `manifest.json`: full ninfer config and argv, pi and rustc versions.
  - `ledger.jsonl`: phase, task, review, merge, and gate events with wall-clock time, pi
    child usage, and a snapshot of ninfer gauges.
  - `requests.jsonl`: every ninfer request in the run.
  - `server.jsonl`: gauges every 5 s.
  - `tiles.jsonl`: `/ops/summary?window=15m`, fetched back to back every 15 min from
    preflight to the end of the run, with `gap_ms`/`overlap_ms` per tile.
  - `report.md`: produced by `plan/scripts/ledger_report.sh`.
  - `decisions.md`, `known_issues.md`, and `reviews/`.
- `plan/TASKS.md`: the specs every run was held to.

First, check the manifests. They must differ only in `--spec`/speculative settings and
`--kv-dtype`. Anything else that differs is a confound, and you must report it.

## A. Inference efficiency (server-attributable)

Build these per run, per phase, and for the whole run from `requests.jsonl`, using the
phase windows in the ledger.

| Metric | Notes |
|--------|-------|
| Decode tok/s per request: p50, p10 | `output / decode`. **Also split by concurrent load**: count the requests whose `[ts - total, ts]` window overlaps, because speculative methods degrade differently under batching |
| Speculative acceptance: `accepted / drafted` | The core DFlash vs MTP signal. Also compare tokens accepted per decode-second |
| TTFT p50/p90, queue p90 | Queueing appears when 4 sequences are busy |
| Prefix cache: `cached / prompt`; re-prefilled tokens | Long agent conversations depend on this |
| Preemptions, paused seconds, replayed tokens | KV pressure. Lower-precision KV (`k8v4`) should mean fewer of these |
| Spill pages, RAM restores, host→device KV bytes and seconds | From `tiles.jsonl`. The run total is exact if the tile gaps are ~0. Per-phase values are approximate at 15-minute resolution. Never use ninfer `*_total` counters: they span the whole retained log |
| Non-`stop_token` finishes | `output_limit`, `context_capacity`, or `cancelled` = harmed agent turns |

## B. Agentic process (trajectory-attributable)

From `ledger.jsonl`:

| Metric | Notes |
|--------|-------|
| Total wall clock, wall clock per phase | |
| Wall clock per fixed-ID task (P1.*, P2, P3.*, P4, P6.H, P6.F*) | Fixed IDs are comparable one-to-one across all 4 runs |
| Child tokens (input, output, cache-read) and turns per task | From `task_end` usage fields |
| First-pass review PASS rate; mean rounds; escalated and dropped tasks | |
| Child errors and relaunches; post-merge gate failures; merge conflicts | |
| Number of P5 work packages and P7 fix tasks the run created | Structure, not quality |
| Human interventions | Must be 0 |

**Normalise.** Report wall clock per 1k output tokens next to raw wall clock. Then
separate "faster because the server decodes faster" from "faster because the run took a
shorter path" (fewer rounds, fewer WPs, less re-work).

## C. Output quality (blinded)

Before scoring, have a helper check out the four PR heads under random labels `W X Y Z`.
Do not look at the mapping until every row is scored. Score each item 0–5 and cite evidence.

1. **Gates**: `cargo fmt --check`, clippy with `-D warnings`, test, and release build.
2. **Behaviour contract** preserved (TASKS.md). Verify black-box with a temp
   `ENCODE_QUEUE_DIR`.
3. **Feature spec compliance** for F1–F4. Check every stated exit code, message, and file
   format with your own black-box checks, not the run's tests.
4. **Research fidelity**: snippet counts match the raw pages, and rules are supported by
   their cited snippets.
5. **CONVENTIONS.md quality**: owner preferences encoded, checkable rules, conflicts
   resolved.
6. **Conformance**: enum dispatch with a `macro_rules!` delegation macro (no `dyn` ports);
   `thiserror` only (`grep -r anyhow` must be empty); private fields; newtypes that parse
   at the boundary; no `unwrap`/`expect`/`panic` outside tests; dependencies point inward
   only.
7. **Test quality**: mutate one line per feature and see whether a test fails.
8. **Overall code quality**: readability, idiomatic Rust, no dead code, README accuracy.
9. **Honesty**: does `known_issues.md` match what you actually found?

## D. Drift and confounds

- **Within-config drift**: for every A/B/C metric, report |run1 − run2| for each config.
  Call a config difference meaningful only if it exceeds both configs' drift.
- **Structural drift**: compare `docs/CONVENTIONS.md`, `docs/refactor/PLAN.md` (WP count
  and shape), and the final `src/` layout across the four runs. Did the same config
  converge on the same design?
- Manifest differences beyond spec and KV dtype; source page hash differences between
  runs (`docs/research/*/SOURCE.md`); tile gaps (collector downtime); server uptime at preflight.

## Output

Write `bench-comparison.md` with:
1. **Verdict** (one paragraph): which config to use for agentic coding, and the confidence
   given the drift.
2. Side-by-side tables for A, B, and C: 4 run columns plus a mean per config.
3. A drift table: within-config versus between-config deltas for the key metrics.
4. The three largest real differences, each with its most likely cause (server or
   trajectory).
5. Recommendations: what to change in the server config, and what to re-run to raise
   confidence.
