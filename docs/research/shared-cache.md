# Deltrove: shared cache and metadata-store research

**Reviewed:** 17 October 2026\
**Status:** research and recommendation; no cache server selected, installed, or benchmarked.\
**Scope:** optional shared key-value cache for small immutable metadata/results, plus the separate large-object NVMe cache tier. This does not propose replacing PostgreSQL as the authority for domain metadata.

## Recommendation

Keep PostgreSQL authoritative and keep the MVP cache optional. Start with the existing design: request-local batching/coalescing plus a bounded process-local Moka cache for small immutable values. Add a shared network cache only after the API and object/metadata representations are stable and measured cross-process reuse at S3 justifies its extra hop, operating surface, and failure modes. A small prototype may start once the cache API and key format are stable, but it does not advance the production decision gate.

Valkey is the preferred open-license candidate to qualify first. Its upstream project identifies the server as BSD-3-Clause licensed. Dragonfly is a technically plausible Redis-protocol candidate, but the current upstream `main` license is BSL 1.1, with a version-specific change date and an additional-use grant. It is not an Apache-2.0 release today: the current license names Apache-2.0 as the change license effective November 1, 2030 for that licensed work, subject to the BSL's earlier fourth-anniversary rule for each version. Recheck the exact release, full license, change date, and intended hosted/self-hosted distribution at adoption time. Do not assume the terms allow a hosted Deltrove service. [Dragonfly license](https://github.com/dragonflydb/dragonfly/blob/main/LICENSE.md), [Valkey project](https://github.com/valkey-io/valkey).

The current evidence does not show that Dragonfly or Valkey will improve Deltrove's end-to-end performance. Their published descriptions and compatibility claims are vendor/project statements, not Deltrove workload measurements. A shared tier adds a network round trip and another dependency; it can lose to local Moka on a warm API process, especially for tiny metadata. Qualification must compare cold, warm, and mixed API-replica states against the PostgreSQL/process-local baseline under equal resource limits.

## Keep cache classes separate

| Data class | Starting path | Cache treatment |
| --- | --- | --- |
| Small immutable metadata: tree pages, graph/changeset metadata, bounded derived summaries | PostgreSQL canonical rows/pages and existing object adapter, with bounded local Moka | Candidate for shared KV only when values fit a strict encoded/retained-byte cap and reuse spans processes. Key includes repository, record/type, immutable content ID, and encoding/derivation version. |
| Large immutable blobs or file payloads | Immutable object store, streamed to clients | Separate optional local NVMe/disk cache, with checksums, byte/range semantics, disk budget, in-flight pinning, fill concurrency and eviction rules. Do not store large bodies in the shared metadata KV. |
| Mutable or security-sensitive domain state | PostgreSQL transaction/read path | Never made authoritative by any cache. This includes refs/generations, membership, policy, approvals, checks, locks, quota enforcement, idempotency outcomes, durable job state, object-retention pins, and recovery epochs. |
| Performance hints | Cache or bounded process-local counters | May be stale or discarded. They cannot decide authorization or exact quota. Rate limiting that protects an origin needs a separate bounded/fleet-aware enforcement point; local hints alone multiply with replica count. |

This split follows the architecture's object-store contract: canonical object bytes are immutable and streamed, while representation IDs must be explicit for any alternate encoding. A network KV is a small-value metadata path. An NVMe content cache is a byte-serving path with a much larger retained-data and disk-integrity surface. Reusing one cache policy for both obscures memory, disk, admission, eviction, and authorization limits.

## Candidate comparison

