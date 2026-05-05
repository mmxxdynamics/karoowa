# PRD: Foundation & Core Primitives (M1 Phases 1.0-1.2)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD covering multiple phases |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** Foundation & Core Primitives (Phases 1.0, 1.1, 1.2)
> **Owner:** TBD
> **Stakeholders:** Core maintainers, prospective open-source contributors
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Establish the foundational layer of Karoowa — the monorepo workspace, cryptographic primitives, and core domain types — so that all subsequent M1 work (storage, consensus, networking, API, CLI, agents) has a stable, tested, and well-structured base to build on.

This PRD is the first of six feature PRDs that decompose M1 (v0.1). It covers Phases 1.0, 1.1, and 1.2 from `specs/development/dev_plan.md` and implements the lowest layers of the Karoowa architecture stack.

### Expected Business Outcomes

- **Monorepo is production-ready from day one.** CI, linting, formatting, license hygiene, and cross-import guardrails are enforced on every PR before any blockchain code is written.
- **Open-core boundary is structurally enforced.** The `core/` and `enterprise/` directory split, CI guardrails, and `LicenseGate` trait stub ensure the open-core model is baked into the repo layout, not bolted on later.
- **Crypto primitives are trustworthy.** All cryptographic operations (hashing, signing, Merkle proofs) use audited crates (`ed25519-dalek`, `sha3`, `blake3`) with property-based tests and known test vectors — no hand-rolled crypto.
- **Core domain types are stable and serialization-locked.** Block, Transaction, Receipt, Account, and Config types have fixed serialization vectors so that any encoding change breaks tests intentionally, preventing silent wire-format drift.
- **Contributors can onboard immediately.** The repo looks and feels like a credible OSS project from the first commit: README, CONTRIBUTING, CODE_OF_CONDUCT, PR templates, issue templates, and a green CI pipeline.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| `cargo build --workspace` success on empty stubs | Pass | N/A (greenfield) |
| `cargo test --workspace` runtime (Phases 1.0-1.2) | < 60s on commodity laptop | N/A |
| Crypto primitive property tests (proptest) | 100% of public API surface | N/A |
| CI pipeline pass rate on clean PRs | 100% | N/A |
| Cross-import guardrail false negatives | 0 | N/A |

### User Problems

- **No codebase exists yet.** Karoowa is greenfield — there is no code to build on. The inherited `files/` directory contained only a design sketch (`Cargo.toml` + `README.md`), not a working implementation. Every crate must be created from scratch.
- **Contributors need a credible repo to contribute to.** Without CI, linting, license files, and contribution docs, external contributors have no signal that the project is serious or safe to invest time in.
- **Domain types must be right the first time.** Block headers, transactions, and state types are load-bearing — every other crate (storage, consensus, API, SDK) depends on them. Getting the type surface wrong forces cascading refactors across the entire workspace.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-FC-001 | We believe that **establishing CI, linting, and contribution scaffolding before writing any blockchain code** will **reduce contributor onboarding friction and prevent accumulation of tech debt** | Time for a new contributor to open a passing PR | Track first-contributor PR cycle time post Phase 1.0 |
| H-FC-002 | We believe that **using audited crypto crates with property-based tests** will **prevent cryptographic bugs from reaching consensus or networking layers** | Crypto-related bug count in Phases 1.3+ | Track bugs traceable to crypto primitives |
| H-FC-003 | We believe that **locking serialization vectors for core types from day one** will **prevent silent wire-format drift as the codebase grows** | Serialization-related breaking changes caught by tests vs. discovered in integration | Count serialization test failures that prevented bugs |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-FC-001 | As an **Open-Source Contributor**, I want a clean Cargo workspace with 8 named crates, CI on every PR, and contribution docs, so that I can fork, build, and contribute confidently. | Phase 1.0 (T1.0.1-T1.0.8); parent REQ-009, REQ-012 | US-015 |
| US-FC-002 | As a **Chain Builder**, I want reliable cryptographic primitives (hashing, signing, Merkle proofs), so that I can trust the foundation my chain is built on. | Phase 1.1 (T1.1.1-T1.1.6); parent REQ-001 | US-001 |
| US-FC-003 | As a **Chain Builder**, I want well-defined core domain types (Block, Transaction, Receipt, Account, Config), so that I can build consensus, storage, and API layers on a stable type surface. | Phase 1.2 (T1.2.1-T1.2.8); parent REQ-001 | US-001 |
| US-FC-004 | As an **Open-Source Contributor**, I want the open-core boundary (`core/` vs `enterprise/`) enforced by CI from the first commit, so that I know my contributions won't accidentally leak into proprietary code or vice versa. | Phase 1.0 (T1.0.3, T1.0.4); parent REQ-012 | US-015 |
| US-FC-005 | As a **Chain Builder**, I want a `LicenseGate` trait stub, so that enterprise features can be gated behind license checks when the enterprise layer ships. | Phase 1.0 (T1.0.3); parent REQ-012 | US-019 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **Open-Source Contributor** | Primary consumer of this PRD's output. The workspace structure, CI, contribution docs, and crate boundaries are designed for contributors to navigate and build on. |
| **Chain Builder** | Depends on crypto primitives (Phase 1.1) and core types (Phase 1.2) being correct and stable. These are the foundation every chain built on Karoowa inherits. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **First contribution** | Fork repo -> clone -> `cargo build --workspace` -> make a change -> open PR -> CI passes -> review | Open-Source Contributor |
| **Crypto primitive usage** | Import `karoowa-crypto` -> generate keypair -> sign a message -> verify signature -> hash data -> build Merkle tree -> verify proof | Chain Builder |
| **Core type construction** | Import `karoowa-core` -> construct a Transaction -> sign it -> build a Block with transactions -> compute block hash -> verify tx_root matches Merkle root of transactions | Chain Builder |

