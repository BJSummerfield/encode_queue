#!/usr/bin/env bash
# Usage: preflight.sh <run-id>     e.g. preflight.sh mtp-run1
# Run from the repo root on main, right after restarting ninfer.
set -euo pipefail
source "$(dirname "$0")/common.sh"

run_id="${1:?run-id required}"; dir="$BENCH_ROOT/$run_id"
[[ -e "$dir" ]] && { echo "$dir already exists; refusing to overwrite" >&2; exit 1; }
git rev-parse --verify -q bench-base-v1 >/dev/null || { echo "tag bench-base-v1 missing" >&2; exit 1; }
git rev-parse --verify -q "refs/heads/bench/$run_id" >/dev/null && { echo "branch bench/$run_id already exists" >&2; exit 1; }
[[ -z "$(git status --porcelain)" ]] || { echo "working tree not clean" >&2; exit 1; }

summary=$(curl -sf --max-time 5 "$NINFER_OPS/summary?window=15m") || { echo "ninfer ops unreachable at $NINFER_OPS" >&2; exit 1; }
logged=$(curl -sf --max-time 5 "$NINFER_OPS/metrics" | awk '/^ninfer_requests_total/ { s += $NF } END { print s + 0 }')
if (( logged > 50 )) && [[ "${ALLOW_STALE_LOG:-}" != 1 ]]; then
  echo "ninfer ops already counts $logged requests. Rotate the request log when restarting ninfer so" >&2
  echo "counters start at zero for this run (or set ALLOW_STALE_LOG=1 to accept unreliable counter deltas)." >&2
  exit 1
fi
cargo build -q

mkdir -p "$dir"/{reviews,research/raw,scratch}
ln -sfn "$dir" "$BENCH_ROOT/current"
: > "$dir/decisions.md"; : > "$dir/known_issues.md"

jq -n --arg run_id "$run_id" --arg base "$(git rev-parse bench-base-v1)" \
  --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg host "$(hostname)" \
  --arg rustc "$(rustc --version)" --arg pi "$(pi --version 2>/dev/null || echo unknown)" \
  --argjson server "$(jq '.config' <<<"$summary")" \
  '{run_id: $run_id, base_sha: $base, preflight_at: $at, host: $host, rustc: $rustc,
    pi_version: $pi, max_in_flight: 3, max_review_rounds: 3, server: $server}' > "$dir/manifest.json"

BENCH_DIR="$dir" "$(dirname "$0")/ledger.sh" preflight
jq -r '"ready: \(.run_id)  spec=\(.server.speculative)  kv=\(.server.argv | index("--kv-dtype") as $i | if $i then .[$i+1] else "default" end)"' "$dir/manifest.json"