| Candidate | Current evidence | Benefits to test | Costs and unknowns |
| --- | --- | --- | --- |
| Process-local Moka (baseline) | Existing Rust scaling plan uses custom weights, per-entry admission, and same-instance miss coalescing. Moka's weight is approximate; it is not a hard process RSS bound and does not coalesce across replicas. [Moka cache builder](https://docs.rs/moka/latest/moka/future/struct.CacheBuilder.html), [Moka cache API](https://docs.rs/moka/latest/moka/future/struct.Cache.html) | No network hop or separately operated service; simple outage behavior; locality and key ownership are clear. | Duplicate entries across processes; no shared warm-up or fleet-wide invalidation. Must keep actual in-flight and referenced values within process budgets. |
| Valkey (candidate to qualify first) | Upstream describes Valkey as a distributed in-memory datastore optimized for cache/realtime workloads and publishes the server under BSD-3-Clause. Its docs define `maxmemory` and approximate LRU/LFU/TTL policies; Cluster shards keys by slots and requires a cluster-aware client. [Valkey repository](https://github.com/valkey-io/valkey), [eviction documentation](https://valkey.io/topics/lru-cache/), [cluster tutorial](https://valkey.io/topics/cluster-tutorial/) | Permissive server license; familiar RESP/Redis ecosystem; explicit eviction controls; cluster option if one instance is insufficient. | Rust client/version/feature compatibility and client health/reconnect semantics must be tested. Cluster requires routing, topology, networking and operations; it is not a transparent single endpoint. `maxmemory` does not cap total RSS or replication/AOF buffers. A hot repo/key can still concentrate load. |
| Dragonfly (conditional candidate) | Current upstream docs say Dragonfly accepts Redis and Memcached API requests, and upstream license is BSL 1.1. [Dragonfly docs](https://www.dragonflydb.io/docs), [current license](https://github.com/dragonflydb/dragonfly/blob/main/LICENSE.md) | Redis-compatible protocol is a plausible integration path; candidate for a single-node/shared cache where its implementation and deployment model fit. | BSL 1.1 distribution/hosting restrictions require review; release terms can differ. Compatibility is not proof that every command, error, expiry, transaction, script, connection, or cluster behavior matches the chosen Rust client. Do not introduce it as a mandatory self-hosted or hosted dependency without exact-version licensing acceptance. |

Valkey eviction policies are approximate and operate around a configured `maxmemory`; upstream documentation notes replication/AOF memory outside the eviction accounting, and that actual RSS may remain high after key eviction due to allocator behavior. Therefore process/container memory limits and response/fill bounds remain necessary even with server-side eviction. For this immutable cache, `allkeys-lru` or `allkeys-lfu` are starting candidates for an experiment, not decisions. Expiry is useful for bounded retention but cannot substitute for authorization or correctness invalidation.

Do not choose a cache by claimed peak QPS. Compare its operationally useful hit rate and total request cost against Moka and direct origin reads. Include server CPU, client CPU, network bytes, PostgreSQL/origin work avoided, connection usage, p95/p99, retry/fallback amplification, and memory/eviction behavior. The win must remain positive after including the added hop and failure controls.

## Data and safety contract

Only immutable, reproducible, bounded values are eligible. Initial candidates are immutable graph/changeset metadata, versioned tree pages, and derived results with exact immutable inputs and a derivation version. A result whose value depends on a mutable ref, permissions, policy, approval state, checks, or current membership is not a cacheable authority. If it is cached as a performance hint, the authoritative state must still be checked independently before use.

Use a canonical key shape conceptually equivalent to:

```text
deltrove:v1:{tenant-or-repo-scope}:{repository-id}:{kind}:{content-id}:{representation-or-derivation-version}
```

The concrete key must be length-bounded, unambiguous, and safely encoded; avoid raw user-controlled path text. Include tenant/repository scope even if content IDs are globally content-addressed, so accidental cross-repository reuse is not an access grant. Include canonical schema/format/algorithm version so rolling upgrades cannot parse old bytes as new. For canonical content-addressed values, compare the cached bytes with the expected digest obtained from an authoritative membership/metadata mapping, not a checksum stored beside those same cache bytes. A self-reported cached checksum detects accidental corruption only if the expected checksum is independently trusted; it does not authenticate the value. For derived metadata that has no canonical content digest, either verify an application MAC/signature using a key outside the cache and verify its trusted derivation/input mapping, or treat the cache entry as an untrusted hint and recompute from canonical data before use. If neither verification nor recomputation is feasible, do not serve the cached value as correct. Reject malformed, overlong, mismatched, or unsupported-version values as misses and emit bounded diagnostics without logging source bytes.

Authorization and repository membership must be evaluated against authoritative state independently of cache hits. Cache lookup must not happen before repository authorization in a way that leaks hit timing, value existence, or bytes across tenants. Membership checks can be batched for bounded object/page batches, as already proposed in the API design. TTL alone is insufficient for auth: revocation and membership changes can outlive an arbitrary TTL. Never mark private source as publicly cacheable.

Write cached values only after authoritative publication/readiness is committed. Cache fill is best effort; failed writes do not fail a successful authoritative operation. Immutable keys make overwrites unnecessary; use set-if-absent or verify an existing value against an independently trusted source, and treat any mismatch as a cache integrity fault. Use TLS/authenticated network access, network isolation, narrowly scoped credentials/ACLs, and separate namespaces per deployment/tenant boundary as appropriate. Assume any cache administrator or compromised cache can alter both payload and cache-stored checksum/input mapping: validate against an independently trusted digest, membership/derivation record, or application MAC key kept outside the cache, otherwise recompute from canonical data before use.

Never use cache presence, absence, expiry, or eviction to decide:

- whether an object belongs to a repository, whether a user can read it, or whether a path is within an authorized boundary;
- which ref/generation is current, whether a review is approved, whether a check passed, or whether a lock is held;
- exact quota availability, unique request outcomes, accepted job state, object retention, or whether a durable operation completed;
- whether a restored system may accept callbacks or old workers.

Negative caching is disabled for authorization and membership by default. A bounded short negative cache may be considered only for immutable, non-security-sensitive lookups where the origin's absence is stable and keyed by all relevant immutable versions. Never cache transient storage errors, permission denials, stale refs, pending/failed snapshots, or missing content as long-lived absence. A published object that is missing from the immutable store is an integrity/recovery incident, not a cacheable `not found`.

Restores need explicit invalidation epochs. After restoring SQL and object state, allocate a fresh non-reused recovery epoch under the recovery fence; old cache namespaces and late fills/callbacks cannot become current by reusing a restored counter. Namespace cache entries by the applicable representation/derivation version and, where mutable generations influence a derived value, by the exact authoritative generation. Flush/epoch advance is an operational performance action, never a substitute for fencing stale workers or revalidating domain state.

## Bound the whole cache path

The application must enforce limits regardless of server product:

- maximum concurrent client connections and in-flight commands, with finite acquisition/connect/command deadlines;
- bounded pool size per process and a fleet-wide connection budget including rollout surge and worker/API classes;
- maximum key length, value/encoded response size, batch count/bytes, and request/response buffering; stream or reject oversized results;
- per-entry admission size, total approximate retained cache bytes, and maximum in-flight fills/waiters per key;
- bounded origin-miss concurrency and tenant/storage budgets, plus circuit breaking/backoff and bounded retries;
- capped fallback request rate so cache outage/flush does not multiply origin load by all API replicas;
- bounded fill work and cancellation propagation so timed-out clients do not leave unbounded server work.

The existing Rust plan's experimental 256 MiB weighted Moka cache and other example limits are hypotheses for a particular example host, not supported production values. F0 must define workload-specific limits and objectives before measurements; each future capacity claim must say how far offered load may exceed configured capacity before requests are rejected or shed. Do not let a full cache command queue accumulate stale fills indefinitely. Prefer a miss to waiting behind unrelated cache/server work when the deadline budget is exhausted.

Cache failure behavior is: attempt the cache within its short deadline; on a miss/error, use a bounded single-flight origin path; if the shared miss/origin budget is full, reject or defer with a retryable, bounded error. Never create a cache outage thundering herd by unlimited SQL/object fallthrough. Where a request can safely use Moka, its local result is still subject to the same authorization, membership, byte, and in-flight bounds.

## Licensing and implementation constraints

Project-owned implementation remains Apache-2.0; C and C++ are excluded as implementation-language choices. Running a separately packaged cache service is not the same as writing Deltrove in its implementation language, but deployment still brings licensing, support, security-update, image, and operator commitments. Do not add C/C++ application code or require a C/C++ SDK for the shared-cache path. Existing protocol-compatible product dependencies are subject to normal exact-version SBOM and license review.

Valkey's upstream repository identifies the server as BSD-3-Clause. The Valkey GLIDE project is a multi-language client whose core is written in Rust, but its current README does not list a generally supported Rust end-user API among its language clients; an internal Rust core does not itself prove there is a supported Rust client. Do not assume GLIDE is directly consumable as a stable Rust library. Evaluate the exact Rust RESP client already selected/considered by the project (or a thin project-owned protocol adapter) against pinned Valkey and Dragonfly releases. The client is a separate license/version line from the server. [Valkey GLIDE README](https://github.com/valkey-io/valkey-glide), [Valkey GLIDE repository](https://github.com/valkey-io/valkey-glide).

Dragonfly's current license explicitly says Business Source License 1.1. Its Additional Use Grant is framed around use as part of one's own product/service, excludes an in-memory data store product or service, and restricts making the licensed work available as a service where third parties access substantial functionality that competes with Dragonfly's offerings. The intended-use question is whether Deltrove's deployment would itself offer or expose the in-memory datastore functionality as a competing service; the mere fact that a hosted source-control product uses Dragonfly internally does not automatically answer that question either way. This document is not a legal conclusion. Before an adoption decision, attach the exact release's license text and record counsel's/owner's assessment of the actual service boundary for (a) Deltrove-hosted operation, (b) customer-operated self-hosted deployment, and (c) image/package redistribution. If the intended deployment is not clearly permitted, defer Dragonfly for that mode or obtain a commercial license. Do not treat its expected future Apache license as a present permission.

## Decision outcome

Current direction: **go** with process-local Moka and PostgreSQL as the MVP; **defer** production shared KV to S3; **qualify Valkey first** if S2 measurements identify an origin/read bottleneck that cross-process reuse can address; **keep Dragonfly conditional** on its exact release/license and protocol evidence. A production shared cache is rejected if domain correctness, authorization, or durable-state transitions depend on it; if independent authorization/membership cannot be maintained; if outage behavior exceeds the declared safety, origin-load, or degraded-capacity/error budget; or if measured gains do not outweigh the added hop and operating cost. A deployment may rely on cache capacity for normal peak performance only if cache loss preserves safety and origin bounds and results in the explicitly budgeted degraded service/error behavior. No candidate has passed a Deltrove benchmark or fault test as of this review.

## Primary sources reviewed

- Dragonfly current license: https://github.com/dragonflydb/dragonfly/blob/main/LICENSE.md
- Dragonfly current docs: https://www.dragonflydb.io/docs
- Dragonfly current quick start/Redis protocol example: https://github.com/dragonflydb/dragonfly/blob/main/docs/quick-start/README.md
- Valkey repository and license declaration: https://github.com/valkey-io/valkey
- Valkey official eviction docs: https://valkey.io/topics/lru-cache/
- Valkey official cluster tutorial/spec: https://valkey.io/topics/cluster-tutorial/ and https://valkey.io/topics/cluster-spec/
- Valkey GLIDE project/README: https://github.com/valkey-io/valkey-glide
- Moka official Rust docs: https://docs.rs/moka/latest/moka/future/struct.CacheBuilder.html and https://docs.rs/moka/latest/moka/future/struct.Cache.html

Source state was checked on 17 October 2026. Recheck release tags and license files before pinning a server or client; `main`/`unstable` pages can move.
