# Karoowa — Decision Log

> **Purpose:** A flat list of every locked decision made during the planning conversation, with date, rationale, and provenance. Append-only. When a decision is reversed, mark the original "Superseded by D-NNN" and add a new entry — never edit history.
>
> **Source:** distilled from the changelog and resolved Open Questions in `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`. The PRD remains the canonical, full-detail spec.

---

## Conventions

- **D-NNN** decision IDs are stable. They never get reused or renumbered.
- **Status:** `Locked` | `Locked (in principle)` (specifics still pending) | `Superseded by D-NNN`
- **Reversal:** older decisions are not deleted. They're marked superseded with a pointer to the replacement.

---

## Product strategy

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-001 | **Product name = Karoowa.** Full rename from inherited "ChainFlux" identity, including crate names (`karoowa-*`), CLI binary (`karoowa`), JSON-RPC method prefix (`kw_*`), Docker image tags, and brand. | 2026-04-09 | Locked | Parent PRD OQ-012 resolved |
| D-002 | **Brand vibe = "effortless harmony."** Tagline candidate: *"Light enough to launch anything."* Booking-platform etymology from a separate context is not carried over. | 2026-04-09 | Locked | Parent PRD OQ-013 resolved |
| D-003 | **Target audience = three concurrent tiers**: (a) hobbyists / individuals, (b) small Rust-comfortable chain-builder teams, (c) enterprise teams with permissioned-chain needs. | 2026-04-09 | Locked | Parent PRD OQ-001 resolved |
| D-004 | **Business model = open-core.** Public OSS layer (Apache 2.0) + private proprietary enterprise layer for compliance / multi-tenancy / hardened operations / paid support. | 2026-04-09 | Locked | Parent PRD OQ-002 resolved |
| D-005 | **Definition of v1.0 success = all of**: (a) published framework with adoption, (b) reference chain in production, (c) external audit completed, (d) contributor community, (e) revenue / customers. | 2026-04-09 | Locked | Parent PRD OQ-003 resolved |
| D-006 | **Karoowa is agent-native** as a top-line differentiator. Each persona has a corresponding AI agent that ships in the same milestone where the underlying capability lands. | 2026-04-09 | Locked | Parent PRD ASM-010 |

## Audience and personas

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-007 | **Primary personas (human form)**: Solo / Hobbyist Operator, Chain Builder, Validator Operator, dApp / Client Developer, Open-Source Contributor, Enterprise Operator, Security Auditor (later milestones). Each has a paired agent form. | 2026-04-09 | Locked | Parent PRD §2 |
| D-008 | **Agent capability bundling per milestone**: M1 = Dev bundle (CLI/Dev + Monitoring + Onboarding), M2 = Ops bundle (CI/CD & Deployment + Observability + Operator + Scaffolding), M3 = Security/Optimization bundle (Vulnerability Scanner + Auto-Scaling/Gas Optimizer + Integration + Contributor), M4 = Enterprise Governance bundle (Governance/Policy + Finance/Treasury + Compliance) — last bundle gated to enterprise layer. | 2026-04-10 | Locked | Parent PRD OQ-015 resolved |

## Repo and licensing

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-009 | ~~**Two repos.** Public `karoowa/karoowa` (OSS) + private `karoowa/karoowa-enterprise` (proprietary).~~ | 2026-04-10 | **Superseded by D-011** | Parent PRD OQ-023 round 4 |
| D-010 | ~~**License enforcement = repo separation + signed license file + trial mode + EULA.**~~ | 2026-04-10 | **Superseded by D-012** | Parent PRD OQ-022 round 4 |
| D-011 | **Single monorepo** with `core/` and `enterprise/` top-level directories (Strapi-style). Lower maintenance for a small/solo team, single codebase, easier shared-code refactoring. Leak risk mitigated by CI guardrails rather than physical separation. | 2026-04-10 | Locked | Parent PRD OQ-023 round 5; OQ-029 confirmed |
| D-012 | **Open-core enforcement = monorepo + `enterprise/` folder + CI guardrails + signed license file + EULA.** CI guardrail script fails any build where `core/` imports from `enterprise/`. Enterprise features require a signed license file at startup; optional trial mode. No online phone-home (hostile to air-gapped enterprise). | 2026-04-10 | Locked | Parent PRD OQ-022 round 5; OQ-029 confirmed |
| D-013 | **OSS layer license = Apache 2.0.** Enterprise layer license = proprietary, drafted before any enterprise feature ships (placeholder `LICENSE-ENTERPRISE.md` in M1.0). | 2026-04-09 | Locked (placeholder OK for M1) | Parent PRD §6 |

