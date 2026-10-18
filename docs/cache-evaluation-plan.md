# Deltrove: metadata and shared-cache evaluation plan

**Date:** 17 October 2026\
**Status:** experiment protocol; no performance results or cache adoption decision.\
**Purpose:** produce a reproducible, license-aware decision on whether a shared network KV cache adds enough value beyond PostgreSQL plus bounded process-local Moka, while separately evaluating a disk cache for large immutable objects.

See [shared-cache research](research/shared-cache.md) for candidate facts, licensing, and the data safety contract. The architectural baseline remains PostgreSQL authority with process-local Moka. Shared cache is optional and is considered for production only in S3 after S2. A throwaway prototype can begin earlier once the API/key contract is stable; it cannot be represented as a passed production gate.

## Decision question and hypotheses

The decision is whether sharing a bounded set of immutable, small values across API processes reduces origin work and improves an agreed end-to-end workload enough to justify another network hop, service, client, security boundary, and outage mode. Separately determine whether a bounded local NVMe cache improves large-object checkout/materialization without compromising streaming, integrity, dirty-file safety, or recovery.

Compare these distinct arms:

1. **Origin baseline:** PostgreSQL/object storage, no app cache.
2. **Moka baseline:** the current intended process-local weighted cache with same-process single-flight/coalescing; Moka capacity is not treated as a hard RSS limit.
3. **Moka plus Valkey shared KV:** retain the baseline Moka policy and add one exact pinned Valkey release/topology and Rust client/configuration, only for eligible small metadata/results. Measure the incremental benefit beyond Moka; an optional shared-only diagnostic is labeled separately.
4. **Moka plus Dragonfly shared KV:** retain the baseline Moka policy and add one exact pinned Dragonfly release/topology and Rust client/configuration, only if its exact license fits every intended deployment/distribution mode. A shared-only diagnostic cannot replace this comparison.
5. **Local NVMe object cache:** separately evaluated large immutable file/blob path. It does not stand in for shared KV and it does not cache mutable metadata authority.
6. **KV plus NVMe candidate:** only after the separate small-value and large-object paths are characterized, if layering gives a plausible workload benefit. Account for duplicate cache memory/disk and independent cold/failure behavior.

Freeze the aggregate resource envelope across scored arms: API/worker/client and cache-server CPU/RAM, cache disk, database/object service resources, network and node count all count. Retain Moka behavior while explicitly rebudgeting its capacity if required to fit that envelope; disclose the capacity change and include a fixed-Moka-budget sensitivity run. If an arm adds hardware or changes resources, publish it as a separate resource-adjusted/cost-normalized comparison, not a like-for-like speedup. Report same-resource and same-service-target comparisons separately.

The shared cache is not adopted solely because it improves synthetic cache hit rate or a microbenchmark. Adopt only if it meets the preregistered workload objectives, maintains all safety/fault controls, and has acceptable cost/operation evidence. `defer` is a valid result when the capacity bottleneck is not measured or the API/schema is unstable. `no shared cache` is a valid result when Moka/origin is simpler and sufficient.

## Prerequisites and stages

### F0: workload and acceptance contract

Before collecting any comparison result, F0 identifies representative workloads and numerical objectives for the alpha/MVP/large-organization scopes. The workload document must separate synthetic fixtures from observed pilot traces, include the same repository/data seed and request mix in every arm, and explicitly record expected object-size/hotness skew, concurrency, repo/tenant mix, regions, RTT, and cache warm-up. Choose supported capacity and overload policy from measured product needs and deployment constraints, not from example limits in the scaling plan.

F0 preregistration includes:

- a versioned workload manifest with fixture IDs, content hashes, generator versions, commands and any approved/normalized trace source;
- the latency/throughput/origin-reduction targets, resource ceilings, error/rejection budgets, run duration, warm-up period, repetitions, and analysis rule for each experiment;
- host/container sizes, CPU/memory reservations, database/object-store settings, network RTT/bandwidth/packet loss, client/request limits and consistent concurrency controls;
- exact definition of cold, process-warm, node-warm, region-warm, post-flush and post-restart states;
- the metric/export commands and a run ID layout so a reviewer can reproduce the report;
- allowed version/license modes and explicit unsupported scenarios;
- a frozen failure oracle that detects stale authority, unauthorized hits, poisoned bytes, unbounded origin retries, and a cache requirement for successful domain correctness.

