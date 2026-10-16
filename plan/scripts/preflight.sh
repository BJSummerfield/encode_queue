#!/usr/bin/env bash
# Usage: preflight.sh <run-id>     e.g. preflight.sh mtp-run1
# Run from any checkout of main, AFTER ninfer was restarted with a rotated request log.
# Creates an isolated single-branch clone at ~/eq-runs/<run-id> (only main + the base tag
# exist in it, so agents cannot see other branches), the metrics dir, and the manifest.
set -euo pipefail
source "$(dirname "$0")/common.sh"

run_id="${1:?run-id required}"
dir="$BENCH_ROOT/$run_id"
clone="${EQ_RUNS:-$HOME/eq-runs}/$run_id"
origin=$(git remote get-url origin)

[[ -e "$dir" ]] && { echo "$dir already exists; refusing to overwrite" >&2; exit 1; }
[[ -e "$clone" ]] && { echo "$clone already exists; refusing to overwrite" >&2; exit 1; }

summary=$(curl -sf --max-time 5 "$NINFER_OPS/summary?window=15m") || { echo "ninfer ops unreachable at $NINFER_OPS" >&2; exit 1; }
logged=$(curl -sf --max-time 5 "$NINFER_OPS/metrics" | awk '/^ninfer_requests_total/ { s += $NF } END { print s + 0 }')
if (( logged > 50 )) && [[ "${ALLOW_STALE_LOG:-}" != 1 ]]; then
  echo "ninfer ops already counts $logged requests. Rotate the request log and restart ninfer +" >&2
  echo "ninfer-metrics first (see plan/README.md), or set ALLOW_STALE_LOG=1 to accept unreliable deltas." >&2
  exit 1
fi

git clone -q --single-branch --branch main --no-tags "$origin" "$clone"
git -C "$clone" fetch -q --no-tags origin "refs/tags/bench-base-v1:refs/tags/bench-base-v1"
git -C "$clone" remote set-branches origin main
[[ "$(git -C "$clone" rev-parse HEAD)" == "$(git -C "$clone" rev-parse bench-base-v1^{commit})" ]] \
  || { echo "main has moved past bench-base-v1; refusing (both runs must start identically)" >&2; exit 1; }
extra=$(git -C "$clone" for-each-ref --format='%(refname)' \
  | grep -v -x -E 'refs/heads/main|refs/remotes/origin/main|refs/remotes/origin/HEAD|refs/tags/bench-base-v1' || true)
[[ -z "$extra" ]] || { echo "unexpected refs in clone: $extra" >&2; exit 1; }
(cd "$clone" && cargo build -q)

mkdir -p "$dir"/{reviews,research/raw,scratch}
ln -sfn "$dir" "$BENCH_ROOT/current"
: > "$dir/decisions.md"; : > "$dir/known_issues.md"

jq -n --arg run_id "$run_id" --arg base "$(git -C "$clone" rev-parse bench-base-v1^{commit})" \
  --arg clone "$clone" --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg host "$(hostname)" \
  --arg rustc "$(rustc --version)" --arg pi "$(pi --version 2>/dev/null || echo unknown)" \
  --argjson stale "$([[ "${ALLOW_STALE_LOG:-}" == 1 ]] && echo true || echo false)" \
  --argjson server "$(jq '.config' <<<"$summary")" \
  '{run_id: $run_id, base_sha: $base, repo: $clone, preflight_at: $at, host: $host, rustc: $rustc,
    pi_version: $pi, max_in_flight: 3, max_review_rounds: 3, stale_counters: $stale, server: $server}' \
  > "$dir/manifest.json"

BENCH_DIR="$dir" "$(dirname "$0")/ledger.sh" preflight
jq -r '"ready: \(.run_id)  spec=\(.server.speculative)  kv=\(.server.argv | index("--kv-dtype") as $i | if $i then .[$i+1] else "default" end)"' "$dir/manifest.json"
echo "next: start plan/scripts/sample_server.sh in another pane, then:  cd $clone && pi"
