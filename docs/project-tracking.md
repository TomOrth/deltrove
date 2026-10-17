# Project tracking

The [GitHub repository](https://github.com/TomOrth/deltrove) contains the architecture and implementation plan. The [Deltrove Linear project](https://linear.app/orth-projects-general/project/deltrove-438e1fe166bf) in the OpenScion (SCI) team tracks execution.

Project-owned work is licensed under [Apache-2.0](../LICENSE). The implementation has not started. Issues were seeded in Backlog without assignees or dates; consult Linear for live status rather than inferring it from this map.

## Working agreement

- Repository documents own architecture, scope and acceptance semantics. Linear owns status, assignments and dependencies. Update both deliberately when scope changes; automatic GitHub/Linear synchronization has not been configured.
- Preserve roadmap IDs (F/M/S packages and A task cards) in issue titles. Link pull requests to their Linear issue; use [the contribution guide](../CONTRIBUTING.md) and the roadmap handoff template.
- Issue dependencies represent full-package acceptance prerequisites. Follow roadmap notes for partial early work such as M7 installation/recovery, bringing S1 forward, or optional enterprise capabilities. Decompose those slices before assigning implementation.
- Completed repository/project setup is not acceptance of F0 or another implementation package. Agent reports, planning documents and unrun checks do not satisfy implementation/test gates.

## Milestones

| Milestone | Packages | Exit criteria |
| --- | --- | --- |
| Foundation and feasibility | F0–F5 | F0–F5: workload/acceptance contract, build and fault harness, PG/Temporal/Restate experiments, native format and local I/O evaluation, Git/reuse baseline and accepted architecture decisions. |
| Small-team MVP | M1–M8 | M1–M8: durable storage and domain/read APIs; sparse CLI and web review; CI/landing; Git migration; permissions; install/upgrade/restore; release evidence. Private alpha is M1–M4 plus the minimum M7 recovery subset; HA is separately qualified. |
| Large-organization qualification | S1–S8 | S1–S8: explicit HA/durability profiles, API/worker scale, optional cache tiers and enterprise governance; workload-driven many-repo/hot-monorepo scaling, disaster recovery and enterprise qualification. |

## Roadmap packages

| Plan ID | Linear issue | Work | Acceptance predecessors |
| --- | --- | --- | --- |
| F0 | [SCI-1](https://linear.app/orth-projects-general/issue/SCI-1/f0-scope-and-workload-contract) | Scope and workload contract | None |
| F1 | [SCI-2](https://linear.app/orth-projects-general/issue/SCI-2/f1-build-and-test-spine) | Build and test spine | F0 |
| F2 | [SCI-3](https://linear.app/orth-projects-general/issue/SCI-3/f2-workflow-comparison) | Workflow comparison | F1 |
| F3 | [SCI-4](https://linear.app/orth-projects-general/issue/SCI-4/f3-native-format-and-storage-feasibility) | Native format and storage feasibility | F1 |
| F4 | [SCI-5](https://linear.app/orth-projects-general/issue/SCI-5/f4-git-and-reuse-baseline) | Git and reuse baseline | F1 |
| F5 | [SCI-6](https://linear.app/orth-projects-general/issue/SCI-6/f5-architecture-acceptance) | Architecture acceptance | F2, F3, F4 |
| M1 | [SCI-7](https://linear.app/orth-projects-general/issue/SCI-7/m1-durable-storage-and-authoritative-mutations) | Durable storage and authoritative mutations | F5 |
| M2 | [SCI-8](https://linear.app/orth-projects-general/issue/SCI-8/m2-native-repository-engine-and-useful-read-api) | Native repository engine and useful read API | M1 |
| M3 | [SCI-9](https://linear.app/orth-projects-general/issue/SCI-9/m3-safe-developer-workspace-and-browser) | Safe developer workspace and browser | M2 |
| M4 | [SCI-10](https://linear.app/orth-projects-general/issue/SCI-10/m4-reviews-ci-landing-and-workflow-adoption) | Reviews, CI, landing and workflow adoption | M2, M3, F2 |
| M5 | [SCI-11](https://linear.app/orth-projects-general/issue/SCI-11/m5-git-migration-and-compatibility) | Git migration and compatibility | M2, F4 |
| M6 | [SCI-12](https://linear.app/orth-projects-general/issue/SCI-12/m6-finish-the-team-workflow-and-integration-boundaries) | Finish the team workflow and integration boundaries | M4, M5 |
| M7 | [SCI-13](https://linear.app/orth-projects-general/issue/SCI-13/m7-install-upgrade-recover-and-operate) | Install, upgrade, recover and operate | M4, M5, M6 |
| M8 | [SCI-14](https://linear.app/orth-projects-general/issue/SCI-14/m8-small-team-release-qualification) | Small-team release qualification | M1, M2, M3, M4, M5, M6, M7 |
| S1 | [SCI-15](https://linear.app/orth-projects-general/issue/SCI-15/s1-ha-with-explicit-durability) | HA with explicit durability | M8 |
| S2 | [SCI-16](https://linear.app/orth-projects-general/issue/SCI-16/s2-independent-api-and-worker-scale) | Independent API and worker scale | M8 |
| S3 | [SCI-17](https://linear.app/orth-projects-general/issue/SCI-17/s3-sharedregional-cache-tiers) | Shared/regional cache tiers | S2 |
| S4 | [SCI-18](https://linear.app/orth-projects-general/issue/SCI-18/s4-enterprise-identity-and-governance) | Enterprise identity and governance | M8 |
| S5 | [SCI-19](https://linear.app/orth-projects-general/issue/SCI-19/s5-many-repository-placement) | Many-repository placement | S1, S2 |
| S6 | [SCI-20](https://linear.app/orth-projects-general/issue/SCI-20/s6-hot-monorepo-and-workflow-capacity) | Hot monorepo and workflow capacity | S1, S2 |
| S7 | [SCI-21](https://linear.app/orth-projects-general/issue/SCI-21/s7-disaster-recovery-and-geographic-placement) | Disaster recovery and geographic placement | S1, S4 |
| S8 | [SCI-22](https://linear.app/orth-projects-general/issue/SCI-22/s8-enterprise-release-qualification) | Enterprise release qualification | S1, S2, S4, S7 |

## Initial agent task cards

| Card | Linear issue | Parent package | Work | Predecessors |
| --- | --- | --- | --- | --- |
| A01 | [SCI-23](https://linear.app/orth-projects-general/issue/SCI-23/a01-workload-fixtures) | F0 | Workload fixtures | None |
| A02 | [SCI-24](https://linear.app/orth-projects-general/issue/SCI-24/a02-acceptance-contract) | F0 | Acceptance contract | A01 |
| A03 | [SCI-25](https://linear.app/orth-projects-general/issue/SCI-25/a03-build-and-contract-skeleton) | F1 | Build and contract skeleton | F0 |
| A04 | [SCI-26](https://linear.app/orth-projects-general/issue/SCI-26/a04-domain-transaction-fixture) | F1 | Domain transaction fixture | A03 |
| A05 | [SCI-27](https://linear.app/orth-projects-general/issue/SCI-27/a05-independent-ci-simulator-and-fault-runner) | F1 | Independent CI simulator and fault runner | A03 |
| A06 | [SCI-28](https://linear.app/orth-projects-general/issue/SCI-28/a06-client-portability-spike) | F1 | Client portability spike | A03 |
| A07 | [SCI-29](https://linear.app/orth-projects-general/issue/SCI-29/a07-harness-integration-review) | F1 | Harness integration review | A04, A05, A06 |
| A08 | [SCI-30](https://linear.app/orth-projects-general/issue/SCI-30/a08-pg-lifecycle-arm) | F2 | PG lifecycle arm | F1 |
| A09 | [SCI-31](https://linear.app/orth-projects-general/issue/SCI-31/a09-temporal-lifecycle-arm) | F2 | Temporal lifecycle arm | F1 |
| A10 | [SCI-32](https://linear.app/orth-projects-general/issue/SCI-32/a10-restate-lifecycle-arm) | F2 | Restate lifecycle arm | F1 |
| A11 | [SCI-33](https://linear.app/orth-projects-general/issue/SCI-33/a11-native-feasibility-arm) | F3 | Native feasibility arm | F1 |
| A12 | [SCI-34](https://linear.app/orth-projects-general/issue/SCI-34/a12-gitreuse-feasibility-arm) | F4 | Git/reuse feasibility arm | F1 |
| A13 | [SCI-35](https://linear.app/orth-projects-general/issue/SCI-35/a13-consolidated-decision-review) | F5 | Consolidated decision review | A08, A09, A10, A11, A12 |

Begin with A01's workload fixtures, then A02's acceptance contract (drafting can overlap once A01 has a draft). F1 follows accepted F0; workflow/native/reuse experiments run after the shared harness. The [implementation roadmap](implementation-roadmap.md) contains the full sequencing and acceptance requirements.