The preregistration must be committed or otherwise timestamped before the first scored run. It may be revised after a pilot only by documenting the old rule, reason, and new version; do not retroactively change the winning threshold. Numerical targets are not supplied here because no F0 contract or measurements exist yet.

### F3: common metadata/cache evaluation contract (A14)

Extend the F0 workload contract to define reusable query and object fixtures, metrics, result checksums, and negative controls for A15–A17. A14 does not select a cache product or claim a speedup. It should establish the measurement commands and report schema before S2/S3 so no candidate gets a custom easier workload.

Required A14 output: `docs/evaluations/scaling-evaluation-contract.md` (future artifact), containing manifest schema, exact fixture references, workload command, environment capture, metric names/units, result-validation oracle, negative-control results, and explicitly excluded workloads. Reuse F0 inputs where suitable and record gaps instead of silently replacing pilot evidence with synthetic data.

### S3: shared cache qualification (A16)

After S2 has bounded API/worker classes, tenant quotas and fleet connection budgets, run A16 under that shared contract. A16 compares the no-shared-cache/process-local-Moka baseline with Valkey and Dragonfly, plus separate disk cache runs for larger objects. The report is an adopt/defer/no-shared-cache decision, not an implementation request. Any adopted service requires a separate implementation/review decision and S8 qualification.

## Candidate protocol and version matrix

Before a scored run, record exact image digest/version/build, source/license URL and hash, server settings, topology, Rust compiler/toolchain, dependency lockfile, client crate/version/features/TLS/reconnect behavior, protocol mode, and deployment mode. Run compatibility tests against every exact server release; Redis-protocol naming or an upstream compatibility statement is not sufficient.

Validate only required operations for the narrow cache contract: connect/auth/TLS; ping/readiness; GET; immutable fill (`SET NX` or selected alternative); TTL/expiration if used; delete/namespace-epoch advancement; bounded multi-get or pipelining if used; byte/string encoding; error classification; timeout/cancellation; reconnect; server restart/flush; and cluster redirection/topology discovery only if a clustered deployment is actually proposed. Avoid commands/functions/scripts the product does not need. Check whether the selected Rust client has explicit compatibility/version support and supported cluster/TLS/ACL options. Valkey GLIDE's Rust core does not establish a supported direct Rust client API; client compatibility is an experiment item.

For each candidate run license review against the **exact pinned artifact** and intended modes: hosted Deltrove operation, customer-run self-hosted install, image/package redistribution, and internal development. Store the actual license text or stable release link/hash in the evidence bundle. For Dragonfly, specifically recheck the BSL version, Additional Use Grant, per-release change date and intended hosted-service boundary. If a deployment mode is ambiguous or disallowed, mark that mode unsupported or defer that candidate; a different license for future releases does not change the pinned artifact's terms.

## Workload matrix

The F0/A14 manifest supplies actual sizes and rates. The matrix below defines coverage, not benchmark values.

| Dimension | Required variants |
| --- | --- |
| Cache state | Empty/cold; one-process warm; all replicas independently warm; shared cache warm; mixed rollout with old/new representation versions; flush/loss/restart; skewed hot key/repo. |
| Value class | Small immutable graph/changeset metadata; paged tree entries; bounded derived result with fixed inputs/version; negative miss; large blob/file through the separate NVMe path. |
| Access pattern | Uniform; Zipf/power-law; scan; repeated same-key burst; many keys with one hot tenant/repo; writes/fills interleaved with reads; realistic mixed read/write and miss ratio from F0. |
| Deployment | Single process; multiple API replicas sharing cache; separate worker/API connection pools; rollout surge at documented replica count; representative network RTT and failure injection. |
| Load shape | Offered load below and at the F0 target; controlled over-capacity load; slow cache; slow origin; disconnect/reconnect; concurrent cancellation; fanout/batch request within configured size limits. |
| Representation and security | Valid current version; old version; malformed/oversized value; digest mismatch; cross-tenant same content ID; authorization revoked during warm cache; membership removed; stale generation; poison attempt. |

