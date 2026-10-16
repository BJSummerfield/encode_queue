#!/usr/bin/env bash
# Usage: ledger_report.sh [bench-dir]   -> markdown on stdout
# Phase table (wall clock + server counter deltas), task table, and request-level stats.
set -euo pipefail
source "$(dirname "$0")/common.sh"
dir="$(readlink -f "${1:-$BENCH_DIR}")"

jq -rs '
  def d($a; $b; $k): (($b.server[$k] // 0) - ($a.server[$k] // 0));
  def secs($a; $b): ((($b.epoch_ms - $a.epoch_ms) / 1000) | floor);
  def pct($n; $d): if $d > 0 then ((1000 * $n / $d | round) / 10 | tostring) + "%" else "-" end;
  . as $all
  | (map(select(.event == "phase_start")) | map({key: .task, value: .}) | from_entries) as $ps
  | (map(select(.event == "phase_end"))   | map({key: .task, value: .}) | from_entries) as $pe
  | ($all | first) as $first | ($all | last) as $last
  | "# Run report: \($first.task // "")\n",
    "Total wall clock: \(secs($first; $last)) s\n",
    "## Phases\n",
    "| phase | wall s | out tok | prefill tok | cached % | spec accept % | preempt | spill pages | RAM restores | replayed tok |",
    "|---|---|---|---|---|---|---|---|---|---|",
    ( $ps | keys_unsorted[] as $p | select($pe[$p]) | $ps[$p] as $a | $pe[$p] as $b
      | "| \($p) | \(secs($a; $b)) | \(d($a;$b;"completion_tokens_total")) | \(d($a;$b;"computed_prefill_tokens_total")) | \(pct(d($a;$b;"cached_prompt_tokens_total"); d($a;$b;"prompt_tokens_total"))) | \(pct(d($a;$b;"mtp_accepted_tokens_total"); d($a;$b;"mtp_drafted_tokens_total"))) | \(d($a;$b;"preemptions_total")) | \(d($a;$b;"spill_pages_total")) | \(d($a;$b;"ram_restores_total")) | \(d($a;$b;"replayed_tokens_total")) |" ),
    "\n## Tasks\n",
    "| task | role | round | wall s | outcome | child tokens out | child turns |",
    "|---|---|---|---|---|---|---|",
    ( [ .[] | select(.event == "task_start" or .event == "task_end") ]
      | group_by([.task, .role, (.round // 0)])[]
      | (map(select(.event == "task_start")) | first) as $a
      | (map(select(.event == "task_end")) | last) as $b
      | select($a and $b)
      | "| \($a.task) | \($a.role) | \($a.round // "") | \(secs($a; $b)) | \($b.verdict // $b.status // "") | \($b.output_tokens // "") | \($b.turns // "") |" ),
    "\n## Merges\n",
    ( map(select(.event == "merge")) | map("- \(.task): rounds=\(.rounds // "?") outcome=\(.outcome // "?")") | .[] )
' "$dir/ledger.jsonl"

if [[ -s "$dir/requests.jsonl" ]]; then
  # Phases are serial, so each request (ts = completion time) belongs to exactly one phase window.
  jq -rn --slurpfile ledger "$dir/ledger.jsonl" '
    def p($q): if length == 0 then "-" else (sort | .[((length - 1) * $q | floor)] * 100 | round / 100) end;
    def row($name; $r):
      ($r | map(select(.decode > 0 and .output > 1))) as $d
      | "| \($name) | \($r | length) | \($r | map(.output) | add // 0) | \($r | map(.computed) | add // 0) | \($d | map(.output / .decode) | p(0.5)) / \($d | map(.output / .decode) | p(0.1)) | \($r | map(.ttft) | p(0.5)) / \($r | map(.ttft) | p(0.9)) | \($r | map(.queue) | p(0.9)) | \(if ($r | map(.drafted) | add // 0) > 0 then (($r | map(.accepted) | add) * 1000 / ($r | map(.drafted) | add) | round / 10) else "-" end) | \($r | map(.preemptions) | add // 0) | \($r | map(.paused) | add // 0 | round) | \($r | map(.replayed) | add // 0) | \($r | map(select(.finish != "stop_token")) | length) |";
    [inputs] as $req
    | $ledger as $l
    | ($l | map(select(.event == "phase_start")) | map({key: .task, value: .epoch_ms}) | from_entries) as $ps
    | ($l | map(select(.event == "phase_end")) | map({key: .task, value: .epoch_ms}) | from_entries) as $pe
    | "\n## Requests by phase (ninfer per-request log)\n",
      "| phase | reqs | out tok | prefilled tok | decode tok/s p50 / p10 | ttft s p50 / p90 | queue s p90 | spec accept % | preemptions | paused s | replayed tok | non-stop finishes |",
      "|---|---|---|---|---|---|---|---|---|---|---|---|",
      ( $ps | keys_unsorted[] as $p | select($pe[$p]) | row($p; $req | map(select(.ts >= $ps[$p] and .ts <= $pe[$p]))) ),
      row("**run**"; $req)
  ' "$dir/requests.jsonl"
fi
