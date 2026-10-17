# Adversarial review of the technical plan

Reviewed `docs/technical-plan.md` in full against `docs/research/backend-stack.md` and the primary references cited by the plan. The proposal is careful about calling itself research and explicitly defers destructive GC, offline commit queues, and single-repository distributed storage. No issue below blocks phase 0; several decisions should be settled before format freeze, landing implementation, or enabling deletion.

## Findings

### P1 — GC safety is specified as an invariant, not yet as a runnable protocol

Sections 8 and 12 correctly say blobs precede SQL publication, GC must account for upload/publication, and a grace period alone is unsafe. But step 1 only says “reserve/lookup” an idempotent request; it does not define whether this durable request reservation pins every object while upload is in progress, nor how a sweeper prevents deletion between its final liveness check and a concurrent publish. The phrase “snapshot-aware mark/sweep with upload/publication barriers” leaves the race implementation open. PITR has a related gap: “restored DB must never require bytes absent from the retained store” is the invariant, but no retention scheme ties object versions/roots to the oldest supported database recovery point.

**Concrete fix:** specify an upload lease/reservation state machine before accepting objects, then a transactional handoff to durable changeset/review roots. Define the GC mark generation/snapshot, sweep claim/quarantine, and how publication of a previously marked object blocks or cancels deletion. Record backup/PITR watermarks and retain objects needed by every advertised restore point. Keep deletion disabled until concurrent-publication and restore tests pass, as the plan already proposes. S3's per-key strong consistency does not create a transaction with Postgres; it cannot close this cross-store race ([S3 consistency](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html), [Postgres PITR](https://www.postgresql.org/docs/current/continuous-archiving.html)).

### P1 — Canonical delta-to-tree derivation needs one explicit format contract before IDs ship

Section 7 has a coherent delta-first direction—first-parent file operations define authored changeset identity, while the tree root is derived/versioned separately. The earlier backend memo implied snapshot-first identity by saying a changeset identifies its tree root; I corrected that memo to match the plan. Remaining format decisions include exact operation schema and path-byte ordering/uniqueness, canonical representation of modes/types and deletes, precisely which fields enter the changeset digest, and how derivation versions preserve or migrate semantics. The directory page/sharding format is also expressly open. “Restricted deterministic CBOR” is a useful encoding constraint, not a complete canonical VCS schema. If derivation semantics change, versioned indexes alone do not guarantee that the same changeset continues to denote the same derived snapshot.

**Concrete fix:** before durable IDs, publish a versioned delta-first schema defining domain separators, all hashed changeset fields, path-byte sorting (without Unicode normalization), duplicate-path rejection, operation semantics, tree-root derivation, and derivation-version compatibility. The changeset ID plus parent and operations should deterministically imply the tree; decide whether clients submit an expected root for validation or the server returns it after derivation. Include golden vectors for non-UTF-8 paths, deletion, modes, empty trees, and merges. RFC 8949 supports deterministic CBOR encoding, while correctly leaving application schema and semantics to the application ([RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html)). This should be an ADR/format gate, not an excuse to prematurely freeze the page layout.

### P1 — Schedule is a low-confidence estimate for a very broad pilot scope

The 16–22 week estimate covers a custom immutable VCS, Git fidelity/export, crash-safe sparse workspace application on two OSes, review/CI/landing, security, recovery, and operational hardening. The assumptions include 2–4 experienced engineers and appropriate VCS expertise, but the memo has no delivery evidence or prototype result supporting the range. A “constrained pilot” label helps, yet the listed deliverables are still substantial and phase 0 is the only explicit stop gate.

**Concrete fix:** label the total as a low-confidence planning envelope; make each phase end in a demonstrable artifact and re-estimate after phases 0 and 1. Separate minimum pilot scope from desirable pilot completeness, and keep Git export fidelity, two-OS workspace hardening, custom merge handling, and HA production readiness as independently gated work. Do not communicate 16–22 weeks as a forecast until the native tree/write spike and Git migration sample have measured results.

### P2 — Landing's final transaction needs a concrete invariant and lock/version recipe

