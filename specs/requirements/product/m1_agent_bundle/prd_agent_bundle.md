# PRD: M1 Dev Agent Bundle (M1 Phase 1.11)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** M1 Dev Agent Bundle (Phase 1.11)
> **Owner:** TBD
> **Stakeholders:** Core maintainers, hobbyists, chain builders
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Ship Karoowa's first AI agents — the agent runtime framework, LLM provider abstraction, vector memory store, and three M1 agents (Onboarding, Monitoring, CLI/Dev) — running in in-process mode. This is Karoowa's key differentiator: agents are a top-line product feature, not an afterthought.

This is the sixth and final M1 feature PRD and the **highest-risk phase in M1**. The viability spike (T1.11.1) may force scope changes to the local-model fallback. It depends on all prior Feature PRDs (the complete node, CLI, Docker devnet, and install path).

### Expected Business Outcomes

- **Agent-native from v0.1.** Karoowa ships AI agents in its first release, establishing the "agent-native blockchain" positioning before competitors.
- **Hobbyists get guided onboarding.** The Onboarding Agent walks first-time users through install, key generation, devnet join, and first-block — reducing support burden and improving first impressions.
- **Operators get basic monitoring.** The Monitoring Agent reads `/metrics` and `/health`, summarizes node status, and flags issues in natural language — a preview of the full Operator Agent in M2.
- **Developers get a CLI assistant.** The CLI/Dev Agent wraps the `karoowa` CLI, taking natural-language requests and suggesting commands.
- **LLM provider is pluggable.** The `LlmProvider` trait decouples agents from any specific LLM, with Anthropic (hosted) and Gemma 4 (local via Ollama) as launch providers.
- **Agent memory works.** LanceDB provides embedded vector storage for agent RAG and context persistence.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| Onboarding Agent success rate (clean Linux, hosted LLM) | > 90% on fixed scenario set | N/A |
| Onboarding Agent success rate (clean Linux, local Gemma 4 E2B) | > 70% (degraded, documented) | N/A |
| In-process node + agent combined memory (hosted LLM) | < 1.5 GB RSS | N/A |
| In-process node + agent combined memory (local Gemma 4 E2B via Ollama) | TBD — viability spike determines | N/A |
| LLM provider switch (config-only, no recompile) | Works | N/A |

### User Problems

- **Hobbyist onboarding is intimidating.** Without an agent, new users must read docs, understand CLI flags, debug network issues, and troubleshoot errors on their own.
- **Monitoring requires manual effort.** Without an agent, operators must manually query `/metrics`, interpret Prometheus counters, and correlate health signals.
- **CLI learning curve.** Without an agent, users must memorize subcommands and flags to interact with Karoowa.
- **No local-model fallback.** Without a pluggable LLM provider, hobbyists who don't want to acquire an API key can't use agents at all.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-AB-001 | We believe that **shipping AI agents alongside infrastructure in the same milestone** will make Karoowa meaningfully easier to operate than alternatives | Manual ops actions / node-day with agents enabled vs. disabled | Compare agent-enabled vs. disabled devnet over 7 days |
| H-AB-002 | We believe that **Gemma 4 E2B (5B params) running locally via Ollama** is viable as a no-key fallback on 4 GB VPS hardware | Peak RAM, latency, Onboarding Agent success rate | Viability spike T1.11.1 |
| H-AB-003 | We believe that **a pluggable `LlmProvider` trait** is sufficient for agents without leaking provider-specific features | All three agents work identically across Anthropic and local providers (modulo quality) | Test each agent with each provider on the same scenario set |
| H-AB-004 | We believe that **LanceDB** is the right embedded vector store for M1 agent memory | Insert/query latency, filtering capability, stability | Integration test during T1.11.6 |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-AB-001 | As a **Solo / Hobbyist Operator**, I want an Onboarding Agent that walks me through install, key generation, devnet join, and first-block, so that I can recover from mistakes without reading full docs. | Phase 1.11 (T1.11.7); parent REQ-011 M1 Dev bundle | US-021 |
| US-AB-002 | As a **Validator Operator**, I want a basic Monitoring Agent that reads node metrics and health, so that I get natural-language summaries instead of raw Prometheus counters. | Phase 1.11 (T1.11.8); parent REQ-011 M1 Dev bundle | US-022 |
| US-AB-003 | As a **Chain Builder**, I want a CLI/Dev Agent that suggests commands from natural-language descriptions, so that I can interact with Karoowa without memorizing the CLI reference. | Phase 1.11 (T1.11.9); parent REQ-011 M1 Dev bundle | US-023 |
| US-AB-004 | As a **Validator Operator**, I want to switch an agent's LLM provider via config (e.g., from Anthropic to local Ollama) without recompiling, so that I can choose between quality and cost/privacy. | Phase 1.11 (T1.11.2-T1.11.4); parent REQ-014 | US-021..US-026 |
| US-AB-005 | As a **Solo / Hobbyist Operator**, I want agents to run in-process inside the `karoowa` binary, so that I don't need to manage separate sidecar processes on my limited hardware. | Phase 1.11 (T1.11.10); parent REQ-015 | US-021 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **Solo / Hobbyist Operator** | Primary consumer of the Onboarding Agent. First user to interact with agents. In-process mode is designed for this persona. |
| **Validator Operator** | Uses the Monitoring Agent for basic observability. May switch LLM providers based on ops requirements. |
| **Chain Builder** | Uses the CLI/Dev Agent for command discovery and chain management. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **Agent-assisted onboarding** | Run `karoowa agent onboard` -> agent generates wallet -> joins devnet -> waits for first block -> confirms success in natural language -> if any step fails, agent diagnoses and proposes fix | Hobbyist |
| **Agent monitoring** | Run `karoowa agent monitor` -> agent polls `/metrics` and `/health` -> summarizes node status in natural language -> flags anomalies | Validator Operator |
| **CLI assistance** | Run `karoowa agent dev` -> describe what you want in natural language -> agent suggests the right CLI command | Chain Builder |
| **LLM provider switch** | Edit agent config to change provider from `anthropic` to `ollama` -> restart agent -> agent works with local model (degraded quality documented) | Validator Operator |

