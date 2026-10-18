# Deltrove metadata evaluation plan

**Date:** 17 October 2026\
**Status:** proposed experiment contract; no candidate has been implemented, benchmarked, or accepted.\
**Owner:** roadmap integrator. Work cards A14/A15/A17/A18 are [mapped to Linear](project-tracking.md); their experiment acceptance is still pending.

## 1. Decision and sequence

PostgreSQL remains the MVP default. Do a thin semantics/adapter feasibility spike during F3/F5, using the F1 domain fixture and oracle. Do not build a second production backend or run a broad capacity bakeoff until F0 supplies a measured or explicitly synthetic workload, numerical targets, hardware/network budget, and the adoption trigger. Run the full capacity comparison as S6 only if the accepted trigger is met or a product requirement requires it sooner. If a candidate wins S6, database migration and disaster-recovery qualification are separate acceptance packages; a performance win does not authorize migration.

The comparison arms are:

1. Tuned PostgreSQL primary with the selected HA and acknowledgment profile.
2. PostgreSQL repo-sharded placement for the many-repository workload.
3. CockroachDB on an exact pinned version and intended license/deployment profile.
4. TiDB only if A15 first proves either the required serializable behavior or a reviewed invariant-preserving protocol. Otherwise mark the authoritative-domain arm **disqualified by semantics**, not merely slower.
5. Shared cache enabled/disabled as a separate axis for read/object workloads. It is not a database arm and cannot mask origin correctness or cold-cache capacity.

Keep different workload shapes separate: many independent repositories and one hot repository/ref. A result for one shape cannot support a capacity claim for the other. Cross-region runs are a separately labeled topology, not pooled with same-region results.

## 2. Prerequisites from F0/F1

Before measuring, the integrator freezes the following in the checked-in workload manifest and experiment config:

- Corpus source (pilot or reproducible synthetic), generator/version, seed and hashes; repository count, files/bytes, tree width/depth, object size distribution, history/graph shape, mutable rows, refs, checks/check callbacks, and tenant skew.
- Offered and accepted reads/writes per second, concurrent clients/workers, retries, burst duration, CI wait/fan-out, landing contention, cache cold/warm state, region placement and expected failure model. Label synthetic assumptions separately from pilot observations.
- Host types/count, CPU/RAM/disk/network, DB node count, storage class, replication and quorum placement, SQL pool limits including rollout surge, client location, proxy/pooler, TLS, versions, kernel/runtime, and topology diagram.
- Numerical service targets, saturation/overload behavior, and improvement threshold for considering a more complex backend. Set these in F0 before candidate runs. Record explicit values; `TBD` blocks scoring. Do not invent figures in this plan or derive a threshold after seeing results.
- Acknowledgment profiles to test: the MVP single-node durable-storage profile; the intended production HA profile; and any explicitly claimed site-loss profile. For each, state which DB/object copies must survive and whether acknowledged writes may be lost.

Use identical application-domain code and behavior wherever the SQL dialect supports it. Dialect-specific adapters and required protocol changes are part of measured engineering cost and must be reported, not optimized away.

## 3. Common semantic oracle and negative controls (A14)

Build one independent, durable oracle outside the candidate DB and worker process. It records client request keys/payload hashes, observed responses and acknowledgment ordering. After faults, independently read back and reconcile the authoritative operation ledger, membership, ref generations, audit/outbox state and reachable object digests; distinguish accepted outcomes, committed-but-unacknowledged outcomes and unknown outcomes. SQL audit/test instrumentation is supplemental evidence, not an external atomic observation of commit. The oracle cannot infer commit from a timeout or an external log alone. Distinguish logical operations from transaction retries, worker attempts, CI executions and duplicate callbacks. Oracle writes are independently durable and their failure behavior is documented.

Exercise the same invariant cases against the PostgreSQL fixture first, then against each candidate adapter. Expected properties come from the accepted roadmap, not from the behavior of the system under test:

