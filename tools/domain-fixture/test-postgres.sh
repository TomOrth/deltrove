#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
for executable in initdb pg_ctl psql; do
  if ! command -v "$executable" >/dev/null; then
    echo "$executable is required on PATH for the disposable PostgreSQL test cluster" >&2
    exit 1
  fi
done
cluster="$(mktemp -d /tmp/deltrove-del26.XXXXXX)"
cleanup() {
  if [[ -f "$cluster/data/postmaster.pid" ]]; then
    pg_ctl -D "$cluster/data" -m immediate -w -t 15 stop >/dev/null || true
  fi
  rm -rf "$cluster"
}
trap cleanup EXIT
mkdir "$cluster/socket"
initdb -D "$cluster/data" --no-locale --encoding=UTF8 --auth=trust --username=del26 >/dev/null
pg_ctl -D "$cluster/data" -l "$cluster/postgres.log" -w -t 15 \
  -o "-k $cluster/socket -h '' -p 5432 -c fsync=on -c synchronous_commit=on" start >/dev/null
export DATABASE_URL="postgresql://del26@localhost/postgres?host=$cluster/socket&port=5432"
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -c 'SHOW server_version; SHOW fsync; SHOW synchronous_commit;'
cargo test --locked -p domain-fixture --test postgres -- --ignored --test-threads=1
