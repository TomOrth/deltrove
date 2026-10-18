# Distributed metadata research

**Date:** 17 October 2026\
**Status:** source-backed feasibility framing; no Deltrove implementation, benchmark, or migration has been run.\
**Decision:** PostgreSQL remains the MVP metadata authority. CockroachDB and TiDB are conditional distributed-SQL candidates for a later, workload-triggered comparison. Neither is a transparent database URL change. Shared caches can reduce repeated reads and object traffic, but they do not increase the capacity of authoritative metadata writes.

## Scope and current invariants

Deltrove's domain database owns repository membership, changesets and edges, refs and generations, reviews, approvals, checks, policy, idempotency outcomes, retention pins, job state, audit, and the transactional outbox. The first release uses Rust/Axum/Tokio, SQLx with PostgreSQL, critical mutations at `SERIALIZABLE`, bounded whole-transaction retry, and a durable outcome ledger for unknown commit results. Landing revalidates the exact candidate, live policy/check/approval state, target generation, attempt fence, and recovery epoch in its final short transaction. Immutable object bytes must be verified and durable before SQL makes them reachable.

These are constraints on every candidate. The governing details are in [durability and workflows](../durability-and-workflows.md), the [technical plan](../technical-plan.md), and the [implementation roadmap](../implementation-roadmap.md). In particular, an engine's transaction isolation name, PostgreSQL wire protocol, quorum durability, or retry API is not by itself evidence that these application invariants hold.

