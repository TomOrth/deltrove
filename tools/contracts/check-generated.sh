#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT
mkdir -p "$temp_dir/go/generated" "$temp_dir/ts"
cp "$repo_root/clients/go/oapi-codegen.yaml" "$temp_dir/go/oapi-codegen.yaml"

(cd "$temp_dir/go" && go run github.com/oapi-codegen/oapi-codegen/v2/cmd/oapi-codegen@v2.8.0 \
  -config oapi-codegen.yaml "$repo_root/api/openapi.yaml")
npm --prefix "$repo_root/clients/typescript" exec -- openapi-typescript \
  "$repo_root/api/openapi.yaml" -o "$temp_dir/ts/api.d.ts"

if ! cmp -s "$temp_dir/go/generated/api.gen.go" "$repo_root/clients/go/generated/api.gen.go"; then
  echo "generated Go client is stale; run tools/contracts/generate-clients.sh" >&2
  exit 1
fi
if ! cmp -s "$temp_dir/ts/api.d.ts" "$repo_root/clients/typescript/src/generated/api.d.ts"; then
  echo "generated TypeScript types are stale; run tools/contracts/generate-clients.sh" >&2
  exit 1
fi
echo "generated Go and TypeScript outputs match the OpenAPI source"
