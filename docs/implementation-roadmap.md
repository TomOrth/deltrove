# Open Scion: agent-ready implementation roadmap

**Date:** 16 October 2026. **Status:** proposed work, not implemented or tested.\
**Objective:** deliver a useful self-hosted source-control service for a small team, then qualify a platform for a large organization. The backend, frontend and developer CLI are required throughout. Temporal **and Restate** are early experiments, not deferred enterprise features.

This document governs implementation order. The [technical plan](technical-plan.md) supplies architecture and semantics, the [Rust scaling plan](rust-backend-scaling.md) supplies resource budgets, and the [workflow evaluation plan](workflow-evaluation-plan.md) supplies executable experiment requirements W01–W18. If feasibility selects an existing engine instead of a new native format, revise the affected packages before implementation; do not quietly build both products.

## 1. Scope and architecture to carry through every stage

| Component | Working choice | First delivery |
| --- | --- | --- |
| Server and compute workers | Rust, Tokio/Axum/Tower, SQLx; bounded CPU pool | One binary/image with combined `serve` and separate `api`/`worker` modes. |
| Authoritative metadata | PostgreSQL, explicit SQL/migrations | Short `SERIALIZABLE` critical mutations, constraints, generations, idempotency ledger and atomic outbox. |
| Repository bytes | Versioned immutable content-addressed objects | Durable local-filesystem mode; separately qualified S3-compatible adapter. |
| Local file I/O | Ordinary Tokio/bounded blocking I/O | F3 compares experimental Tokio io_uring support and isolated `tokio-uring`; only adopt after measured benefit, fallback and durability/resource tests. |
| CLI/workspace | Proposed Go/Cobra, generated HTTP client, SQLite local metadata | Linux/macOS, explicit sparse directories, safe journaled updates and online commits. Confirm SQLite packaging in F1. |
| Frontend | React/TypeScript/Vite static application | Repository browser, diff, review/check status and landing flow; bundle static assets into the service release. |
| Durable orchestration | Compare PostgreSQL workers, Temporal and Restate | F2 thin lifecycle experiment; M4 real integration and adoption decision. Engines coordinate domain commands; SQL owns domain correctness. |
| Identity and integrations | Existing OIDC provider; scoped CLI/CI credentials | Repo-level authorization, audit, one CI adapter; no custom identity provider. |

C and C++ are excluded as project implementation languages. Established external storage/runtime dependencies retain their own licenses and implementation languages. Go CLI and frontend choices are proposals to freeze in F5, rather than additional server language decisions.

The **minimum private alpha** is one pilot repository, read APIs/browser, sparse edits/online draft commits, one approval rule, one CI adapter, controlled landing and tested backup/restore. The **small-team MVP** adds the declared Git compatibility subset, multi-repository installation, operational packaging, upgrade/recovery procedures and published capacity limits. Neither label implies host-loss HA.

Large-organization readiness adds tested HA, identity/governance, quotas/fairness, enterprise integrations, measured repository/fleet capacity and disaster recovery. Fully virtual workspaces, transparent Git remote compatibility, geographic concurrent writes and Windows native workspaces remain separately approved scope with their own qualification packages.

### Invariants that no package may weaken

1. A published changeset/ref has one atomic SQL outcome and complete verified durable reachable objects. Object storage and PostgreSQL do not share a physical transaction.
2. Commit identity uses the frozen canonical encoding, ordered parents and first-parent deltas; derived indexes/manifests never redefine identity or snapshot meaning.
3. Landing validates the exact candidate, live policy/checks/approvals, target generation, attempt fencing and recovery epoch inside its final short transaction. CI/human waits hold no transaction open.
4. Duplicate delivery and unknown commit outcomes reconcile through stable domain identity. A timed-out or stale worker cannot regain publication authority.
5. Sparse omission is never deletion. Uncommitted user work is never treated as disposable cache. Workspace recovery preserves preimages and edits made during upload/update.
6. Every read path enforces authorization independently of cache hits. One migration stage has one authoritative writer.
7. Resource limits include queued work, retained buffers, streamed response lifetime and fleet-wide DB connections. HTTP cancellation does not imply a running CPU job has stopped.
8. A durability claim names the tested failure model. No asynchronous failover fallback silently inherits a zero-loss claim; restore fences old publishers and assigns a new non-reused recovery epoch.

