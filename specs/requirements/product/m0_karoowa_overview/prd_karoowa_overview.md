# PRD: Karoowa — Overarching Product Vision

| Field | Value |
|-------|-------|
| Created | 2026-04-09 |
| Created By | Karoowa team |
| Milestone | N/A (overarching PRD — covers v0.1 → v1.0 roadmap) |
| Implementation Ticket | N/A — milestone-level PRD |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Scope:** Phase 1 product vision for Karoowa
> **Owner:** TBD
> **Stakeholders:** Core maintainers, prospective open-source contributors, early adopter chain operators
> **Status:** Draft
> **Created:** 2026-04-09
> **Last Updated:** 2026-04-09

---

## 1. Business Objective & Outcomes

### Business Objective

Karoowa is an **agent-native, Linux-native, Rust-based blockchain framework** that lets anyone — from a hobbyist running a node on a laptop, to a small team launching an app-specific chain, to an enterprise standing up a permissioned network — go from zero to production without assembling primitives from scratch or adopting a heavyweight framework like Substrate or Cosmos SDK.

Three concurrent objectives:

1. **Lower the cost and time to launch a custom chain** by shipping production-grade defaults for consensus, storage, networking, crypto, and developer tooling in a single coherent workspace.
2. **Make Karoowa agent-native.** The personas defined in §2 are not just users — they are also the design surface for AI agents that ship inside Karoowa to operate, observe, and build on the system autonomously. Agent capabilities are built **sequentially alongside the related infrastructure**, not bolted on at the end.
3. **Sustain the project commercially via an open-core model.** Everything required for general access — running a node, building a chain, writing dApps, contributing — lives in the public, open-source layer. Capabilities that require enterprise-grade structure (multi-tenant operations, compliance hooks, hardened KMS integrations, premium support) live in a separate **private enterprise layer** that is proprietary and never public.

### Brand Note

The name **Karoowa** evokes effortless harmony — the experience of using the framework should feel light, frictionless, and consistent across the hobbyist, team, and enterprise tiers. ("Light enough to launch anything.")

### Expected Business Outcomes

- **Hobbyist:** A solo dev with no organisation behind them can install Karoowa with one command and run a usable local node within minutes.
- **Team:** A small team can go from `git clone` to a running 4-validator devnet with metrics and Grafana dashboards in **under 15 minutes** on a clean Linux box.
- **Enterprise:** An ops team can stand up a permissioned chain with the proprietary enterprise layer enabled, including features unavailable in the open-source distribution.
- **Plugin extensibility:** A developer can implement a custom consensus engine by implementing a single `ConsensusEngine` trait, without forking the framework.
- **Agent-native operation:** Each persona has a corresponding AI agent that can perform its core duties autonomously — node operation, chain scaffolding, dApp integration, contributor onboarding — built incrementally as the underlying infrastructure for each persona ships.
- **Commercial sustainability:** The enterprise layer generates revenue sufficient to fund ongoing maintenance of the open-source layer.
- **v1.0 mainnet-ready** with a completed external audit, governance module, and at least one reference chain running in production.
- **Community traction:** A meaningful external contributor base (PRs merged, stars, reference deployments) sufficient to validate Karoowa as a credible alternative to Substrate / Cosmos SDK in the Rust ecosystem.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| Time from `git clone` to running devnet (clean Linux box) | < 15 min | Unmeasured |
| `cargo test --workspace` runtime | < 5 min on commodity laptop | Unmeasured |
| Devnet block time (PoA, 4 validators) | ≤ 2s, p99 < 5s | Unmeasured |
| JSON-RPC p99 latency for `kw_getBalance` (warm cache) | < 50ms | Unmeasured |
| External contributors with merged PRs (12 months post v1.0) | ≥ 20 | 0 |
| GitHub stars / forks (12 months post v1.0) | TBD | 0 |
| Reference chains running Karoowa in production | ≥ 1 by v1.0 | 0 |
| External audit findings (high severity) at v1.0 | 0 unresolved | N/A |

> **ASSUMPTION — needs confirmation:** Targets above are placeholder numbers based on what feels credible for a Rust framework of this scope. Replace with team-validated numbers. Also note: the inherited README uses a `cf_*` JSON-RPC prefix; this PRD assumes Karoowa adopts a `kw_*` prefix to match the rename. Confirm the desired prefix.

### User Problems

Karoowa addresses pain points for developers who want to launch or operate a custom chain but find existing options unsatisfying:

- **Substrate is heavy and opinionated.** Devs report a steep learning curve, FRAME-specific abstractions, and tight coupling to the Polkadot ecosystem. Teams that don't want to be in that ecosystem still pay its complexity tax.
- **Cosmos SDK is Go, not Rust.** Teams that want Rust's safety and performance guarantees for crypto/storage code have to choose between language and framework.
- **Rolling your own is a year of yak-shaving.** Building consensus, storage, networking, RPC, and tooling from scratch — even with good libraries (libp2p, RocksDB, ed25519-dalek) — takes a small team 12+ months before they're doing anything novel.
- **No coherent developer experience.** Most existing toolchains require a separate CLI, separate SDK, separate node binary, separate devnet tooling — each with its own config and conventions.
- **Operational gaps in dev tooling.** Devnet bring-up, key management, metrics, and health endpoints are usually afterthoughts that teams reinvent badly.

> **ASSUMPTION — needs confirmation:** These problems are inferred from the project's positioning (Rust, batteries-included, single CLI, Docker devnet) and from general knowledge of the Rust blockchain ecosystem. They have **not** been validated through user research. Validating these via 5–10 conversations with target devs is a prerequisite to taking any of this seriously.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-001 | We believe that **shipping a single Rust workspace with consensus, storage, network, RPC, and CLI** for **small chain-builder teams** will **cut their time-to-devnet from months to under a day**, measured by **time-to-first-block on a clean machine** | Time from `git clone` to first block produced | Run the quickstart on a clean Linux VM end-to-end and time it; ask 3 external devs to do the same |
| H-002 | We believe that **trait-based pluggable consensus** for **chain-builder teams** will **let them implement custom consensus without forking the framework**, measured by **external implementations of `ConsensusEngine` that compile against an upstream `karoowa-consensus`** | Count of external `ConsensusEngine` implementations | Publish trait + reference PoA impl, recruit 1–2 external teams to implement a custom engine |
| H-003 | We believe that **Rust + RocksDB + libp2p** will **outperform Go-based Cosmos SDK** on **crypto-heavy and storage-heavy workloads**, measured by **transactions/sec and state-read latency on a standard benchmark** | TPS and state-read p99 vs. a Cosmos SDK reference chain | Build a benchmark suite; publish results |
| H-004 | We believe that **a credible v1.0 with audit + governance** will **attract a contributor community comparable to mid-tier Rust infra projects**, measured by **external PRs merged and reference deployments** | External contributors, reference deployments | Track post-v1.0 GitHub metrics for 12 months |
| H-005 | We believe that **shipping AI agent operators alongside each persona-facing capability** will **make Karoowa meaningfully easier to operate than alternatives**, measured by **reduction in manual ops actions per node-day on a reference deployment** | Manual ops actions / node-day; agent-handled-vs-human-handled incident ratio | Compare a Karoowa devnet with agent operators enabled vs. disabled, both run for 7 days under the same workload |
| H-006 | We believe that **an open-core split with a private enterprise layer** will **generate revenue sufficient to sustain the open-source project** without alienating OSS contributors, measured by **enterprise customers acquired and OSS contributor retention** | Enterprise revenue; external PR rate before vs. after enterprise layer launch | Track contributor PR rate quarterly; track enterprise sales pipeline once layer is published |

---

## 2. User Stories & User Flows

### Primary Personas