## Technical stack

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-014 | **Implementation language = Rust 1.78+.** Async runtime = tokio. | 2026-04-09 | Locked | Parent PRD §1 |
| D-015 | **Crypto primitives**: SHA3-256 (primary hash), BLAKE3 (fast path), Ed25519 (signing), binary Merkle tree with SHA3-256 internal nodes, addresses = last 20 bytes of `SHA3-256(public_key)`. No hand-rolled crypto. | 2026-04-09 | Locked | Parent PRD §1 architecture |
| D-016 | **L1 hot-path storage = RocksDB** with column families. Battle-tested in Bitcoin Core, Geth, Solana, Cosmos. | 2026-04-10 | Locked | Parent PRD REQ-017, ASM-017 |
| D-017 | **L2 indexing storage = PostgreSQL, deferred** until an explicit consumer requires it. | 2026-04-10 | Locked (deferred) | Parent PRD REQ-017, ASM-019 |
| D-018 | **L3 agent memory storage = LanceDB** (Apache 2.0, embedded, in-process Rust vector store). Re-evaluate Qdrant for M2/M3 if rich filtering becomes a hard requirement. | 2026-04-10 | Locked | Parent PRD OQ-025 resolved, ASM-018 |
| D-019 | **L4 telemetry = Prometheus** for metrics (already present in design); ClickHouse out of v1.0 scope. | 2026-04-10 | Locked | Parent PRD REQ-017, ASM-020 |
| D-020 | **P2P stack = libp2p** with Gossipsub (block/tx broadcast) + Kademlia (peer discovery) on TCP + Noise + Yamux. | 2026-04-09 | Locked | Parent PRD §1 |
| D-021 | **API gateway = Axum**, single port serving JSON-RPC 2.0 + REST + WebSocket. | 2026-04-09 | Locked | Parent PRD REQ-008 |
| D-022 | **JSON-RPC method prefix = `kw_*`.** All inherited `cf_*` references in design docs are renamed. | 2026-04-09 | Locked | Parent PRD ASM-009 |

## Agents and LLMs

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-023 | **`LlmProvider` trait** abstracts agents from any specific LLM. Pluggable via config without recompiling. | 2026-04-10 | Locked | Parent PRD OQ-017, REQ-014 |
| D-024 | **Launch LLM providers**: Anthropic (hosted), OpenAI (hosted), Google Gemma 4 via local backend (`ollama` / `llama.cpp`), generic GGUF local-model provider. | 2026-04-10 | Locked | Parent PRD REQ-014 |
| D-025 | ~~**Local-model floor = quantized 7B** (e.g. via llama.cpp / ollama) is sufficient for M1/M2 agents.~~ | 2026-04-09 | **Superseded by D-026** | Parent PRD ASM-014 |
| D-026 | **Local-model floor = ~3B-class** (lowered from 7B). Hobbyist default = **hosted LLM with in-process agent**; small local model (target: Gemma 4 E2B, 5B params, Apache 2.0) is the no-key fallback at documented degraded capability. **Subject to viability spike T1.11.1.** | 2026-04-10 | Locked | Parent PRD OQ-021 resolved, ASM-014a |
| D-027 | ~~**Sidecar-first** agent runtime. Sidecar default; in-process available for M1/M2 hobbyist convenience.~~ | 2026-04-10 | **Superseded by D-028** | Parent PRD round-4 REQ-015 |
| D-028 | **Three-mode hybrid agent runtime**: (a) **in-process** = M1 hobbyist default, fits low-end hardware; (b) **sidecar** = recommended for ≥8 GB hosts, **mandatory at M3**, "padded room" pattern with loopback proxy + auth + quota; (c) **cloud-hosted** = enterprise capability. | 2026-04-10 | Locked | Parent PRD OQ-016 / OQ-024 resolved, REQ-015 |
| D-029 | **Google Gemma 4 model specs**: Edge variants E2B (5B) and E4B (8B) for CPU/edge devices; workstation variants 26B and 31B for GPUs. **Apache 2.0** licensed weights. Official Ollama + LM Studio integration; GGUF builds on HuggingFace. M1 hobbyist tier targets E2B. **Specs sourced from stakeholder research; should be re-checked against canonical Google source before any binding contract.** | 2026-04-10 | Locked (per stakeholder research) | Parent PRD OQ-026 resolved |