## 2. How to assign work to AI agents

Use an integrating owner for the roadmap, schema/API decisions and merges. Give implementation agents bounded packages and give a separate reviewer the relevant invariants and evidence, rather than asking one agent to build an entire backend. A human or explicitly delegated maintainer owns adoption, licensing and release decisions. Passing a package gate authorizes the next development task, not an external deployment or a changed product scope.

Split a package into reviewable tasks that each change one coherent concern. Assign one owner per migration, public schema or canonical encoding. Declare owned paths and coordinate shared changes; use isolated branches/worktrees when parallel implementation warrants them. The [GitHub repository and Linear issue map](project-tracking.md) are now set up for this work. Do not start parallel edits to the same contract and resolve them by whichever agent merges last.

Every assignment should include this handoff:

```text
Task/package ID and parent milestone:
Objective and user-visible result:
Dependencies and contract/version inputs:
Owned paths; shared paths requiring coordination:
Invariants and explicit out-of-scope behavior:
Implementation deliverables:
Acceptance tests and fault cases; real services required:
Evidence location and exact commands/configuration:
Review risks, unresolved decisions and rollback/recovery:
Status: proposed / active / blocked / review / accepted:
```

A completed handoff includes the implementation revision, dependency versions, test command/results, dataset and seed, topology, important logs/raw measurements, limitations and outstanding failures. Once version control exists, include the commit hash. Mark unavailable checks **not run**, explain why, and keep the gate open. Agent assertions or agreement are not test evidence. Reviewers must inspect the implementation and failure oracle, not just the author's summary.

Suggested long-lived responsibilities are domain/storage, CLI/workspace, frontend/integration and validation/operations. These are roles, not four assumed full-time people. Agents can fill them sequentially. Keep concurrency bounded by review capacity; protocol stability and feedback matter more than agent count.

## 3. Foundation: resolve expensive choices before expanding the product

| ID | Dependencies | Deliverable | Acceptance evidence / exit gate |
| --- | --- | --- | --- |
| **F0 — Scope and workload contract** | None | Select pilot repos or reproducible synthetic stand-ins; record file counts, bytes, directory width/depth, history, binary/large-file use, agents/users, concurrency and CI waits. Define alpha/MVP feature matrix, failure profiles, target latency/capacity and initial threat model. | Checked-in workload manifest and named acceptance scenarios. All numeric targets labeled hypotheses until measured; explicitly list unsupported Git features and unknowns. |
| **F1 — Build and test spine** | F0 | Minimal Rust workspace/CI; formatting/lints/build; real PostgreSQL migrations and test isolation; initial HTTP/error/idempotency contract; Go/TS client smoke requests; SQLite packaging spike. Build a tiny domain/outbox fixture, independent fake CI service, fault hooks and durable acknowledgment oracle. | Clean Linux/macOS client build and supported server build, reproducible fresh install, contract compatibility checks, real SQL commit/rollback/lost-response fixture. Secrets excluded from logs. No production VCS needed. |
| **F2 — Workflow comparison** | F1 | Implement PG, Temporal and Restate versions of the **same thin** prepare/CI/approval/land lifecycle. Pin server/SDK/images and license inventory. Exercise W01–W09 in the [evaluation plan](workflow-evaluation-plan.md). | Three reports with raw evidence and pass/fail/unsupported/not-run per case; preliminary choice and revisit trigger. No arbitrary exactly-once external-effect claim. Do this before growing a custom workflow framework. |
| **F3 — Native format and storage feasibility** | F1 | Experimental canonical encoding/vectors, first-parent delta identity, ordered parents, paged incremental trees, object verification and durable write/publication boundary. Measure narrow edits and huge directories. Compare ordinary local I/O, Tokio's experimental io_uring support and isolated `tokio-uring` per the [I/O evaluation](rust-backend-scaling.md#local-file-io-and-io_uring-evaluation). | Golden/negative vectors and fault fixtures; SQL publication follows durable bytes; unchanged subtrees reused. I/O report confirms exercised code paths, equal budgets/durability, cancellation/fallback and end-to-end measurements. Unsupported experimental arms do not force adoption. Format remains experimental until F5. |
| **F4 — Git and reuse baseline** | F1 | Tuned Git partial/sparse baseline against F0 workloads, simple read API prototype and time-boxed Mononoke build/import/API/operations/license evaluation. Inventory migration features. | Reproducible commands/results and adoption gaps. Record build failures/unsupported cases honestly; compare cold/warm cache and equivalent operations, not a full clone versus a sparse read. |
| **F5 — Architecture acceptance** | F2, F3, F4 | ADRs for native versus reuse, canonical schema, API, client platforms/SQLite driver, storage adapter/local I/O, minimum durability, workflow shortlist and licensing/distribution. Set prototype performance budgets and release criteria. | Review format/API vectors, migration feasibility, local I/O comparison and all three workflow reports. Ordinary Tokio remains default unless evidence justifies an optional optimization. Proceed native only with a concrete useful benefit and viable support burden. Replan if Git/reuse wins; do not preserve obsolete packages by inertia. |

