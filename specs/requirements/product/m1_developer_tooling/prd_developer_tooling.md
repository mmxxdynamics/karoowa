# PRD: Developer Tooling — SDK & CLI (M1 Phases 1.7-1.8)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD covering multiple phases |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** Developer Tooling — SDK & CLI (Phases 1.7, 1.8)
> **Owner:** TBD
> **Stakeholders:** Chain builders, dApp developers, validator operators, hobbyists
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Ship the Rust client SDK and the unified CLI binary so that dApp developers can integrate programmatically and all users can interact with Karoowa through a single `karoowa` command. These are the primary developer-facing surfaces — the tools people actually touch.

This is the fourth of six M1 feature PRDs. It depends on Feature PRDs 1-3 (types, storage, consensus, networking, API) because the SDK wraps the API and the CLI orchestrates the node.

### Expected Business Outcomes

- **dApp developers can integrate without hand-rolling RPC clients.** The `karoowa-sdk` crate provides typed methods for every JSON-RPC endpoint, transaction signing, and wallet management.
- **One binary does everything.** `karoowa node`, `karoowa wallet`, `karoowa devnet`, `karoowa client`, `karoowa genesis`, `karoowa network` — a single install gives operators and developers the full toolchain.
- **The CLI is the primary onboarding surface.** Before Docker or agents, every user's first interaction is `karoowa --version` or `karoowa wallet new`. If the CLI is good, first impressions are good.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| SDK method coverage vs JSON-RPC surface | 100% (all 14 methods) | N/A |
| `karoowa wallet new` time to key generation | < 1s | N/A |
| CLI `--help` completeness (all 6 subcommands documented) | 100% | N/A |
| SDK integration test pass rate against live in-process node | 100% | N/A |

### User Problems

- **No Rust SDK exists.** dApp developers must hand-roll JSON-RPC clients, construct raw requests, and parse untyped responses.
- **No unified CLI.** Without a CLI binary, operators must interact with the node via raw HTTP calls, manual key generation scripts, and ad hoc Docker commands.
- **Transaction construction is error-prone.** Signing, nonce management, and gas estimation require boilerplate that every dApp developer reimplements.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-DT-001 | We believe that **a typed Rust SDK** will reduce dApp integration time from days to hours | Time from "I want to query Karoowa" to working code | Measure with 2-3 external developers |
| H-DT-002 | We believe that **a single CLI binary with 6 subcommands** is more approachable than separate tools | User feedback; support question volume | Track post-launch |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-DT-001 | As a **dApp Developer**, I want a Rust SDK to query chain state and submit transactions, so that I don't have to hand-roll JSON-RPC clients. | Phase 1.7 (T1.7.1-T1.7.5); parent REQ-001 | US-005 |
| US-DT-002 | As a **dApp Developer**, I want a `Wallet` struct that wraps key management and transaction signing, so that I can sign transfers without handling raw bytes. | Phase 1.7 (T1.7.2); parent REQ-001 | US-005, US-009 |
| US-DT-003 | As a **dApp Developer**, I want transaction builder helpers, so that I can construct transfers and contract calls ergonomically. | Phase 1.7 (T1.7.3); parent REQ-001 | US-005 |
| US-DT-004 | As a **Chain Builder**, I want a `karoowa node` command to start a validator node, so that I can run my chain from the command line. | Phase 1.8 (T1.8.3); parent REQ-001 | US-001 |
| US-DT-005 | As a **Validator Operator**, I want `karoowa wallet new` to generate secure keys, so that I can create validator identities safely. | Phase 1.8 (T1.8.2); parent REQ-001 | US-009 |
| US-DT-006 | As a **Chain Builder**, I want `karoowa genesis` to generate and validate genesis configs, so that I can reproducibly bootstrap networks. | Phase 1.8 (T1.8.4); parent REQ-001 | US-004 |
| US-DT-007 | As a **dApp Developer**, I want `karoowa client` for quick one-shot RPC calls from the terminal, so that I can query the chain without writing code. | Phase 1.8 (T1.8.5); parent REQ-001 | US-005 |
| US-DT-008 | As a **Chain Builder**, I want `karoowa devnet` and `karoowa network` utilities, so that I can manage local devnets and inspect peer info from the CLI. | Phase 1.8 (T1.8.6); parent REQ-001 | US-002 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **dApp Developer** | Primary SDK consumer. Uses `NodeClient` and `Wallet` for programmatic integration and `karoowa client` for quick queries. |
| **Chain Builder** | Primary CLI consumer. Uses `karoowa node`, `karoowa genesis`, `karoowa devnet` to run and manage chains. |
| **Validator Operator** | Uses `karoowa wallet` for key management and `karoowa node` for running validators. |
| **Solo / Hobbyist Operator** | First touchpoint — `karoowa --version`, `karoowa wallet new`, `karoowa node --join public-devnet`. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **SDK integration** | Add `karoowa-sdk` dependency -> create `NodeClient` -> query `chain_id()` -> create `Wallet` -> sign transfer -> submit via `send_raw_transaction()` -> poll receipt -> confirm balance change | dApp Developer |
| **Key generation** | Run `karoowa wallet new` -> secure ed25519 keypair generated -> address displayed -> key material saved | Validator Operator |
| **Node start** | Run `karoowa node --validator-key <key> --consensus poa --data-dir ./data` -> node starts -> produces blocks -> serves API | Chain Builder |
| **Genesis generation** | Run `karoowa genesis generate --validators 4 --chain-id 42` -> genesis config written to file -> `karoowa genesis validate` confirms it's valid | Chain Builder |
| **Quick RPC query** | Run `karoowa client get-balance 0xabc...` -> balance printed to stdout | dApp Developer |

