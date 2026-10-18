# Distributed SQL and cache evaluation cards

These five planned cards, now created in Linear, extend the existing roadmap without changing its plan IDs or milestones. They are research and qualification work: none adds multi-database production support to the MVP, and no evaluation result automatically authorizes an implementation or production switch. Full agent-ready descriptions are in [scaling-evaluation-issues.json](scaling-evaluation-issues.json); the roadmap context is in the [implementation roadmap](implementation-roadmap.md).

| Card | Parent | Prerequisites | Purpose |
| --- | --- | --- | --- |
| [A14 — Common metadata/cache evaluation contract](scaling-evaluation-issues.json) | F3 | F1 | Extend F0's existing fixture IDs and define reproducible workloads, baselines, metrics, resource budgets, and shared fault oracles. |
| [A15 — Distributed SQL compatibility/isolation spike](scaling-evaluation-issues.json) | F3 | A14 | Bound the semantic gate for candidate transaction/isolation behavior; document unsupported behavior and a defer/continue recommendation. |
| [A16 — Shared-cache qualification](scaling-evaluation-issues.json) | S3 | S2, A14 | Compare Moka plus Dragonfly/Valkey against Moka alone with equal aggregate fleet resources, and a disk baseline for larger objects; decide adopt/defer/no-cache. |
| [A17 — Distributed SQL capacity comparison](scaling-evaluation-issues.json) | S6 | S1, S2, A15 | If a measured S6 trigger or explicit multi-region requirement exists, compare tuned PostgreSQL/HA and repository-sharded PostgreSQL with CockroachDB; test TiDB only if A15's semantic gate passes. |
| [A18 — Conditional selected-engine migration qualification](scaling-evaluation-issues.json) | S6 | A17 | Only for an alternate engine selected by A17: rehearse migration, restore, rollback boundaries, fencing, and recovery epochs. Record a skip if PostgreSQL remains selected. |

The supporting research destinations are [metadata evaluation plan](metadata-evaluation-plan.md), [cache evaluation plan](cache-evaluation-plan.md), [distributed metadata findings](research/distributed-metadata.md), [shared-cache findings](research/shared-cache.md), and a proposed common evaluation contract at `docs/evaluations/scaling-evaluation-contract.md` (future output). Each card requires raw, reproducible evidence, dependency/license pins, negative controls, and an explicit handoff. Existing F0 fixture IDs remain the common corpus; missing workloads are added as named extensions rather than silently substituting data.

## Linear issues

- [SCI-36 — A14](https://linear.app/orth-projects-general/issue/SCI-36/a14-common-metadatacache-evaluation-contract)
- [SCI-37 — A15](https://linear.app/orth-projects-general/issue/SCI-37/a15-distributed-sql-compatibility-and-isolation-spike)
- [SCI-38 — A16](https://linear.app/orth-projects-general/issue/SCI-38/a16-shared-cache-qualification-shared-process-local-and-disk-baselines)
- [SCI-39 — A17](https://linear.app/orth-projects-general/issue/SCI-39/a17-conditional-distributed-sql-capacity-comparison)
- [SCI-40 — A18](https://linear.app/orth-projects-general/issue/SCI-40/a18-conditional-selected-engine-migration-restore-rollback-and-epoch)

See [project tracking](project-tracking.md) for milestone and dependency mappings. All experiments remain unrun; issue creation does not accept their gates.
