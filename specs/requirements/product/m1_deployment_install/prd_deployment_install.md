# PRD: Deployment & Install (M1 Phases 1.9-1.10)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD covering multiple phases |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** Deployment & Install (Phases 1.9, 1.10)
> **Owner:** TBD
> **Stakeholders:** Validator operators, hobbyists, chain builders
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Make Karoowa deployable and installable — Docker images for operators, prebuilt binaries and package managers for hobbyists, a public devnet for zero-config experimentation, and Grafana dashboards for observability. This is where Karoowa goes from "code that compiles" to "software you can run."

This is the fifth of six M1 feature PRDs. It depends on Feature PRDs 1-4 (the entire node stack and CLI).

### Expected Business Outcomes

- **4-validator devnet in Docker in under 5 minutes.** Chain builders can spin up a realistic multi-validator environment with one `docker compose up`.
- **Hobbyist installs in under 60 seconds.** A solo dev on a clean Linux box can install Karoowa without Cargo, Docker, or any toolchain via `curl | sh`, Homebrew, or APT/RPM.
- **Public devnet available from day one.** Hobbyists can join a live network and see blocks within minutes, without bootstrapping their own validators.
- **Observability out of the box.** Grafana dashboards show block height, peer count, RPC throughput, and latency — operators don't build their own monitoring from scratch.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| Time from `docker compose up` to 4-validator consensus | < 60s | N/A |
| Hobbyist install time (`curl \| sh` on clean Linux) | < 60s | N/A |
| Time from install to first block on public devnet | < 5 min | N/A |
| Docker image size (compressed) | < 50 MB | N/A |
| Public devnet uptime | 99.5% (devnet SLO) | N/A |

### User Problems

- **No way to run Karoowa without building from source.** Without Docker images or prebuilt binaries, only Rust developers can use Karoowa.
- **No observability.** Without dashboards, operators fly blind — they can't see if consensus is healthy, if peers are connected, or if RPC latency is degrading.
- **No public network to experiment on.** Without a public devnet, hobbyists must set up their own validators to see Karoowa do anything.
- **Package manager installs don't exist.** macOS users expect `brew install`, Linux users expect `apt install` — Karoowa must meet them where they are.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-DI-001 | We believe that **multiple install channels** (curl, Homebrew, APT, RPM, Docker) will lower the barrier to first-use for non-Rust developers | Install channel usage distribution; time-to-first-block per channel | Track install telemetry (opt-in) post-launch |
| H-DI-002 | We believe that **a public devnet with faucet** will be the primary onboarding path for hobbyists | Percentage of new users who join public devnet vs. run local node | Track devnet join count vs. local node starts |
| H-DI-003 | We believe that **pre-configured Grafana dashboards** will reduce time-to-observability from hours to minutes | Time from devnet start to operator viewing live metrics | Measure during QA |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-DI-001 | As a **Chain Builder**, I want a Docker image and Compose setup for single-node and 4-validator devnets, so that I can test consensus and networking without manual setup. | Phase 1.9 (T1.9.1-T1.9.3); parent REQ-001 | US-002, US-010 |
| US-DI-002 | As a **Validator Operator**, I want pre-configured Grafana dashboards, so that I can monitor block production, peer count, and RPC performance immediately. | Phase 1.9 (T1.9.4); parent REQ-001 | US-008 |
| US-DI-003 | As a **Solo / Hobbyist Operator**, I want to install Karoowa with a single command (`curl \| sh`), so that I can run a node without learning Cargo or Docker. | Phase 1.10 (T1.10.1-T1.10.2); parent REQ-013 | US-016 |
| US-DI-004 | As a **Solo / Hobbyist Operator**, I want to install Karoowa via Homebrew, APT, or RPM, so that I can use my OS's native package manager. | Phase 1.10 (T1.10.3-T1.10.4); parent REQ-013 | US-016 |
| US-DI-005 | As a **Solo / Hobbyist Operator**, I want to join the public devnet with `karoowa node --join public-devnet`, so that I can experiment on a live network without running my own validators. | Phase 1.10 (T1.10.5, T1.10.7); parent REQ-016 | US-018 |
| US-DI-006 | As a **Solo / Hobbyist Operator**, I want a faucet to get test tokens, so that I can submit transactions on the public devnet. | Phase 1.10 (T1.10.6); parent REQ-016 | US-018 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **Chain Builder** | Uses Docker Compose for local multi-validator testing. |
| **Validator Operator** | Uses Docker for production-like deployments; depends on Grafana for monitoring. |
| **Solo / Hobbyist Operator** | Primary consumer of install channels and the public devnet. First impression of Karoowa. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **Docker devnet** | Write `.env` with 4 validator keys -> `docker compose -f devnet.yml up` -> 4 validators reach consensus -> Grafana shows live data | Chain Builder |
| **Hobbyist install** | `curl -fsSL install.karoowa.io \| sh` -> binary installed -> `karoowa node --join public-devnet` -> syncing blocks within 5 min | Hobbyist |
| **Homebrew install** | `brew install karoowa/tap/karoowa` -> `karoowa --version` -> works | Hobbyist |
| **Public devnet join** | Install Karoowa -> `karoowa node --join public-devnet` -> node connects to bootnodes -> syncs -> request faucet tokens -> submit transaction | Hobbyist |

