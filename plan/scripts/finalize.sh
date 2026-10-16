#!/usr/bin/env bash
# Usage: finalize.sh     (P8; safe to re-run by hand)
# Stops the collector, waits for the last 15-minute tile so the RAM-tier data covers the
# whole run (up to 15 min; the run's ledger has already ended, so wall clock is unaffected),
# does a final request collection, saves the run summary, and writes report.md.
set -euo pipefail
source "$(dirname "$0")/common.sh"
dir="$(readlink -f "$BENCH_DIR")"
stop_sampler "$dir"
left=$(( $(last_tile_end "$dir") + TILE_MS - $(now_ms) ))
if (( left > 0 )); then
  echo "waiting $(( left / 1000 )) s for the final RAM-tier tile..."
  sleep "$(awk -v ms="$left" 'BEGIN { printf "%.3f", ms / 1000 }')"
fi
collect_tile "$dir"
collect_requests "$dir" 5000
"$(dirname "$0")/ledger_report.sh" "$dir" > "$dir/report.md"
echo "wrote $dir/report.md ($(wc -l < "$dir/requests.jsonl") requests, $(wc -l < "$dir/tiles.jsonl") tiles)"
