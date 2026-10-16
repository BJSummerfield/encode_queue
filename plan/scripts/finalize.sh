#!/usr/bin/env bash
# Usage: finalize.sh     (P8; safe to re-run by hand)
# Stops the collector, does a final request collection, saves the whole-run ninfer summary, writes report.md.
set -euo pipefail
source "$(dirname "$0")/common.sh"
dir="$(readlink -f "$BENCH_DIR")"
stop_sampler "$dir"
collect_requests "$dir" 5000
curl -sf --max-time 30 "$NINFER_OPS/summary?window=all" > "$dir/summary.json" \
  || echo "warning: could not fetch ninfer summary" >&2
"$(dirname "$0")/ledger_report.sh" "$dir" > "$dir/report.md"
echo "wrote $dir/report.md ($(wc -l < "$dir/requests.jsonl") requests collected)"