---

## 3. High-Level Requirements

### Phase 1.0 — Workspace Skeleton, CI, License Stub

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-FC-001 | Initialize root Cargo workspace with 8 member crates (`karoowa-crypto`, `karoowa-core`, `karoowa-consensus`, `karoowa-storage`, `karoowa-network`, `karoowa-api`, `karoowa-sdk`, `karoowa`), `enterprise/` placeholder, and shared workspace dependencies | US-FC-001 | Must Have | See below |
| REQ-FC-002 | Add LICENSE (Apache 2.0), LICENSE-ENTERPRISE.md placeholder, and root README with project description, quickstart placeholder, and workspace crate table | US-FC-001 | Must Have | See below |
| REQ-FC-003 | Scaffold `LicenseGate` trait, `LicenseInfo`, `Edition` enum, and `OssLicenseGate` default implementation in `karoowa-core` | US-FC-005 | Must Have | See below |
| REQ-FC-004 | CI cross-import guardrail script that fails the build if any `core/` file imports from `enterprise/` | US-FC-004 | Must Have | See below |
| REQ-FC-005 | GitHub Actions CI baseline with fmt, clippy, test, build, cross-import guardrail, and `cargo deny` for license/dependency hygiene | US-FC-001 | Must Have | See below |
| REQ-FC-006 | Community files: CODEOWNERS, CONTRIBUTING.md, CODE_OF_CONDUCT.md, PR template, issue templates | US-FC-001 | Must Have | See below |
| REQ-FC-007 | Phase 1.0 sanity check: all six checks pass on a fresh clone, CI green, tagged `v0.0.1-skeleton` | US-FC-001 | Must Have | See below |

### Phase 1.1 — `karoowa-crypto`

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-FC-008 | `Hash` type (32-byte) with `From<[u8; 32]>`, `Display`, `FromStr`, serde round-trip. SHA3-256 and BLAKE3 hashing functions | US-FC-002 | Must Have | See below |
| REQ-FC-009 | `Address` type (20 bytes, derived from last 20 bytes of SHA3-256 of public key). Hex encoding/decoding with `0x` prefix | US-FC-002 | Must Have | See below |
| REQ-FC-010 | `Keypair` type wrapping ed25519-dalek. `generate(&mut OsRng)`, `from_seed(&[u8; 32])`, `.address()`, `.public_key_bytes()` | US-FC-002 | Must Have | See below |
| REQ-FC-011 | `Signature` type. `keypair.sign(message)`, `signature.verify(public_key, message)`. Serializable | US-FC-002 | Must Have | See below |
| REQ-FC-012 | Binary Merkle tree (SHA3-256 internal nodes). `from_leaves`, `.root()`, `.proof(index)`, `verify_proof(root, leaf, index, proof)` | US-FC-002 | Must Have | See below |
| REQ-FC-013 | Comprehensive unit tests + property tests (proptest) for all crypto primitives. Round-trip serialization. Test vectors against known SHA3/ed25519 fixtures | US-FC-002 | Must Have | See below |