Each persona below has a **human form** (the user) and an **agent form** (an AI agent shipped inside Karoowa that performs the persona's core duties autonomously). Agent forms are built sequentially alongside the infrastructure that the human persona depends on — see REQ-011.

| Persona | Human Form | Agent Form (product feature) |
|---------|-----------|-----------------------------|
| **Solo / Hobbyist Operator** — Individual with no organisation, running a Karoowa node on a laptop or VPS, possibly hosting a small app on shared public infra. Cares about install ergonomics, sane defaults, low resource footprint. | Drives single-binary install, sane defaults, minimal config, low-RAM mode, public devnet onboarding. | **Onboarding Agent** — guides install, key generation, first-block, troubleshooting. |
| **Chain Builder** — Small team (1–5 devs) launching a new app-specific or permissioned chain. Comfortable in Rust, wants to focus on their domain logic, not reinvent consensus or storage. | Drives the framework, CLI, devnet tooling, SDK. | **Scaffolding Agent** — generates new chain skeletons, custom consensus stubs, genesis configs. |
| **Validator Operator** — Operates a node on someone else's Karoowa network. Cares about reliability, observability, key management, and ops ergonomics. | Drives node binary UX, metrics, health endpoints, Docker images, key management. | **Operator Agent** — monitors node health, applies routine remediations, escalates incidents, rotates keys safely. |
| **dApp / Client Developer** — Builds applications that read/write to a Karoowa chain. Cares about API stability and SDK ergonomics. | Drives JSON-RPC, REST, WebSocket, and SDK design. | **Integration Agent** — generates client code, signs and submits transactions, debugs failed receipts. |
| **Open-Source Contributor** — Rust developer contributing to the framework itself. Cares about clean trait boundaries, test coverage, contribution ergonomics. | Drives workspace structure, trait design, CI, docs, contribution flow. | **Contributor Agent** — triages issues, runs lints/tests on draft PRs, suggests fixes. |
| **Enterprise Operator** — Ops/platform engineer deploying Karoowa with the proprietary enterprise layer enabled (multi-tenant, compliance, hardened KMS, premium support). | Drives the private enterprise layer: tenancy, audit logging, KMS integration, RBAC, support workflows. | **Compliance Agent** *(enterprise-only)* — produces audit reports, monitors policy compliance, gates risky operations. |
| **Security Auditor** *(later milestones)* | Drives audit-readiness: deterministic builds, reproducible tests, threat model docs, fuzz coverage. | **Audit Assistant Agent** — runs continuous fuzzing, drift checks against threat model. |

> **Agent personas are a top-line product feature.** They are not separate from the human personas — they are a parallel implementation track. When we ship a new capability for a human persona, we ship the corresponding agent capability in the same milestone where it's reasonable to do so.

### User Stories

| ID | User Story | Spec Reference |
|----|-----------|----------------|
| US-001 | As a **Chain Builder**, I want to spin up a single-node devnet with one command, so that I can start experimenting in minutes. | README §Quick Start; `karoowa node` CLI |
| US-002 | As a **Chain Builder**, I want to spin up a multi-validator devnet via Docker Compose, so that I can test consensus and networking realistically. | README §Docker 4-Validator Devnet |
| US-003 | As a **Chain Builder**, I want to plug in a custom consensus algorithm by implementing a trait, so that I don't have to fork the framework. | `karoowa-consensus` crate; `ConsensusEngine` trait |
| US-004 | As a **Chain Builder**, I want a deterministic genesis configuration tool, so that I can reproducibly bootstrap a network across machines. | `karoowa genesis` CLI |
| US-005 | As a **dApp Developer**, I want a Rust SDK to query chain state and submit transactions, so that I don't have to hand-roll JSON-RPC clients. | `karoowa-sdk` crate |
| US-006 | As a **dApp Developer**, I want JSON-RPC, REST, and WebSocket endpoints on a single port, so that I can choose the right interface per use case without managing multiple addresses. | `karoowa-api` crate; README §JSON-RPC |
| US-007 | As a **dApp Developer**, I want to subscribe to new blocks, pending transactions, and contract events over WebSocket, so that my application reacts to chain state in real time. | `kw_subscribe` methods (v0.2 roadmap) |
| US-008 | As a **Validator Operator**, I want a `/health` endpoint and Prometheus metrics, so that I can monitor node liveness and performance. | README §Endpoints |
| US-009 | As a **Validator Operator**, I want secure key management with OS-entropy-derived keys, so that I'm not generating validator keys with weak randomness. | `karoowa wallet new`; `karoowa-crypto` |
| US-010 | As a **Validator Operator**, I want a hardened Docker image and Compose setup, so that I can deploy nodes without bespoke ops work. | `Dockerfile`, `docker-compose.yml` |
| US-011 | As a **Chain Builder**, I want a WASM smart-contract VM with an ABI encoder/decoder, so that users of my chain can deploy contracts. | v0.3 roadmap |
| US-012 | As a **Chain Builder**, I want state sync and light-client support, so that new nodes can join without replaying every block from genesis. | v0.4 roadmap |
| US-013 | As a **Chain Builder**, I want cross-chain bridge primitives and an IBC adapter, so that my chain can interoperate with the wider ecosystem. | v0.5 roadmap |
| US-014 | As a **Chain Builder**, I want a governance module and a completed external audit at v1.0, so that I can defensibly run Karoowa on mainnet. | v1.0 roadmap |
| US-015 | As an **Open-Source Contributor**, I want a clean workspace with crate boundaries, `cargo test --workspace` passing, and CI on every PR, so that I can contribute confidently. | `Cargo.toml` workspace; README §Contributing |
| US-016 | As a **Solo / Hobbyist Operator**, I want a one-command install (e.g. `curl ... \| sh` or a single static binary release), so that I can run a Karoowa node without learning Cargo or Docker. | New — hobbyist tier |
| US-017 | As a **Solo / Hobbyist Operator**, I want a low-resource mode with sane defaults, so that I can run a node on a laptop or small VPS without exhausting RAM/disk. | New — hobbyist tier |
| US-018 | As a **Solo / Hobbyist Operator**, I want a public devnet I can join in one command, so that I can experiment without bootstrapping my own network. | New — hobbyist tier |
| US-019 | As an **Enterprise Operator**, I want the proprietary enterprise layer (multi-tenancy, RBAC, audit log, KMS integration, compliance hooks) gated behind a license check, so that my deployment meets internal compliance and support obligations. | New — enterprise tier |
| US-020 | As a **Chain Builder or dApp Developer**, I want EVM-bytecode compatibility (eventually), so that my chain can host existing Solidity contracts without users learning a new toolchain. | New — milestone TBD |
| US-021 | As a **Solo / Hobbyist Operator**, I want an Onboarding Agent that walks me through install and first-block, so that I can recover from common mistakes without reading the full docs. | Agent persona — paired with M1/M2 |
| US-022 | As a **Validator Operator**, I want an Operator Agent that monitors my node and applies routine remediations, so that I'm not paged for problems the system can fix itself. | Agent persona — paired with mempool/observability work |
| US-023 | As a **Chain Builder**, I want a Scaffolding Agent that generates new chain skeletons, consensus stubs, and genesis configs, so that I can start a new chain by describing what I want. | Agent persona — paired with M2/M3 |
| US-024 | As a **dApp Developer**, I want an Integration Agent that generates SDK client code and helps debug failed transactions, so that I can integrate faster. | Agent persona — paired with SDK work |
| US-025 | As an **Open-Source Contributor**, I want a Contributor Agent that triages issues and runs draft-PR checks, so that maintainer attention is focused on the things only humans can decide. | Agent persona — paired with REQ-009 |
| US-026 | As an **Enterprise Operator**, I want a Compliance Agent that produces audit reports and gates risky operations, so that I can demonstrate compliance posture to auditors on demand. | Agent persona — enterprise layer |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **Hobbyist install** | Run a single install command → start `karoowa` with defaults → join the public devnet → see first block within minutes | Solo / Hobbyist Operator |
| **Single-node bring-up** | Generate a validator key → start `karoowa node` with PoA → produce first block → query via REST/RPC | Chain Builder |
| **Devnet bring-up** | Generate 4 validator keys → write `.env` → `docker compose -f devnet.yml up` → 4 validators reach consensus → observe via Grafana | Chain Builder |
| **Custom consensus** | Implement `ConsensusEngine` trait in a downstream crate → wire into a node binary → run alongside reference PoA/PoS engines | Chain Builder |
| **Client integration** | Use `karoowa-sdk` to construct + sign a transfer → submit via `kw_sendRawTransaction` → poll receipt → confirm balance change | dApp Developer |
| **WebSocket subscriptions** | Connect to `/ws` → subscribe to `newBlocks` → receive push notifications on each block | dApp Developer |
| **Smart contract deployment** *(v0.3)* | Compile contract to WASM → deploy via SDK → invoke via ABI → query state | Chain Builder + dApp Developer |
| **EVM contract deployment** *(milestone TBD)* | Compile Solidity → deploy via existing EVM tooling pointed at a Karoowa node → invoke and query | Chain Builder + dApp Developer |
| **Validator operations** | Deploy node via Docker → expose metrics to Prometheus → set up Grafana dashboards → respond to alerts | Validator Operator |
| **Enterprise deployment** | Provision enterprise-layer Karoowa with license key → enable multi-tenancy + RBAC + audit log + KMS integration → onboard tenants | Enterprise Operator |
| **Agent-assisted onboarding** | Hobbyist invokes the Onboarding Agent → agent runs install, generates keys, joins devnet, explains errors in natural language | Solo / Hobbyist Operator + Onboarding Agent |
| **Agent-assisted operations** | Operator Agent monitors node → detects degraded state → applies remediation playbook → escalates only when human input needed | Validator Operator + Operator Agent |

---

## 3. High-Level Requirements

This is an overarching PRD, so requirements are stated at the milestone level. Each milestone will be decomposed into a feature PRD with its own detailed BDD scenarios.

### Milestone Map

| Milestone | Roadmap Tag | Scope Summary | Status |
|-----------|------------|--------------|--------|
| M1 | v0.1 | Core primitives, PoA consensus, RocksDB storage, JSON-RPC/REST/WS gateway, Docker single-node + 4-validator devnet, CLI (`node`, `wallet`, `devnet`, `client`, `genesis`, `network`), hobbyist install, public devnet, M1 Dev agent bundle | **Greenfield — net-new implementation.** The README + Cargo manifest in `files/` were a design sketch, not code. Tracked in `specs/development/dev_plan.md` (Phases 1.0 → 1.11). |
| M2 | v0.2 | Full BFT consensus, PoS engine, mempool, WebSocket subscriptions, M2 Ops agent bundle, sidecar runtime | Tracked in `specs/development/dev_plan.md` (Phases 2.0 → 2.7). |
| M3 | v0.3 | WASM smart-contract VM, ABI encoder/decoder, contract SDK, M3 Security/Optimization agent bundle, sidecar runtime mandatory | Tracked in `specs/development/dev_plan.md` (Phases 3.0 → 3.7). |
| M4 | v0.4 | State sync protocol, light-client support, EIP-compatible tx format | Planned |
| M5 | v0.5 | Cross-chain bridge primitives, IBC adapter | Planned |
| M6 | v1.0 | Mainnet-ready: external audit, governance module, reference deployment | Planned |

### Requirements Table

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-001 | M1 (v0.1) — Core primitives, PoA consensus, storage, API gateway, CLI, Docker devnet | US-001, US-002, US-004, US-006, US-008, US-009, US-010, US-015 | Must Have | See below |
| REQ-002 | M2 (v0.2) — Mempool, WebSocket subscriptions, full BFT consensus, PoS engine | US-003, US-007 | Must Have | See below |
| REQ-003 | M3 (v0.3) — WASM smart-contract VM, ABI encoder/decoder, contract SDK | US-011 | Should Have | See below |
| REQ-004 | M4 (v0.4) — State sync, light-client support, EIP-compatible tx format | US-012 | Should Have | See below |
| REQ-005 | M5 (v0.5) — Cross-chain bridge primitives, IBC adapter | US-013 | Could Have | See below |
| REQ-006 | M6 (v1.0) — Governance module, external audit, reference mainnet deployment | US-014 | Must Have for v1.0 | See below |
| REQ-007 | Pluggable `ConsensusEngine` trait — every consensus engine implements the same trait so downstream teams can plug in their own without forking | US-003 | Must Have | See below |
| REQ-008 | Single-port multi-protocol API gateway — JSON-RPC, REST, and WebSocket on one port | US-006, US-007 | Must Have | See below |
| REQ-009 | Workspace + contribution health — `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` pass on every PR | US-015 | Must Have | See below |
| REQ-010 | EVM bytecode compatibility — Karoowa nodes can host and execute Solidity contracts via existing EVM tooling. Milestone TBD; tracked here so it isn't lost. | US-020 | Should Have (milestone TBD) | See below |
| REQ-011 | Agent-native operation — Agents ship in capability bundles per milestone. **M1 Dev bundle:** CLI/Dev Agent + basic Monitoring Agent (covers Onboarding Agent persona). **M2 Ops bundle:** CI/CD & Deployment Agent + Observability Agent (covers Operator + Scaffolding Agent personas). **M3 Security/Optimization bundle:** Vulnerability Scanner + Auto-Scaling/Gas Optimizer (covers Integration + Contributor Agent personas). **M4 Enterprise bundle:** Governance/Policy Agent + Finance/Treasury Agent (covers Compliance Agent persona, gated to enterprise layer). | US-021..US-026 | Must Have (per-milestone) | See below |
| REQ-012 | Open-core boundary — Karoowa is split into a public OSS layer and a private enterprise layer. **OSS (public):** core blockchain nodes & consensus, agent runtime framework (without advanced AI policy/governance), basic on-chain oracles, dev CLI, SDKs, M1–M3 agent bundles. **Enterprise (proprietary, license-gated):** agent governance/policy engine, high-availability nodes, multi-tenancy, advanced analytics, GUI dashboards, SSO/SAML/RBAC, MPC key management, audit/compliance tooling, M4 Governance + Finance agents, custom SLAs and premium support. **Topology:** single monorepo with `core/` and `enterprise/` top-level directories (Strapi-style). **Enforcement:** (1) CI guardrails fail builds where any `core/` file imports from `enterprise/`; (2) community builds explicitly exclude `enterprise/`; (3) enterprise features require a signed license file at startup; (4) commercial EULA covers legal usage. Documented in `LICENSE`, root `README.md`, and module-level READMEs. | US-019 | Must Have before any enterprise feature ships | See below |
| REQ-013 | Hobbyist install path — A working `karoowa` binary is reachable via **multiple channels** without requiring Cargo, Docker, or build toolchains: (a) one-line shell installer (`curl -fsSL install.karoowa.io \| sh`); (b) prebuilt static binaries on GitHub Releases for Linux x86_64/aarch64, macOS, Windows; (c) package managers (Homebrew, APT, RPM, Chocolatey, Scoop); (d) optional Docker image for containerised installs. | US-016, US-017, US-018 | Must Have for hobbyist tier | See below |
| REQ-014 | Pluggable LLM provider — Agents are decoupled from any specific LLM provider via a `LlmProvider` trait abstraction. **Launch providers:** **Anthropic** (hosted), **OpenAI** (hosted), **Google Gemma 4** via local backend (`ollama` / `llama.cpp`) — sizes E2B (5B), E4B (8B), 26B, 31B; Apache 2.0; GGUF on HuggingFace — and a **generic GGUF local-model** provider for any compatible model. Provider is selectable per-agent via config without recompiling. **Hobbyist default:** hosted provider with API key (Anthropic recommended); **no-key fallback:** Gemma 4 E2B running locally via `ollama` at documented degraded capability. | US-021..US-026 | Must Have for any agent | See below |
| REQ-015 | Hybrid agent runtime — agents run in one of three modes selected per deployment: **(a) in-process** (M1 hobbyist default; agent runs inside the `karoowa` binary, calls a hosted LLM, fits low-end hardware); **(b) sidecar** (separate process, loopback-only proxy with auth + quota, the "padded room" pattern; recommended for ≥8 GB hosts; **mandatory at M3**); **(c) cloud-hosted runtime** (enterprise capability, agent runs in Karoowa-managed infra). | US-021..US-026 | Must Have | See below |
| REQ-016 | Public Karoowa Devnet — A foundation-operated public devnet exposes RPC, WebSocket, and a faucet so hobbyists can join in one command. Progression: **Devnet → Public Testnet → Mainnet** with SLOs of **99.5% / 99.9% / 99.95%** respectively. A named **Karoowa Infrastructure Lead** owns uptime and a budget line item funds it. | US-018 | Must Have for hobbyist tier | See below |
| REQ-017 | Database strategy — Karoowa adopts a **layered storage architecture** with deliberate, minimal database choices: **(L1 hot path)** RocksDB for blocks, state, receipts, and tx index — inherited and load-bearing, kept. **(L2 indexing, optional)** PostgreSQL for derived data needed by dApp queries and a future block explorer; introduced only when an explicit consumer requires it. **(L3 agent memory)** **LanceDB** (Apache 2.0, embedded, in-process) for agent RAG and memory. **(L4 telemetry)** Prometheus for metrics (already present); ClickHouse only if v1.0 surfaces a concrete need (deferred). All four layers are abstracted behind crate-level traits so individual implementations can be swapped (e.g. LanceDB → Qdrant) without leaking into agent or consensus code. | US-022, US-024 | Must Have (L1 already met) | See below |

### BDD Scenarios

#### REQ-001: M1 (v0.1) — Core node up and running

**Happy Path:**

**Scenario: Single-node devnet produces its first block**
**Given** a clean Linux machine with Rust 1.78+ and Docker installed
**And** the Karoowa repository has been cloned and built with `cargo build --release`
**When** the operator generates a validator key with `karoowa wallet new`
**And** starts a node with `karoowa node --validator-key <key> --consensus poa --data-dir ./data`
**Then** the node exposes `/health` returning HTTP 200 within 10 seconds
**And** the JSON-RPC method `kw_blockNumber` returns a value greater than 0 within 30 seconds
**And** Prometheus metrics are exposed on port 9090

**Scenario: 4-validator Docker devnet reaches consensus**
**Given** four validator keys have been generated and exported to `docker/.env`
**When** the operator runs `docker compose -f docker/devnet.yml up -d`
**Then** all four validator containers report healthy within 60 seconds
**And** each validator's `kw_blockNumber` advances at the configured block time
**And** all four validators agree on the latest block hash
**And** the Grafana dashboard at `http://localhost:3000` shows live block production

**Sad Paths** *(to be added during refinement)*

#### REQ-002: M2 (v0.2) — Mempool, WebSocket subscriptions, BFT/PoS

**Happy Path:**

**Scenario: dApp subscribes to new blocks over WebSocket**
**Given** a Karoowa node is producing blocks
**And** a dApp client has connected to `ws://node:8545/ws`
**When** the client sends `{"id":1,"method":"kw_subscribe","params":["newBlocks"]}`
**Then** the client receives a subscription confirmation with a subscription ID
**And** the client receives a push notification containing the block header within one block-time of every new block produced

**Scenario: Pending transaction enters the mempool and is broadcast**
**Given** a validator node with peers connected
**When** a client submits a signed transaction via `kw_sendRawTransaction`
**Then** the transaction appears in `kw_pendingTransactions` within 1 second
**And** peer nodes report the same transaction in their mempool within 5 seconds
**And** the transaction is included in a block within the next 3 blocks

**Sad Paths** *(to be added during refinement)*

#### REQ-003: M3 (v0.3) — WASM smart contracts

**Happy Path:**

**Scenario: Developer deploys a WASM contract and invokes it**
**Given** a Karoowa node running with the WASM VM enabled
**And** a compiled WASM contract artifact
**When** the developer submits a contract-deployment transaction via the SDK
**Then** the transaction is mined and a receipt with a contract address is returned
**And** invoking a method via the ABI encoder returns the expected value
**And** the contract's storage is queryable via `kw_getCode` and state-read RPCs

**Sad Paths** *(to be added during refinement)*

#### REQ-004: M4 (v0.4) — State sync and light clients

**Happy Path:**

**Scenario: A new full node joins via state sync instead of full replay**
**Given** an existing Karoowa network with N blocks of history
**When** a new node is started with `--sync-mode state-sync`
**Then** the new node downloads a recent state snapshot from peers
**And** reaches the chain head in significantly less time than full replay would take
**And** subsequent block validation continues normally

**Sad Paths** *(to be added during refinement)*

#### REQ-005: M5 (v0.5) — Cross-chain bridge primitives

**Happy Path:**

**Scenario: A token is locked on chain A and minted on chain B via the bridge**
**Given** two Karoowa chains running the bridge module
**When** a user submits a lock-and-mint transaction on chain A
**Then** chain A emits a bridge-out event
**And** the bridge relayer observes the event
**And** chain B mints the corresponding wrapped asset to the destination address within the configured finality window

**Sad Paths** *(to be added during refinement)*

#### REQ-006: M6 (v1.0) — Mainnet readiness

**Happy Path:**

**Scenario: Governance proposal is submitted, voted on, and executed on-chain**
**Given** a v1.0 Karoowa network with the governance module enabled
**When** a token holder submits a parameter-change proposal
**Then** the proposal enters the voting period
**And** validators cast votes until the quorum and threshold are met
**And** the parameter change is applied at the configured execution height
**And** subsequent blocks reflect the new parameter value

**Scenario: External audit produces no unresolved high-severity findings**
**Given** the v1.0 release candidate has been frozen
**When** an external security audit is conducted
**Then** all high-severity findings are either resolved or have an accepted documented mitigation
**And** the audit report is published alongside the v1.0 release

**Sad Paths** *(to be added during refinement)*

#### REQ-007: Pluggable `ConsensusEngine` trait

**Happy Path:**

**Scenario: A downstream team implements a custom consensus engine**
**Given** a downstream Rust crate that depends on `karoowa-consensus`
**When** a developer implements `ConsensusEngine` for a custom struct `MyEngine`
**And** wires `MyEngine` into a custom node binary
**Then** the custom binary compiles against an unmodified upstream `karoowa-consensus`
**And** runs alongside reference PoA and PoS engines on a devnet
**And** produces blocks validated by other Karoowa nodes running the same engine

**Sad Paths** *(to be added during refinement)*

#### REQ-008: Single-port multi-protocol API gateway

**Happy Path:**

**Scenario: All three API protocols are reachable on a single port**
**Given** a running Karoowa node bound to port 8545
**When** a client sends a JSON-RPC POST to `http://node:8545/rpc`
**Then** the client receives a valid JSON-RPC 2.0 response
**And** a REST GET to `http://node:8545/api/v1/status` returns HTTP 200 with node status
**And** a WebSocket upgrade to `ws://node:8545/ws` succeeds
**And** all three protocols share the same underlying state and block height

**Sad Paths** *(to be added during refinement)*

#### REQ-009: Workspace + contribution health

**Happy Path:**

**Scenario: Contributor opens a PR and CI passes end-to-end**
**Given** a contributor has forked the repo and pushed a feature branch
**When** they open a pull request
**Then** CI runs `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`
**And** all checks pass within the configured time budget
**And** the PR is mergeable without manual intervention from maintainers beyond review

**Sad Paths** *(to be added during refinement)*

#### REQ-010: EVM bytecode compatibility

**Happy Path:**

**Scenario: Existing Solidity contract deploys and executes on a Karoowa node**
**Given** a Karoowa node running with the EVM execution environment enabled
**And** an unmodified Solidity contract compiled with a standard EVM toolchain
**When** a developer deploys the contract using existing EVM tooling pointed at the Karoowa node
**Then** the deployment transaction is mined and a contract address is returned
**And** invoking a contract method via the same tooling returns the expected value
**And** state queries via Karoowa's API surface return the contract's storage

**Sad Paths** *(to be added during refinement)*

> **Open question:** Which milestone does this slot into? See OQ-006.

#### REQ-011: Agent-native operation

**Happy Path:**

**Scenario: Onboarding Agent walks a hobbyist through first-block**
**Given** a Solo Operator on a clean machine with the `karoowa` binary installed
**When** they run `karoowa agent onboard`
**Then** the agent generates a wallet key
**And** joins the public devnet
**And** waits for the first block to be observed
**And** confirms success in natural language
**And** if any step fails, the agent diagnoses the failure and proposes a fix without escalating to docs

**Scenario: Operator Agent applies a remediation without paging a human**
**Given** a Karoowa node running with the Operator Agent enabled
**And** a known-remediable failure mode (e.g. peer connection drop, disk pressure on logs)
**When** the failure occurs
**Then** the agent detects the condition from metrics within one observation window
**And** applies the matching remediation playbook
**And** records the action in an audit log
**And** does not escalate unless the remediation fails or the failure is outside the playbook

**Scenario: Scaffolding Agent generates a new chain skeleton**
**Given** a Chain Builder describing a target chain in natural language
**When** they run `karoowa agent scaffold`
**Then** the agent produces a working Cargo workspace with the appropriate consensus engine, genesis config, and crate scaffolding
**And** the generated workspace builds and runs a single-node devnet without further edits

**Sad Paths** *(to be added during refinement)*

#### REQ-012: Open-core boundary

**Happy Path:**

**Scenario: OSS distribution does not contain enterprise code**
**Given** the public Karoowa OSS distribution
**When** a contributor inspects the source tree
**Then** no enterprise-layer modules (multi-tenancy, RBAC, audit logging, KMS integration, Compliance Agent) are present in the public repository
**And** all features available in the OSS distribution are documented as such
**And** no OSS feature requires a license check to function

**Scenario: Enterprise feature is gated behind a license check**
**Given** a Karoowa binary built with the enterprise layer linked in
**When** an operator attempts to enable an enterprise feature without a valid license
**Then** the feature refuses to start
**And** a clear error message points the operator to licensing
**And** the OSS portion of the binary continues to function normally

**Sad Paths** *(to be added during refinement)*

#### REQ-013: Hobbyist install path

**Happy Path:**

**Scenario: Solo operator installs Karoowa with a single command**
**Given** a clean Linux x86_64 machine with no Rust toolchain and no Docker
**When** the operator runs the documented one-command install
**Then** a `karoowa` binary is installed on PATH within 60 seconds
**And** `karoowa --version` returns a valid semver string
**And** `karoowa node --join public-devnet` produces a synced node observing blocks within 5 minutes

**Scenario: macOS user installs Karoowa via Homebrew**
**Given** a macOS machine with Homebrew installed
**When** the user runs `brew install karoowa`
**Then** the `karoowa` binary is installed and on PATH
**And** `brew upgrade karoowa` updates to the latest release without manual intervention

**Sad Paths** *(to be added during refinement)*

#### REQ-014: Pluggable LLM provider

**Happy Path:**

**Scenario: Operator switches an agent from a hosted to a local LLM**
**Given** an Operator Agent currently configured with the Anthropic hosted provider
**When** the operator edits the agent config to use the local `ollama` provider with a specified model
**And** restarts the agent
**Then** the agent starts successfully using the local provider
**And** subsequent agent decisions are produced by the local model
**And** no code changes were required to switch providers

**Scenario: A new provider is added without modifying existing agents**
**Given** the `LlmProvider` trait and at least two existing implementations
**When** a contributor adds a new provider implementation in a downstream crate
**Then** any existing agent can use the new provider purely via configuration
**And** the existing provider implementations are unchanged

**Sad Paths** *(to be added during refinement)*

#### REQ-015: Hybrid agent runtime

**Happy Path:**

**Scenario: Hobbyist runs the Onboarding Agent in-process on a 4 GB VPS**
**Given** a Solo Operator on a 4 GB VPS with the freshly installed `karoowa` binary
**And** the hobbyist has provided a hosted LLM API key (or accepted the small-local-model fallback)
**When** they run `karoowa agent onboard`
**Then** the agent runs inside the `karoowa` binary without spawning a separate sidecar
**And** the combined resident memory of node + agent stays under 1.5 GB
**And** the agent completes the onboarding flow successfully
**And** a warning is logged that in-process mode is intended for hobbyist use and is not recommended beyond M2

**Scenario: Operator runs an agent as an isolated sidecar process**
**Given** a Karoowa node and a sidecar-mode Operator Agent on a host with at least 8 GB RAM
**When** the agent starts
**Then** the agent runs in its own process with no direct file or network access to the node beyond the loopback proxy
**And** all node API calls from the agent are mediated by the proxy with auth and quota enforcement
**And** killing the agent process does not affect node liveness

**Scenario: M3 deployment refuses in-process mode**
**Given** a Karoowa node built from an M3 release
**When** an operator attempts to start an agent in `--mode in-process`
**Then** the node refuses to start the agent
**And** logs a clear error pointing to sidecar or cloud-hosted runtime as the supported options

**Scenario: Enterprise customer runs agents in Karoowa cloud-hosted runtime**
**Given** an enterprise customer with a valid license file
**When** they configure agents with `--mode cloud-hosted`
**Then** agents are dispatched to the Karoowa-managed agent runtime
**And** node-side credentials remain on the customer's host
**And** agent decisions arrive via the same loopback proxy interface as sidecar mode

**Sad Paths** *(to be added during refinement)*

#### REQ-017: Database strategy

**Happy Path:**

**Scenario: Hot path uses RocksDB for blocks and state**
**Given** a running Karoowa node
**When** a new block is finalised
**Then** the block, its receipts, and updated state entries are persisted to RocksDB column families
**And** subsequent reads via the storage trait return the persisted values without going through any other backend

**Scenario: Storage backend is swappable behind a crate-level trait**
**Given** the `karoowa-storage` crate exposing a `BlockStore` and `StateStore` trait
**When** a contributor implements the traits with an alternative backend (e.g. `redb`, `sled`)
**Then** the alternative implementation can be wired into a node binary without modifying `karoowa-core` or `karoowa-consensus`
**And** the workspace test suite passes against either backend

**Scenario: Agent memory is stored in an embedded vector store**
**Given** an agent configured with the agent-memory backend enabled
**When** the agent records a context entry
**Then** the entry is embedded and persisted to the local vector store
**And** subsequent semantic queries from the same agent return the entry as a relevant result

**Sad Paths** *(to be added during refinement)*

> **Open question:** L2 (PostgreSQL indexing) and L3 (vector store choice) are still subject to OQ-025. They are listed in REQ-017 as the planned direction; the BDD scenario above for L3 assumes an embedded backend has been selected.

#### REQ-016: Public Karoowa Devnet

**Happy Path:**

**Scenario: Hobbyist joins the public devnet in one command**
**Given** a freshly installed `karoowa` binary
**When** the hobbyist runs `karoowa node --join public-devnet`
**Then** the node connects to the documented public devnet bootnodes
**And** begins syncing blocks within 30 seconds
**And** can request test tokens from the public faucet endpoint
**And** the devnet status page reports the connected node count incrementing

**Sad Paths** *(to be added during refinement)*

> **Operational note:** REQ-016 implies an ongoing operating cost (validators, RPC, faucet, monitoring). Owner and budget tracked in OQ-019 — must be assigned before launch.

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-001 | Performance | PoA devnet block time | ≤ 2s (p99 < 5s) |
| NFR-002 | Performance | JSON-RPC `kw_getBalance` p99 latency (warm cache) | < 50ms |
| NFR-003 | Performance | Workspace test suite runtime | < 5 min on commodity laptop |
| NFR-004 | Reliability | Node uptime under steady load | ≥ 99.9% over 7-day soak |
| NFR-005 | Reliability | Devnet recovery from single-validator restart | Network continues block production without manual intervention |
| NFR-006 | Security | All cryptographic operations use audited primitives (`ed25519-dalek`, SHA3, BLAKE3) | No hand-rolled crypto |
| NFR-007 | Security | Validator keys derived only from OS entropy (`OsRng`) | Enforced in code review + lint |
| NFR-008 | Security | External security audit completed before v1.0 | 0 unresolved high-severity findings |
| NFR-009 | Scalability | Network supports at least 100 connected peers per node via libp2p | Validated on devnet stress test |
| NFR-010 | Maintainability | All public APIs have rustdoc | Enforced via `#![deny(missing_docs)]` on public crates |
| NFR-011 | Portability | First-class support for Linux x86_64 and aarch64; macOS dev support best-effort | CI runs on both Linux targets |
| NFR-012 | Observability | Every node exposes Prometheus metrics on `/metrics` and a health probe on `/health` | Required in node binary by v1.0 |

> **ASSUMPTION — needs confirmation:** All numeric targets are placeholders. Replace once benchmarks are baselined.

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-001 | Karoowa serves three concurrent audience tiers: hobbyists, small chain-builder teams, and enterprises with permissioned-chain needs. Each tier has different ergonomics expectations and the framework must accommodate all three. | If any tier is dropped, packaging and docs simplify significantly | Track adoption in each tier post-release |
| ASM-002 | Open-core is the right model: public OSS for general access, private proprietary enterprise layer for compliance/multi-tenancy/support. Revenue from the enterprise layer funds OSS maintenance. | If OSS contributors react negatively to the boundary, contributor velocity may drop. If enterprise demand is weak, the model isn't sustainable. | Track contributor PR rate before/after enterprise layer launch; track enterprise sales pipeline |
| ASM-003 | RocksDB will scale through v1.0 workloads without a storage rewrite | If RocksDB hits scaling limits, M4 (state sync) and M6 may need re-architecture | Run sustained-write benchmarks at projected v1.0 scale |
| ASM-004 | libp2p Gossipsub + Kademlia is sufficient for the intended network sizes (≤ 1000 nodes) | If larger networks are required, networking layer needs additional protocols | Stress test on a synthetic large network |
| ASM-005 | A trait-based plugin model is sufficient for custom consensus, without requiring runtime loading or dynamic dispatch across process boundaries | If runtime loading is required (e.g., for closed-source plugins), the trait model is insufficient | Recruit 1–2 external teams to implement a custom engine and observe friction |
| ASM-006 | Audit budget and timeline are available for v1.0 | If not, v1.0 must be deferred or scope-cut | Confirm with sponsor; line up audit firm by M5 |
| ASM-007 | The team has capacity to maintain CI, docs, and release engineering through v1.0 | If not, contributor experience and release cadence degrade | Resource plan by milestone |
| ASM-008 | EVM compatibility **is** required, milestone TBD. The Karoowa-native API uses a `kw_*` prefix; EVM support will be additive (e.g. an `eth_*` shim and an EVM execution environment alongside WASM). | If EVM is needed earlier than planned, M2/M3 sequencing may shift to land it sooner | Confirm target milestone with sponsor |
| ASM-009 | Full rename ChainFlux → Karoowa: crate names (`karoowa-*`), CLI binary (`karoowa`), JSON-RPC method prefix (`kw_*`), Docker images, docs, and brand. Confirmed. | None — locked | N/A |
| ASM-010 | Agent capabilities (Onboarding, Operator, Scaffolding, Integration, Contributor, Compliance) are top-line product features built sequentially alongside the matching infrastructure, not deferred to a post-v1.0 add-on. | If agent work delays infrastructure work, milestone slip. If agent work is descoped, Karoowa loses a key differentiator. | Track agent capability shipment per milestone; user interviews on whether agents change purchase intent |
| ASM-011 | Hobbyist tier requires a single-binary install path with no Cargo / Docker prerequisites. macOS dev support is best-effort but install ergonomics target Linux first. | If hobbyists require Cargo, the audience tier collapses into "developers who already use Rust" | Test the install on a clean VM with no toolchain |
| ASM-012 | Sidecar-first is the right default agent runtime model. The "padded room" pattern (each agent in its own container/process, loopback-only proxy with auth + quota) is borrowed from production AI fleets and maps well onto Karoowa's security model. In-process mode exists only as a hobbyist convenience. | If sidecar overhead is too high for hobbyist machines, in-process becomes the default and security guarantees weaken | Benchmark sidecar memory/latency overhead on a low-end VM during M1 |
| ASM-013 | A pluggable LLM provider trait is sufficient — agents do not need to negotiate provider capabilities at runtime. Each agent declares its provider in config and the provider implements a uniform `LlmProvider` interface. | If agents need provider-specific features (function calling shapes, tool schemas), the abstraction leaks and the trait grows | Validate by implementing 3 providers (Anthropic, OpenAI, local llama.cpp) end-to-end before declaring REQ-014 done |
| ASM-014 | ~~Local-model inference via quantized 7B is good enough for M1/M2 agent capabilities.~~ **SUPERSEDED 2026-04-10 by ASM-014a after OQ-021/024 resolution.** | — | — |
| ASM-014a | Local-model inference for hobbyists targets **Gemma 4 E2B (5B params, Apache 2.0, GGUF via Ollama)** as the no-key fallback. The hobbyist default remains **hosted LLM with in-process agent**; the local fallback exists so hobbyists can opt out of an API key at the cost of degraded capability. E4B (8B) is offered where hardware allows. 26B/31B reserved for ≥enterprise hardware. | If even Gemma 4 E2B exceeds 4 GB VPS limits or is too weak for the Onboarding Agent, hobbyists must use a hosted provider, and ASM-011 (no API key required) becomes a soft promise rather than a hard guarantee | Prototyping spike per OQ-021/024: download Gemma 4 E2B GGUF, run on a 4 GB VM with `ulimit` cap, measure peak RAM, latency, and Onboarding Agent success rate on a fixed scenario set |
| ASM-015 | A core-team-operated public Karoowa devnet is required for hobbyist tier credibility. Operating cost is acceptable as a cost of acquisition. | If devnet ops cost is unsustainable, hobbyist onboarding regresses to "bring your own network" and adoption suffers | Cost model + ownership decision before M1 PRD finalised |
| ASM-016 | The agent capability bundling (M1 Dev → M2 Ops → M3 Security/Optimization → M4 Enterprise Governance) maps cleanly onto persona forms from §2 — i.e. capability bundles and persona-named agents are the same things organised two ways, not two parallel implementations. | If they diverge (e.g. an Onboarding Agent persona doesn't fit cleanly into the M1 Dev bundle), we duplicate work and confuse contributors | Cross-check during M1 PRD creation; explicitly map each persona-form agent to its capability bundle |
| ASM-017 | RocksDB remains the right hot-path storage engine for v1.0. It is battle-tested in production blockchains (Bitcoin Core, Geth, Solana, Cosmos), its LSM design fits the write-heavy block-append workload, and its column families let us cleanly separate blocks, state, receipts, and tx index. Pure-Rust alternatives (`sled`, `redb`) are interesting but not yet load-bearing-grade for our scale. | If RocksDB hits a scaling wall or compaction stalls become operationally painful, we have to migrate to Pebble or another LSM — expensive but possible because of REQ-017's storage trait abstraction | Soak test at projected v1.0 write rate during M2/M3 |
| ASM-018 | Agent memory and RAG use **LanceDB** (Apache 2.0, embedded, columnar/vector, runs in-process) for M1. Rationale: zero ops, smallest install footprint, fits hobbyist tier. Trade-offs accepted: weaker filtering than Qdrant, newer codebase. | If LanceDB cannot handle filtering needs (e.g. multi-tenant compliance queries) or has stability issues at scale, the enterprise tier may swap to Qdrant or Milvus — REQ-017's storage trait abstraction makes this cheap | Integrate LanceDB into a small spike during M1; benchmark against Onboarding Agent corpus; document the trait surface so a Qdrant alternative is straightforward |
| ASM-019 | PostgreSQL indexing (L2) is **deferred until an explicit consumer requires it**. We do not introduce Postgres just because dApps "might" want it — that's premature operational cost. The storage trait abstraction in REQ-017 makes adding it later cheap. | If a dApp on day one needs rich SQL queries that the JSON-RPC surface can't serve, we have to scramble to add Postgres reactively | Track dApp query requirements when each consumer integrates; add Postgres when the third unrelated request appears |
| ASM-020 | ClickHouse and other analytical OLAP stores are **not** in v1.0 scope. Prometheus + Grafana are sufficient for operator-side observability through v1.0. | If audit / compliance requirements at v1.0 demand long-horizon historical querying (e.g. multi-year trace replay), we may need ClickHouse or equivalent | Validate against compliance requirements once OQ-014's enterprise scope is locked |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-001 | Rust 1.78+ toolchain available on target Linux distros | Upstream Rust | Resolved | Build prerequisite |
| DEP-002 | `libp2p` crate stability through v1.0 timeframe | Upstream libp2p maintainers | In Progress | Networking layer |
| DEP-003 | `rocksdb` crate (Rust bindings) stability | Upstream | In Progress | Storage layer |
| DEP-004 | `ed25519-dalek`, `sha3`, `blake3` audited crypto crates | Upstream | Resolved | Crypto layer |
| DEP-005 | `axum` + `tokio` async runtime stability | Upstream | Resolved | API gateway |
| DEP-006 | External security audit firm engaged before v1.0 | TBD | Blocked — not yet sourced | M6 release gate |
| DEP-007 | CI infrastructure (GitHub Actions or equivalent) configured for the workspace | Karoowa team | TBD | Contribution health (REQ-009) |
| DEP-008 | Reference WASM contract toolchain (likely `cargo-contract` or equivalent) for M3 | TBD | TBD | M3 deliverable |
| DEP-009 | Rename from ChainFlux → Karoowa applied across crate names, CLI binary, RPC prefix, Docker tags, and docs | Karoowa team | Not started | Branding consistency, API stability |

### Exclusions

| Item | Rationale |
|------|-----------|
| EVM bytecode compatibility | Karoowa defines its own `kw_*` API surface and (in M3) uses WASM, not EVM. Adding EVM compatibility would substantially expand scope. |
| Mobile SDK / wallet | Out of scope for Phase 1; the focus is the chain framework and Rust SDK. |
| Hosted / managed-service offering | Karoowa is a framework, not a service. |
| Browser-based block explorer | Not in roadmap through v1.0; Grafana provides operator-side observability. |
| Tokenomics, validator incentive design, and economic security modeling | Chain operators design their own tokenomics; Karoowa provides primitives, not policy. |
| Full formal verification of consensus | An external audit is in scope; formal verification is not. |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Architecture diagram | Inherited `README.md` (ASCII diagram) | Needs first-class architecture doc |
| Workspace structure | `Cargo.toml` | Authoritative (pending rename) |
| Build / dev workflow | `Makefile`, `Dockerfile`, `docker-compose.yml` | Authoritative (pending rename) |
| Detailed technical specs per crate | TBD — to be created during plan step | Not started |

> **Note:** Technical and architecture documents live separately from this PRD. The next step after this PRD is approved is to scaffold `specs/architecture/` with per-crate technical specs.

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-001 | What is the actual target audience? | Sponsor | — | **Resolved 2026-04-09** — Three concurrent tiers: (a) hobbyists / individuals, (b) small Rust-comfortable chain-builder teams, (c) enterprise teams with permissioned-chain needs | Resolved |
| OQ-002 | Is there a monetization or commercial model in scope? | Sponsor | — | **Resolved 2026-04-09** — Open-core: public OSS layer for general access, private proprietary enterprise layer for compliance / multi-tenancy / hardened operations / paid support | Resolved |
| OQ-003 | What does success look like at v1.0? | Sponsor | — | **Resolved 2026-04-09** — All of (a) published framework with adoption, (b) reference chain in production, (c) external audit, (d) contributor community, (e) revenue / customers. Each milestone celebrated independently. | Resolved |
| OQ-004 | Which BFT algorithm for M2 — Tendermint-style, HotStuff, or something else? | Tech lead | Before M2 PRD | — | Open |
| OQ-005 | Which WASM runtime for M3 — `wasmtime`, `wasmer`, or a custom interpreter? | Tech lead | Before M3 PRD | — | Open |
| OQ-006 | EVM compatibility is confirmed required (REQ-010). Which milestone should it land in? Options: bundled with M3 (alongside WASM VM) as a parallel execution environment, or its own dedicated milestone. | Sponsor + Tech lead | Before M3 PRD | — | Open |
| OQ-007 | Who is the PRD owner and decision-maker for trade-offs? | — | TBD | — | Open |
| OQ-008 | What is the team size and capacity? Drives milestone sequencing. | — | TBD | — | Open |
| OQ-009 | Audit firm and budget for v1.0 — when does sourcing begin? | Sponsor | By end of M4 | — | Open |
| OQ-010 | What CI platform and what are the time/cost budgets per PR run? | Tech lead | Before M2 | — | Open |
| OQ-011 | Do we need EIP-compatible transaction encoding (M4) for any specific integration, or is it speculative? | Sponsor | Before M4 PRD | — | Open |
| OQ-012 | Rename scope. | Sponsor | — | **Resolved 2026-04-09** — Full rename: crate names (`karoowa-*`), CLI binary (`karoowa`), JSON-RPC prefix (`kw_*`), Docker images, brand | Resolved |
| OQ-013 | Origin of the name "Karoowa". | Sponsor | — | **Resolved 2026-04-09** — Name kept for "effortless harmony" vibe (loosely inspired by 軽 *karu* "light/effortless" + 和 *wa* "harmony"). Etymology pitch material from a separate B2C context is **not** carried over — Karoowa is positioned as blockchain infra, not a booking platform. Tagline candidate: "Light enough to launch anything." | Resolved |
| OQ-014 | Open-core boundary — which modules are OSS vs. enterprise? | Sponsor + Tech lead | — | **Resolved 2026-04-10** — **OSS:** core blockchain nodes & consensus, agent runtime framework (without advanced AI policy/governance), basic on-chain oracles, dev CLI, SDKs, M1–M3 agent bundles. **Enterprise (proprietary, license-gated):** agent governance/policy engine, HA nodes, multi-tenancy, advanced analytics, GUI dashboards, SSO/SAML/RBAC, MPC key management, audit/compliance tooling, M4 Governance + Finance/Treasury agents, custom SLAs and premium support. Pattern modelled on Sardis (open SDKs/CLI/runtimes; closed core banking, policy firewall, MPC wallet) and ElasticSearch X-Pack. | Resolved |
| OQ-015 | Sequencing of agent capabilities. | Sponsor + Tech lead | — | **Resolved 2026-04-10** — Capability bundles per milestone: **M1 (MVP) Dev bundle** = CLI/Dev Agent + basic Monitoring Agent. **M2 (Beta) Ops bundle** = CI/CD & Deployment Agent + Observability Agent. **M3 (GA) Security/Optimization bundle** = Vulnerability Scanner + Auto-Scaling/Gas Optimizer. **M4 (Enterprise) Governance bundle** = Governance/Policy Agent + Finance/Treasury Agent (gated to enterprise layer). Persona-named agents from §2 map onto these bundles — see ASM-016. | Resolved |
| OQ-016 | Agent runtime — in-process, sidecar, or remote? | Tech lead | — | **Resolved 2026-04-10** — **Sidecar-first hybrid.** Sidecar process is the default (separate process, loopback-only proxy with auth + quota — "padded room" pattern from production AI fleets). In-process mode is offered for hobbyist convenience in M1/M2. Sidecar mode required by M3 for security and scalability. **Cloud-hosted runtime is an enterprise capability.** See REQ-015. | Resolved |
| OQ-017 | Which LLM(s) power the agents? | Tech lead | — | **Resolved 2026-04-10** — **Pluggable provider trait** (`LlmProvider`). Launch providers: Anthropic (hosted), OpenAI (hosted), at least one local backend (`llama.cpp` / `ollama`). Provider configurable per-agent. Default leans hosted for capability; local backend exists so hobbyists are not forced to acquire an API key. Inspired by Acorn / Espressif IoT agent platforms. See REQ-014. | Resolved |
| OQ-018 | Hobbyist install mechanism. | Tech lead | — | **Resolved 2026-04-10** — **All channels.** (a) `curl -fsSL install.karoowa.io \| sh`, (b) prebuilt static binaries on GitHub Releases for Linux x86_64/aarch64, macOS, Windows, (c) package managers: Homebrew, APT, RPM, Chocolatey, Scoop, (d) optional Docker image. See REQ-013. | Resolved |
| OQ-019 | Public devnet — exists, who operates it, who funds it? | Sponsor | — | **Resolved 2026-04-10 (in principle)** — **Yes.** Core team / Karoowa Foundation operates the public devnet. Modelled on Linera's devnet. Open access, public RPC, faucet, status page. Progression: **Devnet → Public Testnet → Mainnet.** Operating cost accepted as cost of acquisition. **Owner and budget still need to be assigned by name** — see OQ-020. See REQ-016. | Resolved (in principle) |
| OQ-020 | Devnet operating ownership. | Sponsor | — | **Resolved 2026-04-10 (principle) / Open (specifics)** — Karoowa Foundation owns the public devnet. A **named "Karoowa Infrastructure Lead"** (person TBD) is accountable for uptime and cost. Devnet ops are funded as a line item in the foundation budget (treasury or grant). Initial SLO target: **99.5% uptime** during devnet phase, raised to 99.9% at testnet, 99.95% at mainnet. *Specifics still to assign:* the actual person, the actual budget number, the on-call rota. | Resolved (in principle) |
| OQ-021 | Local-model viability — is a 7B-class model adequate and runnable for hobbyist M1/M2 agent flows? | Tech lead | — | **Resolved 2026-04-10** — **No, ASM-014 was wrong.** A quantized 7B model does not fit reliably on a 4 GB VPS alongside a Karoowa node, and 7B-on-CPU is too slow/weak for useful agent reasoning without hardware acceleration. Decision: **lower the local-model floor to ~3B-class** (e.g. Gemma 3-class or smaller), **and** offer a hosted-LLM fallback as the recommended hobbyist default. Hobbyists who refuse a key get the small local model with degraded capability and clearly documented limits. ASM-014 superseded by ASM-014a. See REQ-014. | Resolved |
| OQ-022 | License enforcement mechanism for the enterprise layer. | Sponsor + Tech lead | — | **Resolved 2026-04-10 (revised)** — **Monorepo with `enterprise/` folder + CI guardrails + signed license file + EULA.** (1) Enterprise code lives in an `enterprise/` directory inside the monorepo (modelled on Strapi EE). (2) **CI guardrails** prevent cross-imports: a build-time check fails if any file under `core/` or other public crates imports from `enterprise/`. Community builds explicitly exclude the `enterprise/` directory. (3) Enterprise features require a **signed license file** at startup (modelled on Elasticsearch X-Pack); an optional **trial license** unlocks features for a limited window. (4) Commercial EULA covers legal usage. No online phone-home activation in v1.0. ⚠️ **This reverses the prior round's "two repos" decision** — see OQ-029. | Resolved (revised) |
| OQ-023 | Repo topology for the open-core split. | Tech lead | — | **Resolved 2026-04-10 (revised)** — **Single monorepo** with `core/` and `enterprise/` top-level directories. Rationale: lower maintenance for a small/solo team, single codebase, easier shared-code refactoring; leak risk is mitigated by CI guardrails (see OQ-022) rather than physical separation. Strapi and GitLab use this pattern successfully. ⚠️ **This reverses the prior round's "two repos" decision** — see OQ-029. | Resolved (revised) |
| OQ-024 | Sidecar + agent + local model on a 4 GB VPS — feasible? | Tech lead | — | **Resolved 2026-04-10** — **No, not with a 7B model.** Same root cause as OQ-021. Decision: M1 hobbyist tier defaults to **in-process agent + hosted LLM** for low-end hardware, with a clear documented warning. Sidecar mode is offered for users on ≥8 GB hardware or with GPU, and **becomes mandatory at M3** when production-grade security and quotas matter. REQ-015 amended below. | Resolved |
| OQ-025 | Database strategy — vector store choice, L2 trigger, telemetry retention. | Tech lead | — | **Resolved 2026-04-10** — **L3 (agent memory): LanceDB** for M1. Embedded, Apache 2.0, zero-ops, runs in-process — best fit for the hobbyist install footprint. Re-evaluate Qdrant in M2/M3 if rich filtering or hybrid search become hard requirements. **L2 (Postgres indexing):** unchanged — deferred until a real consumer needs it. **L4 (telemetry):** Prometheus only at M1; ClickHouse remains out of v1.0 scope. | Resolved |
| OQ-026 | Confirm Gemma generation, license, sizes, runtime support. | Tech lead | — | **Resolved 2026-04-10** — Per stakeholder research: **Google Gemma 4** is the current generation (Jan 2026). Sizes: **E2B (5B)** and **E4B (8B)** edge variants for CPU/edge devices; **26B** and **31B** workstation variants for GPUs. **Apache 2.0** licensed weights. Official **Ollama** and **LM Studio** integration; **GGUF** builds available on HuggingFace. M1 hobbyist tier targets E2B (5B) primarily; E4B (8B) where hardware allows; 26B/31B reserved for ≥enterprise hardware. ⚠️ **Note:** I cannot independently verify these specs from this session — they come from the stakeholder research brief. They should be re-checked by a human against the canonical Google source before any binding contract or marketing copy. | Resolved (per stakeholder research) |
| OQ-027 | Karoowa Infrastructure Lead — name the actual person and confirm they have capacity. | Sponsor | Before devnet launch | — | Open |
| OQ-028 | Devnet budget line item — actual annual figure, source, renewal. | Sponsor | — | **Resolved 2026-04-10 (in principle, scenarios)** — Budget scenarios from research: **Low (~$850/yr)** = 1 small VM + static IP + minimal monitoring; **Medium (~$1,850/yr)** = 2-region HA + load balancer; **High (~$4,850/yr)** = multi-region (6 VMs) + alerting + monitoring. **M1 default: Low scenario**, funded from Karoowa treasury or sponsor. Renewal: annual review at each milestone gate. *Final figures pending sponsor sign-off.* | Resolved (in principle) |
| OQ-029 | **Confirm reversal:** OQ-022 + OQ-023 were locked as "two repos" in round 4 and reversed to "monorepo + enterprise/ folder + CI guardrails + license file" in round 5. | Sponsor | — | **Resolved 2026-04-10** — Reversal confirmed by sponsor. Final answer: **monorepo with `core/` and `enterprise/` directories**, CI guardrails preventing cross-imports, signed license file gating enterprise features at startup. | Resolved |
| OQ-030 | Telemetry retention policy — how long do node metrics persist locally? Default: Prometheus 15-day retention. Confirm or adjust based on observability and compliance needs. | Tech lead | Before M2 | — | Open |

> All open questions should be resolved (or explicitly deferred with rationale) before the corresponding milestone PRD is finalized.

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| EVM compatibility | Karoowa uses its own API and (M3) WASM | Not planned |
| Mobile SDK | Phase 1 focuses on the framework + Rust SDK | Possible Phase 2 |
| Hosted service | Karoowa is a framework | Not planned |
| Block explorer UI | Operator observability via Grafana is sufficient for Phase 1 | Possible Phase 2 |
| Tokenomics design | Chain operators design their own | Out of framework scope |
| Formal verification | External audit covers v1.0 | Possible Phase 2 |
| Privacy / zero-knowledge primitives | Out of Phase 1 scope | Possible Phase 2 |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-09 | Initial draft. Project renamed from ChainFlux → Karoowa. Many fields flagged as ASSUMPTION pending stakeholder validation. | Generated from inherited `README.md`, `Cargo.toml`, `Dockerfile`, `docker-compose.yml`, `Makefile`, and assumed positioning |
| 2026-04-09 | Resolved OQ-001/002/003/012/013. Repositioned vision around three audience tiers (hobbyist, team, enterprise), open-core business model, and **agent-native operation** as a top-line product feature with paired human + agent personas. Added REQ-010 (EVM compatibility, milestone TBD), REQ-011 (agent-native operation, per-milestone), REQ-012 (open-core boundary), REQ-013 (hobbyist install). Added user stories US-016..US-026, hobbyist + enterprise + agent-assisted user flows, ASM-010 (agent-native), ASM-011 (hobbyist install). Added open questions OQ-014..OQ-019. | Stakeholder Q&A session |
| 2026-04-10 | Resolved OQ-014/015/016/017/018/019 from research input. Locked **open-core module list** (REQ-012). Locked **agent capability bundling per milestone** (REQ-011: M1 Dev → M2 Ops → M3 Security/Optimization → M4 Enterprise Governance). Added REQ-014 (Pluggable LLM provider), REQ-015 (Sidecar-first agent runtime), REQ-016 (Public Karoowa Devnet). Added ASM-012..ASM-016 covering sidecar runtime, pluggable LLM trait, local-model viability, devnet ops cost, and persona-vs-bundle reconciliation. Added new open questions OQ-020 (devnet ownership), OQ-021 (local-model viability), OQ-022 (license enforcement), OQ-023 (repo topology for open-core), OQ-024 (sidecar overhead on low-end hardware). | Karoowa Platform Research brief |
| 2026-04-10 | Resolved OQ-020/021/022/023/024 from second research brief. **Superseded ASM-014** with ASM-014a: 7B local model is not viable on hobbyist hardware; new floor is ~3B-class with hosted LLM as the recommended hobbyist default. **Amended REQ-015** from "sidecar-first" to a three-mode hybrid runtime (in-process for hobbyist M1/M2, sidecar for ≥8 GB hosts and mandatory at M3, cloud-hosted as enterprise capability). **Amended REQ-014** to add Google Gemma family and a generic GGUF local provider, with hosted as hobbyist default. **Amended REQ-016** with concrete SLO targets (99.5%/99.9%/99.95% across devnet/testnet/mainnet) and accountability for the Karoowa Infrastructure Lead role. **Locked open-core repo topology** (OQ-023): two repos, public + private. **Locked enterprise license mechanism** (OQ-022): repo separation + signed license file + trial mode + EULA. Added **REQ-017 Database strategy** with a four-layer architecture (RocksDB hot path, optional Postgres indexing, embedded vector store for agent memory, Prometheus telemetry). Added ASM-017..ASM-020 for storage choices. Added open questions OQ-025 (DB strategy details), OQ-026 (Gemma generation/specs verification), OQ-027 (named Infrastructure Lead), OQ-028 (devnet budget). | Second research brief + database strategy decision |
| 2026-04-10 | **Round 5 research brief applied.** Resolved OQ-025 (LanceDB for L3 agent memory), OQ-026 (Google Gemma 4 confirmed: E2B 5B / E4B 8B / 26B / 31B, Apache 2.0, GGUF + Ollama), OQ-028 (devnet budget scenarios: $850 / $1,850 / $4,850 per year, M1 default low). **Reversed OQ-022 + OQ-023:** moved from "two repos" to **monorepo with `core/` and `enterprise/` directories**, with CI guardrails preventing cross-imports and a signed license file gating enterprise features at startup. Reversal logged for sponsor confirmation as **OQ-029**. Updated REQ-012 (open-core enforcement now monorepo + CI + license file), REQ-014 (Gemma 4 specifics locked), REQ-017 (LanceDB locked for L3), ASM-014a (Gemma 4 E2B as the local fallback target), ASM-018 (LanceDB locked). Added OQ-029 (confirm reversal) and OQ-030 (telemetry retention). | Round 5 research brief |
