#!/usr/bin/env bash
# Usage: ledger.sh <event> [task-id] [role] [key=value ...]
# Appends one line to $BENCH_DIR/ledger.jsonl with a wall-clock timestamp and a compact
# snapshot of ninfer's cumulative counters, so per-phase deltas need nothing else.
# Numeric values (e.g. output_tokens=1234) are stored as numbers.
set -euo pipefail
source "$(dirname "$0")/common.sh"
[[ -d "$BENCH_DIR" ]] || { echo "BENCH_DIR $BENCH_DIR missing; run preflight" >&2; exit 1; }

event="${1:?event required}"; shift
task=""; role=""
[[ $# -gt 0 && "$1" != *=* ]] && { task="$1"; shift; }
[[ $# -gt 0 && "$1" != *=* ]] && { role="$1"; shift; }

fields='{}'
for kv in "$@"; do
  fields=$(jq -c --arg k "${kv%%=*}" --arg v "${kv#*=}" \
    '. + {($k): ($v | if test("^-?[0-9]+(\\.[0-9]+)?$") then tonumber else . end)}' <<<"$fields")
done

jq -nc --arg ts "$(now_iso)" --argjson epoch_ms "$(now_ms)" \
  --arg event "$event" --arg task "$task" --arg role "$role" \
  --argjson fields "$fields" --argjson server "$(ninfer_snapshot)" \
  '{ts: $ts, epoch_ms: $epoch_ms, event: $event, task: $task, role: $role} + $fields + {server: $server}' \
  >> "$BENCH_DIR/ledger.jsonl"

# Self-heal the collector at every phase boundary (request collection resumes losslessly).
if [[ "$event" == "phase_start" && "$(ensure_sampler "$(readlink -f "$BENCH_DIR")")" == started ]]; then
  "$0" note "$task" msg="collector was not running; restarted"
fi
