#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

(cd clients/go && go run github.com/oapi-codegen/oapi-codegen/v2/cmd/oapi-codegen@v2.8.0 \
  -config oapi-codegen.yaml ../../api/openapi.yaml)
npm --prefix clients/typescript ci
npm --prefix clients/typescript run generate
