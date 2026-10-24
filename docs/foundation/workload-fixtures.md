# F0 workload fixtures (A01)

**Status:** proposed synthetic stand-ins for F0 review; no pilot repository has been supplied or measured. These fixtures are reproducibility inputs, not performance results or capacity claims. Numeric sizes are bounded starting hypotheses. Capture real pilot observations separately when permission and data become available; do not substitute these values for them.

## Stable catalogue

The versioned machine-readable catalogue is [manifest.json](../../fixtures/workloads/manifest.json). Its seed is **2301**. Fixture IDs are stable and intended for reuse by A02, F1/F2 and later evaluations:

| Fixture ID | Default shape | Purpose |
| --- | ---: | --- |
| `tiny-graph` | 5 nodes: root, two divergent branches, and an ordered two-parent merge | Tiny correctness graph; topology is only a harness input, not a production graph implementation. |
| `wide-tree` | 64 files in one directory | Directory fanout and narrow edit cases. |
| `deep-tree` | One leaf at depth 24 | Path traversal/depth cases. |
| `binary-mix` | 8 deterministic SplitMix64 1 KiB blobs + marker | Binary hashing/transfer cases; no customer binary characteristics inferred. |
| `many-repos` | 8 repositories, one small README each | Repository-routing/fleet overhead setup. |
| `hot-repo` | 256 small source-like files in one repository | Single-repository skew setup; not an actual hot-write trace. |
| `long-ci-waits` | 12 waits, durations 30 to 360 seconds, alternating callback order | Durable wait/callback cases. Durations are simulated metadata, not wall-clock sleeps. |

The checked-in manifest is the single source for seed, fixture IDs and profile sizes; the Rust generator loads it directly. Binary fixture bytes use SplitMix64: add `0x9E3779B97F4A7C15` to the 64-bit state, apply the SplitMix64 xor-shift/multiply sequence, then emit each generated word's low byte. The stream starts from manifest seed 2301 and continues across blobs. SHA-256 uses the pinned `sha2` crate. The `extended` profile is an opt-in generation ceiling for larger local experiments: 4,096 wide files, depth 128, 64 binary blobs, 256 repositories, 16,384 hot-repo files and 1,000 wait records. It remains synthetic and is not a production workload size recommendation. Each generated fixture contains `fixture.json` with per-file byte counts/SHA-256 and a canonical corpus digest.

## Reproduce

Requires Rust/Cargo **1.95.0** for the recorded run. The scoped package under `tools/workloads` does not add a root workspace or the F1 build skeleton. `serde_json` 1.0.151 parses the manifest and serializes output; `sha2` 0.10.9 computes SHA-256. Exact direct and transitive versions are pinned in the package manifest and lockfile. From the repository root:

```sh
cargo run --locked --manifest-path tools/workloads/Cargo.toml -- all --out /tmp/workloads
cargo run --locked --manifest-path tools/workloads/Cargo.toml -- all --profile extended --out /tmp/workloads-extended
cargo test --locked --manifest-path tools/workloads/Cargo.toml
```

The output path must be absent or empty. To verify determinism, generate `all` into two empty directories and recursively compare them; generated file ordering and JSON serialization are stable. There is no external service, database, benchmark harness, or production VCS algorithm involved. The exact recorded invocations used `--offline --locked` because crates.io DNS was unavailable; pinned dependencies were cached locally.

## Recorded evidence

Commands run on **rustc 1.95.0 (59807616e 2026-04-14)** and **cargo 1.95.0 (f2d3ce0bd 2026-03-21)**, local macOS environment:

