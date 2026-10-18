# Research synthesis and review record

**Date:** 16 October 2026.\
**Scope:** research and technical planning. No product code, deployments or performance tests were created.

**Current server decision:** Rust, selected by the user after the initial comparison. The earlier Go server recommendation is superseded; the Go CLI proposal remains. The follow-up [Rust backend scaling plan](rust-backend-scaling.md) incorporates three additional Luna research passes and primary-agent review.

## Research method

The primary agent read [Anthropic's Scion article](https://www.anthropic.com/engineering/redesigning-source-control), inspected the empty workspace and applicable ancestor instructions, and dispatched three **GPT-6 Luna** subagents with distinct research scopes:

1. Existing source-control systems, reuse, licensing, and migration: [prior-art memo](research/prior-art.md).
2. Backend language/storage/protocol choices, transaction correctness, recovery, and scale: [backend memo](research/backend-stack.md).
3. CLI/workspace/daemon choices, frontend frameworks, browser integration, and portability: [client/frontend memo](research/clients-and-frontend.md).

The user subsequently confirmed a self-hostable small-team starting point with a large-organization evolution plan, requested broader systems-language research, and excluded C/C++. The research was extended to Zig, Swift, C#/.NET, and JVM languages alongside Go/Rust. Each Luna agent then read and reviewed the full synthesized technical plan. The primary agent reviewed all three memos, all three review reports and the resulting plan, checked additional primary sources, and incorporated the material corrections below.

This is one coordinated research effort with multiple reviewers, not three independent experimental validations. The primary agent made the final architecture decisions; agent agreement does not prove correctness or comparative performance.

## Decisions and disagreements resolved

| Question | Research assessment | Final proposed decision |
| --- | --- | --- |
| Native tool or Git extension? | The prior-art pass placed more weight on Git interoperability; backend/client passes assumed a native service. | Evaluate Git-backed APIs, a tiny native prototype and Mononoke first. Proceed with the native design only if useful benefits and migration feasibility survive the gate. |
| Reuse Mononoke? | Strongest architectural precedent, but externally unsupported and not a turnkey library/service. | Time-box a build/import/operations evaluation. Keep independent implementation and license-dependent fork as separate alternatives. |
| Go or Rust? | The initial passes preferred Go for whole-product delivery simplicity; the user subsequently selected Rust for the server. | Rust server/domain/workers. The Go CLI stays behind the generated HTTP contract; schemas/vectors cross the language boundary. No Go server or FFI is required. |
| Other systems-oriented languages? | .NET/JVM are credible service options; Swift/Zig require more project-specific ecosystem and target validation. | Retain a six-candidate comparison; do not choose a less familiar ecosystem solely on runtime or compiler appeal. |
| Commit object model? | One early backend paragraph described snapshot-first identity, conflicting with the delta-first plan. | Corrected the memo: canonical first-parent deltas determine changeset identity; versioned manifest roots remain derived. |
| RPC or HTTP? | Typed RPC is useful, but browser transport and duplicated APIs add scope. | Axum HTTP/OpenAPI with binary endpoints. Tonic is a later measured alternative; the historical Connect-Go server option is not selected. |
| Shared daemon and offline commits? | The first client draft proposed both. They are not needed to prove the online-first workflow. | In-process cache first; offline materialized edits/status/diff; online authoritative commits; defer offline commit queues and shared daemon. |
| Serialized landing? | A simple queue is practical for a pilot but can limit a hot trunk. | Deliberate per-target MVP coordination; exact-candidate checks and CAS. Large-fleet landing policy is separate research, not presumed Scion behavior. |
| Frontend? | React/Vite fits an authenticated application; Next.js/Svelte remain viable with different requirements or team expertise. | React/TypeScript/Vite static app; server-generated diff, lazy editor, accessible unified fallback. |
| Enterprise scale? | Postgres and object bytes can scale differently; repo sharding does not divide one enormous monorepo. | Stage read/cache/worker scaling first; single-repo metadata distribution remains an explicit redesign/benchmark milestone. |

## Material findings and disposition

The detailed reports retain the findings as recorded during review. This table records subsequent resolution in the final [technical plan](technical-plan.md); it does not claim that future implementation work is complete.

| Finding / owner | Resolution or remaining gate |
| --- | --- |
| Delta-first versus snapshot-first contradiction — primary/backend | Corrected research memo and plan alignment. Semantic snapshot invariance across derivation representations is explicit. Format ADR and golden vectors remain mandatory before stable IDs. |
| Canonical bytes insufficiently specified — backend | Added exact hashed-field/schema decisions, path-byte ordering, duplicate rejection, mode/delete rules and logical-snapshot invariance. Restricted CBOR is a candidate; format/page layout are gated experiments. |
| GC/publication race — primary/backend | Simplified first deletion mode to maintenance-only with persistent write fencing and drained operations; actual deletion stays disabled until tests pass. Concurrent GC is deferred rather than described as already solved. |
| DB/object backup coupling — backend | Added recovery watermarks, backup manifests and conservative retention of baseline plus subsequently published objects. RPO/RTO and concrete storage settings remain pilot decisions. |
| Upload resumption versus compression ambiguity — primary/backend | Added upload state machine, session pins, staged-byte quotas, multipart transfer and raw-byte range rules. Compression at rest does not change object identity. |
| Ref CAS insufficient for approvals/policy races — primary/backend | Added final-transaction lock/generation recipe and revocation ordering, with atomic ref/audit/outbox result and whole-transaction retries. Concurrency tests remain a release gate. |
| Outdated-main commits and upload-time edits — primary/client | Commits freeze a workspace-parent snapshot, independent of current main; later edits remain dirty. Unknown outcomes reconcile through operation ID. Sync and explicit merge behavior are distinct. |
| Overbroad Git re-export promise — prior-art/client | Exactness is confined to unchanged preserved imported objects and supported refs/hash formats. Raw blobs and non-commit/nested tags are included in the inventory/test gate. Native history has new Git identities. |
| Native migration mistaken for Git compatibility — prior-art/client | One writer at each stage, native workspaces without `.git`, one-way mirror/export and deferred Git gateway are explicit throughout. |
| Unsupported CitC details — prior-art | Removed filesystem/workspace claims. The accessible Google publication abstract establishes central-monorepo context only; full-paper details remain unverified in this pass. |
| Go concurrency described too broadly — primary/backend | Corrected single-primary capacity versus concurrent independent transactions and per-ref serialization. |
| MVP/enterprise scope and timeline too optimistic — backend/client | Added a minimum alpha slice, independent fidelity/merge/platform/HA gates, low-confidence estimate labeling and re-estimation after phases 0/1. Enterprise readiness is not included in a pilot estimate. |
| Platform promise unclear — client | Linux/macOS first and Windows later are stated in product scope, workspace guidance and delivery plan. SQLite driver/build portability is an explicit spike. |
| Browser security tests missing — client | Added hostile-content/XSS, cookie/session and CSRF checks alongside accessibility, stale-comment and large-diff tests. |
| Cache access revocation overstated — primary | Auth checks remain separate from cached bytes; already downloaded content cannot be recalled. Initial design avoids presigned source-object URLs. |
| S3-compatible storage too abstract — primary | Added SeaweedFS as first self-hosted integration candidate, Garage as an alternative and local filesystem mode for a simple installation. Neither candidate is certified until conformance/recovery testing. |

## Additional primary-source checks by the primary agent

These complement the citations in the three memos. Each check supports a limited fact, not the entire proposed architecture.

- [Mononoke README](https://github.com/facebook/sapling/blob/main/eden/mononoke/README.md) and [architecture](https://github.com/facebook/sapling/blob/main/eden/mononoke/docs/1.3-architecture-overview.md): external-support/OSS omissions and canonical/derived/storage separation. This is the closest implementation reference found.
- [PostgreSQL transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html) and [SELECT locking](https://www.postgresql.org/docs/current/sql-select.html): database concurrency mechanisms; application invariants and retries still require design.
- [S3 consistency](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html) and [conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html): per-key behavior does not supply a joint Postgres/object-store transaction.
- [Protobuf serialization is not canonical](https://protobuf.dev/programming-guides/serialization-not-canonical/) and [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html): RPC serialization and stable content identity are separate contracts.
- [Connect-Go](https://github.com/connectrpc/connect-go), [browser client limits](https://connectrpc.com/docs/web/getting-started/) and [Tonic-Web limits](https://docs.rs/tonic-web/latest/tonic_web/): concrete protocol alternatives; browser client/bidirectional streaming cannot be assumed.
- [Git fast-export](https://git-scm.com/docs/git-fast-export), [fast-import](https://git-scm.com/docs/git-fast-import) and [cat-file](https://git-scm.com/docs/git-cat-file): streaming conversion and original-object retention have different fidelity properties.
- [Git merge-base](https://git-scm.com/docs/git-merge-base) and [merge-file](https://git-scm.com/docs/git-merge-file): multiple merge bases and a file-level merge primitive require explicit higher-level semantics.
- [Go rooted filesystem APIs](https://go.dev/blog/osroot): useful traversal defense, with platform and operation limitations; not a multi-file transaction solution.
- [OpenAPI Go generator](https://github.com/oapi-codegen/oapi-codegen), [TypeScript generator](https://openapi-ts.dev/introduction) and [Cobra](https://github.com/spf13/cobra): proposed client tooling. The earlier [pgx](https://github.com/jackc/pgx) server option is historical; Rust SQLx is now proposed.
- [.NET Native AOT](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/), [Java virtual threads](https://docs.oracle.com/en/java/javase/25/core/virtual-threads.html), [Swift server](https://www.swift.org/documentation/server/) and [Zig overview](https://ziglang.org/learn/overview/): alternatives assessed without unmeasured performance rankings.
- [SeaweedFS](https://github.com/seaweedfs/seaweedfs), [Garage](https://github.com/deuxfleurs-org/garage) and [MinIO](https://github.com/minio/minio): concrete self-hosting candidates, repo license labels and current maintenance status. MinIO's public repository is archived and marked unmaintained as checked on this date; do not treat historical tutorials as current operational guidance.
- [FoundationDB architecture](https://apple.github.io/foundationdb/architecture.html), [TiKV transactions](https://tikv.org/deep-dive/distributed-transaction/introduction/) and [CockroachDB licensing](https://www.cockroachlabs.com/docs/stable/licensing-faqs): possible later storage investigations and a reason not to assume every distributed SQL option is an unrestricted OSS default.

## Remaining uncertainties

1. **Workload and team skills:** neither a target repository nor actual contributors are known. Rust is selected for the server; dependency choices, benchmarks, schedule and capacity still require evidence.
2. **New-engine benefit:** no native-format prototype, tuned Git benchmark or Mononoke build has been run. Phase 0 can legitimately select Git-backed APIs or a different reuse path.
3. **VCS format/algorithms:** exact canonical schema, paging, graph acceleration and merge edge cases need executable specifications and conformance tests.
4. **Operational proof:** no failure injection, authorization attack testing, restore, HA failover, GC deletion or upgrade rehearsal has been performed.
5. **Enterprise readiness:** directory ACLs, a hot single-monorepo storage model, global placement, deeper integrations and virtualization are future work, not features obtained automatically from horizontal API replicas.
6. **Primary-source limits:** public Scion details are not an implementation specification. The full Google paper was not accessible. Repository `main`, language docs and license terms can change; versions/notices must be pinned at implementation time.

## Rust backend follow-up

At the user's request, three additional GPT-6 Luna agents researched [storage/metadata](research/rust-storage-scaling.md), [runtime/caches](research/rust-runtime-scaling.md) and [operations/testing](research/rust-operations-scaling.md). The primary agent reviewed all three, checked upstream runtime/middleware/cache interfaces, updated the technical plan and produced the [Rust backend scaling synthesis](rust-backend-scaling.md).

The follow-up chooses Tokio/Axum/Tower, SQLx-first, bounded Rayon/worker execution and an `object_store` adapter spike. It preserves Postgres/object durability boundaries and introduces explicit stream-held permits, cancellation-aware CPU admission, weighted-cache caveats, fleet/rollout database budgets and multi-worker fencing. The runtime memo was corrected during review to include Moka custom weights, retain job permits through actual CPU exit and qualify disconnect cancellation. Bulk queue-age proposals are separated from interactive snapshot-readiness objectives. No capacity measurements or implementation tests have been performed.

All three follow-up agents reviewed the synthesis. Their clarifications were incorporated: sum every pool maximum per process, explicitly defer separate cache mode, enumerate adopted/candidate/deferred SLOs and account for large backing allocations retained by small stream slices. They reported no material correctness contradictions; implementation and benchmark gates remain open by design.

## ACID and durable-execution follow-up

The user next requested Luna research into durability/ACID and Temporal or DBOS, confirming that “BDOS” meant DBOS. Three GPT-6 Luna agents produced [Temporal research](research/temporal-durability.md), [DBOS research](research/dbos-durability.md) and an [independent ACID boundary review](research/acid-workflow-boundaries.md). The primary agent reviewed these, checked current official SDK/repository and PostgreSQL sources, and created the [durability/workflow decision](durability-and-workflows.md). All three agents then reviewed the synthesis.

| Finding | Disposition |
| --- | --- |
| ACID and durable orchestration are different guarantees | Domain mutations remain atomic SQL transactions; workflow history coordinates retries/waits, with no PostgreSQL/S3 transaction implied. |
| Initial ACID memo missed the new official DBOS Rust SDK | Corrected using the official repository. Rust exists but current transactional-step support is missing; no other-language shim is presumed necessary. |
| Temporal Rust maturity descriptions vary with old material | Use the official September GA announcement and current releases, pin exact versions and test client APIs. Root's fresh client-README read did not support an agent's blanket Public Preview claim; optional experimental APIs remain a gate. |
| Vendor comparison suggested extra Temporal storage requirements | Temporal supports PostgreSQL; Cassandra is not mandatory. Temporal/application persistence still has a dual-write/recovery boundary. |
| DBOS library versus Conductor license/recovery distinction | SDK and proprietary optional control plane are separate. Exact Rust recovery/Conductor feature support requires release-specific verification. |
| Earlier isolation guidance allowed RC + proven locks/CAS | Tightened current plan to `SERIALIZABLE` for critical metadata mutations, full retries and invariant-wide protocol participation. A lower level requires a reviewed proof/test case. |
| Local durable commit versus HA/disaster recovery | Added explicit profiles: WAL/object flush, configured synchronous replicas, fencing/eligible promotion, surviving-copy failure scope and coordinated backups. No automatic async fallback for a zero-loss acknowledgment profile. |
| Workflow start/Activity retry/retention gaps | Preserve atomic outbox, stable operation IDs, input/candidate reconciliation, domain outcome retention and idempotent external CI handling. |
| Independent restore can replay obsolete workflow state | Fence old authority, assign a fresh recovery epoch, reconcile domain/engine/CI state and check epoch on late publications. |

That pass recommended PostgreSQL/outbox/fenced workers for the core and a focused Temporal comparison before building general workflow machinery. The later roadmap expands that comparison as recorded below. DBOS remains an exact-version watch-list candidate. No workflow engine, dependency or deployment was installed, and ACID/durability claims still require the specified failure, failover and restore evidence.

## Agent implementation roadmap and Restate expansion

The user plans to use AI agents for implementation and requested a multi-step small-team and large-organization plan, including tests for **Temporal and Restate**. The primary agent created the [implementation roadmap](implementation-roadmap.md) and [workflow evaluation plan](workflow-evaluation-plan.md), reviewed their dependency and acceptance gates, and aligned the architecture/durability/scaling summaries. The prior Luna reports remain dated research inputs; they did not review these new deliverables.

The roadmap replaces the earlier optimistic phase/time estimate with F0–F5 feasibility, M1–M8 small-team delivery and S1–S8 enterprise qualification. It defines task handoffs, shared-contract ownership, independent review and actual test evidence. A minimal alpha requires working backend/frontend/CLI plus tested backup/restore, and does not imply host-loss HA. Many-repo sharding and one-hot-monorepo capacity have separate gates.

The workflow plan requires PG, Temporal and Restate thin experiments at F2; M4 tests the selected approach against real landing; M8 covers small-team outage/restore, and S1/S6 add distributed/sustained-load cases. An independent acknowledgment/outcome oracle covers unknown commits, duplicate starts/effects, stale workers, policy races, retention, upgrades and misaligned engine/domain restore. No experiment has been executed.

The primary agent checked Restate's [official Rust SDK](https://github.com/restatedev/sdk-rust), [journaled-action API](https://docs.rs/restate-sdk/latest/restate_sdk/context/trait.ContextSideEffects.html), [deployment versioning](https://restate.dev/blog/dealing-with-versioning-in-long-running-agents) and [replicated deployment guide](https://docs.restate.dev/guides/local-to-replicated). Journaled results do not remove the gap before an external effect is recorded; domain idempotency remains required. Actual version routing, retention and HA need pinned-release tests.

Earlier research excluded Restate from the all-OSS shortlist. The new user instruction explicitly includes testing it. Its [server license](https://github.com/restatedev/restate/blob/main/LICENSE) is BSL 1.1 with Additional Use Grant/change terms, while the [Rust SDK](https://github.com/restatedev/sdk-rust/blob/main/LICENSE) is MIT. The plan therefore includes required evaluation with an explicit distribution/license adoption gate; it does not describe the current server as open source or assume every production deployment is forbidden. No implementation or mandatory dependency decision has been made.

## Local I/O follow-up

The user asked whether io_uring and `tokio-uring` fit the Rust backend, then requested that evaluation be added to the plan. The primary agent checked the [official `tokio-uring` API](https://docs.rs/tokio-uring/0.5.0/tokio_uring/), [project README](https://github.com/tokio-rs/tokio-uring), [Tokio file-read API](https://docs.rs/tokio/latest/tokio/fs/fn.read.html) and [runtime builder](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.enable_io_uring). These distinguish the separate current-thread/owned-buffer runtime from Tokio's own experimental, operation-specific io_uring support. Generic filesystem documentation and specific-operation documentation differ in coverage descriptions; implementation must pin versions and verify the actual adapter call path.

The roadmap now includes local-I/O comparison in F3/A11, an F5 decision, real adapter durability in M1, release qualification in M8 and disk-cache re-evaluation in S3. Ordinary multithreaded Tokio remains the default. The [detailed experiment](rust-backend-scaling.md#local-file-io-and-io_uring-evaluation) holds budgets/durability constant, checks cancellation and fallback, and requires service-level measurements. No I/O implementation, benchmark or speedup is claimed, and the earlier Luna agents did not review this later addition.

## OSS demand and adoption follow-up

The user requested Luna research into whether a completed Deltrove could become a popular open-source project. Three GPT-6 Luna agents produced [demand](research/oss-demand.md), [landscape](research/oss-landscape.md) and [adoption](research/oss-adoption.md) memos. The primary agent reviewed the memos and key primary sources, then wrote the [opportunity assessment](oss-opportunity-assessment.md).

The finding is a plausible focused infrastructure opportunity with unproven migration demand and broad popularity. GitHub/Anthropic reports support the problem category; Git-compatible tools and incumbent improvements are substantial alternatives. Two direct Scion-inspired implementations already exist, including another OpenScion. Their public scope and licenses were checked, but implementations and benchmark claims were not independently validated.

Review corrected CarterT27/scion's storage description to CockroachDB/S3, distinguished source-available GitButler from OSS comparators, reconciled differing cached popularity snapshots and separated stars from usage. A proposed Git-backed API pilot remains an experiment under existing feasibility gates; it does not change the native architecture or select a production foundation. Adoption advice was aligned with the existing offline contract and bounded workflow-engine evaluation. Proposed interview/pilot thresholds are decision hypotheses, not market estimates. No outreach, customer interviews, deployment trials or Deltrove benchmarks have run.

All three agents reviewed the synthesized opportunity assessment. Refinements clarified attention-versus-usage and high-volume-workload-versus-general-demand evidence; no material contradiction remained. This review is not an independent market validation.

## Product rename

The project name changed from Open Scion to Deltrove after the OSS landscape review identified an independent project using OpenScion. This is a product naming and discoverability decision, not a change to the technical direction.

## Distributed metadata and shared-cache evaluation follow-up

On 17 October 2026 the user requested Luna agents to add CockroachDB/TiDB and Dragonfly/shared-cache evaluations to the research, roadmap and agent-ready tracker. Three GPT-6 Luna agents drafted [distributed metadata research](research/distributed-metadata.md), the [metadata evaluation](metadata-evaluation-plan.md), [shared-cache research](research/shared-cache.md), the [cache evaluation](cache-evaluation-plan.md), and [A14–A18 task cards](scaling-evaluation-issues.md). The primary agent reviewed the documents and upstream isolation/license sources, integrated the architecture summaries and reviewed the issue payloads. A metadata agent independently reviewed the cache protocol and cards. Follow-up agent turns reached their usage limit, so the primary agent completed the remaining corrections and integration.

| Review finding | Disposition |
| --- | --- |
| Distributed SQL was only a broad future storage suggestion | Added thin F3/F5 compatibility/isolation investigation, conditional S6 capacity comparison and separate selected-engine migration/recovery acceptance. PostgreSQL remains the MVP authority; no production multi-database support is promised. |
| TiDB Snapshot Isolation permits write skew | Require a reviewed invariant-preserving protocol and adversarial negative controls before admitting it to an authoritative-metadata capacity comparison. MySQL protocol/dialect is a separate adapter. |
| PostgreSQL wire compatibility is not behavioral compatibility | CockroachDB must qualify SQLx, exact SQL/constraints/migrations, retry and unknown-outcome handling, job claims, fences and recovery. Current licensing and source availability are recorded, not assumed from older OSS descriptions. |
| A shared cache can appear faster simply by adding hardware or replacing the wrong baseline | Compare Moka plus Valkey/Dragonfly against Moka alone; account for aggregate server/client/database/object/network resources. Label extra-resource and cost-normalized arms separately. Large-object NVMe caching remains a separate path. |
| An external oracle cannot atomically observe a SQL commit | Use durable client acknowledgment evidence plus independent post-fault authoritative outcome/object read-back. Instrumentation supplements reconciliation rather than manufacturing atomic commit evidence. |
| Cached checksums or immutable input keys do not authenticate derived output | Test forged payload plus checksum/input mapping. Require an independently trusted canonical digest/mapping or application authenticator; otherwise recompute before trusted use. Authorization, exact quotas, fencing and durable outcomes remain outside the cache. |
| Cache availability versus safe degraded capacity | Cache outage may reduce capacity and cause bounded retryable rejection; it cannot lose accepted domain state, authorize stale mutations or unleash unbounded origin fallback. |
| Planning/card drift | Corrected evaluation-plan paths and kept future measurements separate from dated research. A13/F5 consume bounded findings or explicit dated deferral; optional engine/cache adoption does not create unconditional MVP or enterprise release blockers. |

Primary-source checks included [TiDB isolation](https://docs.pingcap.com/tidb/stable/transaction-isolation-levels/), [CockroachDB's September source notice](https://www.cockroachlabs.com/blog/source-code-protection/), [Dragonfly's current license](https://github.com/dragonflydb/dragonfly/blob/main/LICENSE.md) and [Valkey's license](https://github.com/valkey-io/valkey/blob/unstable/COPYING). Source facts and proposed gates remain distinct. No database/cache experiment, migration, install, benchmark or production adoption was executed; document and tracker validation is not implementation acceptance.

The primary agent created SCI-36–SCI-40 (A14–A18) in the existing Deltrove Linear project, set parent/milestone/prerequisite relations, and linked the affected F3/F5/S3/S5/S6/S7/S8/A13 issues without turning conditional review inputs into hard release blockers. The [tracking map](project-tracking.md) and JSON ledger record the exact issue identities.
