# Contributing to Deltrove

Deltrove is currently a planning-stage project. Architecture, experiments and acceptance gates are documented in the [implementation roadmap](docs/implementation-roadmap.md); implementation has not started.

Work is tracked in the [Deltrove Linear project](https://linear.app/orth-projects-general/project/deltrove-438e1fe166bf). The [tracking map](docs/project-tracking.md) connects roadmap IDs to Linear issues. Repository documents own architecture and acceptance semantics; Linear owns current status, dependencies and assignments. There is no automatic bidirectional synchronization configured.

## Picking up work

1. Read the relevant roadmap package and its prerequisites. Start with F0's A01/A02 cards before the build/harness work in F1.
2. Claim or coordinate the Linear issue, identify owned paths and use the roadmap handoff template. Coordinate changes to shared schemas, migrations, canonical formats and test fixtures with the integrating owner.
3. Keep changes reviewable and scoped to the accepted task. Include the Linear identifier in the branch name and pull request description so the work can be traced.
4. Run the acceptance checks appropriate to the change. Record exact commands, versions, results and remaining limitations; label unavailable checks **not run**. Documentation checks do not establish product correctness or performance.
5. Request independent review of implementation and evidence. A task is complete only after its acceptance gate passes; do not mark work complete solely because an agent reports success.

## Design boundaries

The backend language is Rust. Proposed clients use Go/Cobra and React/TypeScript. C and C++ are excluded as project implementation choices. PostgreSQL owns authoritative ACID domain mutations; object storage, CI and workflow histories have separate durability/commit boundaries.

Temporal and Restate evaluation is required before expanding workflow machinery. Their production adoption remains an evidence-based decision, including pinned licensing/distribution checks. Ordinary Tokio remains the I/O default until the planned io_uring comparison demonstrates an acceptable benefit without weakening resource or durability guarantees.

Project-owned work is licensed under [Apache-2.0](LICENSE). Reusing third-party code requires preserving applicable license/notice obligations and compatibility with this distribution; architectural inspiration is not permission to copy implementation code under different terms.