F2/F3/F4 can run in parallel after F1 if their fixture/schema ownership is explicit. F2's fixture intentionally avoids depending on the finished native engine. Compare all three workflow arms early; F2 is not production certification or permission to omit M4 recovery/versioning tests.

Temporal is the prior leading orchestration candidate, but baseline or Restate can win on evidence. Restate's current server is BSL 1.1 and its Rust SDK is MIT; **evaluation is required, mandatory adoption is not predetermined**. Capture exact versions and intended hosting/distribution before deciding. An optional Restate integration must not silently become a requirement for an installation presented as entirely open source. See the [license and evaluation gate](workflow-evaluation-plan.md#2-restate-facts-and-licensing-gate).

## 4. Small-team MVP: implement complete vertical slices

### M1 — Durable storage and authoritative mutations

**Depends on F5.** Implement the chosen object adapters, repository/identity schema, upload sessions and retention pins, membership, operation outcomes and outbox. Include repo authorization from the first endpoints; authorization is not a late hardening task. Ship migrations with explicit compatibility/rollback limits and a minimal storage inspection/fsck command.

Critical writes use short serializable transactions with whole-transaction retry, constraints and expected generations. Verify objects before publication; distinguish upload complete, changeset published and derived snapshot ready. Bound upload sessions, bytes, object size, concurrency and DB pools.

If F5 selects an io_uring optimization, qualify the real adapter and Linux deployment profile here. Preserve a documented ordinary-I/O installation path, bound requests crossing dedicated runtime threads, and retain in-flight buffers/permits until the operation actually finishes. I/O completion never substitutes for required synchronization or object-before-SQL publication.

**Tests/gate:** real PostgreSQL and each supported object adapter; corrupt/truncated/digest-mismatched writes, quota races, duplicate request/different payload, cross-repo membership, lost SQL commit response, crashes between each object/SQL step and permission revocation. Verify the acknowledgment ledger and reachable objects after restart. Qualify local flush/rename/directory persistence and provider completion semantics for the promised failure profile. A process-kill test alone does not prove power-loss durability. Acceptance requires one acknowledged publication with safe retry and no partial visibility.

### M2 — Native repository engine and useful read API

**Depends on M1.** Implement changesets, graph/ancestry/merge-base, incremental derived trees, readiness dependencies, file/tree/history/diff endpoints and bounded derivation workers. Draft creation takes an explicit workspace parent, without requiring the current main tip. Start with documented merge/content restrictions; return explicit unsupported/conflict results.

**Tests/gate:** canonical cross-language vectors, randomized graph/tree operations against a small reference model, delete/mode/symlink/byte-path fixtures, multiple merge bases, malformed object rejection and derived-index rebuild equivalence. Exercise concurrent derivation, stale lease publication, cold cache and a narrow edit in a wide tree. Observe actual CPU/retained-memory bounds under cancelled requests. Deliver a CLI query and basic web browser using the same API as an early integration proof.