**SeaweedFS Filer metadata is a separate metadata plane.** Filer's path namespace and file entries describe storage files; Deltrove's PostgreSQL domain metadata decides repository authorization, object membership, ref state, reviews, and publication. SeaweedFS documents a selectable store for Filer metadata and separate blob/volume interfaces, but that does not make a Filer store replacement a Deltrove database replacement. Keep Filer storage metadata, Deltrove domain SQL, and immutable bytes as distinct authorities. See the [SeaweedFS architecture](https://github.com/seaweedfs/seaweedfs/blob/master/README.md) and [REST API](https://github.com/seaweedfs/seaweedfs/blob/master/REST_API.md).

## Workload distinction

Evaluate the actual F0 workload in two independent dimensions:

| Shape | Likely bottleneck | Candidate response |
| --- | --- | --- |
| Many repositories with moderate activity | Aggregate write capacity, fleet connections, noisy-neighbor isolation, backup/restore and placement operations | First measure tuned PostgreSQL HA. Repo-to-primary/shard placement can add aggregate capacity while each repository stays on one authority. Test placement, skew, migrations and fenced moves. Distributed SQL may be justified if cross-repo operations or operational requirements favor one distributed transactional system. |
| One hot repository/ref or very large monorepo | Per-ref serialization, hot rows/keys, graph/tree/index work, landing validation and jobs; a repo shard cannot split the ref invariant | Profile the engine and target-ref protocol separately. Distributed SQL can distribute unrelated keys, but a hot key still coordinates. A design that spans ranges/shards may add transaction coordination and retry cost. Test against same-repo contention and realistic graph sizes. |
| Read/checkout heavy, repeated objects | Object GETs, egress, cache misses, authorization checks, metadata query fanout | Immutable object and derived-data caching, request coalescing and batching. Keep current policy/ref/auth checks authoritative or use an explicit bounded-staleness contract for the particular response. Cache hits must not bypass authorization. |
| Geographically distributed writers | Network/quorum round trips, placement, conflict/retry rate and failover behavior | Compare same-region and cross-region write paths using the intended replica placement. A quorum spanning regions can add WAN round trips to commits; replica topology, locality and read consistency affect the result. Do not infer low latency from a distributed SQL label. |

PostgreSQL is not globally single-threaded: independent transactions execute concurrently. The proposed baseline is a single metadata write primary with HA, not a single worker. PostgreSQL's documented synchronous replication can wait until a configured synchronous standby has flushed commit WAL; `remote_apply` can wait until replay makes changes visible there. These settings alter latency and failure behavior and must be configured and measured, not assumed. [PostgreSQL synchronous replication](https://www.postgresql.org/docs/current/warm-standby.html).

Repository sharding and a distributed database solve different scaling problems. A repo-shard directory can distribute many independent repositories, but operations crossing repositories need explicit semantics and a single hot repository remains local to its shard. A distributed database can spread one repository's records across ranges, yet invariants involving multiple keys then depend on distributed transaction behavior. Keep the A/B test explicit about both shapes.

## Candidates

### Tuned PostgreSQL primary with HA

This is the control and remains the MVP default. Use a realistic schema/index/query plan, short serializable critical transactions, bounded pool sizes including rollout surge, idempotency outcomes, and the current outbox/job-claim model. Compare a single node only as an inner-loop diagnostic; the production comparison must include the selected HA acknowledgment profile, leader fencing, connection proxying if any, backup/WAL archive, and an operator-performed restore. PostgreSQL synchronous standby acknowledgments can add network/disk latency; asynchronous replication has a loss window and cannot satisfy an asserted zero-loss failover profile.

The primary hypothesis is that tuned PostgreSQL HA meets the F0 workload with a smaller compatibility and operating burden. This is not a measured conclusion. Report throughput and tails at the target hardware, pool budget, failure settings, and failover profile.

### PostgreSQL repo sharding

For a many-repository target, prototype a directory mapping `(organization, repository) -> shard authority`, while each repository's correctness-critical state is owned by one PostgreSQL primary. Include tenant/repo skew, one hot repository, cross-repo reads/admin operations, shard placement changes, backups, and a live move. A move needs an explicit ownership generation/fence, old-owner rejection, idempotent retry reconciliation, verification, and rollback. Do not treat schema-per-tenant or API replicas as a write-capacity substitute.

Repo sharding is not a candidate for hiding one monorepo's hot-ref serialization. Its result should state the aggregate gain and the maximum supported per-repo profile separately.

### CockroachDB

CockroachDB supports the PostgreSQL wire protocol and much PostgreSQL syntax/features, and defaults to `SERIALIZABLE`; transaction retry errors can still reach the client when the server cannot safely retry. Its own docs describe application retry handling and compatibility as “most,” not complete. SQLx `PgPool` is therefore a connection starting point, not a compatibility proof. Test exact SQL, SQLx transaction behavior, migrations/introspection, constraints, `RETURNING`, savepoints/retry handling, job claim and compare-and-swap queries, and operational tooling against the pinned release. [CockroachDB developer basics](https://www.cockroachlabs.com/docs/stable/developer-basics.html).

CockroachDB replicates ranges across nodes with consensus. Its documentation explains that the leaseholder serves strongly consistent reads and participates in the write quorum; a write's quorum placement and network round trip affect latency. This is especially material for a multi-region topology. Co-locate API traffic with the chosen leaseholder/quorum where possible, and measure realistic region placement. Do not assume a distributed transaction avoids hot-key contention or network latency. [CockroachDB global table/leaseholder explanation](https://www.cockroachlabs.com/blog/global-tables-in-cockroachdb/) and [distributed database tradeoffs](https://www.cockroachlabs.com/blog/fundamental-tradeoffs-distributed-databases/).

**Current licensing/source-availability gate:** on 15 September 2026 Cockroach Labs announced that new CockroachDB and Pebble source would be developed in private repositories; new releases remain under CockroachDB Software License. The current licensing FAQ says releases from 24.3 onward use that license and describes enterprise, free, trial, telemetry, renewal and throttling terms. Therefore do not describe current CockroachDB as an unqualified open-source default or assume public source for new releases. Before an adoption decision, capture the exact server version, license grant/eligibility, telemetry and renewal requirements, source/audit needs, redistribution/managed-service plans, support and upgrade path, and legal review. The upstream page may change after this research date; re-check during the decision. [15 September 2026 source announcement](https://www.cockroachlabs.com/blog/source-code-protection/), [current licensing FAQ](https://www.cockroachlabs.com/docs/stable/licensing-faqs), [license terms last updated 15 September 2026](https://www.cockroachlabs.com/cockroachdb-master-agreement/self-hosted/).

### TiDB

TiDB is MySQL protocol/dialect compatible, so it is a distinct SQL adapter/schema/migration arm. SQLx's MySQL driver would be a different driver and query contract from the MVP `PgPool`. Inventory placeholders, DDL, migration assumptions, data types, constraints, generated keys, transaction behavior, error codes, and job-claim/CAS SQL. Do not call TiDB a PostgreSQL-compatible replacement.

The semantic issue is decisive: TiDB documents `REPEATABLE-READ` as Snapshot Isolation, and explicitly states that this differs from ANSI/MySQL Repeatable Read and permits write skew. Its transaction docs show a write-skew example. Since Deltrove starts critical multi-row mutations at PostgreSQL `SERIALIZABLE`, simply setting TiDB's familiar isolation name cannot meet the contract. Either demonstrate a documented serializable mode with the required semantics on the pinned product/version, or design and prove a Deltrove-specific invariant-preserving protocol (for example, carefully scoped materialized guard rows/unique constraints/locking with a reviewed lock order) under TiDB's actual semantics. That is an architecture change and must pass the common adversarial oracle; absent this proof, disqualify the TiDB arm for authoritative domain mutations. It may still be independently considered for non-authoritative read/index workloads. [TiDB isolation levels](https://docs.pingcap.com/tidb/stable/transaction-isolation-levels/), [TiDB transaction overview](https://docs.pingcap.com/tidb/stable/transaction-overview/).

## Shared caching boundaries

“Shared cache” refers to cache tiers used by API replicas, workers, nodes or regions. It is a performance/read-path feature, not an authoritative mutation store. Start with immutable addressed object bytes and immutable/versioned derived manifests. Key every item with repository identity, object or manifest identity, format/version, and authorization scope where applicable. Verify digest/length on fills; use bounded weighted capacity, single-flight fills, concurrency limits, origin fallback, negative-cache TTLs only where safe, and explicit eviction behavior. A cache miss, corruption, partition, stale entry or full cache must fall back safely or fail clearly.

Mutable metadata requires a narrower policy. Refs, membership, permissions, current policy, checks/approvals, generation, and recovery epoch determine whether a mutation is permitted. Read them from the authoritative SQL path in the final critical transaction. A cache may serve explicitly stale-tolerant browse/search responses only with a documented staleness bound and observable version/watermark; it cannot authorize a write or turn a stale candidate into a successful landing. Every object/tree/history/diff/search/event path must enforce authorization even on cache hits. Revocation cannot recall bytes already downloaded by an authorized client; describe that limit. Shared-cache loss must not lose acknowledged bytes or domain outcomes.

SeaweedFS Cloud Drive/Filer caches and Deltrove's shared cache should not be conflated. A SeaweedFS Filer is a separately operated storage namespace with its own metadata backend. Deltrove's object cache may sit in front of immutable objects, while its authoritative domain rows remain in SQL. Qualification must test the actual chosen combination rather than assume one cache/provider's metadata or replication behavior satisfies the other system's durability contract.

## Evidence status

All statements above are source-backed product behaviors or proposed design constraints. No Deltrove throughput, latency, failover, licensing suitability, compatibility, or recovery result has been measured. The companion [evaluation plan](../metadata-evaluation-plan.md) preregisters the comparison process and acceptance gates; F0 must supply the workload and numerical thresholds before any arm is scored.
