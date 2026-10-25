# DEL-25 — Build and contract skeleton

**Roadmap:** F1, dependent on F0. **Status:** implementation evidence recorded 24 October 2026; integrating-owner review pending.

## Handoff

- **Objective:** establish a minimal reproducible Rust service workspace and provisional HTTP contract with generated Go and TypeScript consumers.
- **Contract source:** [`api/openapi.yaml`](../../api/openapi.yaml). `api/baseline/openapi.yaml` is the compatibility floor for this skeleton; intentional breaking changes require an explicit baseline review/update.
- **Owned paths:** root Cargo/toolchain setup, `crates/deltrove-api`, `api`, `clients`, `tools/contracts`, build workflow and this scoped developer documentation. The existing `tools/workloads` crate remains a workspace member.
- **Invariants:** readiness only means process initialization; no external dependency is claimed checked. The service does not accept or persist operations, and does not return fabricated accepted/durable outcomes. The contract remains provisional; canonical formats, production operation semantics, identity and F5 decisions are open.
- **Out of scope:** VCS algorithms, persistence/migrations, transaction fixture (DEL-26/A04), fake CI/fault runner (DEL-27/A05), SQLite/workspace portability (DEL-28/A06), orchestration, auth implementation and UI.
- **Review risks:** generator and compatibility tool version updates must be intentional. The compatibility gate only evaluates the explicitly pinned compatibility floor and the fixtures in `api/fixtures/incompatible/`; it does not replace semantic review.

## Prerequisites and commands

The pinned local toolchains are Rust 1.95.0, Go 1.26.3 (Go module directive 1.26.0), Node.js 23.9.0, and npm 10.9.2. Build commands may download pinned crates, npm packages and Go module tools on first use. No Python is used.

From a clean checkout, install the pinned Rust toolchain with `rustup toolchain install 1.95.0 --component rustfmt --component clippy`, install the declared Go and Node versions, then run:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
npm --prefix clients/typescript ci
tools/contracts/check-generated.sh
tools/contracts/check-compatibility.sh
tools/contracts/smoke-clients.sh
```

Generation is reproducible with `tools/contracts/generate-clients.sh`. It uses `oapi-codegen` v2.8.0 and `openapi-typescript` v7.10.1; the generated Go client uses `oapi-codegen/runtime` v1.6.0, and TypeScript requests use `openapi-fetch` v0.13.8. The compatibility check uses `oasdiff` v1.33.0. Exact npm dependency resolutions are pinned in `clients/typescript/package-lock.json`; Cargo dependencies are locked in the root `Cargo.lock`.

The existing fixture generator remains available through its previous manifest-path invocation and uses the root workspace lockfile:

```sh
cargo test --locked --manifest-path tools/workloads/Cargo.toml
cargo run --locked --manifest-path tools/workloads/Cargo.toml -- all --out /tmp/del25-workloads
```

## Contract and runtime behavior

The Rust service exposes `GET /health/live`, `GET /health/ready`, and a provisional `GET /v1/operations/{operation_id}` lookup. Readiness reflects process initialization only. The operation schema documents a future outcome shape, while the current lookup returns a structured `operation_not_found` problem for every ID. There is no operation creation endpoint, in-memory acceptance, or persistence.

Errors use a small provisional problem object (`type`, `title`, HTTP `status`, stable `code`, `detail`, and `retryable`) with `application/problem+json`. The schema is an initial integration boundary, not a frozen production protocol.

## Evidence

Commands below were run from the repository root on macOS arm64 on 24 October 2026. Tool versions were rustc 1.95.0 (`59807616e 2026-04-14`), Cargo 1.95.0 (`f2d3ce0bd 2026-03-21`), Go 1.26.3 (`darwin/arm64`), Node.js 23.9.0 and npm 10.9.2. Results are recorded after execution; unavailable checks are named **NOT RUN** with the reason.

| Check | Result |
| --- | --- |
| `cargo build --workspace --locked` | **PASS** — clean workspace build completed on Rust 1.95.0; first attempt was blocked writing the Cargo cache under sandboxing, retry passed with cache access. |
| `cargo fmt --all -- --check` | **PASS** — no formatting differences. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | **PASS** — no warnings. |
| `cargo test --locked` | **PASS** — 6 tests passed across service/workload unit and integration tests; loopback integration required socket access. |
| Existing workload fixture tests and generator | **PASS** — workspace tests include 3 workload tests; existing `cargo run --locked --manifest-path ... -- all` invocation emitted all 7 default fixtures at the documented IDs, counts, payload bytes and corpus hashes. |
| Generated client drift check | **PASS** — regenerated Go source and TypeScript types match checked-in outputs. |
| Baseline compatibility and incompatible fixture rejection | **PASS** — current contract reports no breaking changes; the negative fixture returns 2 `response-property-became-optional` errors and the checker rejects it as expected. The harness also confirms malformed specs are reported with oasdiff's distinct load-error exit code 102 rather than accepted as breaking changes (exit code 1). |
| Real loopback generated Go and TypeScript client requests | **PASS** — both clients called `/health/live` and received `alive`, then checked the generated typed `operation_not_found` 404 response (`status`, `code`, `retryable`); smoke script cleans up the bounded test server. |
| Hosted GitHub Actions workflow | **NOT RUN** — local commands passed; no hosted workflow run was started for this task. |
| Linux/macOS clean-machine matrix | **NOT RUN** — only this development host is available in the task. |
| DEL-26/A04 SQL transaction tests, DEL-27/A05 simulator and DEL-28/A06 portability tests | **NOT RUN** — separate roadmap tasks and out of scope. |

Passing this skeleton gate does not establish F1 as a whole, product readiness, migration safety, performance, authentication or durability.
