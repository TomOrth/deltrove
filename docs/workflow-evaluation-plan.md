# Temporal and Restate evaluation and test plan

**Date:** 16 October 2026. **Status:** planned experiments, not implemented or executed.\
**Required by:** [implementation roadmap](implementation-roadmap.md), packages F2, M4, M8, S1 and S6. PostgreSQL-backed workers are the baseline; **both Temporal and Restate must be evaluated** before the first workflow adoption decision.

## 1. Experiment boundaries

Implement the same small Rust lifecycle in three isolated experiment arms:

| Arm | Implementation | Durable state to inspect |
| --- | --- | --- |
| PG | Explicit PostgreSQL state machine, transactional outbox, leased/fenced Rust workers | Domain operation/outcome rows, queue attempts, event/outbox records. |
| Temporal | Pinned Temporal server/client/SDK and Rust workflow/Activity workers | Above domain records plus Temporal workflow/history/task-queue records. |
| Restate | Pinned Restate server/Rust SDK and registered Rust workflow handlers | Above domain records plus Restate invocation/journal/deployment records. |

The domain service implements `PrepareCandidate`, `RecordCheck`, `CancelOperation` and `AttemptLand` with stable operation identity, policy/candidate/generation validation and short SQL transactions. These operations are shared Rust code across the experiment arms; engine-specific orchestration is separate. A simplified fixture repository is sufficient for F2. No production object-format choice or full CLI/UI is required to start the experiment.

Run **prepare -> request fake CI -> wait for callback and approval -> attempt landing -> notify**. Store approval/check truth in the domain DB and deliver wakeups at least once. Candidate content is small, immutable fixture data. The fake CI service durably records request/run keys in its own store and can accept a request then drop the response, delay completion, duplicate callbacks or reject deduplication. Do not merely mock a successful HTTP return.

One engine owns an operation. Run arms against separate DB schemas/databases and isolated engine state, with the same seeded inputs and faults. Keep fixtures, runner behavior, hardware/quotas and domain code comparable. Do not deploy three controllers for the same live operation or build a generic engine-pluggability framework.

## 2. Restate facts and licensing gate