---

## 3. High-Level Requirements

### Agent Runtime Framework

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-AB-001 | `karoowa-agents` crate with `LlmProvider` trait: `complete(prompt: Prompt) -> Result<Completion>`. Provider config struct selecting provider by name. | US-AB-004 | Must Have | See below |
| REQ-AB-002 | `AnthropicProvider` implementation: HTTPS client, API key from env var, completion request with tool-use support | US-AB-004 | Must Have | See below |
| REQ-AB-003 | `GemmaLocalProvider` implementation: talks to Ollama HTTP API on localhost. Configurable model name (default: `gemma4-e2b`). | US-AB-004 | Must Have | See below |
| REQ-AB-004 | `Agent` trait: `name()`, `system_prompt()`, `tools()`, `step(input) -> Output`. Tool-use via the chosen LLM provider's tool-calling shape. | US-AB-001..US-AB-003 | Must Have | See below |
| REQ-AB-005 | LanceDB integration for agent memory: `MemoryStore::insert(entry)`, `MemoryStore::query(query, top_k) -> Vec<Entry>`. Embedding via small open-source model (Ollama) or hosted. | US-AB-001..US-AB-003 | Must Have | See below |

### M1 Agents

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-AB-006 | `OnboardingAgent`: tools = `run_install`, `generate_wallet`, `join_devnet`, `wait_for_block`, `explain_error`. System prompt focused on first-time-user guidance. | US-AB-001 | Must Have | See below |
| REQ-AB-007 | `MonitoringAgent`: tools = `read_metrics`, `read_logs`, `report_status`. Polls `/metrics` and `/health`, summarizes in natural language. | US-AB-002 | Must Have | See below |
| REQ-AB-008 | `CliDevAgent`: wraps the CLI, takes natural-language requests, suggests `karoowa` commands with explanations. | US-AB-003 | Should Have | See below |

### CLI Entry Point & Runtime

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-AB-009 | `karoowa agent <name>` CLI subcommand entry point. In-process runtime mode only. | US-AB-005 | Must Have | See below |
| REQ-AB-010 | **VIABILITY SPIKE** (T1.11.1): Provision 4 GB VPS, install Ollama, pull Gemma 4 E2B (5B), run sample prompt, measure peak RAM and latency. Document result: viable / viable-with-tweaks / not viable. | US-AB-004 | Must Have | See below |

### BDD Scenarios

#### REQ-AB-001: LlmProvider trait

**Scenario: Agent switches from hosted to local LLM via config**
**Given** an agent currently configured with the `anthropic` provider
**When** the operator edits the agent config to use `ollama` with model `gemma4-e2b`
**And** restarts the agent
**Then** the agent starts successfully using the local provider
**And** subsequent agent responses are produced by the local model
**And** no code changes were required