```sh
cargo fmt --manifest-path tools/workloads/Cargo.toml --all -- --check
cargo test --offline --locked --manifest-path tools/workloads/Cargo.toml
cargo clippy --offline --locked --manifest-path tools/workloads/Cargo.toml --all-targets -- -D warnings
cargo run --offline --locked --manifest-path tools/workloads/Cargo.toml -- all --out /tmp/del23-rust-a > /tmp/del23-rust-a.json
cargo run --offline --locked --manifest-path tools/workloads/Cargo.toml -- all --out /tmp/del23-rust-b > /tmp/del23-rust-b.json
diff -r /tmp/del23-rust-a /tmp/del23-rust-b
diff /tmp/del23-rust-a.json /tmp/del23-rust-b.json
cargo run --offline --locked --manifest-path tools/workloads/Cargo.toml -- all --profile extended --out /tmp/del23-rust-extended > /tmp/del23-rust-extended.json
```

Results: **3 Rust tests passed** (2 unit tests and 1 CLI integration test); formatting and Clippy with warnings denied passed; recursive and summary diffs were empty. Tests independently scan emitted bytes for all 7 fixtures and recompute file hashes, byte counts, file counts, totals and canonical corpus digests. They check that a nonempty output directory is refused without modifying a sentinel, and independently verify the tiny graph's parent existence, topological order, divergent fork and ordered merge parents. Extended generation completed with counts matching the manifest. A normal Cargo invocation initially failed to resolve `index.crates.io`; all later checks passed offline against cached crates.

| Fixture ID | Files | Payload bytes | Canonical corpus SHA-256 |
| --- | ---: | ---: | --- |
| `tiny-graph` | 1 | 264 | `deac514dfb5053174d61615373e7ba83cde866869921a316e862848421dc68d8` |
| `wide-tree` | 64 | 2,038 | `151c959d5b81fd8cf8ecaa01f41c179c4375201f542822e66d331872cbad5765` |
| `deep-tree` | 1 | 32 | `ad37a03f486764243d9c5237d2d219ef76bf133cf1b82374ff502cc47a2249c0` |
| `binary-mix` | 9 | 8,247 | `bc1fabe30e39267ea479c6ba1233987462269f2de9bc806afa625f6c3e1a6c67` |
| `many-repos` | 8 | 184 | `418a5db1d6958a006895181f997440602826119ff3908f62672078b393987b9c` |
| `hot-repo` | 256 | 9,362 | `924cd9b66f590746a9fdb9df1effa9b0cac00d0807922a8b83cc66718a1dfaff` |
| `long-ci-waits` | 1 | 992 | `f165d847b00dfcf512494c4e23a7021f723b54e9db0cb519220dfbdc6c7f0530` |

The corpus digest is SHA-256 over compact JSON serialization of the sorted path/byte-count/file-hash records. It excludes generated directory metadata and timestamps.

## Roadmap handoff

- **Task/package ID and parent milestone:** A01 — Workload fixtures; F0 — Scope and workload contract.
- **Objective and user-visible result:** named, deterministic synthetic inputs for correctness and later benchmark harnesses.
- **Dependencies and contract/version inputs:** roadmap §§1, 2, 8; technical-plan provisional workload envelope; workflow plan W01–W09 fixture boundaries. No real repository input supplied.
- **Owned paths:** `tools/workloads/**`, `fixtures/workloads/**`, `docs/foundation/workload-fixtures.md`.
- **Invariants and explicit out-of-scope behavior:** manifest is the source for seed/IDs/profile sizes; fixed-seed reproducible content hashes and bounded defaults; no production VCS algorithms, claims about real customer workloads, or capacity targets.
- **Implementation deliverables:** scoped Rust generator driven by the machine-readable fixture manifest, independent shape/hash/sentinel tests and this evidence record.
- **Acceptance checks and fault cases; real services required:** deterministic two-run comparison, independent structure/count/hash checks and nonempty-output refusal; no real services required.
- **Evidence location and exact commands/configuration:** commands and results above; Rust/Cargo 1.95.0; pinned dependencies; seed 2301; default and extended profiles.
- **Review risks, unresolved decisions and rollback/recovery:** A02 should reconcile the named scenarios and decide whether to add pilot-specific shapes after pilot data exists. Extended mode may consume more disk/time and is opt-in. Remove generated output directories after use.
- **Status:** review. No F0 acceptance implied. Product correctness/performance checks are not run; no production product exists.
