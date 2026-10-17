# Deltrove: open-source opportunity assessment

**Checked:** 16 October 2026. **Status:** desk research and recommendations; no customer interviews, product trials or Deltrove benchmarks. Three GPT-6 Luna agents researched demand, competitors and adoption. The primary agent reviewed their memos, checked key primary sources and resolved the differences below.

**Naming follow-up:** the owner selected Deltrove after this research identified another project using the former Open Scion name. Naming recommendations below explain that decision; competitor names retain their original spelling.

## 1. Judgment

**Deltrove could become a useful, visible open-source infrastructure project. There is not enough evidence to predict broad popularity, and completing the full technical plan would not establish demand on its own.** The most plausible early audience is platform/DevEx teams, agent-platform developers and CI operators with an existing, measurable repository-access or workspace cost. Ordinary small teams with a few agents may have little reason to migrate.

For the full proposed server, CLI and review application, a credible success case is a focused community using it repeatedly for a workload conventional Git setups handle poorly. Becoming a general Git replacement would require much stronger evidence. These are qualitative judgments, not estimated probabilities, user counts or a market-size forecast.

The best near-term investment is a differentiated, reversible pilot. Keep the full source-control ambition, but require evidence of repeat use before funding its entire ecosystem. A small-team deployment target makes testing accessible; it does not mean every small team is a prospective user.

## 2. Evidence for and against