### M3 — Safe developer workspace and browser

**Depends on M2; CLI and web tracks can run in parallel against the accepted API.**

| Track | Deliverable | Required acceptance tests |
| --- | --- | --- |
| CLI/workspace | Login/token use; sparse clone/materialize; add/remove; status/diff; online commit; scope updates; sync; local cache/doctor; `--json`, deadlines, stable exit codes. | Kill/restart at every journal boundary; disk-full/permission failure; overlapping dirty updates; sparse exclusion versus deletion; symlink/path traversal; supported filename semantics; edits during upload; lost commit response; offline cached status/diff. Watcher loss triggers a scan. Never discard dirty data to pass a test. |
| Frontend | Authenticated repository/tree/file/history browser; bounded diff navigation; operation/readiness states; first review shell. | Browser end-to-end reads against the real API; hostile source/filename/comment rendering; session/CSRF checks; keyboard/accessibility coverage; huge diff pagination and stale states. No unbounded whole-repo graph/diff render. |
| Contract integration | Generated Go/TS clients and shared format/API examples. | Both clients agree on pagination, errors, operation identity, stale generation and pending-versus-ready behavior. Test real network loss, not only mocked request success. |

**Gate:** a developer makes a sparse edit, submits a draft from an older workspace parent and sees the exact accepted snapshot in the browser; edits made meanwhile remain dirty. Another developer can read it without cloning the repository.

### M4 — Reviews, CI, landing and workflow adoption

**Depends on M2, M3 and F2.** Build immutable review revisions, comments/one approval rule, check records and one CI adapter. Implement candidate preparation, declared two-parent merge/conflict semantics, explicit resolution/abort, protected target transitions and audit. Keep complex rename heuristics, stacked automation and broad policy languages outside alpha.

Integrate the F2-selected orchestration approach with **real** candidate/domain commands. Preserve atomic outbox, input/candidate reconciliation, domain outcomes and fenced ownership. If choosing PG, keep a bounded explicit state machine rather than a framework. If choosing an engine, add its supported deployment/versioning/repair procedure; routine object reads and small unrelated jobs need not route through it.

**Tests/gate:** run W01–W12, including two competing landings, write skew, permission/policy/approval/check changes during landing, target advancement, late CI, cancellation after landing, duplicate starts and history-retention expiry. Replay/route N and N+1 code and rehearse rollback. Verify exact tested tree equals landed tree. Unsupported CI deduplication produces explicit reconciliation, not hidden duplicate certainty. Record the final workflow adoption ADR with code/operations burden and licensing. Thin F2 successes alone cannot pass M4.

**Private alpha checkpoint:** M1–M4 plus the M7 minimal install/backup/restore subset must pass on one pilot repo. Keep access bounded and state its support limits. This checkpoint does not bypass M5–M8 for the broader MVP.

### M5 — Git migration and compatibility

**Depends on M2 and F4; can proceed alongside M3/M4.** Build resumable shadow import, original-object retention, declared ref/tag/hash-format support and one-way export. Publish a per-feature matrix for LFS, submodules, attributes/filters, hooks, unusual paths, signed/nested tags and binaries; unsupported features must be detected in preflight.

**Tests/gate:** compare file bytes/modes/history/ref mapping against named Git fixtures and a pilot corpus. Exact unchanged imported export requires retained original blobs/trees/commits/tags and identifiers; new native changesets get new Git identity. Rehearse resumable import, final delta freeze, one-writer cutover, credential changes and recovery. Once native writes begin, rollback requires a validated export/forward-recovery procedure; it is not simply repointing to stale Git. Never allow an undocumented bidirectional writer arrangement.

### M6 — Finish the team workflow and integration boundaries

**Depends on M4; migration UI depends on M5.** Complete repo administration, multi-repository permissions, check/review UI, CLI wait/cancel/resolve/abort and user-facing recovery messages. Add scoped CI credentials, audit querying, rate/byte/job quotas and event reconnection. Alpha auth/audit already exists; this package broadens and verifies it.