### Phase 1.2 — `karoowa-core` Primitives

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-FC-014 | `Transaction` type: `from`, `to`, `value`, `nonce`, `gas_price`, `gas_limit`, `data`, `signature`. Hashing, signing, serialization | US-FC-003 | Must Have | See below |
| REQ-FC-015 | `BlockHeader` type: `parent_hash`, `state_root`, `tx_root`, `receipt_root`, `height`, `timestamp`, `proposer`, `consensus_data`. Hashing | US-FC-003 | Must Have | See below |
| REQ-FC-016 | `Block` type: `header` + `transactions`. `block.hash()`. Validation: `tx_root` matches Merkle root of transactions | US-FC-003 | Must Have | See below |
| REQ-FC-017 | `Receipt` type: `tx_hash`, `status`, `gas_used`, `logs`, `output`. `Log` type with `address`, `topics`, `data` | US-FC-003 | Must Have | See below |
| REQ-FC-018 | `Account` state type: `nonce`, `balance`, `code_hash`, `storage_root`. `StateDiff` for tracking per-block changes | US-FC-003 | Must Have | See below |
| REQ-FC-019 | `ChainConfig` and `GenesisConfig` types. Genesis loading from JSON/TOML | US-FC-003 | Must Have | See below |
| REQ-FC-020 | Crate-wide error types via `thiserror`. `Result<T>` alias | US-FC-003 | Must Have | See below |
| REQ-FC-021 | Re-exports, module structure, and comprehensive unit tests with fixed serialization vectors | US-FC-003 | Must Have | See below |

### BDD Scenarios

#### REQ-FC-001: Cargo workspace initialization

**Scenario: Empty workspace compiles successfully**
**Given** a fresh clone of the Karoowa repository
**When** the developer runs `cargo build --workspace`
**Then** all 8 `karoowa-*` member crates compile without errors
**And** `cargo metadata --format-version 1` lists all 8 crates

**Scenario: Enterprise directory exists but contains no Rust source**
**Given** a fresh clone of the Karoowa repository
**When** the developer inspects the `enterprise/` directory
**Then** the directory exists and contains a `README.md` placeholder
**And** no `.rs` source files exist under `enterprise/`

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-002: License and README

**Scenario: Legal surface is complete and correct**
**Given** a fresh clone of the Karoowa repository
**When** the developer inspects the root directory
**Then** `LICENSE` contains the Apache 2.0 license text
**And** `LICENSE-ENTERPRISE.md` contains the enterprise license placeholder
**And** `README.md` contains the project name, tagline "Light enough to launch anything", quickstart placeholder, workspace crate table with 8 crates, and links to LICENSE files
**And** no file in the repository references "ChainFlux" or the `cf_*` prefix

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-003: LicenseGate trait stub

**Scenario: OSS license gate denies all enterprise features**
**Given** the `OssLicenseGate` default implementation
**When** a caller checks `is_feature_enabled("any_feature")`
**Then** the result is `false`
**And** `license_info()` returns `Edition::Oss` with an empty feature list

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-004: Cross-import guardrail

**Scenario: Clean tree passes the guardrail**
**Given** a Karoowa workspace where no `core/` file imports from `enterprise/`
**When** the cross-import guardrail script runs
**Then** the script exits with code 0

**Scenario: Cross-import is detected and rejected**
**Given** a Karoowa workspace where a file under `core/` contains `use enterprise::foo`
**When** the cross-import guardrail script runs
**Then** the script exits with a non-zero code
**And** the error message names the offending file and line number

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-005: CI baseline

**Scenario: CI runs all checks on a pull request**
**Given** a contributor has pushed a feature branch
**When** they open a pull request against `main`
**Then** GitHub Actions runs `cargo fmt --all -- --check`
**And** runs `cargo clippy --workspace --all-targets -- -D warnings`
**And** runs `cargo test --workspace`
**And** runs `cargo build --workspace --release`
**And** runs the cross-import guardrail script
**And** runs `cargo deny check`
**And** all six jobs pass on the empty workspace

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-006: Community files

**Scenario: Repository has standard OSS community files**
**Given** a fresh clone of the Karoowa repository
**When** the developer inspects the repository
**Then** `.github/CODEOWNERS` exists and assigns `core/` ownership
**And** `CONTRIBUTING.md` exists and references `specs/development/dev_plan.md`
**And** `CODE_OF_CONDUCT.md` contains the Contributor Covenant 2.1 text
**And** `.github/PULL_REQUEST_TEMPLATE.md` exists with checklists for task linkage, tests, and clippy
**And** `.github/ISSUE_TEMPLATE/` contains bug and feature templates

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-007: Phase 1.0 sign-off