| Observation | What it supports | What it does not establish |
| --- | --- | --- |
| GitHub's October 6 engineering account describes agent/CI load growth, ref contention and a redesign that preserves existing Git workflows. [GitHub engineering](https://github.blog/engineering/architecture-optimization/building-git-infrastructure-for-agent-scale-development/) | Pressure is visible in both Anthropic's workload and GitHub's highest-volume hosted workloads. | Demand for a replacement VCS or prevalence across ordinary teams. Incumbent investment also raises the competitive bar. |
| Anthropic describes an unusual high-volume workload and explicitly limits Scion's suitability; its broader recommendation includes bringing server APIs and better serving architecture to Git. [Anthropic article](https://www.anthropic.com/engineering/redesigning-source-control) | A server-oriented model can address a real class of problems; Git-compatible server improvements deserve testing. | That most teams face the same pain, or that our implementation would reproduce its results. |
| Jujutsu and GitButler have high public star counts and retain Git-based adoption paths. [Jujutsu](https://github.com/jj-vcs/jj), [GitButler](https://github.com/gitbutlerapp/gitbutler) | There is public interest in alternatives to the conventional Git experience. | Equivalent interest in migrating authoritative storage, hosting, review and CI. Stars do not count active users. GitButler's current Fair Source terms differ from an OSI open-source project. |
| GitButler explicitly documents parallel agents and tradeoffs between shared-workspace branches and isolated worktrees. [Parallel-agent guide](https://docs.gitbutler.com/ai-agents/parallel-agents) | The workflow matters enough to build products around it. | That generic agent integration or avoiding worktree setup is an unoccupied niche. |
| Sapling exposes its client, while its public repository marks Mononoke/EdenFS unsupported externally. [Sapling](https://github.com/facebook/sapling) | Productizing a self-hosted server experience could be valuable. | That an internally successful server is a simple small-team deployment or that competitors cannot close the gap. |

Sources are project/vendor accounts and documented product behavior, not an independent survey of purchasing or migration intent. No causal relationship between architecture, language, stars and adoption is established.

## 3. Direct competitors and the project name

This is **not an empty category**. The research identified two especially relevant public repositories:

| Project | Published scope relevant to us | Implication |
| --- | --- | --- |
| [CarterT27/scion](https://github.com/CarterT27/scion) | Independent Apache-2.0 Rust implementation inspired by Anthropic. Its README describes a server, CLI, daemon, importer and CockroachDB/S3 storage; review/search/CI orchestration are outside that repository. It explicitly limits production-scale claims. | There is already a permissively licensed implementation to evaluate alongside Git and Mononoke. A separate Rust implementation needs a concrete distinction; consider reuse, interoperability or collaboration only after code/operations review. |
| [tinyhumansai/openscion](https://github.com/tinyhumansai/openscion) | Another Scion-inspired project already using **OpenScion**. Its README describes Rust server/client modules, a web application, local storage options and GPL-3.0-only licensing. | The name and broad backend/frontend/CLI concept already overlap. Evaluate discoverability and a distinct identity before launch. Reusing implementation requires a licensing decision consistent with this project's Apache-2.0 direction. This is not a trademark determination. |

Repository descriptions establish claimed scope, not correctness, security, supported operations or production maturity. We have not built or audited these implementations. Their benchmark results are not Deltrove results and should not be compared without matching workloads, durability, topology and timers. Some published results favor Git on particular cases; a new format does not automatically win.

For attention context only, root's GitHub-page reads showed roughly 32k stars for Jujutsu, 22k for GitButler and 7k for Sapling; the direct Scion-inspired projects were far smaller. Cached/live snapshots differed across research passes, so precise counts are deliberately not used to rank adoption. Repository age, publicity and sponsorship also differ. None of these figures supports a star target or a forecast for this project. [Jujutsu](https://github.com/jj-vcs/jj), [GitButler](https://github.com/gitbutlerapp/gitbutler), [Sapling](https://github.com/facebook/sapling).

## 4. Most promising positioning to test

Proposed promise: **“A self-hosted source-control service that makes repeated agent and CI access to large repositories cheaper and easier to operate.”** That is a hypothesis. Choose one measurable benefit before using it as a product claim.

Potential initial users and their buying or adoption reason:

- **Platform/DevEx owners:** repeated hydration, mirror maintenance or CI setup consumes measurable time, disk, bandwidth or operator effort.
- **Agent-platform developers:** many short-lived jobs need stable source snapshots, selected files and history/diff/ancestry APIs; they can embed a useful service without migrating every human developer.
- **Organizations already committed to self-hosting:** deployment control matters, and they can name an operational gap that existing Git forges do not meet. Self-hosting alone is insufficient differentiation because established alternatives exist.

A low-friction first experiment is a server-side source-data API over an existing Git repository: selected-file access, diff/history/ancestry and one CI integration, with Git remaining canonical for that experiment. Test against tuned Git mirrors, shared object stores, partial/sparse clones and relevant host APIs. Git already provides demand-fetching and shared-repository working trees; comparing only against repeated full clones would be misleading. [Git partial clone](https://git-scm.com/docs/partial-clone), [Git worktree](https://git-scm.com/docs/git-worktree).

This is a **recommended F0/F4 validation experiment**, not a decision to replace the approved native architecture or operate two writable authorities. If real users require native authoritative commits to solve the pain, test that separately with explicit import/export, rollback and workflow migration. If the Git-backed service is sufficient, F5 can legitimately select that route under the existing feasibility gate. Do not build both production engines just to avoid choosing.

For a full native implementation, potential differentiators are an independently deployable product, trustworthy recovery/export, clear API contracts, low operator effort and integrations users actually maintain. Rust, Apache-2.0, io_uring, ACID terminology or a workflow engine can support delivery; none is evidence of product demand or unique value by itself. All claimed advantages require comparison with the direct alternatives.

## 5. A bounded validation program

The following thresholds are **proposed project decision rules**, not empirical market benchmarks or sufficient evidence for all enterprise requirements. Use independent teams and record negative responses and churn, rather than selecting only enthusiastic friends.

| Stage | Work and measurement | Proposed continuation gate |
| --- | --- | --- |
| Problem discovery | Interview 12–15 target teams about the last concrete source-access/workspace incident, present workaround, recurring cost and who can approve a trial. Request a permitted real or sanitized workload. | At least 5 independently describe the same recurring, quantified problem, and 3 agree to test a representative workload. Interest without a workload or trial commitment is weaker evidence. |
| Comparative prototype | Implement one source-data/API or workspace operation and one CI/agent integration. Predeclare the team's pain metric and acceptable regressions; compare against its tuned current setup. | A repeatable material improvement in the agreed metric, with correct output and safe failure behavior. If an ordinary Git configuration achieves the same outcome cheaply, narrow or stop that feature. |
| Independent alpha | Recruit 3–5 teams to install and use a disposable/mirrored repository for 4–8 weeks. Track activation without maintainer intervention, recurring workflows, disabled integrations, failures and operator hours. | At least 3 activate; at least 2 retain weekly usage through the final four weeks and choose to continue. Each pilot demonstrates restore/export for the tested scope and has no unresolved integrity issue. |
| Broader beta | Release installable artifacts, compatibility/support/security documentation and reproducible evidence for the narrow workflow. Offer manual check-ins or opt-in aggregate usage reporting. | Continued 4-/12-week retention, usable upgrades and support effort within committed maintainer capacity. Set the next cohort's explicit goals from alpha data; do not manufacture a universal user-count threshold. |

Record separate measures for attention, activation, retention and contributions:

- **Attention:** stars, demo views, newsletter interest and downloads. Useful for discovery; weak evidence of use.
- **Activation:** independent installs that complete a real repository/workflow, with a defined time-to-first-useful-result.
- **Retention:** independent teams repeatedly completing successful workflows; document the definition and denominator. Downloads or an imported repository alone do not qualify.
- **Ecosystem:** third-party integrations and repeat contributors who land reviewed changes across releases, rather than one-off generated PR volume.
- **Sustainability:** maintainer hours per active installation, unresolved integrity/security defects, support response, successful upgrades/restores and release ownership.

No outreach, interviews, prototype or telemetry collection has been performed for this assessment. Repository contents and source text are unnecessary for routine aggregate usage measurement.

## 6. Conditions that would change the recommendation

Invest further if unrelated teams volunteer painful workloads, the same narrow feature repeatedly wins under equal correctness/durability conditions, independent operators retain it, and contributors can maintain understandable boundaries. Use the selected Deltrove identity consistently and validate a distinct product promise before a broad launch.

Reduce scope or collaborate if an existing Scion-inspired implementation already meets the users' needs, a small extension to a Git forge removes the pain, users want only agent sandbox orchestration, or no one will keep the integration enabled. A busy launch with stars but no repeat usage is not sufficient.

The current backend/frontend/CLI plan remains a technical proposal. This assessment recommends evidence to prioritize it; it does not silently close/rewrite Linear milestones, choose a new foundation, adopt third-party code, rename the project or publish a launch.

## 7. Review of the Luna research

| Research pass | Useful contribution | Primary-agent review and disposition |
| --- | --- | --- |
| [Demand](research/oss-demand.md) | Separates unusual scale cases from ordinary teams; identifies agent/CI segments and falsifiable trial gates. | Verified Anthropic's explicit suitability caveat and GitHub's October 6/7 article. Strong adjacent demand is not direct proof of migration willingness. |
| [Landscape](research/oss-landscape.md) | Distinguishes Git-compatible clients, forges, agent tooling and direct native implementations. | Checked direct repositories, corrected the CockroachDB/S3 description, reconciled differing public metric snapshots and incorporated the second OpenScion project. No competitor implementation was certified. |
| [Adoption](research/oss-adoption.md) | Emphasizes reversible trial, interoperability, safe operation, contribution boundaries and retained usage. | Kept its source-data API suggestion as an experiment rather than an unapproved architecture pivot. A bespoke engine-adapter framework is not required; maintain the existing bounded domain-command approach and an OSS installation path. |

The memos are coordinated desk research, not three independent validations of market demand. The primary agent's synthesized judgment favors a narrow, measurable adoption path and explicitly rejects an unsupported popularity forecast.

All three Luna agents also reviewed this assessment. Their reviews found no material contradiction; the primary agent incorporated refinements separating public star counts from usage and high-volume hosted workloads from ordinary-team prevalence. Agent agreement is additional editorial review, not market evidence.