| Scenario | Required property |
| --- | --- |
| Concurrent same-ref landings from the same observed generation | At most one authorized transition wins; losing caller receives conflict/stale result; exactly one generation advance and one logical audit/outbox outcome. |
| Two concurrent operations that each observe a cross-row predicate (e.g. policy/check/approval quota or uniqueness guard) | No execution admits a state that violates the declared invariant. Include write-skew schedules that pass if only a snapshot is used. |
| Candidate/policy/check/approval/generation changes while a landing waits | The final mutation revalidates all current values and rejects stale candidate or stale authorization. No open transaction is held during CI/human wait. |
| Duplicate request, same key/same payload; same key/different payload | First returns its durable logical outcome on replay; second is rejected without a second mutation. |
| Lost connection during commit response | Treat as unknown outcome; reconcile from operation ledger. Retry cannot create a second publication/ref move. |
| Serialization/deadlock/transaction retry | Retry the complete transaction with stable inputs and bounded attempts; do not repeat external side effects in the transaction callback. Exhaustion returns a recoverable result and preserves reconciliation. |
| Expired job lease and paused former worker | Replacement may proceed; old worker cannot publish after a newer fence/attempt token. |
| Delayed/duplicate/forged check callback or callback from prior recovery epoch | No stale or unauthorized state transition; current candidate/generation/epoch must match. |
| Crash after durable object completion but before SQL publication | No partial logical visibility; safe orphan remains pinned or is conservatively collectible only under retention rules. |
| SQL commit after all required objects are durable, then process/host failure according to profile | Every acknowledged published object remains reachable and verifiable; no success acknowledgment precedes required bytes. |
| GC concurrent with upload pin/publication and backup/restore retention | No object reachable from any retained domain/backup root is deleted. Restore verifies retained digests. |
| Authorization revocation with hot cache entries | Every API read rechecks the current access contract; a cache hit does not disclose newly forbidden data or permit a mutation. Previously downloaded bytes are acknowledged as non-recallable. |

**Negative controls:** deliberately run a weakened isolation setting or omit the invariant guard in an isolated fixture. The oracle must detect a write-skew or duplicate-publication failure. Then disable current-generation/policy revalidation and verify the stale-landing case fails. Remove fencing from one worker completion and verify stale publication is detected. Mark a SQL transaction successful before object durability and verify the publication fault is detected. A passing harness without these known-bad controls does not establish oracle sensitivity.

The Oracle test results are correctness gates, not throughput samples. Any invariant violation is a hard fail regardless of performance. Save the interleaving schedule/seed, logs, SQL state, oracle journal and minimized reproducer.

## 4. A14 — common baseline and contract

**Deliverable:** accepted F0/F1 manifest plus implementation-neutral transaction, durability, cache and result contracts. Define logical requests and outcome comparison so each arm can be driven identically. Add instrumentation for retries, commit ambiguity, pool wait, lock/contention, job claim/fence, object acknowledgments, cache state and queue age.

**Acceptance:** manifest is reproducible; numeric gates and production topology are frozen; the PostgreSQL implementation passes the oracle; each negative control is detected; raw output is machine-readable and versioned. A14 creates the prerequisite to A15/A17, not a backend recommendation.

## 5. A15 — SQL semantics and adapter feasibility (F3/F5)

This is a deliberately small spike. Use the same minimal schema and operation protocol for: domain insert/membership/idempotency/outbox; protected-ref generation CAS/landing; policy/check/approval revalidation; job claim and fencing; unknown commit outcome read-back; and required constraints/uniqueness. Include DDL migration up/down or forward-only policy, online schema-change assumptions, index/introspection checks, and restore/snapshot tooling inspection. Pin SQLx, drivers, candidate versions, migration tool, server settings, and all patches.

For CockroachDB, test SQLx PostgreSQL protocol/driver behavior but separately record gaps in syntax, types, constraints, errors, migration tooling, transaction retries, and operational integrations. A successful connection or basic CRUD is not a pass. Demonstrate whole-transaction retry on server retry errors and preserve unknown-commit idempotency.

For TiDB, use its MySQL protocol/SQLx MySQL adapter as a separate implementation. First establish whether the exact product/version has a supported isolation mode with semantics sufficient for the critical invariant. Its documented default mode is pessimistic transaction handling, but its `REPEATABLE-READ` behavior is Snapshot Isolation and allows write skew; automatic transaction retry is not a safe substitute. If there is no qualifying serializable mode, prototype a Deltrove guard/locking protocol and run every adversarial oracle schedule, including guard rows, absent-row predicates, policy updates, ref moves, retry/reconnect and deadlock ordering. If the protocol cannot be completely specified, independently reviewed, and shown correct against all modeled invariants, record TiDB as **not suitable as authoritative domain metadata**. It can only remain a candidate for a non-authoritative index/read workload with explicitly weaker requirements.