Run at least the same logical query/read set in every applicable arm. Include cold and warm end-to-end user/API flows, not just cache GET throughput. Report cache lookup time as one component, with total request latency and work avoided.

## Measurements and boundedness evidence

Collect per-run, per-arm p50/p95/p99 and max for API operation, cache lookup, origin miss and fill; logical requests/s; offered/accepted/rejected requests; hit/miss/eviction/admission counts by bounded label; origin SQL/object reads/bytes; cache network bytes; retries/fallback attempts; queue age; server/client CPU; RSS/heap/allocator stats where available; disk occupancy/IOPS/bytes for NVMe; active and pending connections; in-flight commands/fills/waiters; response/batch sizes; PostgreSQL pool acquisition/lock/query times; and request deadlines/cancellations.

Every arm must show enforced maxima for active connections, concurrent commands, request and response sizes, batch item/byte counts, per-entry admission, retained/in-flight bytes, fill concurrency/waiters, retry count/deadline, origin miss concurrency, and fallback budget. The shared cache's `maxmemory` or Moka's weight alone is not adequate proof of process/container RSS bounds. Include rollout surge in fleet SQL/cache connection totals. Demonstrate that accepted requests beyond capacity are rejected/deferred within the registered error/deadline contract and do not accumulate unbounded queues or retained buffers.

Measure service quality with cache unavailable as an explicit scenario. Cache outage may increase bounded origin reads or produce bounded retryable errors, but it must not create unlimited database/object traffic, compromise authorization, or cause a correctness transition to differ. If the canonical origin is also unavailable, reads can fail within the declared service contract; no stale cache result may fabricate mutable authority.

For large-object disk-cache arms, measure range/resume correctness, checksum verification, cache-fill cancellation, disk-full behavior, eviction under pressure, in-flight pin safety, cold/warm throughput and tail latency, disk wear/IO, and whether active/dirty working content is ever evicted. Validate the actual I/O code path and the existing Tokio/io_uring evaluation separately; a device-local improvement is not an object-store/network improvement.

## Fault and security suite

For each cache candidate, run the same deterministic oracle on the pinned version/configuration. Preserve request IDs and event timeline so the reviewer can verify the outcome ordering.

1. **Cold and warm:** start from empty; repeat with local Moka warmed, shared KV warmed, and one replica restarted. Confirm equal authoritative results and expected origin-read reduction. Cold first-fill bursts must remain within fill/waiter/origin budgets.
2. **Outage:** stop or isolate the cache, inject latency/timeouts, exhaust client connections, and restore service. Exercise circuit breaker/open-close behavior, retries, connection recovery and mixed cache-up/cache-down replicas. Confirm fallback caps and no thundering herd.
3. **Flush and restart:** flush one namespace and all keys, restart cache process/node, and test failover/reconnect if candidate topology provides it. Confirm misses refill safely and in-flight callers coalesce within explicit limits. No correctness or authorization state disappears because the cache is empty.
4. **Skew and eviction:** drive a single hot key/repo and tenant skew, scan/high-cardinality churn, and values at/over admission limits. Confirm bounded hot-key load, fair tenant budgets, and that eviction only causes a miss. Record the chosen eviction behavior and its effect on origin load/tails.
5. **Concurrent fill/invalidation race:** pause an old derivation/fill, commit a newer derivation representation or advance the cache/recovery epoch, then release the old fill. Old values must not publish into the current namespace or be returned for a new key. Exercise delete/flush epoch races and rolling version changes.
6. **Revocation and membership:** warm content then revoke repository membership or change policy while requests are concurrent. Each response must follow the authoritative authorization/membership rule, independent of TTL/cache warmth. Authorization denial is not negative-cached as origin absence.
7. **Cross-tenant and key collision:** request equal content IDs in two tenant/repo scopes, plus ambiguous/overlong/malformed keys and alternate encodings. No value leaks across scope; key encoding is injective for the allowed input domain.
8. **Poisoning/integrity:** inject wrong type, wrong digest, wrong representation version, truncated bytes, malformed serialization and attacker-controlled cache contents. Include an attacker changing both payload and its entry-supplied checksum/input mapping. Verify against an expected canonical digest from a trusted authority or an application authenticator whose key is outside the cache; an entry's self-reported hash and a derived-input key cannot authenticate its output. Unverifiable derived values are hints requiring canonical recomputation before trusted use. Each invalid entry is rejected as a miss/integrity error before disclosure or use; canonical store/origin fallback is bounded.
9. **Restore epoch:** restore SQL/object fixture to a prior snapshot while cache and old worker/callback remain active. Fence old workers, allocate a fresh non-reused epoch, and ensure old entries/fills cannot authorize new operations. Durable domain correctness comes from the restored DB/object contract and recovery protocol.
10. **Overload:** exceed configured cache/origin capacity by the preregistered factor, slow consumers, cancel waiters, and trigger retries during restart. Confirm permits, retained bytes, connection pools, queues and fallback rates stay bounded; the full service remains semantically correct when the cache is absent, while over-capacity work fails in a bounded manner.