**Scenario: A new provider is added without modifying existing agents**
**Given** the `LlmProvider` trait and existing Anthropic + Gemma implementations
**When** a contributor adds an `OpenAiProvider` implementation in a downstream crate
**Then** any existing agent can use the new provider purely via configuration
**And** the existing provider implementations are unchanged

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-002: AnthropicProvider

**Scenario: Anthropic provider completes a prompt**
**Given** an `AnthropicProvider` configured with a valid API key
**When** the provider receives a completion request with a prompt and tool definitions
**Then** a `Completion` is returned containing the model's response
**And** if the model invoked a tool, the tool call is parsed and returned

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-003: GemmaLocalProvider

**Scenario: Local Gemma provider completes a prompt via Ollama**
**Given** Ollama is running on localhost with the `gemma4-e2b` model loaded
**And** a `GemmaLocalProvider` configured to connect to Ollama
**When** the provider receives a completion request
**Then** a `Completion` is returned
**And** the response quality may be degraded compared to the hosted provider (documented)

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-005: LanceDB agent memory

**Scenario: Agent stores and retrieves context via vector memory**
**Given** an agent with a `MemoryStore` backed by LanceDB
**When** the agent inserts a context entry "The devnet bootnode IP is 10.0.0.1"
**And** later queries "What is the bootnode address?"
**Then** the original entry is returned as a top-k result

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-006: OnboardingAgent

**Scenario: Onboarding Agent walks a hobbyist through first-block**
**Given** a Solo Operator on a clean machine with the `karoowa` binary installed
**And** the Onboarding Agent is started with `karoowa agent onboard`
**When** the agent runs its onboarding flow
**Then** the agent generates a wallet key
**And** joins the public devnet
**And** waits for the first block to be observed
**And** confirms success in natural language
**And** if any step fails, the agent diagnoses the failure and proposes a fix without escalating to docs

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-007: MonitoringAgent

**Scenario: Monitoring Agent summarizes node health**
**Given** a running Karoowa node with the Monitoring Agent enabled via `karoowa agent monitor`
**When** the agent polls `/metrics` and `/health`
**Then** the agent produces a natural-language summary of node status (block height, peer count, sync status)
**And** if any metric is anomalous (e.g., peer count = 0), the agent flags it with a recommended action

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-008: CliDevAgent

**Scenario: CLI/Dev Agent suggests the right command**
**Given** a Chain Builder running `karoowa agent dev`
**When** the user types "I want to check the current block height on my node"
**Then** the agent suggests `karoowa client block-number --rpc http://localhost:8545`
**And** explains what the command does

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-009: In-process runtime

**Scenario: Agent runs in-process on a 4 GB VPS**
**Given** a Solo Operator on a 4 GB VPS with the `karoowa` binary and a hosted LLM API key
**When** they run `karoowa agent onboard`
**Then** the agent runs inside the `karoowa` binary without spawning a separate sidecar
**And** the combined RSS of node + agent stays under 1.5 GB
**And** the onboarding flow completes successfully

**Sad Paths** *(to be added during refinement)*

#### REQ-AB-010: Viability spike

