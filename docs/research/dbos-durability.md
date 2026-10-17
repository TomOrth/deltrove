# DBOS Durable Execution for the Rust server

**Research date:** 16 October 2026. **Status:** planning only; no SDK integration or benchmark was performed.

## Recommendation

Keep Deltrove's PostgreSQL outbox/queue with lease and fencing checks as the authority for VCS mutations and short background jobs. The official Rust DBOS SDK exists, but is still an early, pre-release port and does not currently provide transactional steps. Its own README says Rust database writes in the workflow demos are at-least-once. That gap overlaps the hardest Deltrove invariants: a commit, approval/check record, or landing/ref update must not be duplicated or become durable without the matching durable job state.

Revisit DBOS as a workflow layer for long waits—CI orchestration, human approval waits, or resumable import coordination—after the Rust SDK's transaction support and multi-replica recovery behavior are demonstrated. It should not replace the transactional landing path. Do not add another-language service or shim to obtain DBOS features: the plan selected Rust throughout the server, and DBOS's Rust interface may close the gap.

## Rust SDK status and maturity

The official project is [`dbos-inc/dbos-transact-rust`](https://github.com/dbos-inc/dbos-transact-rust), published as the `dbos` crate, with MIT licensing. The repository has a real implementation, examples, and a published `0.5.0` GitHub release labeled **Pre-release** (9 September 2026); DBOS showed Rust repo activity through 9 October. This demonstrates an active port, not a GA/support commitment. Main DBOS quickstarts still emphasize Python, TypeScript, Go and Java; no Rust support policy was found. GA status, support SLA, and production readiness remain unknown. [Rust repo](https://github.com/dbos-inc/dbos-transact-rust), [Rust releases](https://github.com/dbos-inc/dbos-transact-rust/releases), [DBOS organization repositories](https://github.com/dbos-inc).

The Rust repo README describes durable workflows and steps, crash recovery, a Postgres system database, and demo apps. It also explicitly states that the Rust widget-store database writes are **at-least-once because Rust has no transactional step yet**. The open design issue confirms the gap: Rust has no datasource abstraction, so application writes and the DBOS checkpoint commit separately; a crash after the write but before the checkpoint causes the write to repeat. That is directly inconsistent with treating DBOS steps as exactly-once SQL operations. [Rust README](https://github.com/dbos-inc/dbos-transact-rust#readme), [issue #48](https://github.com/dbos-inc/dbos-transact-rust/issues/48).

The important distinction is:

- A normal durable step checkpoints its result after it runs. If the step performs an external side effect (HTTP/CI launch/email) and the process dies after that effect but before checkpoint, recovery can repeat the effect. Use an idempotency key or downstream deduplication; “durable” does not make arbitrary side effects exactly once.
- A database transaction step can atomically commit application writes and its DBOS completion record in one database transaction, avoiding that gap. DBOS documents this for other SDKs and their datasources. Rust does not currently expose this feature. [DBOS transaction guide (Go)](https://docs.dbos.dev/golang/tutorials/transaction-tutorial), [Rust issue #48](https://github.com/dbos-inc/dbos-transact-rust/issues/48).

There is a possible manual SQLx pattern in the issue discussion: persist a stable `(workflow_id, step_key)` result row in the same app transaction as the write, then read/adopt it on retry. That can provide application-level idempotency, but depends on stable replay keys and careful race handling; it is additional project code, not a built-in Rust SDK guarantee. The issue also notes that Rust does not expose the active step ID. [Issue #48 discussion](https://github.com/dbos-inc/dbos-transact-rust/issues/48).

## Fit with Deltrove's PostgreSQL design

Deltrove's plan uses SQLx/Postgres for canonical metadata, ref CAS, approvals/checks and landing; it publishes immutable object data separately, with outbox jobs, fenced claims and idempotent publication. This split correctly recognizes that a SQL transaction cannot atomically include an object-store write. DBOS would add a separate workflow/checkpoint state machine, not erase those existing consistency boundaries. [Rust backend plan](../rust-backend-scaling.md#5-postgresql-capacity-and-job-fencing), [technical plan](../technical-plan.md#12-security-retention-and-recovery).

DBOS can use Postgres as its system database. Other SDKs can also wrap an application datasource and atomically write the checkpoint with app data; Go documents that sharing its system DB engine can put both in one transaction. The Rust SDK's missing datasource support means that capability cannot currently be assumed for SQLx or for Deltrove's `PgPool`. Pointing DBOS at the same Postgres server is not enough by itself: atomicity requires the app operation and checkpoint to share the same transaction/connection. Whether the Rust SDK can safely reuse Deltrove's existing pool, schema, connection budget, migrations and privilege model is unverified. [DBOS datasource semantics (Go)](https://docs.dbos.dev/golang/reference/datasources), [Rust issue #48](https://github.com/dbos-inc/dbos-transact-rust/issues/48).

For a prototype, use a separately named DBOS schema/database on the existing PostgreSQL service, with distinct pool and connection budgets; do not let SDK startup own Deltrove migrations or share a connection invisibly. Even that adds writes, indexes, retention, backup and restore coupling to the primary DB. DBOS's system state must be included in the same recovery plan as application rows if workflows are expected to resume consistently. Deltrove's database/object recovery watermark rules still govern object retention.

## Recovery, replicas, queues, upgrades, and operations

The generic DBOS recovery guide says one process without Conductor can recover its previous pending workflows on restart. In a distributed setup without Conductor, operators must assign executor IDs and arrange recovery for workflows owned by a failed executor. With Conductor connected, recovery is coordinated automatically. Thus Conductor is not required for single-process execution, but automated failure detection/reassignment in a multi-replica deployment is not an SDK-only assumption. The Rust repository's open issue #41 specifically records that the Rust Conductor client was missing; current official Rust docs do not establish that this has since shipped. Verify against the exact crate version before considering Conductor. [DBOS workflow recovery](https://docs.dbos.dev/production/workflow-recovery), [Rust issue #41](https://github.com/dbos-inc/dbos-transact-rust/issues/41).

DBOS queues offer durable enqueueing and queue/partition concurrency and rate limits in documented SDKs. That is useful for workload classes and tenant fairness, but it does not substitute for Deltrove's stale-worker fencing rule: an expired lease must not let an old worker publish a ref after a new worker takes over. Rust has active issue reports around dispatch ownership, recovery, checkpoint races and queue behavior; these are evidence to test, not proof every scenario fails. [DBOS queue guide (TypeScript)](https://docs.dbos.dev/typescript/tutorials/queue-tutorial), [Rust issue tracker](https://github.com/dbos-inc/dbos-transact-rust/issues).

Workflow code upgrades need explicit version strategy. DBOS documents application versioning/blue-green draining and patch-based compatibility for its established SDKs. The Rust code-version and replay compatibility story is not verified here. For any long-lived workflow, changing step order, inputs or serialization while old runs remain pending can alter replay behavior; test rolling deploy and rollback with persisted runs before relying on upgrade support. [DBOS workflow upgrades (Python)](https://docs.dbos.dev/python/tutorials/upgrading-workflows).

Retention is a product/data policy, not a housekeeping default: DBOS workflow history includes inputs, outputs, steps, messages/events/streams; documented retention deletes completed workflow history, while active workflows are retained. Those records may include sensitive metadata or references to VCS work and can grow with long-running imports. Select retention and audit needs explicitly, and test database growth/cleanup. [DBOS retention guide](https://docs.dbos.dev/production/dbos-cloud/retention/).

The SDK/library is MIT and the Rust repo says it needs only Postgres, with no separate orchestrator for basic use. Conductor is an optional control plane for workflow management and distributed recovery, but its self-hosted edition is proprietary, requires a license key for production, and operates its own Postgres database; it is separate from application system databases. This weakens the fit with the project's self-hosting goal if Conductor is required for safe replica failover. The open-source library can still run without it, with more manual operations responsibility. [Rust MIT license](https://github.com/dbos-inc/dbos-transact-rust/blob/main/LICENSE), [Conductor self-hosting/license](https://docs.dbos.dev/conductor/self-hosting/hosting-conductor).

## Use-case decision

| Deltrove work | Recommendation |
| --- | --- |
| Commit acceptance, changeset publication, approval/check writes, target ref landing/CAS | A DBOS workflow may coordinate landing, but its domain command must revalidate policy/candidate and perform the ref CAS transactionally; replay must resolve through an idempotent outcome. Keep that authority in SQLx/Postgres. |
| Manifest/tree derivation, indexes, Git import chunks | Existing fenced job queue remains the default. Evaluate DBOS only if resumable orchestration materially simplifies parent dependencies/checkpointing; bulk CPU/memory admission and storage publication still need Deltrove controls. |
| CI request/response and human approval waits spanning minutes/days | Good eventual workflow candidate, provided external calls are idempotent and workflow versioning/recovery are proven. The authoritative check/approval/ref decision still commits through the Deltrove database path. |
| Notifications/webhooks and other external effects | Candidate for durable retry, but endpoints must deduplicate by stable event/workflow ID; a crash can repeat an effect before its checkpoint. |

This is a judgment from the plan's invariants and documented SDK gaps, not a benchmark. Existing outbox jobs are at-least-once too, but already use idempotency, leases and fencing; DBOS should demonstrate a benefit beyond reimplementing that contract.

## Proof-of-concept gates

Before any production adoption, pin and test a specific `dbos` crate version with:

1. **Transaction gap:** kill the process after an SQLx app transaction commits but before DBOS records the step; prove no duplicate commit/ref/approval and recover the original result. Repeat for the manual step-key pattern if evaluated, including concurrent duplicate execution and lost DB response.
2. **Replica fencing:** pause worker A beyond lease/recovery timeout, let worker B resume the same workflow, then release A. Assert only the current fencing token can publish, complete, or advance a ref. Include DB failover and network partition.
3. **External CI/idempotency:** crash after CI accepts a request but before checkpoint. Confirm retry uses the same request key and produces one logical check run; handle late webhooks and duplicate callbacks.
4. **Queue pressure:** mix interactive derivations, bulk imports and delayed approval/CI workflows. Measure queue age, fairness, Postgres locks/connections/WAL, memory and cleanup under overload; prove work admission remains bounded.
5. **Deployment lifecycle:** deploy N+1 with N workflows pending; restart/rollback across versions; verify deterministic replay, status repair, cancellation and no workflow is stranded. Validate Rust Conductor support or exercise the manual executor recovery procedure.
6. **Retention/restore:** back up and restore application plus DBOS state to one consistent watermark; test expired workflow history against audit requirements, and confirm object pins outlive any workflow that still references unpublished content.

## Unknowns and contradictions to resolve

- Rust crate GA/support policy and stable public docs are not established by the pre-release tag or repository activity.
- Rust transactional-step/datasource support is missing in both the README and open issue evidence as of this review; recheck the release before a later decision.
- Current Rust multi-replica recovery and Conductor support are unclear: DBOS generic docs describe the model, while the Rust repo's open issue #41 says its client is absent.
- SDK internals, queue/fencing semantics, schema migration ownership, transaction pool sharing, workflow retention controls, and replay compatibility have not been validated against Deltrove's SQLx setup.
- DBOS marketing claims of “exactly once” apply to its atomic database transaction primitive; they must not be generalized to arbitrary external calls or to Rust's current ordinary steps.

Sources above are primary DBOS documentation, the official `dbos-inc` Rust repository/release/issues, and the current Deltrove plans. Generic Go/TypeScript/Python documentation was used only to explain DBOS concepts; those SDK features are not credited to Rust.