---

## 3. High-Level Requirements

### Phase 1.7 — `karoowa-sdk`

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-DT-001 | `NodeClient` struct wrapping `reqwest` with methods mirroring the JSON-RPC surface: `chain_id()`, `block_number()`, `get_balance(addr)`, `get_block_by_number(n)`, `get_block_by_hash(h)`, `get_transaction_by_hash(h)`, `get_transaction_receipt(h)`, `get_transaction_count(addr)`, `get_code(addr)`, `syncing()`, `peer_count()`, `node_info()`, `send_raw_transaction(tx)`, `pending_transactions()` | US-DT-001 | Must Have | See below |
| REQ-DT-002 | `Wallet` struct wrapping `karoowa-crypto::Keypair`. `Wallet::generate(chain_id)`, `wallet.address()`, `wallet.sign_transfer(to, value, nonce, gas_price, gas_limit)` | US-DT-002 | Must Have | See below |
| REQ-DT-003 | Transaction builder helpers: `TransferBuilder` for value transfers, `ContractCallBuilder` placeholder for M3 | US-DT-003 | Must Have | See below |
| REQ-DT-004 | Async examples in `examples/` directory matching the quickstart flow | US-DT-001 | Should Have | See below |
| REQ-DT-005 | Integration tests against a live in-process node | US-DT-001 | Must Have | See below |

### Phase 1.8 — `karoowa` CLI

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-DT-006 | `clap` skeleton with top-level binary and subcommands: `node`, `wallet`, `devnet`, `client`, `genesis`, `network` | US-DT-004..US-DT-008 | Must Have | See below |
| REQ-DT-007 | `karoowa wallet` — `new` (generate keypair), `address <key>` (derive address from key), `sign <key> <message>` (sign arbitrary message) | US-DT-005 | Must Have | See below |
| REQ-DT-008 | `karoowa node` — start a node with `--validator-key`, `--consensus`, `--data-dir`, `--bootnodes`, `--rpc-port`, `--metrics-port`, `--license-file` (no-op for now) | US-DT-004 | Must Have | See below |
| REQ-DT-009 | `karoowa genesis` — `generate` (create genesis config), `validate` (verify a genesis config file) | US-DT-006 | Must Have | See below |
| REQ-DT-010 | `karoowa client` — quick wrapper over SDK for one-shot RPC calls (e.g., `get-balance`, `block-number`, `send-tx`) | US-DT-007 | Must Have | See below |
| REQ-DT-011 | `karoowa devnet` and `karoowa network` — utilities for local devnet bring-up and peer info dumps | US-DT-008 | Should Have | See below |

### BDD Scenarios

#### REQ-DT-001: NodeClient

**Scenario: SDK queries chain state from a running node**
**Given** a running Karoowa node with blocks produced
**And** a `NodeClient` connected to the node's RPC endpoint
**When** the developer calls `client.block_number().await`
**Then** the result is a positive integer matching the current chain height

**Scenario: SDK submits a transaction and polls receipt**
**Given** a running node and a `Wallet` with sufficient balance
**When** the developer signs a transfer via `wallet.sign_transfer(to, value, nonce, gas_price, gas_limit)`
**And** submits it via `client.send_raw_transaction(signed_tx).await`
**Then** a transaction hash is returned
**And** `client.get_transaction_receipt(tx_hash).await` eventually returns a receipt with status success

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-002: Wallet

**Scenario: Wallet generates a keypair with chain-specific context**
**Given** a chain ID of 42
**When** the developer calls `Wallet::generate(42)`
**Then** a wallet with a valid ed25519 keypair is returned
**And** `wallet.address()` returns a valid 20-byte address with `0x` prefix

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-003: Transaction builders