**Tests/gate:** two-user/two-repo isolation across file/tree/history/diff/events/cache, role revocation, token expiry, unauthorized guessed digests and review/comment races. End-to-end developer/CI/reviewer/admin journeys through CLI plus browser; status/error behavior remains consistent under retries and stale screens. Verify users understand pending, failed and uncertain outcomes. Limit browser payloads and ensure secret/source logs stay within the declared policy.

### M7 — Install, upgrade, recover and operate

**Can start with M1; complete gate depends on M4, M5 and M6.** Package an OCI image/static web assets, Compose installation, signed client artifacts, configuration validation, metrics/logs and runbooks. Produce the minimal backup/restore subset before private alpha. Finish consistency checking, upgrade/restore, client workspace repair, quota management and maintenance GC dry-run.

**Tests/gate:** fresh install on documented prerequisites; persistent-volume restart; interrupted migration and supported N-to-N+1 upgrade; coordinated DB/object/selected-engine backup and restore into a clean environment. Use a new recovery epoch and fenced old credentials; reconcile engine/domain state before reopening writes. Retain backup dependencies and pending-operation objects. GC remains disabled/dry-run until reachability, backup-window and publisher-fencing proofs pass; any deletion feature is a separate reviewed acceptance task. Publish the exact single-node durability/failure scope and operator repair procedure.

### M8 — Small-team release qualification

**Depends on M1–M7.** Exercise realistic multi-user and AI-agent workloads; define a candidate release configuration and measured capacity envelope. Run all W01–W15 for the selected workflow path, plus the storage/workspace/security regressions. Test direct PostgreSQL/object integration, not only an in-memory emulator.

**Release evidence:** zero invariant failures; ledger-backed recovery; bounded CPU/memory/queues/connections; clean install/upgrade/restore; supported Git migration; completed Linux/macOS CLI and browser journeys; pinned dependency/license inventory; raw latency/throughput distributions and reproducible scripts. Define pilot load, duration, fault schedule and acceptance thresholds before each qualification run. Record failures and rerun affected gates after fixes. Publish limitations, not extrapolated enterprise capacity.

