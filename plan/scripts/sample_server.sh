#!/usr/bin/env bash
# Usage: started automatically by preflight.sh (and restarted by ledger.sh if it dies).
# - every $INTERVAL s: point-in-time gauges -> server.jsonl
# - every $REQ_INTERVAL s: new per-request records -> requests.jsonl (/ops/requests caps at 5000 rows)
# - every 15 min, back to back from preflight: /ops/summary?window=15m -> tiles.jsonl
set -euo pipefail
source "$(dirname "$0")/common.sh"
INTERVAL="${INTERVAL:-5}"; REQ_INTERVAL="${REQ_INTERVAL:-30}"
dir="$(readlink -f "$BENCH_DIR")"
echo "collecting -> $dir at $(now_iso)"
last_req=0
while true; do
  jq -nc --arg ts "$(now_iso)" --argjson epoch_ms "$(now_ms)" --argjson s "$(ninfer_snapshot)" \
    '{ts: $ts, epoch_ms: $epoch_ms} + $s' >> "$dir/server.jsonl"
  if (( $(date +%s) - last_req >= REQ_INTERVAL )); then
    collect_requests "$dir" 1000
    last_req=$(date +%s)
  fi
  due=$(( $(last_tile_end "$dir") + TILE_MS )); left=$(( due - $(now_ms) ))
  if (( left <= INTERVAL * 1000 )); then
    (( left > 0 )) && sleep "$(awk -v ms="$left" 'BEGIN { printf "%.3f", ms / 1000 }')"
    collect_tile "$dir"
    continue
  fi
  sleep "$INTERVAL"
done
