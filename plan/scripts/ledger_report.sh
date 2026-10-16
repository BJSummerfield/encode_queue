#!/usr/bin/env bash
# Usage: ledger_report.sh [bench-dir]   -> markdown on stdout
# Everything is attributed by timestamp: ledger events give phase/task windows, request
# records fall in a phase by completion time, and 15-minute tiles by the phase containing
# the tile's end (tiles are coarser than short phases; the run total is exact).
set -euo pipefail
source "$(dirname "$0")/common.sh"
dir="$(readlink -f "${1:-$BENCH_DIR}")"
touch "$dir/requests.jsonl" "$dir/tiles.jsonl"

jq -rn --slurpfile l "$dir/ledger.jsonl" --slurpfile req "$dir/requests.jsonl" \
       --slurpfile tiles "$dir/tiles.jsonl" --slurpfile m "$dir/manifest.json" '
  def secs($a; $b): ((($b - $a) / 1000) | floor);
  def p($q): if length == 0 then "-" else (sort | .[((length - 1) * $q | floor)] * 100 | round / 100) end;
  def sum(f): map(f // 0) | add // 0;
  def r2: . * 100 | round / 100;
  def reqrow($name; $r):
    ($r | map(select(.decode > 0 and .output > 1))) as $d
    | "| \($name) | \($r | length) | \($r | sum(.output)) | \($r | sum(.computed)) | \($d | map(.output / .decode) | p(0.5)) / \($d | map(.output / .decode) | p(0.1)) | \($r | map(.ttft) | p(0.5)) / \($r | map(.ttft) | p(0.9)) | \($r | map(.queue) | p(0.9)) | \(if ($r | sum(.drafted)) > 0 then (($r | sum(.accepted)) * 1000 / ($r | sum(.drafted)) | round / 10) else "-" end) | \($r | sum(.preemptions)) | \($r | sum(.paused) | round) | \($r | sum(.replayed)) | \($r | map(select(.finish != "stop_token")) | length) |";
  def tilerow($name; $t):
    "| \($name) | \($t | length) | \($t | sum(.ram_tier.spill_pages)) | \($t | sum(.ram_tier.restores)) | \($t | sum(.ram_tier.kv_h2d_gib) | r2) | \($t | sum(.ram_tier.kv_h2d_seconds) | r2) | \($t | map(.ram_tier.peak_host_kv_gib // 0) | max // 0) | \($t | sum(.scheduling.preemptions)) | \($t | sum(.scheduling.paused_seconds) | round) | \($t | map(.scheduling.peak_paused // 0) | max // 0) |";

  $l as $l | $req as $req | $tiles as $tiles | $m[0] as $m
  | ($l | map(select(.event == "phase_start")) | map({key: .task, value: .epoch_ms}) | from_entries) as $ps
  | ($l | map(select(.event == "phase_end"))   | map({key: .task, value: .epoch_ms}) | from_entries) as $pe
  | [$ps | keys_unsorted[] | select($pe[.])] as $phases
  | ($l | first.epoch_ms) as $t0 | ($l | last.epoch_ms) as $t1
  | "# Run report: \($m.run_id)\n",
    "Server: spec=\($m.server.speculative) kv=\($m.server.argv | index("--kv-dtype") as $i | if $i then .[$i+1] else "default" end) instance=\($m.server.instance) (up \($m.server_uptime_min_at_preflight) min at preflight)\n",
    "Total wall clock (preflight → last ledger event): \(secs($t0; $t1)) s\n",
    "## Requests by phase (ninfer per-request records, exact)\n",
    "| phase | reqs | out tok | prefilled tok | decode tok/s p50 / p10 | ttft s p50 / p90 | queue s p90 | spec accept % | preemptions | paused s | replayed tok | non-stop finishes |",
    "|---|---|---|---|---|---|---|---|---|---|---|---|",
    ( $phases[] as $p | reqrow("\($p) (\(secs($ps[$p]; $pe[$p])) s)"; $req | map(select(.ts >= $ps[$p] and .ts <= $pe[$p]))) ),
    reqrow("**run**"; $req),
    "\n## RAM tier and scheduling (15-minute tiles; per-phase rows are approximate, the run row is exact)\n",
    "| phase | tiles | spill pages | RAM restores | KV h2d GiB | KV h2d s | peak host KV GiB | preemptions | paused s | peak paused |",
    "|---|---|---|---|---|---|---|---|---|---|",
    ( $phases[] as $p | tilerow($p; $tiles | map(select(.window_end_ms > $ps[$p] and .window_end_ms <= ($pe[$p] + $TILE)))) ),
    tilerow("**run**"; $tiles),
    (if ($tiles | sum(.gap_ms)) > 10000 then "\n⚠ tiles have \($tiles | sum(.gap_ms) / 1000 | round) s of gaps (collector was down); RAM-tier totals undercount." else empty end),
    "\n## Tasks\n",
    "| task | role | round | wall s | outcome | out tok | turns | tool calls |",
    "|---|---|---|---|---|---|---|---|",
    ( [ $l[] | select(.event == "task_start" or .event == "task_end") ]
      | group_by([.task, .role, (.round // 0)]) | sort_by(first.epoch_ms)[]
      | (map(select(.event == "task_start")) | first) as $a
      | (map(select(.event == "task_end")) | last) as $b
      | select($a and $b)
      | "| \($a.task) | \($a.role) | \($a.round // "") | \(secs($a.epoch_ms; $b.epoch_ms)) | \($b.verdict // $b.status // "") | \($b.output_tokens // "") | \($b.turns // "") | \($b.tool_calls // "") |" ),
    "\n## Merges\n",
    ( $l | map(select(.event == "merge")) | map("- \(.task): rounds=\(.rounds // "?") outcome=\(.outcome // "?")") | .[] )
' --argjson TILE "$TILE_MS"
