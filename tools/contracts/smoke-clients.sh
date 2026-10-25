#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
cargo build --locked -p deltrove-api

port=$((30000 + $$ % 20000))
server_log="${TMPDIR:-/tmp}/deltrove-api-$port.log"
RUST_LOG=info DELTROVE_BIND="127.0.0.1:$port" target/debug/deltrove-api >"$server_log" 2>&1 &
server_pid=$!
cleanup() {
  kill "$server_pid" 2>/dev/null || true
  stopped=false
  for _ in {1..20}; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
      wait "$server_pid" 2>/dev/null || true
      stopped=true
      break
    fi
    sleep 0.1
  done
  if [[ "$stopped" != true ]]; then
    kill -KILL "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -f "$server_log"
}
trap cleanup EXIT

export DELTROVE_BASE_URL="http://127.0.0.1:$port"
ready=false
for _ in {1..40}; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$server_log" >&2
    exit 1
  fi
  if curl --fail --silent --connect-timeout 0.2 --max-time 0.5 \
    "$DELTROVE_BASE_URL/health/ready" >/dev/null; then
    ready=true
    break
  fi
  sleep 0.1
done
if [[ "$ready" != true ]]; then
  echo "service did not become ready at $DELTROVE_BASE_URL" >&2
  cat "$server_log" >&2
  exit 1
fi

(cd clients/go && go run ./cmd/smoke)
npm --prefix clients/typescript run build
npm --prefix clients/typescript run smoke
