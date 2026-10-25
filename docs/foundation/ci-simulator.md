# DEL-27 — Independent CI simulator and fault runner

**Roadmap:** F1 / A05. **Status:** implementation evidence recorded 24 October 2026; integrating-owner review pending.

Implementation revision: `c7d5022cffcae19a3fb101e8766e6dd867432998` on `feature/del-27-ci-simulator`.

## Boundary and public contract

The standalone package at [`experiments/ci-simulator`](../../experiments/ci-simulator) models a durable external CI provider. It owns a SQLite database independent of the domain fixture database. It does not implement domain transitions, authorization, orchestration, VCS behavior, a production CI adapter, or host-loss durability.

The stable request key is the JSON object `{ "repo_id", "operation_id", "check_name", "candidate_id" }`. The immutable payload contains `candidate_digest` and `command`. A physical execution has its own `run_id`; each client submission is recorded with a distinct internal attempt ID. Callback delivery is separately identified by `callback_id` and a monotonically increasing `attempt`.

Example request:

```json
{
  "key": { "repo_id": "repo-fixture-1", "operation_id": "op-7", "check_name": "unit", "candidate_id": "cand-3" },
  "payload": { "candidate_digest": "sha256:abc", "command": "cargo test" }
}
```

`POST /v1/requests` accepts requests. `?drop_response=true` commits the submission and execution first, then resets the streamed HTTP response; a caller observes a real transport error either while receiving the response or while reading its body. Reconcile with `GET /v1/reconcile` and all four key query fields. `found` returns physical run IDs and status. `unknown` means no record is visible; following an ambiguous submission it is not proof that acceptance did not happen.

With `--dedup true`, the same key and same payload returns the existing run ID and adds a submission attempt without creating a physical execution. A different payload for that key receives `409 Conflict`. With `--dedup false`, repeated identical submissions create separate physical runs and are reported as such. Payload conflicts are rejected in both modes. The simulator makes no exactly-once physical-execution claim when deduplication is disabled.

Execution status remains `accepted` in this transport fixture, including the callback `result`. It does not run check commands or model terminal check results; A07 must define any completion-state contract needed by the integrated harness.

Callbacks are explicitly scheduled at `POST /v1/callbacks/schedule` with `callback_id`, `run_id`, and a receiver `target_url`. Scheduling requires an existing physical run and binds that callback ID permanently to its run, stable key, and receiver. `POST /v1/callbacks/deliver` takes only the callback ID; the simulator loads that binding, writes a durable pending attempt before sending, and posts `{ "callback_id", "attempt", "run_id", "key", "result" }` to the receiver. It marks an attempt delivered only when the receiver returns 2xx. Failed and pending attempts remain visible from `GET /v1/callbacks/attempts?callback_id=...`; retry keeps the callback ID and increments the attempt. This lets a fixture hold a durable callback over a simulator restart and request duplicate deliveries to an independent receiver. `POST /v1/outcomes/ack` is a separate fixture-client acknowledgment ledger keyed by `outcome_id`; it is observability for the harness, not the authoritative DEL-26 domain outcome. Reusing an outcome ID with different key or result is rejected. `GET /v1/counts` reports values reconstructed from durable submission, execution, callback-delivery, and client-acknowledgment rows, not memory counters.

## Fault runner

`fault-runner demo --target <path-to-fixture-target>` spawns only the supplied target as its direct child. It proves readiness and command completion through pipe handshakes with deadlines, pauses the child and confirms the stopped state with `waitpid`, resumes it on Unix, kills and waits for it, then starts and handshakes with a new child. Drop cleanup kills and reaps an owned child. It never accepts an arbitrary PID and never signals unrelated processes. `fixture-target` is a tiny pipe-controlled test process, not domain code.

This validates process failure and SQLite recovery on the same host. It does not prove survival of host loss, disk loss, power loss, or filesystem/hardware failures. SQLite WAL with `synchronous=FULL` is the declared local durability profile; it is independent of the DEL-26 authoritative database.

## Reproduction

Requires Rust/Cargo 1.95.0. Dependencies are exact-pinned and locked in this package; `[workspace]` isolates its lockfile from the root workspace.

```sh
cd experiments/ci-simulator
cargo build --locked
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo run --locked --bin fault-runner -- demo --target target/debug/fixture-target
```

The HTTP integration tests use real loopback requests and separate simulator processes. They verify genuine response-body transport loss after durable acceptance, reconciliation, restart persistence, same-key deduplication, conflicting-payload rejection, disabled-dedup physical duplicates, durable delayed callback scheduling and duplicate delivery attempt numbers.

## Evidence and limits

| Check | Result |
| --- | --- |
| `cargo build --locked` | **PASS** — completed on Rust 1.95.0 / Cargo 1.95.0, macOS arm64, 24 October 2026 |
| `cargo fmt --all -- --check` | **PASS** — no formatting differences |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | **PASS** — no warnings |
| `cargo test --locked` | **PASS** — 4 real HTTP integration tests passed, including independent callback receiver deliveries, restart-held callback, duplicate receipt, failed delivery/retry and invalid callback binding; unit and doc test targets compiled and ran |
| `cargo run --locked --bin fault-runner -- demo --target target/debug/fixture-target` | **PASS** — observed target ready, pipe ping, OS-confirmed pause/resume, work handshake, forced kill/reap, restarted target ready/ping and clean exit |
| Same-host process kill/restart and SQLite recovery | **PASS** — accepted key, one run, receiver URL and scheduled callback survived simulator process restart; two receiver deliveries shared a callback ID and carried attempts 1 and 2 |
| Linux/macOS hosted workflow | **NOT RUN** — hosted CI was not started |
| Host-loss durability, power-loss and disk-loss tests | **NOT RUN** — outside this process-level fixture |
| DEL-26 integration and domain outcome oracle | **NOT RUN** — DEL-26 is independently owned; A07 performs harness integration |
| W01–W09 and F2 engine comparison | **NOT RUN** — this package is the simulator component, not the lifecycle arms |