## Hobbyist and devnet

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-030 | **Hobbyist install path = all channels.** (a) `curl -fsSL install.karoowa.io \| sh`, (b) prebuilt static binaries on GitHub Releases for Linux x86_64/aarch64 + macOS + Windows, (c) package managers (Homebrew, APT, RPM, Chocolatey, Scoop), (d) optional Docker image. **No Cargo, no Docker, no toolchain required.** | 2026-04-10 | Locked | Parent PRD OQ-018 resolved, REQ-013 |
| D-031 | **Public Karoowa Devnet** is core-team operated. Progression: **Devnet → Public Testnet → Mainnet** with SLOs **99.5% / 99.9% / 99.95%**. | 2026-04-10 | Locked (in principle) | Parent PRD OQ-019 / OQ-020 resolved, REQ-016 |
| D-032 | **Devnet budget M1 default = Low scenario ~$850/yr** (1 small VM + static IP + minimal monitoring). Med ~$1,850. High ~$4,850. Funded from Karoowa treasury or sponsor. **Final figures pending sponsor sign-off.** | 2026-04-10 | Locked (in principle) | Parent PRD OQ-028 resolved |
| D-033 | **Karoowa Infrastructure Lead** is the named role accountable for devnet uptime and ops. **Specific person TBD by sponsor.** | 2026-04-10 | Locked (in principle) | Parent PRD OQ-027 |

## Project shape

| ID | Decision | Date | Status | Source |
|----|----------|------|--------|--------|
| D-034 | ~~**M1 = inherited v0.1 backfill** + new features layered on top.~~ | 2026-04-09 | **Superseded by D-035** | Parent PRD round 1–6 |
| D-035 | **Karoowa is greenfield.** The inherited `files/` directory (`Cargo.toml`, `Dockerfile`, `docker-compose.yml`, `Makefile`, `README.md`) was a *design sketch*, not built code. M1 is a net-new implementation, not a backfill. Phase plan in `specs/development/dev_plan.md` (Phases 1.0 → 1.11). | 2026-04-10 | Locked | Confirmed by sponsor 2026-04-10 |
| D-036 | **EVM bytecode compatibility is required**, milestone TBD. Tracked as REQ-010. Likely slot: bundled with M3 (alongside WASM VM) or its own milestone. | 2026-04-09 | Locked (milestone TBD) | Parent PRD OQ-006 |
| D-037 | **Three feature PRDs per milestone is the working unit**, not one giant milestone PRD. Each phase in `dev_plan.md` is roughly the granularity of a feature PRD. | 2026-04-10 | Locked | Iterative planning consensus |

---

## Open decisions (not yet locked)

These are open in the parent PRD and need a decision before the corresponding phase starts.

| Phase | Open question | Owner | What's needed |
|-------|---------------|-------|---------------|
| Phase 1.0 | OQ-A002 — CI guardrail script implementation language | Tech lead | Bash + ripgrep recommended; confirm |
| Phase 1.0 | OQ-A007 — repo URL (`karoowa/karoowa`?) | Sponsor | Decide org + repo name |
| Phase 1.10 | OQ-027 — named Infrastructure Lead | Sponsor | Pick the person |
| Phase 1.10 | OQ-028 — devnet budget final figure | Sponsor | Pick a number, fund it |
| Phase 1.11 | OQ-021 / OQ-024 — local-model viability spike | Tech lead | Run T1.11.1 |
| Phase 2.4 | OQ-004 — BFT consensus algorithm | Tech lead | Tendermint / HotStuff / custom |
| Phase 3.0 | OQ-005 — WASM runtime | Tech lead | `wasmtime` / `wasmer` |
| Phases 3.x | OQ-006 — EVM milestone slot | Sponsor + tech lead | M3, M3.5, or own milestone |
| Parent PRD | OQ-007 — PRD owner | Sponsor | Designate |
| Parent PRD | OQ-008 — team capacity | Sponsor | Capacity plan |
| Parent PRD | OQ-009 — audit firm sourcing | Sponsor | Begin sourcing by end of M4 |
| Parent PRD | OQ-010 — CI platform + budget | Tech lead | GitHub Actions assumed; confirm |
| Parent PRD | OQ-011 — EIP-compatible tx encoding (M4) | Sponsor | Concrete need or speculative? |
| Parent PRD | OQ-030 — telemetry retention policy | Tech lead | Default: Prometheus 15-day |

---

## Where to look for more

- **Full PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`
- **Product vision:** `specs/strategy/01_product_vision_and_strategy.md`
- **Technical strategy:** `specs/strategy/02_technical_strategy.md`
- **Development plan:** `specs/development/dev_plan.md`