The plan correctly says ref CAS alone does not validate approvals and asks for target ID plus generation, policy/check versions, and transactionally checked generations or locks. That is directionally right, but leaves the correctness proof to implementation. In particular, define which mutable rows are read/locked in the final SQL transaction, how revocation races with landing, and what retries occur after serialization/deadlock failures. The planned output candidate identity should bind the exact target, patch revision, merge result, and required-check set.

**Concrete fix:** specify a lock order or generation-row protocol for target ref, review revision, policy, approvals, and check set; atomically verify those versions and update the ref plus landing/audit/outbox rows. State that any changed generation makes the candidate stale. Add race tests for permission revocation, policy/check changes, and same-ref CAS. Postgres documents row-lock behavior and serialization failures/retries; the application still has to establish its invariant ([row locks](https://www.postgresql.org/docs/17/explicit-locking.html), [isolation](https://www.postgresql.org/docs/current/transaction-iso.html)).

### P2 — Upload protocol details are needed before freezing the API or storage contract

The plan defers chunked blobs pending measurements, which is consistent with a small MVP. It still promises resumable uploads while leaving session identity, lease/expiry, concurrent retry behavior, raw versus stored/compressed byte accounting, and cleanup after abandoned uploads unspecified. Those details affect idempotency, quota enforcement, hash verification, and the GC race described above.

**Concrete fix:** choose a simple single-object upload protocol for MVP, including resumability (or explicitly defer it), idempotency, content-length/hash validation, maximum sizes, expiry, and cleanup/pinning rules. Specify whether compression is transparent storage encoding or content-identity input; keep content digest over raw bytes. Introduce chunk manifests only behind measured need and a separately versioned format.

### P2 — Horizontal plan is honest, but scale gates should avoid implying repo sharding solves hot-repo capacity

Sections 14 and 15 accurately distinguish concurrent Postgres transactions from one primary's finite aggregate capacity and acknowledge that repository sharding cannot split one hot monorepo. The proposed next step—“benchmark graph/index partitioning and distributed metadata transactions”—is a research milestone, not yet an evolution design. A distributed store would not remove per-ref serialization or make the landing queue parallel by itself.

**Concrete fix:** preserve that qualification and attach the scale decision to measured per-repo transaction/WAL/lock/index bottlenecks. Define a single-repository workload threshold and an investigation scope (which metadata can partition independently; what remains linearized) before selecting FoundationDB/TiKV. No need to add distributed storage to the MVP.

## Checks that passed review

- The plan no longer implies that all writes on one Postgres primary serialize. It correctly describes independent concurrent transactions and a same-ref coordination point.
- It does not require offline queued commits in the MVP; offline local edits/status are separated from accepted server changesets.
- The plan's changeset model is delta-first; the backend research memo has been corrected to keep derived tree roots outside changeset identity and to leave chunk manifests conditional on measurements.
- The first-parent merge delta rule, arbitrary imported parent preservation, and separation of changeset acceptance from ref movement are mutually coherent.
- Language comparisons are framed as fit judgments, not comparative benchmark findings. The cited official material supports Go modules/concurrency, Rust ownership/Cargo, .NET runtime/tooling, Java virtual threads, and Swift server/concurrency capabilities. Zig's versioned docs support its low-level allocator/build model; Zig's relative service ecosystem suitability remains judgment and should stay qualified.
- The plan treats derived indexes as rebuildable, requires reader readiness states, defers GC deletion, and calls for concurrent publication tests. Those are strong safeguards; the GC protocol detail above is what remains missing.

## Optional follow-ups

- Keep the stated constraints for tree derivation and make a wide-directory benchmark an exit gate before fixing persistent paging layout; this avoids committing to an overbuilt tree structure prematurely.
- The Rust/Go choice should follow actual contributor skills after phase 0, as the plan says. Do not optimize language based on hypothetical service throughput before database/object I/O measurements.
- The article-derived architecture description should remain separate from proposed details. Its published account does not validate this plan's CBOR, Postgres, S3, hash, cache, or timing choices.
