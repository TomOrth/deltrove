#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
baseline="$repo_root/api/baseline/openapi.yaml"
temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT

if [[ -n "${OASDIFF_BIN:-}" ]]; then
  oasdiff_bin="$OASDIFF_BIN"
else
  GOBIN="$temporary_dir/bin" go install github.com/oasdiff/oasdiff@v1.33.0
  oasdiff_bin="$temporary_dir/bin/oasdiff"
fi
oasdiff=("$oasdiff_bin")

"${oasdiff[@]}" breaking --fail-on ERR "$baseline" "$repo_root/api/openapi.yaml"

shopt -s nullglob
fixtures=("$repo_root"/api/fixtures/incompatible/*.yaml)
if ((${#fixtures[@]} == 0)); then
  echo "no incompatible contract fixtures were found" >&2
  exit 1
fi
for fixture in "${fixtures[@]}"; do
  set +e
  output="$("${oasdiff[@]}" breaking --fail-on ERR "$baseline" "$fixture" 2>&1)"
  status=$?
  set -e
  if ((status == 0)); then
    echo "expected compatibility check to reject $fixture" >&2
    exit 1
  fi
  if ((status != 1)); then
    printf 'oasdiff failed while checking %s (exit %s):\n%s\n' "$fixture" "$status" "$output" >&2
    exit "$status"
  fi
  echo "compatibility check rejected $(basename "$fixture") as expected"
done

# Verify malformed specs stay distinct from a valid breaking-change result.
malformed_fixture="$temporary_dir/malformed.yaml"
printf 'openapi: [\n' >"$malformed_fixture"
set +e
output="$("${oasdiff[@]}" breaking --fail-on ERR "$baseline" "$malformed_fixture" 2>&1)"
status=$?
set -e
if ((status != 102)); then
  printf 'expected malformed spec load failure (exit 102), got %s:\n%s\n' "$status" "$output" >&2
  exit 1
fi
echo "compatibility harness distinguishes malformed specs (oasdiff exit 102) from breaking changes (exit 1)"
