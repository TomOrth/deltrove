# DEL-26 / A04 — Domain transaction fixture

**Roadmap:** F1, after A03. **Status:** local acceptance evidence recorded 24 October 2026; independent review pending. This is a PostgreSQL transaction experiment, not the production repository schema, VCS format, HTTP mutation service, or workflow engine.

Implementation revision: `ac67d453d2ce8881f69b04413cd40e2376c8aa9c` on `feature/del-26-domain-fixture`.

## Contract and ownership

The root workspace now includes `crates/domain-fixture`. Owned paths are that crate and its fixture SQL migration, `tools/domain-fixture`, `.github/workflows/domain-fixture.yml`, the root Cargo member/lock update, and this handoff. DEL-27 owns an independent simulator store; DEL-28 owns the Go client spike. DEL-29 integrates their interfaces and independently reviews the combined failure oracle.

`DomainFixture::create` creates a NEW schema with a validated `del26_` prefix; an existing schema is rejected. `connect` opens an existing fixture without applying migrations. Each repository has one fixture-authorized actor, generation 0, recovery epoch 1, and worker fencing token 1. This models publication authorization without claiming real authentication, group policy, or tenant administration.

`PublishCandidate` contains `repo_id`, `actor_id`, `request_key`, `expected_generation`, `recovery_epoch`, `fence_token`, and at most 64 KiB of immutable placeholder bytes. IDs are limited to 128 bytes. The stable request key is scoped by repository and actor. All command fields participate in the stored input fingerprint; identical retries return the stored outcome, while changed input under the same key is rejected. Revoked actors cannot replay outcomes.

A successful short SERIALIZABLE transaction performs one generation CAS, retains placeholder candidate bytes, records the operation outcome, and inserts one outbox record. Any failure rolls the whole transaction back. Outbox dispatch is intentionally absent here. No CI, filesystem object write, or human wait occurs within the transaction. The SQLite simulator is not a substitute for this PostgreSQL gate.

Outcome fields are `operation_id`, `repo_id`, `candidate_id`, `generation`, `recovery_epoch`, and `fence_token`. The fixture operation ID hashes the scoped request key; the fixture candidate ID hashes the placeholder bytes. JSON/SHA-256 is used only for this experimental identity and replay fingerprint, and makes no decision about canonical production changeset encoding.

PostgreSQL SQLSTATE `40001` and `40P01` trigger a whole-transaction retry with bounded backoff, up to eight attempts under a ten-second command deadline. A commit transport error or deadline is treated as an unknown outcome: callers must reconcile using the SAME request key. Physical retry counts are instrumentation, not durable logical outcome counts. Connections are bounded to eight; acquire, statement, and lock waits are bounded. Each publication requests `synchronous_commit=on`.

The transaction semantics follow [PostgreSQL 15's isolation rules](https://www.postgresql.org/docs/15/transaction-iso.html) and [SQLx 0.8.6's transaction API](https://docs.rs/sqlx/0.8.6/sqlx/struct.Transaction.html). This fixture does not treat idempotency as a distributed exactly-once side-effect guarantee.

## Reproduce with a disposable PostgreSQL cluster

Recorded host: macOS arm64, Rust/Cargo 1.95.0, PostgreSQL 15.10 (Homebrew). SQLx is pinned to 0.8.6; Rust dependencies are pinned/resolved in the root `Cargo.lock`.

Install PostgreSQL binaries on `PATH` (`initdb`, `pg_ctl`, `psql`) and the root-pinned Rust toolchain. From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
tools/domain-fixture/test-postgres.sh
```

The script creates its own temporary cluster under `/tmp`, listens only on a private Unix-socket directory, keeps `fsync=on` and `synchronous_commit=on`, runs the explicit database gate, then stops its cluster and removes only its generated directory. It does not touch an installed PostgreSQL service or existing user data. Local trust authentication is limited to that private disposable cluster; it is not a deployment configuration.

For an already provisioned disposable test database:

```sh
DATABASE_URL='postgresql://fixture-user@localhost/fixture-db' \
  cargo test --locked -p domain-fixture --test postgres -- --ignored --test-threads=1
```

Real-PostgreSQL cases are explicitly ignored during the ordinary `cargo test` command. Running only the ordinary suite DOES NOT pass A04. The separate domain-fixture CI workflow provisions PostgreSQL 15.10 and invokes the ignored tests explicitly.

## Independent observations and fault boundaries

The oracle uses direct committed SQL queries rather than `publish` or its replay logic. It compares generation transitions, exact input bytes, actor/request identity, outcomes, and one matching outbox record per authorized mutation. A deliberate unrecorded generation change is a negative control: the oracle must reject it.

The concurrent-writer test holds a PostgreSQL row lock, starts two independent SERIALIZABLE writers, and waits until `pg_stat_activity` reports both blocked on that lock. Releasing the lock forces a real serialization failure and retry. Exactly one writer can install generation 1; the other must revalidate and reject the stale generation. Polling has a deadline and uses observed database state, not fixed sleeps as correctness synchronization.

The lost-response helper commits, writes a pipe marker to stderr, and withholds its result. The parent reads that marker, kills and reaps only its own child, verifies stdout contains no response, then retries the original command. A separately fsynced acknowledgment ledger and reopened connection are checked against durable SQL rows. This is process-death evidence with surviving storage, not host, power, storage-controller, PostgreSQL crash, or HA qualification.

An injected outbox-insert exception proves that a failure after the generation/candidate writes leaves no partial generation, candidate, outcome, or outbox state. Other cases cover concurrent duplicate delivery, same-key changed-input rejection, recovery-epoch/fencing rejection, and revocation before replay.

## Evidence and limitations

| Check | Result |
| --- | --- |
| Disposable PostgreSQL setup | PASS — PostgreSQL 15.10; `fsync=on`, `synchronous_commit=on`; private Unix socket |
| Explicit real-PostgreSQL integration suite | PASS — all seven cases passed against PostgreSQL 15.10, including the independent oracle corruption control |
| Rust formatting and Clippy | PASS — root formatting check and all-target/all-feature Clippy with warnings denied |
| Ordinary workspace unit/integration/doc tests | PASS — seven ordinary tests and one doctest passed; the seven PostgreSQL cases were separately executed by the explicit database gate |
| Hosted Linux/PostgreSQL workflow | NOT RUN — this batch has not been published |
| DEL-27/28 combined harness, W01–W09, engine comparison | NOT RUN — integration is A07; the workflow comparison is F2 |
| Production storage/object publication, real auth, HA/host loss | NOT RUN — outside this fixture's scope |

Review risks are the intentionally narrow single-actor fixture policy, experimental IDs, schema lifecycle privileges, retry/unknown-outcome distinction, and independent oracle coverage. No production data is migrated. The SQL migration installs only a newly named fixture schema; failed tests may leave that schema in a caller-provided test database, while the disposable-cluster script always removes its entire own cluster. This handoff does not mark F1 accepted.
