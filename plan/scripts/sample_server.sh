#!/usr/bin/env bash
# Usage: started automatically by preflight.sh (and restarted by ledger.sh if it dies).
# - every $INTERVAL s: gauges + counters -> $BENCH_DIR/server.jsonl (KV pressure over time)
# - every $REQ_INTERVAL s: new per-request records from /ops/requests -> $BENCH_DIR/requests.jsonl
#   (/ops/requests returns at most 5000 rows, so a long run must be collected incrementally)
set -euo pipefail
source "$(dirname "$0")/common.sh"
INTERVAL="${INTERVAL:-5}"; REQ_INTERVAL="${REQ_INTERVAL:-30}"
dir="$(readlink -f "$BENCH_DIR")"
echo "sampling -> $dir/server.jsonl, $dir/requests.jsonl"
last_req=0
while true; do
  jq -nc --arg ts "$(now_iso)" --argjson epoch_ms "$(now_ms)" --argjson s "$(ninfer_snapshot)" \
    '{ts: $ts, epoch_ms: $epoch_ms} + $s' >> "$dir/server.jsonl"
  if (( $(date +%s) - last_req >= REQ_INTERVAL )); then
    collect_requests "$dir" 1000
    last_req=$(date +%s)
  fi
  sleep "$INTERVAL"
done