For any selected io_uring path, repeat the [I/O mixed-load, cancellation, shutdown, unsupported-deployment and durability cases](rust-backend-scaling.md#local-file-io-and-io_uring-evaluation) on the release configuration. Passing a standalone file-I/O microbenchmark is insufficient.

Small-team default remains a simple single-node installation. A production pilot requiring host-loss survival must bring **S1** forward; it cannot inherit an HA claim from M8. Re-estimate after F5 and M4 using observed implementation/review/recovery effort rather than multiplying assumed AI-agent speed.

## 5. Large-organization program: scale specific bottlenecks

Start this program after M8 unless a stated deployment requirement pulls a package forward. Each package qualifies a named topology and workload; none grants universal enterprise scale. S1 and S4 are admission gates for deployments requiring HA and enterprise identity respectively. S2/S3/S5/S6 may advance independently once their prerequisites are met.

| ID | Dependencies | Implementation scope | Acceptance and decision gate |
| --- | --- | --- | --- |
| **S1 — HA with explicit durability** | M8; may be advanced for pilot requirements | Synchronous PostgreSQL replication/promotion policy, fenced primary, replicated object durability and selected workflow-engine HA. Separate node/volume failure domains and capture acknowledgment policy. | W16 plus app/object/engine failover with in-flight commits and an independent acknowledgment ledger. Lose a node, partition and unavailable synchronous replica; unsafe acknowledgments stop, old publishers stay fenced. No-loss claim only for declared covered failures with a surviving acknowledged durable copy. Same-host containers are not host-loss evidence. |
| **S2 — Independent API and worker scale** | M8 | Split process classes, tune read/write/derive/import/CI admission, autoscaling, per-tenant quotas and fleet connection budgets including rollout surge. | Mixed workload and overloaded downstreams; prove bounded queued/running CPU, stream-held permits, retained bytes and SQL pools. Slow consumers/cancelled clients cannot starve lightweight reads. Add replicas only when DB/object service has measured headroom. |
| **S3 — Shared/regional cache tiers** | S2 | Node/regional read caches, bounded fill concurrency, single-flight fetch, eviction and origin fallback. Re-evaluate io_uring for disk-cache reads/fills against actual device and workload. Keep auth and canonical state authoritative. | Cold-start/cache-loss/origin-slowdown tests, isolation, revocation and cache poisoning/integrity rejection. Compare transfer reduction/cost and local I/O CPU/tail latency with cold/warm workloads; retain in-flight resource bounds. Previously downloaded bytes cannot be recalled by revocation. |
| **S4 — Enterprise identity and governance** | M8 | SSO/group lifecycle, service identities, tenant/repo administration, audit export/retention and policy integrations. Path ACLs only if requirements justify comprehensive enforcement. | Revocation across tree/history/diff/blame/search/events/cache and membership changes during mutation. Export/backup/admin access reviewed. Path ACL release is blocked until all derived-data leak paths are tested; sparse checkout is not access control. |
| **S5 — Many-repository placement** | S1, S2 | Repository-to-shard directory, placement policy, per-shard budgets, controlled migration and balancing. Keep each repository's authority explicit. | Move a busy repo while reads/writes/retries continue; fence old owner, verify no duplicate publication or lost accepted outcome, rehearse rollback. Test skewed tenants. Repository sharding distributes many repositories; it does not divide one hot repository. |
| **S6 — Hot monorepo and workflow capacity** | S1, S2; independent of S5 | Profile one large/hot repository: graph/tree/history/index growth, derivation, target-ref serialization, CI fan-out and workflow waits/history. Optimize measured indexes/algorithms/batching first. | W17/W18 plus long-running history/retention/version tests and mixed workload fairness. Separate DB/ref, engine and object bottlenecks. If one-primary metadata capacity fails the target, issue a new distributed-metadata transaction/operations ADR and prototype; no automatic database swap. Engine ownership migration must fence prior controllers. |
| **S7 — Disaster recovery and geographic placement** | S1, S4; include S5/S6 topology if adopted | Independent backup custody, retention/deletion policy, restore/PITR drill, regional read/caching placement and explicitly priced RPO/RTO. Geographic concurrent writes require a separate authority redesign. | Site-loss exercise with DB/object/engine recovery points deliberately misaligned; unique new epoch, old credentials/replicas fenced, retained objects complete and stale callbacks rejected. Measure acknowledged loss and recovery time. Async remote replication cannot silently claim regional RPO zero. |
| **S8 — Enterprise release qualification** | S1, S2, S4, S7 and every advertised optional scale capability | Document supported topologies/capacity, dashboards/on-call runbooks, staged rollout/rollback, maintenance windows, integration contracts and support ownership. | Repeat selected workflow W01–W18 in applicable topology, mixed-load/soak/fault/security suites and clean recovery by an operator who did not build the feature. Publish measured limits and unsupported scenarios; cache/sharding/path ACL claims require their own passed packages. |

S6 can be pulled into F3 feasibility if the first customer requires a huge single monorepo. A small-team prototype against a tiny corpus is insufficient evidence that PostgreSQL, the tree model or a serialized landing target meets that requirement.

## 6. Test organization and evidence

These are test categories to implement, not directories or passing suites that currently exist.

| Layer | Purpose | Required environment / cadence |
| --- | --- | --- |
| Format and algorithms | Canonical vectors, reference-model graph/tree behavior, fuzz malformed objects and bounded parsing. | Fast deterministic checks per relevant change; longer seeded fuzz campaigns before releases. Independent expected results, not tests that restate the implementation. |
| Domain integration | Real SQL isolation, constraints, lost outcomes, outbox/fencing and real object adapter publication. | PostgreSQL plus the supported adapters; affected mutation tests per change. No SQLite substitute for PG semantics. |
| Cross-component contract | Rust API versus generated Go/TS clients, identity/errors/readiness and upgrade compatibility. | Service + generated clients in CI; reject unexplained incompatible changes. |
| Workspace and UI | Journal recovery, dirty-file preservation, OS semantics, browser journeys and hostile rendering. | Linux/macOS native filesystems; supported browsers; release-specific platform matrix. |
| Workflow recovery | W01–W18 and engine-specific replay/version/routing/retention cases. | Real pinned Temporal and Restate at F2, selected production arm thereafter; fixture and real integration results separate. |
| Performance and operations | Mixed-load distributions, admission/resource bounds, backup/restore/upgrade/HA. | Declared CPU/RAM/storage/topology and workload; raw metrics and fault schedule retained. Developer laptop results label only that configuration. |

Maintain a requirement-to-test map with IDs for each invariant, API flow, platform and W-case. Critical mutation tests run with genuine concurrent writers and an independent durable outcome oracle. Functional correctness is a prerequisite to throughput comparisons. Fault injection covers the gap between accepting an external effect and recording it, not just orderly shutdown.

For every performance run define load offered/accepted/rejected, dataset/seed, cold/warm state, measured interval, faults and drain time. Capture p50/p95/p99, logical operations/s, readiness/queue age, retries/duplicate attempts, CPU/RSS, disk/WAL, object traffic and pool/lock usage. Set capacity targets in F0 and refine them with measurements; do not convert an example concurrency limit into a supported user count.

## 7. Execution waves and first assignments

1. **Foundation spine:** F0 then F1. Establish fixtures and review contracts before dispatching parallel code work.
2. **Parallel feasibility:** F2 workflow comparison, F3 native/storage spike and F4 Git/reuse baseline. Integrator consolidates in F5; reopen scope if native value is weak.
3. **Storage/domain spine:** M1 then M2, while operations work starts the M7 install/backup scaffolding. Keep production and experiment namespaces separate.
4. **User-facing parallel work:** M3 CLI and web tracks; M5 migration can proceed on stable M2 contracts. Integrate frequently against real services.
5. **Complete alpha:** M4 selected workflow integration plus the M7 minimum recovery subset. Perform the one-repo end-to-end alpha gate.
6. **MVP qualification:** M6 and remaining M7; then M8. Separate bug fixes from optional feature expansion until the release evidence is complete.
7. **Enterprise tracks:** S1/S2/S4, then caches/many-repo/hot-repo work as measurements demand; S7 recovery and S8 qualification before enterprise claims.

The first agent assignments should be **F0 workload/acceptance specification** and then **F1 build/harness setup**, not a broad instruction to implement the whole VCS. At each accepted milestone the integrator updates the task/dependency ledger and estimates remaining work from observed evidence. No calendar promise is made by this roadmap; the previous optimistic human-team phase estimate is superseded by these milestone gates and the added three-arm workflow evaluation.

## 8. Initial agent task cards

All cards below are **proposed**. Output locations are suggested future paths, not files or services that have already been created. Each card uses the handoff in section 2 and requires integrating-owner acceptance. F0 is accepted after A01/A02; F1 after A03–A07. These cards decompose existing packages rather than adding new prerequisites.

| Card | Depends on | Agent assignment and bounded output | Completion evidence |
| --- | --- | --- | --- |
| **A01 — Workload fixtures** | None | Under F0, write a workload manifest and generators for tiny correctness graphs, wide/deep trees, binaries, many repos, a hot repo and long CI waits. Do not invent a real customer's measurements. | Fixed seeds, fixture sizes/hashes, generation commands and a list separating synthetic assumptions from observed pilot data. A reviewer can regenerate the same corpus. |
| **A02 — Acceptance contract** | A01 draft; reconcile final A01 before F0 acceptance | Under F0, write alpha/MVP/enterprise acceptance requirements, supported-platform and Git-feature matrices, durability failure profiles, threat model and numerical benchmark hypotheses. | Each requirement has an ID and named test/oracle; unsupported and unknown features are explicit. Contract identifies what must survive process, host and site loss in each release. |
| **A03 — Build and contract skeleton** | F0 | Under F1, propose the minimal directory/crate layout, pin toolchains, build a health/readiness endpoint and initial operation/error schema, and wire build/lint/client-generation checks. No VCS algorithms or orchestrator integration. | Fresh build from documented prerequisites; Rust service and generated Go/TS clients exchange a real request; schema compatibility check detects a deliberately incompatible fixture. |
| **A04 — Domain transaction fixture** | A03 | Under F1, implement tiny fixture commands/outcomes, serializable SQL retries, generations and atomic outbox. Use placeholder immutable candidate bytes, not the production format. | Two concurrent writers, same-key mismatch and post-commit lost-response tests against real PostgreSQL; durable oracle observes exactly the authorized logical transitions. |
| **A05 — Independent CI simulator and fault runner** | A03 | Under F1, build a separate durable fake CI service with stable keys, accept-then-drop response, delayed/duplicate callbacks, and provider-dedup enabled/disabled modes. Define controllable fault points. | Simulator restart preserves requests; runner proves it can kill/pause/restart the target and distinguish physical CI executions, callbacks and logical outcomes. Avoid fixed sleeps as correctness synchronization. |
| **A06 — Client portability spike** | A03 | Under F1, validate Go/Cobra/generated client, SQLite driver packaging and supported Linux/macOS architectures. Keep it a login/query/local-journal toy, not the workspace engine. | Reproducible platform builds and a minimal interrupted-local-write recovery test. Document driver/native dependency constraints before promising release artifacts. |
| **A07 — Harness integration review** | A04, A05, A06 | Finish F1 by connecting fixture commands, simulator, fault controls and acknowledgment ledger. Freeze the engine experiment's input/measurement contract. | One seeded happy run and deliberate lost-response/stale-attempt failures are detected by the oracle. The reviewer verifies the oracle fails when a known invariant violation is injected. |
| **A08 — PG lifecycle arm** | F1 | Under F2, implement only prepare/check/approval/land lifecycle using bounded jobs, durable waits, retry deadlines and fenced claims. | W01–W09 report, pinned DB/configuration, raw outcome/attempt evidence and operational repair notes. |
| **A09 — Temporal lifecycle arm** | F1 | Under F2, implement the same lifecycle with real pinned Temporal/Rust workers and shared domain commands. Own Temporal-specific experiment files; do not modify shared semantics unilaterally. | W01–W09 report; restart/callback/start-conflict evidence, Activity side-effect gap and deterministic-workflow constraints documented. |
| **A10 — Restate lifecycle arm** | F1 | Under F2, implement the same lifecycle with real pinned Restate/Rust workers and shared domain commands. Capture server/SDK licensing and actual supported callback/journal APIs. | W01–W09 report; journal/external-effect gap and invocation/deployment identity evidence, plus distribution/hosting implications for the pinned license. |
| **A11 — Native feasibility arm** | F1 | Execute F3 as separate format/vector, incremental-tree, object-durability and local-I/O comparison tasks. Compare ordinary Tokio, experimental Tokio io_uring and isolated `tokio-uring`; coordinate shared fixtures with other agents. | Canonical/negative vectors, wide-directory measurements, publication faults and the [I/O report](rust-backend-scaling.md#local-file-io-and-io_uring-evaluation), with real code-path verification, cancellation/fallback results and explicit unsupported cases. Clear decisions for F5. |
| **A12 — Git/reuse feasibility arm** | F1 | Execute F4 with baseline scripts, migration inventory and time-boxed Mononoke report. Keep benchmark implementation outside production native modules. | Comparable Git/native workload measurements when A11 results arrive, original commands and licensing/API/operations gaps. |
| **A13 — Consolidated decision review** | A08–A12 | Close F2/F3/F4 findings, then execute F5. An independent reviewer examines the reports and common oracle before the integrating owner accepts ADRs. | Evidence-linked decisions, failures/unknowns, selected path and revisit triggers. Produce bounded M1/M2 cards from accepted contracts; do not start them from unresolved experimental APIs. |

A08–A10 may run concurrently with separate engine state after A07. A11/A12 can also run independently within available review/compute capacity. Keep the common domain fixture owned by one integrator and require coordination for any correction affecting comparability. Report experiments as blocked or unsupported when necessary; never replace a real engine test with a mock while marking it passed.
