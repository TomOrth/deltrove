# DEL-28 / A06 — Client portability spike

**Roadmap:** F1, dependent on A03. **Status:** implementation evidence recorded 24 October 2026; integrating-owner review pending.

Implementation revision: `6c7225117c727de6df30317d855baec7f1e923ec` on `feature/del-28-client-portability`.

## Handoff

- **Objective:** validate a small Go/Cobra CLI, reuse the checked-in generated Go API client, and qualify a pure-Go SQLite journal on Linux/macOS amd64 and arm64.
- **Owned paths:** `experiments/client-portability/`, `.github/workflows/client-portability.yml`, and this handoff.
- **Contract:** the CLI imports `github.com/TomOrth/deltrove/clients/go/generated`; the module uses a local replace to `../../clients/go`. It adds no handwritten HTTP request layer and does not edit generated files or the OpenAPI contract.
- **Interface:** `journal.Store` defines context-bounded `Write`, `Read`, and `Close`. The journal is a one-key SQLite toy; it does not represent a workspace engine.
- **Authentication:** unsupported. `deltrove auth` prints the label `session=mock-local`; Cobra rejects positional arguments, so the command accepts no credentials. It stores no token and performs no identity exchange.
- **Out of scope:** workspace checkout/sparse trees, multi-file transactions, real authentication, online commits, domain mutation APIs, power-loss or host-loss durability, Windows, and shared-cache daemons.
- **Driver choice/profile:** `modernc.org/sqlite` v1.61.0 is pure Go. The module builds with `CGO_ENABLED=0`; the application has no C/C++ source and requires no SQLite system library. The connection DSN applies `_pragma=journal_mode(DELETE)` and `_pragma=synchronous(FULL)` whenever the driver opens a connection, and `database/sql` is capped at one open connection per pool. Runtime, seed, helper, and reopen paths all use this profile. A test queries the active driver and verifies `journal_mode=delete` and `synchronous=2` (`FULL`); opening a database also checks that profile and fails if it differs. This profile uses SQLite's rollback journal (temporary `-journal` file) and does not configure WAL/SHM sidecars. The experiment makes no network-filesystem or cross-process concurrent-writer promise.
- **Reproducibility:** versions and checksums are pinned in `go.mod` and `go.sum`; builds use Go 1.26.3, `-trimpath`, `-buildvcs=false`, fixed spike version linker metadata, and no build timestamps. `make reproducibility` byte-compares two builds per output target in the same toolchain/cache environment.

## Recovery oracle

The test launches its own test-binary helper process. The parent has an eight-second process deadline; the helper transaction uses a one-minute context, so the helper deadline cannot roll back the transaction before the parent timeout. The helper changes an existing value, inserts another row, and writes a newline marker to the parent over the subprocess stdout pipe before waiting on parent-owned stdin. The parent reads that causal marker and sends SIGKILL to that owned process. It then opens a new database connection and verifies SQLite recovery rolled back both changes and preserved the old value. The commit-boundary case commits both changes, emits its marker only after `Commit` succeeds, is killed by the same parent, and verifies the new values after reopen. The test does not use sleeps for synchronization. This demonstrates process-death rollback/reopen behavior on the tested filesystem; it does not establish machine-crash, power-loss, storage-controller, or remote-filesystem durability.

## Commands and evidence

Run from `experiments/client-portability/` with Go 1.26.3:

```sh
go mod tidy
gofmt -w cmd/deltrove/*.go internal/journal/*.go
go vet ./...
go test -race ./...
make build-matrix
make reproducibility
```

The workflow in `.github/workflows/client-portability.yml` checks `go env GOHOSTOS`/`GOHOSTARCH` against each declared runner before running vet, race tests, and a native build on Linux amd64 and macOS arm64. It cross-compiles Linux arm64 and macOS amd64; those two builds are compile evidence only.

| Check | Result on 24 October 2026 |
| --- | --- |
| Toolchain | Go 1.26.3, `darwin/arm64` |
| `go mod tidy` | **PASS** — resolved and pinned Cobra 1.10.2, modernc SQLite 1.61.0 and transitive checksums; module registry access was required. |
| `gofmt` | **PASS** — all implementation/test Go files formatted. |
| `go vet ./...` | **PASS** — no diagnostics. |
| `golangci-lint run` with golangci-lint v2.15.0 | **PASS** — 0 issues. |
| `go test -race ./...` | **PASS** — CLI command/HTTP tests and journal recovery tests passed on macOS arm64. |
| Queried SQLite runtime profile | **PASS** — driver reports `journal_mode=delete`, `synchronous=2` (`FULL`); runtime, crash helper, and reopen use the same explicit settings. |
| Native macOS arm64 build/runtime, SQLite close/reopen and process-kill recovery | **PASS** — current host `darwin/arm64`; child process kill tests proved rollback/preimage preservation before commit and committed-state survival after commit acknowledgment. Generated client query reached a local `httptest` HTTP server and decoded `alive`. |
| Linux amd64 native runtime | **NOT RUN** — no Linux host available locally; workflow provides a native runner. |
| Linux arm64 and macOS amd64 cross-build | **PASS** — `make build-matrix` completed with `CGO_ENABLED=0`; cross-builds do not prove runtime or filesystem behavior. |
| macOS arm64 native build | **PASS** — `make build-matrix` output inspected as Mach-O arm64. |
| Linux amd64/arm64 output format | **PASS** — `make build-matrix` outputs inspected as statically linked ELF binaries. |
| Repeated same-environment artifact comparison | **PASS** — `make reproducibility` byte-compared repeated builds for all four targets. |
| Hosted GitHub Actions | **NOT RUN** — no hosted workflow run started. |

No release artifacts are promised until the native Linux/macOS CI matrix is green and the integrating owner reviews this handoff. Linux native runtime, other-architecture runtime, minimum OS versions, and filesystem support remain unqualified.