**Acceptance:** an evidence matrix says pass/fail/unsupported/not run per required statement, transaction behavior, migration feature, job primitive, client behavior, restore capability and license/deployment requirement. Required SQL cases pass the common oracle and negative controls. Every unsupported schema/driver/API feature has a bounded workaround and owner or is a disqualifier. This gate authorizes a capacity candidate, not production use.

## 6. A17 — conditional capacity and shared-cache comparison (S6)

Start only when the accepted F0 trigger is met (for example, measured primary saturation or a stated large-repository requirement). The trigger must point to captured evidence and the target workload. Profile application CPU/graph work, object store, connection cap, ref hotspots, query/lock waits, job queues and API admission first; a non-database bottleneck is not a reason to change SQL storage.

Run a fixed workload matrix against each qualified SQL arm:

| Dimension | Required cases |
| --- | --- |
| Repository shape | Many independent repos with uniform and skewed tenants; one large repo; one hot protected ref; parallel independent refs; cross-repo administration. |
| Transaction mix | Reads, object membership/publication, idempotent writes, ref CAS/landing, policy/check/approval update, job claims and callbacks. Include realistic read/write mix and burst. |
| Load curve | Offered load from below target through saturation and overload, sustained soak, burst/recovery/drain. Record accepted/rejected work and retries; do not report accepted throughput alone. |
| Failure and contention | Node loss/leader transition, replica/network loss, unavailable quorum, slow disk, connection reset before/after commit, hot-key contention, retry storm and worker failover. Include only faults supported by topology; label unsupported. |
| Geography | Client/API and replicas local to a region; intended multi-region placement; WAN delay/fault injection. Compare strong reads/writes separately from stale-tolerant reads. |
| Cache | Cold, warm, shared-cache cold-start, cache-node loss, cache partition, origin slowdown/outage, concurrent same-key miss, eviction under pressure, corruption/digest mismatch and authorization revocation. Compare with cache off at same offered load. |

Compare tuned PostgreSQL HA to repo-sharded PostgreSQL on many-repository workloads. Compare CockroachDB/TiDB only after A15 passes. Keep each arm's equivalent durability/availability profile; if equivalent profiles cannot be configured, mark the comparison non-equivalent and do not claim a winner. For shared cache, report origin reduction and client latency separately, including extra cache storage, egress, CPU, DB lookups, invalidation traffic and warmup time. Warm-cache improvement cannot conceal a cold-path regression or correctness failure.

**Metrics:** p50/p95/p99 and max for API operations and complete logical mutation latency; logical accepted ops/s and rejected/failed ops/s; queue age/lag; serialization/deadlock/retry rates and attempts per logical operation; ambiguous commits; transaction/range/lock waits; connection acquisition/usage; CPU/RSS/disk/WAL/network; cache hit/miss/coalescing/fill/eviction/corruption rates; object QPS/bytes/egress; recovery/failover time; and workload drain/reconciliation time. Capture samples/raw time series, not only dashboard screenshots.

Use repeated runs with fixed seed/order, report run-to-run distribution and confidence method, and retain outliers. Warm each candidate only through a declared same procedure; randomize candidate order where possible. No “best run” comparisons. Record startup/prewarm separately from steady state. Validate correctness oracle at low load, target load, and after faults. Benchmark runs that fail a semantic case are invalid for adoption even when fast.

**Capacity acceptance:** all F0 numerical requirements are met with the target hardware/topology and target acknowledged-durability profile; no semantic or integrity failures occur; overload remains bounded; documented operating overhead and license/deployment fit are acceptable. To prefer a more complex SQL backend, it must exceed the preregistered improvement threshold on the workload that triggered S6, or enable a required topology/capability that the baseline cannot meet. Record unsupported/failing results and revisit triggers. Do not set the threshold after looking at data.

## 7. A18 — conditional migration, upgrade, failover and restore acceptance

This package starts only after an accepted architecture decision selects an alternate SQL engine for experimental implementation and eventual migration qualification. PostgreSQL repository-shard movement remains independently qualified under S5 and does not require selecting an alternate engine or running A18. A18 is not part of the capacity benchmark and cannot be marked complete from an online migration demonstration alone. If no alternate engine is selected, record a linked skip decision; this is not evidence of a passed migration.

