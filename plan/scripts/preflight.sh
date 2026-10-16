#!/usr/bin/env bash
# Usage: preflight.sh <run-id>     e.g. preflight.sh mtp-run1
# Run from any checkout of main, right after restarting ninfer with this run's config.
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
# Each run must start on a freshly restarted server (empty RAM tier, cold prefix cache).
# Old log contents are fine: all metrics are selected by timestamp.
started=$(jq -r '.config.started_unix_ms // 0' <<<"$summary")
uptime_min=$(( ($(now_ms) - started) / 60000 ))
if (( started == 0 || uptime_min > ${MAX_UPTIME_MIN:-120} )); then
  echo "ninfer has been up ${uptime_min} min. Restart ninfer with this run's config right before the run." >&2
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
mkdir -p "$dir"/{reports,dag,fixes,research/raw,scratch}

# Pinned toolchain: this container has no Rust on PATH. Agents run e.g. `$dir/dev cargo test`.
(cd "$clone" && nix develop . --profile "$dir/devshell" -c true 2>/dev/null) || { echo "nix dev shell failed" >&2; exit 1; }
printf '#!/usr/bin/env bash\nexec nix develop %q -c "$@"\n' "$dir/devshell" > "$dir/dev"
chmod +x "$dir/dev"
(cd "$clone" && "$dir/dev" cargo build -q) || { echo "cargo build failed" >&2; exit 1; }
rustc_v=$("$dir/dev" rustc --version)
ln -sfn "$dir" "$BENCH_ROOT/current"
: > "$dir/known_issues.md"

jq -n --arg run_id "$run_id" --arg base "$(git -C "$clone" rev-parse bench-base-v1^{commit})" \
  --arg clone "$clone" --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg host "$(hostname)" \
  --arg rustc "$rustc_v" --arg pi "$(pi --version 2>/dev/null || echo unknown)" \
  --argjson uptime "$uptime_min" \
  --argjson server "$(jq '.config' <<<"$summary")" \
  '{run_id: $run_id, base_sha: $base, repo: $clone, preflight_at: $at, host: $host, rustc: $rustc,
    pi_version: $pi, lanes: 4, max_review_rounds: 3, server_uptime_min_at_preflight: $uptime, server: $server}' \
  > "$dir/manifest.json"

BENCH_DIR="$dir" "$(dirname "$0")/ledger.sh" preflight
echo "collector: $(ensure_sampler "$dir") (pid $(cat "$dir/sampler.pid"))"
jq -r '"ready: \(.run_id)  spec=\(.server.speculative)  kv=\(.server.argv | index("--kv-dtype") as $i | if $i then .[$i+1] else "default" end)  repo=\(.repo)"' "$dir/manifest.json"
