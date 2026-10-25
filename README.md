# Deltrove

Research and a proposed technical plan for an open-source, online-first source-control service inspired by [Anthropic's Scion article](https://www.anthropic.com/engineering/redesigning-source-control).

The implementation is at its build-contract skeleton stage. It includes a minimal Rust HTTP service and generated API clients; repository operations, persistence, authentication, CLI and web application are not implemented.

Project-owned work is licensed under [Apache-2.0](LICENSE). Implementation work is tracked in the [Deltrove Linear project](https://linear.app/orth-projects-general/project/deltrove-438e1fe166bf); see the [roadmap-to-issue map](docs/project-tracking.md) and [contribution guide](CONTRIBUTING.md) before taking a task.

## Start here

- [Open-source opportunity assessment](docs/oss-opportunity-assessment.md): demand, direct Scion-inspired competitors, adoption barriers and proposed pilot gates; primary-agent synthesis of three Luna research passes.
- [Agent-ready implementation roadmap](docs/implementation-roadmap.md): ordered foundation, small-team MVP and large-organization packages; dependencies, parallel agent assignments, acceptance tests and release gates.
- [Temporal and Restate test plan](docs/workflow-evaluation-plan.md): three-arm comparison against PostgreSQL workers, fault matrix W01–W18, engine-specific upgrade/recovery tests and adoption evidence.
- [Metadata evaluation plan](docs/metadata-evaluation-plan.md) and [cache evaluation plan](docs/cache-evaluation-plan.md): staged PostgreSQL/CockroachDB/TiDB and Moka/Dragonfly/Valkey comparisons, correctness prerequisites, capacity triggers and recovery evidence.
- [Scaling evaluation task cards](docs/scaling-evaluation-issues.md): bounded agent assignments for the metadata and cache experiments, linked through the roadmap-to-issue map.
- [Technical plan](docs/technical-plan.md): architecture, backend/frontend/CLI stack comparison, native data model, APIs, migration, correctness, scaling and implementation milestones.
- [Rust backend scaling plan](docs/rust-backend-scaling.md): runtime, storage, io_uring evaluation, worker isolation, cache tiers, fleet limits and validation gates for the selected server language.
- [Durability and workflow decision](docs/durability-and-workflows.md): ACID transaction boundaries, acknowledgment/failover guarantees and Temporal/Restate/DBOS assessment.
- [Research review](docs/research-review.md): synthesis of three Luna research passes, independent plan reviews, corrections and unresolved implementation decisions.

The selected server language is **Rust**. The current client proposal remains Go/Cobra, with React/TypeScript/Vite for the web application, PostgreSQL for metadata, immutable filesystem/S3-compatible object storage and local SQLite. The plan starts with a self-hostable MVP and identifies the additional work needed for a large-organization platform. C and C++ are excluded as implementation choices.

Begin with roadmap packages F0/F1, then run workflow, native-format and Git/reuse feasibility tracks in parallel. Temporal and Restate are evaluated before expanding workflow machinery; Restate's BSL-licensed server and MIT SDK are distinguished in the adoption gate. No engine is assumed to make PostgreSQL, objects and CI one ACID transaction.

## Build-contract skeleton

See [the DEL-25 build and contract handoff](docs/foundation/build-contract-skeleton.md) for pinned prerequisites, build commands, API scope and evidence. The root Cargo workspace includes the existing workload fixture tool. The API source of truth is `api/openapi.yaml`; Go and TypeScript outputs are generated from it.

## Research memos

| Topic | Research | Review of the resulting plan |
| --- | --- | --- |
| Existing systems, reuse and Git adoption | [Prior art](docs/research/prior-art.md) | [Prior-art review](docs/research/prior-art-plan-review.md) |
| Backend, languages, storage and consistency | [Backend stack](docs/research/backend-stack.md) | [Backend review](docs/research/backend-plan-review.md) |
| CLI, workspaces and frontend | [Client/frontend stack](docs/research/clients-and-frontend.md) | [Client/frontend review](docs/research/client-plan-review.md) |
| Rust storage and metadata scaling | [Rust storage research](docs/research/rust-storage-scaling.md) | Synthesized in the Rust backend scaling plan |
| Rust runtime and caching | [Rust runtime research](docs/research/rust-runtime-scaling.md) | Synthesized in the Rust backend scaling plan |
| Distributed SQL metadata | [PostgreSQL, CockroachDB and TiDB](docs/research/distributed-metadata.md) | [Metadata evaluation plan](docs/metadata-evaluation-plan.md) |
| Shared caches | [Dragonfly, Valkey and local caches](docs/research/shared-cache.md) | [Cache evaluation plan](docs/cache-evaluation-plan.md) |
| Rust operations and verification | [Rust operations research](docs/research/rust-operations-scaling.md) | Synthesized in the Rust backend scaling plan |
| Durable execution and ACID | [Temporal](docs/research/temporal-durability.md), [DBOS](docs/research/dbos-durability.md), [ACID boundaries](docs/research/acid-workflow-boundaries.md) | Synthesized in the durability and workflow decision |
| OSS demand, competition and adoption | [Demand](docs/research/oss-demand.md), [landscape](docs/research/oss-landscape.md), [adoption](docs/research/oss-adoption.md) | [Opportunity assessment](docs/oss-opportunity-assessment.md) |

Initial research was checked on 16 October 2026; the metadata/cache follow-up was checked on 17 October 2026. Primary sources are linked next to claims. Proposed designs, performance objectives and effort estimates are labeled separately from published facts. No comparative performance experiments have been run.