---

## 3. High-Level Requirements

### Phase 1.9 — Docker devnet + observability

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-DI-001 | `docker/Dockerfile` — multi-stage build, statically linked release binary, minimal base image, tagged `karoowa/karoowa:dev` | US-DI-001 | Must Have | See below |
| REQ-DI-002 | `docker/docker-compose.yml` — single-node setup with persistent volume and port mapping for RPC (8545) and metrics (9090) | US-DI-001 | Must Have | See below |
| REQ-DI-003 | `docker/devnet.yml` — 4-validator setup with shared bridge network and env-var injected validator keys | US-DI-001 | Must Have | See below |
| REQ-DI-004 | Grafana + Prometheus stack in Docker Compose with pre-loaded dashboard JSON showing block height, peer count, RPC throughput, and RPC latency | US-DI-002 | Must Have | See below |
| REQ-DI-005 | End-to-end test: bring up devnet, wait for 10 blocks, verify all 4 validators agree on head block hash, tear down | US-DI-001 | Must Have | See below |

### Phase 1.10 — Hobbyist install + public devnet

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-DI-006 | GitHub Releases pipeline: cross-compile `karoowa` for Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64. Upload binaries on tag. | US-DI-003 | Must Have | See below |
| REQ-DI-007 | `install.sh` script: detect OS + arch, download matching binary from GitHub Releases, install to `~/.karoowa/bin/karoowa`, print PATH instructions | US-DI-003 | Must Have | See below |
| REQ-DI-008 | Homebrew formula in `karoowa/homebrew-tap` repo | US-DI-004 | Should Have | See below |
| REQ-DI-009 | `.deb` and `.rpm` packages via `cargo-deb` / `cargo-rpm` | US-DI-004 | Should Have | See below |
| REQ-DI-010 | Public devnet provisioned: bootnode running on a small VM, faucet running, status page | US-DI-005 | Must Have | See below |
| REQ-DI-011 | Faucet service: `POST /faucet` signs a transfer from treasury key, rate-limited per IP | US-DI-006 | Must Have | See below |
| REQ-DI-012 | `karoowa node --join public-devnet` flag: pre-configured bootnode list pointing to public devnet IPs | US-DI-005 | Must Have | See below |

### BDD Scenarios

#### REQ-DI-001: Dockerfile

**Scenario: Docker image builds and runs**
**Given** the Karoowa source tree
**When** the developer runs `docker build -f docker/Dockerfile -t karoowa/karoowa:dev .`
**Then** the image builds successfully with a statically linked binary
**And** the compressed image size is under 50 MB
**And** `docker run karoowa/karoowa:dev --version` prints the correct version

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-003: 4-validator devnet

**Scenario: 4-validator Docker devnet reaches consensus**
**Given** four validator keys have been generated and exported to `docker/.env`
**When** the operator runs `docker compose -f docker/devnet.yml up -d`
**Then** all four validator containers report healthy within 60 seconds
**And** each validator's `kw_blockNumber` advances at the configured block time
**And** all four validators agree on the latest block hash

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-004: Grafana dashboard

**Scenario: Grafana shows live block production**
**Given** a running 4-validator Docker devnet with the Grafana/Prometheus stack
**When** the operator navigates to `http://localhost:3000`
**Then** the pre-loaded dashboard shows live block height increasing
**And** peer count shows 3 peers per validator
**And** RPC request count and latency histograms are populated

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-005: E2E devnet test

**Scenario: Devnet produces 10 blocks with consensus**
**Given** a 4-validator Docker devnet started from scratch
**When** the test waits for block height 10
**Then** all 4 validators agree on the block hash at height 10
**And** the test tears down the devnet cleanly

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-007: Install script

**Scenario: Solo operator installs Karoowa with a single command**
**Given** a clean Linux x86_64 machine with no Rust toolchain and no Docker
**When** the operator runs `curl -fsSL install.karoowa.io \| sh`
**Then** a `karoowa` binary is installed to `~/.karoowa/bin/karoowa` within 60 seconds
**And** `karoowa --version` returns a valid semver string
**And** the script prints PATH instructions if `~/.karoowa/bin` is not already on PATH

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-008: Homebrew

**Scenario: macOS user installs Karoowa via Homebrew**
**Given** a macOS machine with Homebrew installed
**When** the user runs `brew install karoowa/tap/karoowa`
**Then** the `karoowa` binary is installed and on PATH
**And** `karoowa --version` returns the latest release version

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-010: Public devnet