**Scenario: Foundation is verified end-to-end**
**Given** all Phase 1.0 tasks (T1.0.1-T1.0.7) are complete
**When** the developer runs all six checks on a fresh clone (`cargo build --workspace --release`, `cargo test --workspace`, `cargo clippy`, `cargo fmt --check`, cross-import guardrail, `cargo deny check`)
**Then** all six commands pass
**And** the CI pipeline on GitHub is green
**And** the commit is tagged `v0.0.1-skeleton`

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-008: Hash type

**Scenario: SHA3-256 hash of known input matches test vector**
**Given** a known input byte sequence
**When** the developer computes `sha3_256(input)`
**Then** the result matches the published NIST SHA3-256 test vector for that input

**Scenario: Hash round-trips through serde**
**Given** a `Hash` value
**When** it is serialized to JSON and deserialized back
**Then** the deserialized value equals the original

**Scenario: Hash displays as hex and parses from hex**
**Given** a `Hash` value
**When** it is formatted via `Display` and parsed via `FromStr`
**Then** the parsed value equals the original

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-009: Address type

**Scenario: Address is derived from public key**
**Given** a `Keypair` with a known public key
**When** the developer calls `keypair.address()`
**Then** the result is the last 20 bytes of SHA3-256 of the public key
**And** the hex representation has a `0x` prefix

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-010: Keypair type

**Scenario: Keypair generates with OS entropy**
**Given** the system's OS random number generator is available
**When** the developer calls `Keypair::generate(&mut OsRng)`
**Then** a valid ed25519 keypair is returned
**And** `keypair.public_key_bytes()` returns 32 bytes
**And** `keypair.address()` returns a valid 20-byte address

**Scenario: Keypair is deterministic from seed**
**Given** a fixed 32-byte seed
**When** the developer calls `Keypair::from_seed(&seed)` twice
**Then** both keypairs produce identical public keys and addresses

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-011: Signature type

**Scenario: Sign and verify round-trip succeeds**
**Given** a `Keypair` and a message
**When** the developer calls `keypair.sign(message)` to produce a `Signature`
**And** calls `signature.verify(keypair.public_key(), message)`
**Then** verification succeeds

**Scenario: Verification fails with wrong public key**
**Given** two different keypairs and a message
**When** the developer signs with keypair A and verifies with keypair B's public key
**Then** verification fails

**Scenario: Verification fails with tampered message**
**Given** a keypair, a message, and a valid signature
**When** the developer modifies the message and verifies against the original signature
**Then** verification fails

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-012: Merkle tree

**Scenario: Merkle root is deterministic**
**Given** a fixed set of leaf hashes
**When** the developer constructs a `MerkleTree::from_leaves(leaves)`
**Then** `tree.root()` returns the same hash on every invocation

**Scenario: Merkle proof verifies correctly**
**Given** a Merkle tree constructed from N leaves
**When** the developer generates a proof for leaf at index `i`
**And** calls `verify_proof(root, leaf, i, proof)`
**Then** verification succeeds

**Scenario: Merkle proof fails for wrong leaf**
**Given** a Merkle tree and a valid proof for leaf at index `i`
**When** the developer calls `verify_proof(root, wrong_leaf, i, proof)`
**Then** verification fails

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-013: Crypto test coverage

**Scenario: Property tests cover all crypto primitives**
**Given** the `karoowa-crypto` test suite
**When** `cargo test -p karoowa-crypto` runs
**Then** proptest-based property tests exercise `Hash`, `Address`, `Keypair`, `Signature`, and `MerkleTree`
**And** round-trip serialization tests pass for all serializable types
**And** test vectors from known SHA3 and ed25519 fixtures all verify

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-014: Transaction type

**Scenario: Transaction is constructed, signed, and serialized**
**Given** a `Keypair` and transaction parameters (from, to, value, nonce, gas_price, gas_limit, data)
**When** the developer constructs a `Transaction`, signs it, and serializes to bincode
**Then** the transaction hash is deterministic
**And** deserialization produces an identical transaction
**And** signature verification succeeds against the signer's public key

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-015: BlockHeader type

**Scenario: Block header hash is deterministic and includes all fields**
**Given** a `BlockHeader` with known field values
**When** the developer computes the header hash
**Then** the hash matches a fixed expected value
**And** changing any single field produces a different hash

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-016: Block type