The oracle records independently durable client requests/responses and reconciles authoritative SQL outcomes and reachable object bytes after faults; it does not directly observe SQL commit atomically or infer a commit from a timed-out request. Supplemental instrumentation and code review establish that ref movements, approvals, checks, locks, quotas, durable job outcomes, object membership/pins and authorization decisions use their authoritative code paths. Cache reads may alter latency and origin load only. Include a deliberate negative-control build/configuration that bypasses independent authorization, accepts a poisoned result including its forged checksum, admits an old-epoch fill, and removes the fallback cap; the harness must fail each control before accepting results.

## Evidence package and commands

Each scored run writes an immutable directory under `artifacts/cache-evaluation/<run-id>/` (future path; no output exists yet). Include:

```text
manifest.json                 # preregistration id/hash, workload id, host/config/version/license pins
commands.txt                  # exact build, start, fixture, load, fault and scrape commands
environment.json              # OS/kernel/container/CPU/memory/network/storage and dependency details
server-config/                 # redacted effective Valkey/Dragonfly/Postgres/object-store configs
raw/                           # original metrics, client logs, fault-controller timeline, correctness oracle output
checksums.txt                  # hashes of all source manifests and raw output files
summary.md                     # analysis script/version, tables, confidence/runs, deviations and decisions
```

Do not put repository contents, credentials, source code, tokens, or unredacted sensitive logs in evidence. Use generated fixtures or approved sanitized traces. `commands.txt` must be runnable from a clean checkout with documented prerequisites. Store raw data alongside summarized charts; never retain only a spreadsheet or screenshot. Generate summary statistics from a checked-in script or command and record its version/hash. Report failed, unsupported, interrupted and excluded runs with the reason; don't silently drop them.

Recommended future command shape (replace placeholder CLI with the implementation selected in F1; this is a protocol example, not an existing executable):

```sh
<fixture-command> generate --manifest <manifest.json> --out <fixture-dir>
<service-command> serve --config <arm-config> --metrics-bind 127.0.0.1:<port>
<load-command> replay --manifest <manifest.json> --arm <arm> --run-id <run-id> --raw-out <run-dir>/raw
<fault-command> apply --scenario <scenario-id> --run-id <run-id>
<analysis-command> summarize --manifest <manifest.json> --input <run-dir>/raw --out <run-dir>/summary.md
```

These paths are proposed outputs and commands. No candidate benchmark has yet been run, no benchmark claim is made, and no numeric outcome should be filled in until F0 preregisters it.

## Adoption gates