**Scenario: Gemma 4 E2B viability is determined**
**Given** a 4 GB VPS with Ollama installed
**When** Gemma 4 E2B (5B params, GGUF) is pulled and a sample prompt is run
**Then** peak RAM is measured and documented
**And** response latency is measured and documented
**And** the result is classified as "viable", "viable-with-tweaks", or "not viable"
**And** if not viable, the no-key fallback is dropped or reduced to a smaller model, and hobbyists must use a hosted provider

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-AB-001 | Performance | In-process node + agent RSS (hosted LLM) | < 1.5 GB |
| NFR-AB-002 | Performance | Agent step latency (hosted LLM, network excluded) | < 500ms |
| NFR-AB-003 | Performance | LanceDB insert latency | < 50ms per entry |
| NFR-AB-004 | Performance | LanceDB query latency (top-5) | < 100ms |
| NFR-AB-005 | Reliability | Agent crash does not affect node liveness (in-process mode: agent task panics are caught) | Verified by test |
| NFR-AB-006 | Security | API keys are read from env vars or config files, never hardcoded | Enforced by code review |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-AB-001 | Gemma 4 E2B (5B) can run on a 4 GB VPS with Ollama alongside a Karoowa node (inherits ASM-014a) | Hobbyists must use hosted LLM; no-key fallback dropped | Viability spike T1.11.1 |
| ASM-AB-002 | LanceDB is stable enough for M1 agent memory (inherits ASM-018) | Swap to Qdrant or alternative; storage trait abstraction makes this cheap | Integration test during T1.11.6 |
| ASM-AB-003 | In-process agent mode is safe for hobbyist use despite running in the same process as the node | If agent bugs cause node crashes, sidecar becomes the hobbyist default too | Monitor stability during testing; M2 warning about in-process mode |
| ASM-AB-004 | A uniform `LlmProvider` trait works without provider-specific tool-calling shapes (inherits ASM-013) | Trait grows; provider-specific adapters needed | Validate by implementing 2 providers end-to-end |
| ASM-AB-005 | Ollama's HTTP API is stable enough to build `GemmaLocalProvider` against | If Ollama API changes, provider impl needs updating | Pin Ollama version; test in CI |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-AB-001 | Feature PRDs 1-5 (complete node, CLI, Docker, install, public devnet) | Karoowa team | Pending | Agents interact with a running node |
| DEP-AB-002 | Anthropic API access (API key) | Anthropic | Resolved | Hosted LLM provider |
| DEP-AB-003 | Ollama runtime | Upstream | Resolved | Local LLM provider |
| DEP-AB-004 | Gemma 4 E2B GGUF weights (HuggingFace) | Google / HuggingFace | Resolved | Local model |
| DEP-AB-005 | LanceDB Rust crate | LanceDB team | In Progress | Agent memory |
| DEP-AB-006 | 4 GB VPS for viability spike | Infrastructure | Not started | T1.11.1 |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| Sidecar agent runtime | M2 Phase 2.7 | M2 |
| Cloud-hosted agent runtime | Enterprise capability | M4+ |
| Full Operator Agent (remediations, key rotation) | M2 Ops bundle | M2 |
| Scaffolding Agent | M2/M3 scope | M2/M3 |
| Integration Agent | M2/M3 scope | M2/M3 |
| Contributor Agent | M2/M3 scope | M2/M3 |
| Agent governance / policy engine | Enterprise layer | M4+ |
| OpenAI provider | Listed in parent REQ-014 but not required for M1 launch; can be added post-launch | Post-M1 |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Parent PRD | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan | `specs/development/dev_plan.md` (Phase 1.11) | Authoritative |
| Agent sequencing | Parent PRD REQ-011, OQ-015 | Resolved |
| LLM provider design | Parent PRD REQ-014, OQ-017 | Resolved |
| Agent runtime modes | Parent PRD REQ-015, OQ-016/024 | Resolved |
| Database strategy (L3) | Parent PRD REQ-017, OQ-025 | Resolved (LanceDB) |
| Predecessor PRDs | Feature PRDs 1-5 | Draft |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-AB-001 | Which embedding model for LanceDB? A small open-source model via Ollama, or a hosted embedding API? Trade-off: local = no API key needed but more RAM; hosted = better quality but requires a key. | Tech lead | Before T1.11.6 | — | Open |
| OQ-AB-002 | Should agents share a single `MemoryStore` instance or each have their own isolated store? | Tech lead | Before T1.11.6 | — | Open |
| OQ-AB-003 | What is the agent config format? TOML file? CLI flags? Environment variables? A combination? | Tech lead | Before T1.11.10 | — | Open |
| OQ-AB-004 | If the viability spike (T1.11.1) shows Gemma 4 E2B is not viable on 4 GB, what is the fallback? Options: (a) drop local fallback entirely, (b) try an even smaller model (Gemma 4 E2B at lower quantization), (c) raise the minimum hardware requirement. | Tech lead | After T1.11.1 | — | Open |
| OQ-AB-005 | Should the `OnboardingAgent` be interactive (conversational) or scripted (run-to-completion)? The parent PRD implies interactive, but a scripted flow may be more reliable for M1. | Tech lead | Before T1.11.7 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| Sidecar runtime mode | Sidecar is M2 Phase 2.7; in-process only for M1 | M2 |
| Agent-to-agent communication | Single-agent execution only for M1 | M2+ |
| Agent marketplacce / plugin system | M1 ships only built-in agents | Post-v1.0 |
| Fine-tuning or custom training | Agents use off-the-shelf models via prompt engineering | Post-v1.0 |
| Agent telemetry / tracing | Basic logging only for M1 | M2 (observability agent) |
| Multi-tenant agent isolation | Enterprise scope | M4+ |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phase 1.11. Highest-risk phase — viability spike may force scope changes. | Generated from `dev_plan.md` Phase 1.11 and parent PRD |