**Scenario: TransferBuilder constructs a valid transfer transaction**
**Given** a `Wallet` and transfer parameters (to, value, nonce, gas_price, gas_limit)
**When** the developer uses `TransferBuilder::new().to(addr).value(100).nonce(0).gas_price(1).gas_limit(21000).sign(&wallet)`
**Then** a signed `Transaction` is returned with all fields populated correctly

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-007: karoowa wallet

**Scenario: Generate a new validator key**
**Given** the `karoowa` binary is installed
**When** the user runs `karoowa wallet new`
**Then** a new ed25519 keypair is generated using OS entropy
**And** the address is printed to stdout
**And** the key material is saved to the specified output path (or a default)

**Scenario: Derive address from existing key**
**Given** an existing validator key file
**When** the user runs `karoowa wallet address <key-file>`
**Then** the derived address is printed to stdout

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-008: karoowa node

**Scenario: Start a single-validator PoA node**
**Given** a valid validator key and an empty data directory
**When** the user runs `karoowa node --validator-key key.json --consensus poa --data-dir ./data`
**Then** the node starts and begins producing blocks
**And** the JSON-RPC endpoint is available at the configured port
**And** `/health` returns HTTP 200

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-009: karoowa genesis

**Scenario: Generate a genesis config for 4 validators**
**Given** four validator key files
**When** the user runs `karoowa genesis generate --validators key1.json,key2.json,key3.json,key4.json --chain-id 42 --output genesis.toml`
**Then** a valid genesis configuration file is written
**And** `karoowa genesis validate genesis.toml` exits with code 0

**Sad Paths** *(to be added during refinement)*

#### REQ-DT-010: karoowa client

**Scenario: Quick balance query from CLI**
**Given** a running Karoowa node
**When** the user runs `karoowa client get-balance 0xabc... --rpc http://localhost:8545`
**Then** the balance is printed to stdout in a human-readable format

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-DT-001 | Performance | `karoowa wallet new` completes | < 1s |
| NFR-DT-002 | Performance | SDK `NodeClient` method call overhead (above network latency) | < 5ms |
| NFR-DT-003 | Usability | All CLI subcommands have `--help` output with descriptions and examples | 100% coverage |
| NFR-DT-004 | Usability | CLI error messages include actionable suggestions (e.g., "did you mean...?", "file not found: check path") | Best effort |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-DT-001 | `reqwest` is the right HTTP client for the SDK | If a lighter client is needed (e.g., for WASM targets), the SDK internals change | reqwest is standard for Rust async HTTP; re-evaluate if WASM SDK is needed |
| ASM-DT-002 | `clap` is the right CLI framework | clap is the de facto standard; unlikely to be wrong | Already validated by ecosystem |
| ASM-DT-003 | Six subcommands is the right top-level structure. No subcommand will need to be added in M1 beyond `agent` (Phase 1.11). | If more subcommands are needed, the clap skeleton expands trivially | Review after Phase 1.11 |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-DT-001 | Feature PRDs 1-3 (types, storage, consensus, networking, API) | Karoowa team | Pending | SDK wraps API; CLI orchestrates all layers |
| DEP-DT-002 | `reqwest` crate | Upstream | Resolved | SDK HTTP client |
| DEP-DT-003 | `clap` crate | Upstream | Resolved | CLI framework |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| `karoowa agent` subcommand | Phase 1.11 scope | M1 Feature PRD 6: Agent Bundle |
| Python/JS/Go SDKs | Rust SDK only for M1 | Post-M1 |
| Contract deployment via SDK | `ContractCallBuilder` is a placeholder; real contract support is M3 | M3 |
| Shell completions (bash/zsh/fish) | Nice-to-have, not blocking | Post-M1 |
| Interactive/TUI mode | CLI is batch-mode only | Post-M1 if needed |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Parent PRD | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan | `specs/development/dev_plan.md` (Phases 1.7, 1.8) | Authoritative |
| Predecessor PRDs | Feature PRDs 1-3 | Draft |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-DT-001 | Should the CLI key file format be JSON (like Ethereum keystores) or a simpler format? | Tech lead | Before T1.8.2 | — | Open |
| OQ-DT-002 | Should the SDK support both sync and async APIs, or async-only? | Tech lead | Before T1.7.1 | — | Open |
| OQ-DT-003 | Should `karoowa client` output JSON by default or human-readable text? Consider `--json` flag. | Tech lead | Before T1.8.5 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| Multi-language SDKs | Rust only for M1 | Post-M1 |
| Contract interaction (real) | Placeholder builder only; M3 scope | M3 |
| GUI / TUI | CLI is the M1 interface | Post-M1 |
| Key import/export from other chains | Karoowa key format only for M1 | Post-M1 |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phases 1.7-1.8. | Generated from `dev_plan.md` Phases 1.7-1.8 and parent PRD |