**Scenario: Hobbyist joins the public devnet**
**Given** a freshly installed `karoowa` binary
**When** the hobbyist runs `karoowa node --join public-devnet`
**Then** the node connects to the public devnet bootnodes
**And** begins syncing blocks within 30 seconds
**And** `kw_blockNumber` returns a value greater than 0 within 60 seconds

**Sad Paths** *(to be added during refinement)*

#### REQ-DI-011: Faucet

**Scenario: Hobbyist requests test tokens from the faucet**
**Given** a running public devnet with the faucet service
**When** the hobbyist sends `POST /faucet` with their address
**Then** the faucet signs and submits a transfer to the hobbyist's address
**And** the hobbyist's balance increases within the next few blocks
**And** a second request from the same IP within the rate limit window is rejected with a clear message

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-DI-001 | Performance | Docker devnet time to first consensus block | < 60s from `docker compose up` |
| NFR-DI-002 | Performance | Docker image size (compressed) | < 50 MB |
| NFR-DI-003 | Reliability | Public devnet uptime | 99.5% (devnet SLO from parent REQ-016) |
| NFR-DI-004 | Security | Install script verifies binary checksum | SHA256 verification |
| NFR-DI-005 | Security | Faucet rate limiting | Max 1 request per IP per 5 minutes |
| NFR-DI-006 | Portability | Prebuilt binaries for Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64 | All five targets build in CI |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-DI-001 | Static linking (musl) produces a working binary for all Linux targets | If dynamic linking is needed, Docker base image changes and install script may need to bundle shared libs | Test on clean minimal Linux containers |
| ASM-DI-002 | A single small VM ($850/yr budget from OQ-028) is sufficient for the public devnet at M1 scale | If traffic exceeds capacity, the devnet needs scaling or rate limiting | Monitor during first month; scale to medium budget if needed |
| ASM-DI-003 | `install.karoowa.io` domain is available and can be configured to serve the install script | If the domain is unavailable, a GitHub raw URL is the fallback | Register domain before Phase 1.10 |
| ASM-DI-004 | distroless or Alpine base image is sufficient — no additional OS packages needed at runtime | If runtime deps are discovered, the base image grows | Test binary in minimal container |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-DI-001 | Feature PRDs 1-4 (complete node + CLI) | Karoowa team | Pending | Docker images and installs ship the built binary |
| DEP-DI-002 | VM provisioning for public devnet | Infrastructure Lead (OQ-027) | Blocked — person TBD | Public devnet hosting |
| DEP-DI-003 | `install.karoowa.io` domain | TBD | Not started | Install script URL |
| DEP-DI-004 | GitHub Releases configured for the repo | Karoowa team | Not started | Binary distribution |
| DEP-DI-005 | `karoowa/homebrew-tap` GitHub repo | Karoowa team | Not started | Homebrew distribution |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| Windows installer (MSI/exe) | Prebuilt binary + PATH is sufficient for M1 | Post-M1 if demand |
| Snap / Flatpak packages | Low priority for M1 | Post-M1 if demand |
| Multi-region public devnet | Low budget tier for M1 (single VM) | Scale at testnet phase |
| Public devnet block explorer | Grafana provides operator-level observability; explorer is post-M1 | Post-M1 |
| Chocolatey / Scoop (Windows package managers) | Listed in parent REQ-013 but lower priority for M1 | Post-M1 |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Parent PRD | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan | `specs/development/dev_plan.md` (Phases 1.9, 1.10) | Authoritative |
| Devnet budget | OQ-028 in parent PRD | Resolved (low tier: $850/yr) |
| Predecessor PRDs | Feature PRDs 1-4 | Draft |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-DI-001 | Which cloud provider for the public devnet VM? AWS/GCP/Hetzner/DigitalOcean? Budget constraint: ~$850/yr. | Infrastructure Lead | Before T1.10.5 | — | Open |
| OQ-DI-002 | Should the install script auto-add `~/.karoowa/bin` to PATH, or just print instructions? Auto-modifying shell rc files is controversial. | Tech lead | Before T1.10.2 | — | Open |
| OQ-DI-003 | Should the faucet treasury key be on the same VM as the validator, or a separate service? | Infrastructure Lead | Before T1.10.6 | — | Open |
| OQ-DI-004 | Grafana dashboard: should it be JSON provisioning (file-based) or a pre-built dashboard ID from grafana.com? | Tech lead | Before T1.9.4 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| Production Kubernetes manifests | Docker Compose is sufficient for M1; K8s is enterprise scope | Post-M1 |
| Public testnet (distinct from devnet) | Devnet is sufficient for M1; testnet phase starts later | Pre-mainnet milestone |
| Automated devnet monitoring / alerting | Manual monitoring is acceptable for M1's low-tier budget | Testnet phase |
| Binary signing / notarization (macOS) | macOS support is best-effort for M1 | Post-M1 |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phases 1.9-1.10. | Generated from `dev_plan.md` Phases 1.9-1.10 and parent PRD |