**Scenario: Block validates its transaction root**
**Given** a `Block` containing a list of transactions
**When** the developer calls block validation
**Then** the block's `tx_root` is verified against the Merkle root of its transactions
**And** validation passes if and only if the roots match

**Scenario: Block hash is derived from its header**
**Given** a `Block` with a valid header
**When** the developer calls `block.hash()`
**Then** the result equals the hash of the block's header

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-017: Receipt type

**Scenario: Receipt captures transaction execution result**
**Given** a transaction that has been executed
**When** a `Receipt` is constructed with tx_hash, status, gas_used, logs, and output
**Then** the receipt round-trips through serialization
**And** each `Log` entry contains an address, topics vector, and data field

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-018: Account state type

**Scenario: Account state tracks nonce and balance**
**Given** an `Account` with initial nonce 0 and balance 1000
**When** a `StateDiff` records a nonce increment and balance deduction
**Then** applying the diff produces an account with nonce 1 and the reduced balance

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-019: Genesis configuration

**Scenario: Genesis config loads from TOML**
**Given** a valid genesis configuration file in TOML format specifying chain_id, initial accounts, and validator set
**When** the developer loads it with `GenesisConfig::from_file(path)`
**Then** the parsed config contains the correct chain_id, initial account balances, and validator addresses

**Scenario: Genesis config loads from JSON**
**Given** a valid genesis configuration file in JSON format
**When** the developer loads it with `GenesisConfig::from_file(path)`
**Then** the parsed config is identical to the equivalent TOML config

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-020: Error types

**Scenario: Crate errors are typed and descriptive**
**Given** an operation in `karoowa-core` that can fail (e.g., invalid genesis config)
**When** the operation fails
**Then** the returned error is a variant of the crate's error enum
**And** the error message is descriptive enough to diagnose the failure without reading source code

**Sad Paths** *(to be added during refinement)*

#### REQ-FC-021: Serialization stability