**Go for a shared KV in S3** only if all are true: (1) S2/F0 measurements identify repeat cross-process reuse as a material bottleneck; (2) one pinned candidate meets the preregistered end-to-end benefit and resource/cost limits against Moka and no-shared-cache controls; (3) all correctness, authorization, isolation, poisoning, invalidation/restore-epoch, and outage/overload controls pass; (4) cache outage/failure remains within the documented bounded error/fallback contract; (5) client and server licenses permit every declared install/hosting/redistribution mode; and (6) operations can monitor, patch, upgrade and disable the cache with a reversible rollout.

**Defer** when the API/key format or S2 capacity evidence is insufficient, the bottleneck is mainly SQL authority/ref serialization/object network rather than reusable metadata reads, or a candidate does not meet the pre-registered thresholds but no decisive failure makes it unsuitable.

**No shared cache** when Moka is sufficient; network lookup misses/cost dominate; shared cache does not materially reduce origin work/end-to-end cost; outage fallback cannot be kept within bounds; auth/membership cannot be independent; poisoning/stale fill can affect returned data; or deployment licensing/operations cannot be accepted. A `no shared cache` result still permits local Moka and a separately qualified NVMe object cache.

All outcomes record supported/unsupported topology, workload, versions, license mode, cost basis, limits, residual risks, and revisit trigger. A new candidate or changed client/server release requires compatibility and relevant fault cases again. Passing a cache experiment does not authorize distributed SQL, stale mutable reads, or making a cache mandatory for the MVP.

## Agent-ready work cards (proposed)

These cards summarize planned work now [mapped to Linear](project-tracking.md); editing this document does not automatically synchronize tracker descriptions or status.

### A14 — Common metadata/cache evaluation contract (F3)

**Depends on:** F1 and accepted F0 workload/acceptance contract.\
**Task:** reuse F0 fixtures; write the versioned common manifest, arm definitions, environment capture, metrics, raw evidence schema, runtime commands and negative controls for A15–A17. Include Moka/origin and distinct metadata-KV versus large-object disk-cache dimensions. Do not select a product or create benchmark claims.\
**Deliverable:** `docs/evaluations/scaling-evaluation-contract.md`; fixture references and checked-in metric/analysis schema as implementation exists.\
**Acceptance:** two reviewers can run the same seeded workload in two arms and compare outputs; a deliberate stale-authority, cross-tenant, poisoned-value and unbounded-fallback fault is detected; all absent numerical targets are left for the preregistration rather than invented.

### A16 — Shared cache qualification (S3)

**Depends on:** S2 and accepted A14/common contract.\
**Task:** run Moka plus Dragonfly/Valkey only after exact pinned release/client/license review; compare against Moka alone within the same aggregate fleet resource budget on the common metadata workloads; evaluate local NVMe separately for larger immutable objects. Run cold/warm/outage/flush/restart/skew/eviction/fill-race/revocation/cross-tenant/poisoning/restore-epoch/overload cases. Produce a go/defer/no-cache decision. No mandatory cache implementation.\
**Deliverable:** evidence bundle under `artifacts/cache-evaluation/<run-id>/` and a dated `docs/evaluations/A16-shared-cache/` report preserving source-backed research, with exact supported versions, license disposition, command/config hashes, measured benefit/cost, unsupported cases, and decision.\
**Acceptance:** all scored runs follow preregistered workloads and resource limits; the oracle's negative controls fail; no candidate cache affects domain correctness; bounded outage/overload behavior passes; exact server/client versions and license files are recorded; recommendation includes residual risks and a revisit trigger. Mark a candidate unsupported/deferred when exact compatibility or license fit cannot be established.

## Relationship to adjacent plans

The [Rust backend scaling plan](rust-backend-scaling.md) supplies the Moka and resource-budget baselines. The [technical plan](technical-plan.md) defines API/object identity, canonical PostgreSQL authority, authorization and roadmap stages. The [durability/workflow plan](durability-and-workflows.md) defines database/object publication and recovery-epoch behavior; this cache plan does not add any durability authority. [Implementation roadmap](implementation-roadmap.md) assigns S2 before S3 and describes A14/A16 as optional research packages. The evaluation outputs remain non-blocking for the small-team MVP unless a measured deployment requirement explicitly changes the roadmap.
