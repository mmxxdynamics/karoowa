# Karoowa — Product Vision & Strategy

> **Purpose:** A condensed reference for what Karoowa is, who it's for, and what success looks like. Distilled from `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` §1–§2 — that PRD remains the canonical, full-detail spec.

---

## What Karoowa is

**Karoowa is an agent-native, Linux-native, Rust-based blockchain framework.** It lets anyone — from a hobbyist on a laptop to a small chain-builder team to an enterprise — go from zero to a running custom chain without assembling primitives from scratch or adopting a heavyweight framework like Substrate or Cosmos SDK.

Three concurrent objectives drive the project:

1. **Lower the cost and time to launch a custom chain** by shipping production-grade defaults for consensus, storage, networking, crypto, and developer tooling in a single coherent workspace.
2. **Be agent-native.** Personas defined for human users are also the design surface for AI agents that ship inside Karoowa to operate, observe, and build on the system autonomously. Agent capabilities are built **sequentially alongside the related infrastructure**, not bolted on at the end.
3. **Sustain the project commercially via an open-core model.** Everything required for general access is in the public OSS layer. Enterprise capabilities (multi-tenancy, compliance, hardened KMS, premium support, governance/treasury agents) are in a separate proprietary enterprise layer.

The brand evokes **effortless harmony** — the experience of using Karoowa should feel light, frictionless, and consistent across the hobbyist, team, and enterprise tiers. Tagline candidate: *"Light enough to launch anything."*

---

## Who Karoowa is for — three audience tiers

| Tier | Who | Why they care |
|------|-----|---------------|
| **Hobbyist** | A solo dev with no organisation, running a node on a laptop or VPS, possibly hosting a small app on shared public infra | One-command install, sane defaults, low resource footprint, public devnet to join, no API keys required |
| **Small chain-builder team** | 1–5 devs comfortable in Rust, launching an app-specific or permissioned chain | Batteries-included framework, pluggable consensus, devnet bring-up in minutes, real SDK |
| **Enterprise** | Ops/platform engineers deploying a permissioned chain with compliance, multi-tenancy, KMS, paid support | Proprietary enterprise layer with the gates and integrations they need |

Each tier has a corresponding human persona **and an agent persona** that ships inside Karoowa to perform that tier's core operational duties autonomously.

| Human Persona | Agent Form | Ships in |
|---|---|---|
| Solo / Hobbyist Operator | **Onboarding Agent** — guides install, key gen, first-block, troubleshooting | M1 Dev bundle |
| Chain Builder | **Scaffolding Agent** — generates chain skeletons, consensus stubs, genesis configs | M2 Ops bundle |
| Validator Operator | **Operator Agent** — monitors node health, applies remediations, escalates incidents | M2 Ops bundle |
| dApp / Client Developer | **Integration Agent** — generates client code, signs/submits txs, debugs receipts | M2/M3 |
| Open-Source Contributor | **Contributor Agent** — triages issues, runs draft-PR checks | Continuous |
| Enterprise Operator | **Compliance Agent** *(enterprise-only)* — produces audit reports, gates risky operations | M4 Enterprise |
| Security Auditor | **Audit Assistant Agent** — continuous fuzzing, drift checks | Later milestones |

> Personas and their agent forms are not parallel implementations. They're two views of the same capability: a thing the system can do, organised around who needs it.

---

## What success looks like at v1.0

All of:

- **(a) Published framework** that other teams adopt
- **(b) Reference chain** running in production on Karoowa
- **(c) Completed external security audit** with zero unresolved high-severity findings
- **(d) Contributor community** — meaningful external PRs, stars, reference deployments
- **(e) Revenue / customers** through the enterprise tier

Each milestone celebrated independently.

---

## Hypotheses we're testing

| ID | Hypothesis | How we measure |
|----|-----------|----------------|
| H-001 | Shipping a single Rust workspace cuts time-to-devnet from months to under a day | Time from `git clone` to first block on a clean machine |
| H-002 | Trait-based pluggable consensus lets downstream teams build custom engines without forking | External `ConsensusEngine` implementations |
| H-003 | Rust + RocksDB + libp2p outperforms Cosmos SDK on crypto/storage workloads | TPS + state-read p99 vs. a Cosmos SDK reference chain |
| H-004 | A credible v1.0 (audit + governance) attracts a contributor community comparable to mid-tier Rust infra projects | External PR rate, reference deployments |
| H-005 | Shipping AI agents alongside each capability makes Karoowa easier to operate than alternatives | Manual ops actions / node-day on a reference deployment |
| H-006 | An open-core split with a private enterprise layer generates revenue without alienating contributors | Enterprise sales pipeline + OSS contributor retention |

---

## Key metrics with current baselines

| Metric | Target | Baseline |
|--------|--------|----------|
| Time from `git clone` to running devnet | < 15 min | Unmeasured |
| `cargo test --workspace` runtime | < 5 min on commodity laptop | Unmeasured |
| PoA devnet block time (4 validators) | ≤ 2s, p99 < 5s | Unmeasured |
| `kw_getBalance` JSON-RPC p99 (warm cache) | < 50ms | Unmeasured |
| External contributors with merged PRs (12 mo post-v1.0) | ≥ 20 | 0 |
| Reference chains running Karoowa in production | ≥ 1 by v1.0 | 0 |
| External audit findings (high severity) at v1.0 | 0 unresolved | N/A |

All targets are **placeholders** — replace with team-validated numbers as benchmarks come online.

---

## Out of scope (Phase 1)

- EVM compatibility from day one (planned, milestone TBD — see decision log)
- Mobile SDK / wallet
- Hosted / managed-service offering (Phase 2 possibility)
- Browser-based block explorer (Grafana suffices for v1.0)
- Tokenomics design (chain operators design their own)
- Formal verification of consensus
- Privacy / zero-knowledge primitives

---

## Where to look for more

- **Full PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`
- **Technical strategy:** `specs/strategy/02_technical_strategy.md`
- **Locked decisions:** `specs/strategy/03_decision_log.md`
- **Development plan:** `specs/development/dev_plan.md`