**Scenario: Serialization vectors are locked**
**Given** a `Transaction`, `BlockHeader`, `Block`, `Receipt`, and `Account` constructed with known field values
**When** each is serialized to bincode
**Then** the output matches a fixed byte vector stored in the test suite
**And** any change to the serialization format causes this test to fail

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-FC-001 | Performance | `cargo build --workspace` (debug, empty stubs) | < 30s on commodity laptop |
| NFR-FC-002 | Performance | `cargo test -p karoowa-crypto` (all tests including proptest) | < 30s |
| NFR-FC-003 | Performance | `cargo test -p karoowa-core` (all tests) | < 30s |
| NFR-FC-004 | Security | All crypto uses audited crates: `ed25519-dalek`, `sha3`, `blake3` | No hand-rolled crypto; enforced by code review |
| NFR-FC-005 | Security | Key generation uses only `OsRng` (no userspace PRNGs) | Enforced in `Keypair::generate` signature |
| NFR-FC-006 | Maintainability | All public items in `karoowa-crypto` and `karoowa-core` have rustdoc | Enforced via `#![deny(missing_docs)]` |
| NFR-FC-007 | Portability | CI runs on `ubuntu-latest`; builds verified on Linux x86_64 | macOS dev support best-effort |
| NFR-FC-008 | Licensing | All transitive dependencies pass `cargo deny check` against allowed list (Apache-2.0, MIT, BSD-2/3, ISC, Unicode-DFS-2016, MPL-2.0) | No GPL/AGPL dependencies |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-FC-001 | ed25519 is the right signature scheme for Karoowa v0.1. No need for secp256k1 or BLS in M1. | If EVM compatibility (REQ-010 from parent PRD) lands earlier than planned, secp256k1 may be needed sooner. The `karoowa-crypto` trait surface should accommodate this. | Confirm with sponsor that ed25519 is sufficient for M1; design the `Keypair`/`Signature` API to be extensible to other schemes |
| ASM-FC-002 | bincode is the right serialization format for internal wire encoding. JSON/TOML are used only for config and human-readable surfaces. | If interoperability with EVM or other chains requires RLP or SSZ encoding, additional serialization support will be needed. | Validate that bincode performance meets NFR targets; design serialization behind traits so format is swappable |
| ASM-FC-003 | 8 crates is the right initial workspace split. No crates will need to be added or merged during M1. | If a crate boundary is wrong (e.g., `karoowa-agents` is needed earlier than Phase 1.11), the workspace layout changes. | Review crate boundaries at the start of each phase; new crates are additive (T1.11.2 adds `karoowa-agents`) |
| ASM-FC-004 | Rust stable (1.78+) is sufficient. No nightly features are required for any M1 work. | If a dependency requires nightly, the toolchain pin changes and contributor friction increases. | `rust-toolchain.toml` pins stable; any nightly requirement is escalated as a blocker |
| ASM-FC-005 | GitHub Actions is the CI platform. No self-hosted runners needed for M1. | If build times exceed GitHub Actions limits, self-hosted runners add ops cost. | Monitor CI times through M1; escalate if approaching limits |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-FC-001 | Rust 1.78+ stable toolchain | Upstream Rust | Resolved | Build prerequisite |
| DEP-FC-002 | `ed25519-dalek` crate (audited) | Upstream | Resolved | Signature scheme |
| DEP-FC-003 | `sha3` crate | RustCrypto | Resolved | Hashing |
| DEP-FC-004 | `blake3` crate | Upstream | Resolved | Alternative hashing |
| DEP-FC-005 | `serde` + `bincode` | Upstream | Resolved | Serialization |
| DEP-FC-006 | `tokio` (async runtime) | Upstream | Resolved | Workspace dependency for later phases |
| DEP-FC-007 | `proptest` (dev dependency) | Upstream | Resolved | Property-based tests |
| DEP-FC-008 | `thiserror` | Upstream | Resolved | Error types |
| DEP-FC-009 | GitHub Actions CI | GitHub | Resolved | CI platform |
| DEP-FC-010 | `cargo-deny` | Upstream | Resolved | License hygiene |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| Storage layer (RocksDB) | Separate feature scope | M1 Feature PRD 2: Storage & Consensus |
| Consensus engine | Separate feature scope | M1 Feature PRD 2: Storage & Consensus |
| Networking (libp2p) | Separate feature scope | M1 Feature PRD 3: Networking & API |
| API gateway | Separate feature scope | M1 Feature PRD 3: Networking & API |
| SDK and CLI | Separate feature scope | M1 Feature PRD 4: Developer Tooling |
| Docker/devnet/install | Separate feature scope | M1 Feature PRD 5: Deployment & Install |
| Agent bundle | Separate feature scope | M1 Feature PRD 6: Agent Bundle |
| Enterprise features (multi-tenancy, RBAC, audit) | Enterprise layer — gated; no enforcement logic in this PRD beyond the `LicenseGate` trait stub | M4+ |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Architecture overview | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan (task details) | `specs/development/dev_plan.md` (Phases 1.0, 1.1, 1.2) | Authoritative |
| Workspace structure | Root `Cargo.toml` (to be created in T1.0.1) | Not started |
| Detailed technical specs per crate | TBD — to be created during implementation | Not started |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-FC-001 | Should `karoowa-crypto` expose a generic `SignatureScheme` trait to accommodate future schemes (secp256k1, BLS), or is a concrete ed25519-only API acceptable for M1? | Tech lead | Before Phase 1.1 | — | Open |
| OQ-FC-002 | Should `GenesisConfig` support both JSON and TOML, or pick one canonical format? dev_plan says "JSON/TOML" but a single canonical format simplifies tooling. | Tech lead | Before T1.2.6 | — | Open |
| OQ-FC-003 | What is the canonical serialization format for wire encoding? bincode is fast but not self-describing. Should we also support a self-describing format (e.g., CBOR) for debugging? | Tech lead | Before Phase 1.2 | — | Open |
| OQ-FC-004 | Should `Cargo.lock` be committed (standard for binary crates) or gitignored? The workspace has both library and binary crates. | Tech lead | Before T1.0.1 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| License file parsing / enforcement logic | Only the trait stub ships in this PRD; actual enforcement is deferred | M4+ |
| secp256k1 / BLS signature schemes | ed25519 is sufficient for M1; other schemes added when EVM compatibility or PoS requires them | M2 (PoS) or EVM milestone |
| Runtime or dynamic dispatch for crypto backends | Concrete implementations with generics are sufficient for M1 | If runtime plugin loading is ever needed |
| Performance benchmarking of crypto primitives | Correctness-first; benchmarking deferred to Phase 1.3+ when there are real workloads to measure against | M1 Feature PRD 2+ |
| Gas metering or execution cost modeling | Core types define `gas_price` and `gas_limit` fields but no metering logic | M3 (WASM VM) |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phases 1.0-1.2, split from the superseded milestone-level M1 PRD. | Generated from `dev_plan.md` Phases 1.0-1.2 and parent PRD `prd_karoowa_overview.md` |