Restate has an official [Rust SDK](https://github.com/restatedev/sdk-rust) and [Rust API documentation](https://docs.rs/restate-sdk/latest/restate_sdk/). Its journaled actions record nondeterministic/external results; application-side idempotency is still needed for an effect that happens before its result is durably recorded. [Journaled action API](https://docs.rs/restate-sdk/latest/restate_sdk/context/trait.ContextSideEffects.html).

The current [Restate server license](https://github.com/restatedev/restate/blob/main/LICENSE) is **Business Source License 1.1**, with an Additional Use Grant and a release-specific change date/license. It explicitly identifies itself as not an open-source license. The [Rust SDK license](https://github.com/restatedev/sdk-rust/blob/main/LICENSE) is MIT. Record the exact server version, license text and intended distribution/hosting model in the evaluation. Do not label the current server MIT or assume that SDK licensing covers it.

The earlier research excluded Restate from the all-OSS shortlist. The user's new instruction explicitly includes it in testing. **Testing inclusion is now required; mandatory production adoption remains a separate licensing and architecture decision.** An optional integration can be evaluated while preserving an installation path without Restate. This plan neither rejects Restate's technical suitability on license alone nor treats its license as already accepted for every distribution model.

Temporal likewise needs a pinned server/SDK/license inventory and tested APIs; SDK availability is not proof of our application's recovery behavior. See the [Temporal research](research/temporal-durability.md). Neither candidate owns an ACID transaction spanning Deltrove's DB, object storage and CI.

## 3. Shared invariants and observable oracle

Assertions must read durable state and a client acknowledgment ledger after recovery, rather than infer success from engine status alone:

- Every acknowledged domain operation has one recoverable logical outcome under the declared durability profile.
- A ref changes only from the expected target/generation, against the exact ready candidate and current policy/checks/approval requirements.
- No stale attempt, cancelled operation or old recovery epoch can publish a transition.
- Repeating a domain command with identical identity/input returns its original result; different input under the same identity is rejected.
- Every published root has complete verified reachable objects; pending/failed snapshots are never reported as ready.
- External CI attempts may repeat. A provider supporting idempotency produces one logical run; a provider without it produces explicit uncertainty/reconciliation instead of a false exactly-once claim.
- Restarts do not require an operator to recreate in-memory timers or callback state. At the adoption gate, document the supported recovery/repair action for every stuck state.

Track separate counters for **attempts**, logical outcomes, physical CI executions, callbacks and ref transitions. A single final ref value does not prove there was no duplicate external effect or unauthorized intermediate movement.

## 4. Common fault matrix

| ID | Fault / stimulus | Required observation | First gate |
| --- | --- | --- | --- |
| W01 | Happy path with approval and CI callback arriving in both orders | Same authorized final outcome; callback order is not a correctness dependency. | F2 |
| W02 | Kill domain process after SQL commit, before HTTP response | Retry returns stored outcome and stable IDs; one logical mutation. | F2 |
| W03 | Crash after committing outbox, before dispatch | Restart eventually dispatches; no accepted operation disappears. | F2 |
| W04 | Engine accepts start but relay loses response; repeat while active and after completion | Stable operation/candidate/input identity reconciles; chosen ID reuse policy cannot create a second logical landing. | F2 |
| W05 | Worker executes a SQL effect, then dies before step/Activity completion is recorded | Retried attempt safely returns domain outcome; no repeated ref movement. | F2 |
| W06 | CI accepts request but response/checkpoint is lost | Stable request key deduplicates where supported; otherwise visible uncertain state plus lookup/manual reconciliation. | F2 |
| W07 | Restart during a durable wait; duplicate or lose a wakeup | DB truth is reconciled and workflow progresses; no reliance on one transient notification. | F2 |
| W08 | Cancel or supersede candidate before, during and after CI/landing | Late effects are rejected if no longer authorized. Cancellation after a completed landing does not silently reverse the ref. | F2 |
| W09 | Pause attempt A beyond takeover; B resumes; then A continues | Only current fenced authority can publish. Prove rejection at each domain publication endpoint. | F2 |
| W10 | Change target, permission, approval or required check while final transaction races | Transaction retries or returns stale/rejected; no stale authorization. Include write-skew fixtures. | M4 |
| W11 | Delayed callback after engine history/idempotency retention expires | Domain ledger and stable identity still prevent duplicate logical effects within the supported application retention contract. | M4 |
| W12 | Deploy N+1 while N workflows are waiting, then restart/rollback workers | Compatible histories resume or remain pinned to available N code; unsupported changes fail visibly without wrong domain effects. | M4 |
| W13 | Engine/database/storage outage with slow callers and retry storms | Durable accepted work remains recoverable; queues, connections, CPU and retained memory stay bounded. | M8 |
| W14 | SIGTERM/drain deadline versus hard kill during active work | Documented shutdown behavior, no false completion, safe replay and released resources after recovery. | M8 |
| W15 | Restore engine ahead of domain DB, then reverse; deliver old callbacks | New recovery epoch and fenced credentials block old publications; operator reconciliation preserves correct domain state. | M8 |
| W16 | Lose a node/partition in the selected HA topology; then restore quorum | Covered acknowledged state survives; unavailable quorum/durability blocks unsafe acknowledgments. No split-brain publishers. | S1 |
| W17 | Mixed long waits, bulk imports, small jobs and one hot repository | Per-class fairness, bounded queue age/resources and declared headroom; distinguish engine throughput from DB/ref limits. | S6 |
| W18 | Rebuild or migrate selected orchestrator ownership with pending operations | Exactly one current controller per operation; domain outcomes and cancellation remain authoritative. | S6 |

F2 runs W01–W09 on the thin fixture. M4 re-runs those tests on the real review/landing path and adds W10–W12; the fixture-only result is insufficient for integration. M8 covers all W01–W15. S1/S6 add the distributed, retention, upgrade and sustained-load coverage. If an engine is not selected, keep its F2 report and pinned rerun instructions, not its production dependency graph.

## 5. Engine-specific tests

### Temporal

Use deterministic workflow APIs and put SQL/HTTP work in Activities. Keep source bytes and credentials out of history. Verify start conflict/reuse semantics and reconcile stored operation/candidate/input digests; test closed and expired-history cases. Exercise callback messages, durable timers, cancellation and late Activity completion with the exact Rust release.

Replay a saved set of N histories against N+1; deliberately introduce an incompatible command-order change and verify the test detects it. Establish worker-version routing/patching or another supported compatibility strategy before runs span releases. Check history growth/continuation and payload limits using the [official limits reference](https://docs.temporal.io/workflow-execution/limits), with values captured for the pinned deployment.

Test the production persistence configuration separately from the convenient local dev server. When PostgreSQL backs both application and Temporal persistence, account for each schema/role/pool and shared disk/WAL capacity; there is no shared app/engine transaction.

### Restate

Use the SDK's journaled action/context APIs for nondeterministic work; do not infer that recording a result makes an arbitrary SQL/CI effect atomically committed with the Restate journal. Keep domain idempotency and final SQL CAS. Validate workflow/key/idempotency retention behavior against the exact SDK/server and confirm wakeup/cancellation APIs instead of assuming Temporal equivalents.

Register immutable N and N+1 worker deployments. Start an invocation on N, suspend it for a callback, register N+1, kill/restart the worker and verify the old invocation's actual routing/replay behavior while new invocations use the chosen version. Keep required old deployment code available. Test any cross-version migration as a separate explicit operation. [Restate deployment/versioning explanation](https://restate.dev/blog/dealing-with-versioning-in-long-running-agents).

For HA tests, qualify log and metadata replication, required quorum, snapshot repository and independent node storage. A default one-node configuration is not a fault-tolerant cluster. The official [multi-node transition guide](https://docs.restate.dev/guides/local-to-replicated) and [local cluster guide](https://docs.restate.dev/guides/cluster) are starting references; pin versions and give nodes distinct durable volumes. A three-container cluster on one machine tests process failures, not survival of that host's loss. Use a supported object store with separately tested snapshot retention rather than copying an old tutorial's dependencies unreviewed.

### PostgreSQL baseline

Implement only the bounded lifecycle state machine needed by the fixture. Test job claims, retry deadlines, durable wakeups, monotonic fencing tokens, fixed lock order and recovery scans. Do not recreate a general workflow engine merely to make the baseline match every vendor feature. Record unsupported capabilities honestly; a much larger bespoke framework is itself evidence in the comparison.

## 6. Benchmark and decision record

Run correctness before comparative throughput. Separate ordinary completion, waits, callbacks, step retries, heavy CPU jobs and final ref contention. Use the same domain transaction isolation and durability profile for every arm; never make a candidate look fast by disabling fsync, using memory-only state or dropping history/acks.

Capture p50/p95/p99 end-to-end and per-stage latency; successful logical operations/s; retries/duplicates; resume time; ready queue age; CPU/RSS/disk/network; PostgreSQL connections/WAL/lock time; engine storage/history growth; and a human-readable operator repair transcript. Record cold/warm caches and accepted versus rejected load. Include a drain period: surviving admission is insufficient if the backlog never clears.

Required report fields:

```text
Arm and exact server/SDK/image/license versions:
Commit, host/container quotas, topology, DB/object settings, dataset and seed:
Passed / failed / unsupported / not run, for every W-test:
Domain oracle results and acknowledgment ledger:
Performance distributions and raw samples:
Restart, upgrade and restore procedure actually exercised:
Operational complexity and unresolved repair cases:
Licensing/distribution fit:
Decision: PG only / Temporal / Restate / investigate further:
Reason and explicit revisit trigger:
```

All invariant tests applicable to the target release are hard gates. A fast arm that loses acknowledged state or lands a stale candidate is disqualified until fixed. Select based on supported semantics, operator recovery, code burden, versioning and deployment/license fit; performance is evaluated only among correct candidates. Temporal remains the prior leading candidate, but the comparison must allow Restate or the baseline to win on evidence. “Not run” is never a pass.

## 7. Experiment sequencing

1. **F1:** build common domain/CI/fault harness and outcome oracle.
2. **F2:** implement the three thin arms, run W01–W09 and write a preliminary decision. The earlier 2–3-day suggestion was for a single small spike; it is not an estimate for all arms or production qualification.
3. **M4:** port the selected approach to real candidate/review/landing operations, run W01–W12 and finalize its adoption ADR before expanding workflow features.
4. **M8:** qualify recovery/overload/restore for the small-team release with W01–W15; keep HA claims limited to tested topology.
5. **S1/S6:** test production HA and sustained enterprise workload; re-evaluate an alternative only when the selected engine fails a measured requirement or the hosting/license model changes.
