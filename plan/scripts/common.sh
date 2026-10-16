# Sourced by the bench scripts. Shared paths and the ninfer counter snapshot.
NINFER_OPS="${NINFER_OPS:-https://llm.mist-gamma.ts.net:8443/ops}"
BENCH_ROOT="${BENCH_ROOT:-$HOME/encode_queue_bench}"
BENCH_DIR="${BENCH_DIR:-$BENCH_ROOT/current}"

now_iso() { date -u +%Y-%m-%dT%H:%M:%S.%3NZ; }
now_ms() { date +%s%3N; }

# Prints one compact JSON object of ninfer counters/gauges (labels summed), or {"unreachable":true}.
ninfer_snapshot() {
  local body
  body=$(curl -sf --max-time 3 "$NINFER_OPS/metrics") || { echo '{"unreachable":true}'; return; }
  awk '!/^#/ && NF >= 2 { name = $1; sub(/\{.*/, "", name); sum[name] += $NF }
       END { for (n in sum) printf "%s %s\n", n, sum[n] }' <<<"$body" \
    | grep -E '^ninfer_(requests|prompt_tokens|cached_prompt_tokens|computed_prefill_tokens|completion_tokens|mtp_drafted_tokens|mtp_accepted_tokens|ram_restores|ram_kv_h2d_bytes|ram_kv_h2d_seconds|preemptions|snapshot_restores|replay_restores|replayed_tokens|spill_pages|log_parse_errors)_total |^ninfer_(running|waiting|paused|host_kv_bytes|host_context_bytes|decode_tokens_per_second|prefill_tokens_per_second|decode_batch_size) ' \
    | jq -Rn '[inputs | split(" ") | {key: (.[0] | ltrimstr("ninfer_")), value: (.[1] | tonumber)}] | from_entries'
}

# Appends request records newer than the newest one already saved to <dir>/requests.jsonl.
# Only requests completed after the run's preflight are kept.
collect_requests() {
  local dir="$1" n="${2:-1000}" since
  since=$( { tail -n 1 "$dir/requests.jsonl" 2>/dev/null || true; } | jq -r '.ts // empty')
  [[ -n "$since" ]] || since=$(jq -s 'map(select(.event == "preflight")) | first.epoch_ms // 0' "$dir/ledger.jsonl")
  curl -sf --max-time 30 "$NINFER_OPS/requests?n=$n" \
    | jq -c --argjson s "$since" 'sort_by(.ts) | .[] | select(.ts > $s)' >> "$dir/requests.jsonl" \
    || echo "warning: request collection failed at $(now_iso)" >&2
}

# Starts the background collector for <dir> unless its recorded PID is still alive.
# Prints "started" or "running".
ensure_sampler() {
  local dir="$1" pid
  pid=$(cat "$dir/sampler.pid" 2>/dev/null || true)
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then echo running; return; fi
  BENCH_DIR="$dir" setsid nohup "$(dirname "${BASH_SOURCE[0]}")/sample_server.sh" \
    >> "$dir/sampler.log" 2>&1 < /dev/null &
  echo $! > "$dir/sampler.pid"
  echo started
}

stop_sampler() {
  local dir="$1" pid
  pid=$(cat "$dir/sampler.pid" 2>/dev/null || true)
  [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  rm -f "$dir/sampler.pid"
}