Write a versioned schema/data mapping for all domain tables, indexes, constraints, sequences/IDs, idempotency ledger, outbox/job state, recovery epochs, audit, migration watermarks and retention roots. Define a one-writer cutover: source stays authoritative during copy/verification; establish a write fence, drain or reconcile in-flight mutations, capture a final consistent watermark, verify rows/references/counts/checksums and invariants, switch the authority epoch, then enable destination writes. Any dual-write/shadow compare path must never publish two authorities.

Test full snapshot and incremental catch-up, interrupted/restarted copy, source and destination schema version skew, lost client response around cutover, duplicate retry, stale worker/callback from old authority, key/constraint conversion, large tables/indexes, online schema changes, and rollback before/after the destination accepts writes. After destination writes begin, rollback requires reverse event/data reconciliation and explicit compatibility proof; a DNS or connection-string flip is not rollback.

For each advertised failure profile, rehearse independent DB plus object restore into an empty environment from deliberately mismatched recovery points. Verify every acknowledged publication, retained backup root, object digest and operation outcome. Fence old DB primaries, API replicas, workers and credentials; allocate a fresh non-reused recovery/authority epoch; reject stale callbacks; pause GC through reconciliation; reconcile workflow/CI state against the operation ledger; then release writes. Record RPO as the exact acknowledged operations possibly lost and RTO from incident start to qualified writes, plus operator actions and restore bytes/time. Never claim RPO 0 from asynchronous replication or a successful process restart.

**Acceptance:** independent operator completes the runbook; no acknowledged state is silently lost within the promised failure profile; there is no split-brain writer; stale authority attempts cannot mutate; all retained reachable objects verify; durable outcomes reconcile; rollback boundary and data loss, if any, are explicit. Missing or unsupported recovery APIs block claiming the topology as supported.

## 8. Caching-specific qualification

Cache only immutable addressed objects or immutable/versioned derivations by default. A mutable browse/search cache must carry a version/watermark and stated maximum staleness; final landing and authorization decisions remain authoritative. For every route, exercise warm hit following membership revocation, policy/check/ref change, repository deletion/transfer, cache key collision across repos, object corruption, cache loss, origin failure, and a read stream in flight during revocation. Assert that no new request gets unauthorized bytes and no stale cache result authorizes a mutation. Document that content already fetched by a formerly authorized client cannot be recalled.

Shared cache must have bounded bytes, in-flight fills, queue length, admission, TTL, and per-tenant fairness; apply request coalescing without allowing one tenant to pin unbounded memory or origin work. Cache loss cannot lose the only copy of an acknowledged object. Filer's directory/file metadata backend remains SeaweedFS-owned storage namespace state; it is not the Deltrove domain DB or this read cache.

## 9. Evidence bundle and reporting

Every run stores: source revision; exact dependency/server/image digest; schema and migration revision; license/terms snapshot and deployment mode; workload manifest and seed; host/topology/network/storage configuration; parameter and pool files; start/stop/fault schedule; warm/cold procedure; raw metrics and logs; oracle journal; failure reproducer; commands to regenerate; analysis script revision; and a short result with pass/fail/unsupported/not-run per requirement. Redact credentials before archiving. Retain raw files needed to independently recompute summary values.

The final decision memo separates (1) semantic qualification, (2) capacity and latency evidence, (3) durability/restore evidence, (4) compatibility and engineering cost, (5) operational burden, (6) licensing/source availability, and (7) residual risks and revisit triggers. List each result as a measurement with configuration, an upstream documented behavior, or an inference. No result is transferable to a different topology, version, dataset, cache state or failure profile without a new qualification.

## 10. Research sources

- PostgreSQL [transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html) and [synchronous replication](https://www.postgresql.org/docs/current/warm-standby.html).
- CockroachDB [developer basics and retries](https://www.cockroachlabs.com/docs/stable/developer-basics.html), [September 2026 source-availability notice](https://www.cockroachlabs.com/blog/source-code-protection/), and [licensing FAQ](https://www.cockroachlabs.com/docs/stable/licensing-faqs).
- TiDB [isolation levels and write-skew behavior](https://docs.pingcap.com/tidb/stable/transaction-isolation-levels/) and [transaction overview](https://docs.pingcap.com/tidb/stable/transaction-overview/).
- SeaweedFS [architecture](https://github.com/seaweedfs/seaweedfs/blob/master/README.md) and [REST API](https://github.com/seaweedfs/seaweedfs/blob/master/REST_API.md), used to distinguish Filer namespace metadata from Deltrove metadata.
