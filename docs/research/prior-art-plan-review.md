# Prior-art review of the technical plan

**Reviewed:** 2026-10-16. This is a source-accuracy and adoption review of `docs/technical-plan.md`; it makes no edits to that plan.

## Findings for resolution

### P1 — Scope the exact Git re-export promise to preserved objects

**Location:** `technical-plan.md` §13, “Git OIDs and signatures” row and importer/exporter paragraph.

The row says retaining original raw commit/tag/tree objects enables “exact re-export,” but it does not explicitly retain raw blob objects, specify original object-hash format, or limit exactness to the unchanged imported graph. File bytes can reproduce blob IDs when encoded and hashed exactly, but an exporter that normalizes or rewrites trees, commits, tags, or refs cannot promise the original IDs/signatures. Git's [fast-export documentation](https://git-scm.com/docs/git-fast-export) says commit signatures default to stripping, transformations can invalidate tag signatures, and `fast-import` cannot tag tree objects. Its documented “one-to-one mirror” example is limited and does not eliminate these cases.

**Resolution:** Define exact re-export narrowly: preserve all required original reachable object bytes (blob/tree/commit/tag), hash algorithm, and ref targets, and emit them unchanged for imported history. Specify which ref kinds and tag targets are supported; handle tree/blob tags outside `fast-import` or reject them in preflight. Native changesets should export as new Git objects with no original-ID/signature promise. Test SHA-1 and SHA-256 repositories, multiple-parent commits, signed commits/tags, nested and non-commit tags, encoding headers, and special refs. Treat fast-export/import as an incremental conversion path, not proof of lossless compatibility.

### P2 — Keep the Git adoption contract explicitly one-way and non-transparent

**Location:** §§1, 3, 5, and 13.

The plan is mostly clear: it calls for import plus one-way export, says a native workspace is not `.git`, defers a transparent Git protocol gateway, and offers mirror/patch/archive integration. That does **not** claim general Git command compatibility. Preserve this distinction in the product wording and release matrix: an importer/exporter is a migration bridge, not a Git remote that ordinary clients can use transparently. A “Git-backed service-side API” prototype is a useful baseline only if scoped as APIs over an existing Git store; it does not establish that the native client can replace Git for tools, hooks, index/staging, IDEs, or CI.

**Resolution:** Name the supported direction and authority at each phase (Git-authoritative import, native-authoritative export, or a separately specified gateway). Keep bidirectional writes and transparent protocol claims out of the MVP unless implemented and tested.

### P2 — The Piper/CitC claim needs an accessible full-paper citation or narrower wording

**Location:** §3, “Piper/CitC-style environment.”

The linked [Google Research publication record](https://research.google/pubs/why-google-stores-billions-of-lines-of-code-in-a-single-repository/) identifies the Google-authored 2016 paper and summarizes the monolithic repository and tradeoffs. Its accessible abstract does not establish CitC's filesystem, workspace overlay, central-editing behavior, or the stated “demonstrates the value” conclusion. The full text was not accessible in this review. Thus “architecture reference” is fine as an analogy, but the line should not present a particular CitC architecture as verified from that landing page.

**Resolution:** Keep the row explicitly labeled as an unverified analogy until the primary full text is available; otherwise limit the cited fact to Google's custom monolithic repository and published discussion of tradeoffs. Do not use CitC as evidence for implementation or performance choices yet.

### P2 — Avoid making the GPL fork sound pre-cleared

**Location:** §3, licensing paragraph.

The plan correctly says Sapling's main project is GPL-2.0, while the website/ISL are MIT and libraries may have different terms; it also correctly treats external reuse as a license decision. The Sapling [repository README](https://github.com/facebook/sapling) says to inspect each library's license/header, and the [Mononoke README](https://github.com/facebook/sapling/blob/main/eden/mononoke/README.md) describes an OSS build with omitted functions. “A GPL-based Mononoke fork is a legitimate alternative product decision” is directionally reasonable but can sound like a settled legal conclusion despite those per-path and dependency checks.

**Resolution:** Call it a possible license-dependent fork path, contingent on a per-component/dependency inventory and counsel review if distribution model requires it. Avoid implying that all Mononoke files or bundled dependencies necessarily share one license.

## Confirmed claims and adoption assessment

- The external-support caveat is correctly attributed: Meta says Mononoke and EdenFS are used internally but are **not supported for external use**; its OSS builds are for unsupported experimentation. This does not mean the code cannot build or run. The Mononoke [README](https://github.com/facebook/sapling/blob/main/eden/mononoke/README.md) says it is still in development and enumerates omitted Thrift APIs, MySQL failover support, CacheLib support, and configuration documentation. §3's time-boxed build/import/API/operations evaluation is a realistic response; retain both nuances.
- The license section is substantially accurate: Sapling's main repo is GPL-2.0; website and ISL are MIT; subprojects can differ. The plan appropriately makes no license choice for the new project and distinguishes an independent implementation from copying Mononoke.
- Jujutsu is correctly treated as a Git-backed client reference rather than a centralized service substitute. The project's [Git compatibility guide](https://github.com/jj-vcs/jj/blob/main/docs/git-compatibility.md) documents colocated operation but also warns that Git's staging area is ignored and that Git conflict/rebase states and mixed mutations have limits. Calling `jj` “immutable-operation ideas” is understandable shorthand, though “operation-log/undo model” is more precise.
- The plan's Git behavior inventory is credible and appropriately conservative: it separates Git objects from working files and calls out index, refs, attributes, hooks, submodules, LFS, signatures, and special refs. The one-writer cutover and explicit rejection/preflight policy are realistic. Its `git merge-file` section also correctly says that this file-level primitive is not the complete Git merge strategy.
- The Scion material in §2 is a concise paraphrase with no direct quotation and is within the source's available word budget. It distinguishes reported architecture from proposed design and avoids converting Anthropic's reported results into this project's guarantee. The separate prior-art memo now contains under 70 words of direct Scion summary, as requested.

## Required edit outside the plan

I corrected one overstatement in `docs/research/prior-art.md`: offline edits to already materialized files remain possible; authoritative commits and requests for uncached history/objects require service access. The plan's §5 offline contract already describes this distinction accurately.
