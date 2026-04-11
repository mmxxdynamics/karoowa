# PRD: Storage & Consensus (M1 Phases 1.3-1.4)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team (drafted by Claude) |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD covering multiple phases |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** Storage & Consensus (Phases 1.3, 1.4)
> **Owner:** TBD
> **Stakeholders:** Core maintainers, chain builder teams
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Deliver persistent storage and a working Proof-of-Authority consensus engine so that Karoowa can produce, validate, and persist blocks. This PRD covers the "data layer" and "agreement layer" — after this ships, a single-validator node can produce blocks and persist them to disk.

This is the second of six M1 feature PRDs. It depends on Feature PRD 1 (Foundation & Core) for the crypto primitives and core domain types it stores and validates.

### Expected Business Outcomes

- **Blocks, state, and receipts persist across node restarts.** A Karoowa node can be stopped and restarted without losing chain history.
- **Storage is swappable.** The `BlockStore`, `StateStore`, and `ReceiptStore` traits abstract RocksDB so alternative backends (redb, sled) can be substituted without modifying consensus or API code.
- **PoA consensus produces blocks.** A single-validator or multi-validator PoA setup can produce and validate blocks in a round-robin leader rotation, providing the reference consensus engine for all subsequent M1 work.
- **Custom consensus is pluggable.** The `ConsensusEngine` trait is designed so downstream teams can implement their own engine against an unmodified upstream `karoowa-consensus` crate.
- **Atomic persistence.** Block, state, and receipt writes are committed together in a single RocksDB write batch, preventing partial-write corruption.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| RocksDB write throughput (10k blocks soak test) | Completes without error | N/A |
| Random block read latency (from 10k-block store) | < 5ms p99 | N/A |
| PoA block production interval (single validator) | Configurable, default 2s | N/A |
| `cargo test -p karoowa-storage` runtime | < 60s | N/A |
| `cargo test -p karoowa-consensus` runtime | < 30s | N/A |

### User Problems

- **No persistence layer exists.** Without storage, blocks produced by consensus exist only in memory and are lost on restart.
- **No consensus engine exists.** Without a working consensus implementation, there is no block production — the node is inert.
- **Chain builders need to plug in custom consensus.** Karoowa's value proposition includes pluggable consensus without forking the framework. The `ConsensusEngine` trait must be designed for downstream extensibility from the start.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-SC-001 | We believe that **RocksDB with column families** will handle Karoowa's write-heavy block-append workload through v1.0 without a storage rewrite | Write throughput at projected v1.0 rate; compaction stall frequency | Soak test at projected scale during M2/M3 |
| H-SC-002 | We believe that **a trait-based storage abstraction** will let contributors swap backends without touching consensus or API code | Downstream implementation of storage traits compiles and passes the test suite | Recruit one contributor to implement a `redb` backend |
| H-SC-003 | We believe that **a single `ConsensusEngine` trait** is sufficient for PoA, PoS, and BFT consensus engines without requiring runtime loading or dynamic dispatch across process boundaries | All three engines implement the same trait; downstream engines compile against unmodified upstream | Validate when PoS (Phase 2.3) and BFT (Phase 2.4) are implemented |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-SC-001 | As a **Chain Builder**, I want blocks, state, and receipts persisted to disk, so that my chain survives node restarts without data loss. | Phase 1.3 (T1.3.1-T1.3.6); parent REQ-001, REQ-017 | US-001 |
| US-SC-002 | As a **Chain Builder**, I want storage abstracted behind traits, so that I can swap RocksDB for an alternative backend without modifying my chain's consensus or API code. | Phase 1.3 (T1.3.1-T1.3.3); parent REQ-017 | US-003 |
| US-SC-003 | As a **Chain Builder**, I want a working PoA consensus engine that produces blocks in round-robin leader rotation, so that my devnet can produce blocks immediately. | Phase 1.4 (T1.4.1-T1.4.7); parent REQ-001, REQ-007 | US-001, US-002 |
| US-SC-004 | As a **Chain Builder**, I want to implement a custom consensus engine by implementing a single trait, so that I don't have to fork the framework to change consensus. | Phase 1.4 (T1.4.1-T1.4.2); parent REQ-007 | US-003 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **Chain Builder** | Primary consumer. Uses the storage layer to persist their chain's data and the consensus engine (or a custom one) to produce blocks. |
| **Validator Operator** | Depends on storage reliability for node operations and consensus correctness for block production. Not directly interacting with these crates at the API level. |
| **Open-Source Contributor** | May implement alternative storage backends or custom consensus engines against the traits defined here. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **Block persistence** | Consensus produces a block -> storage commits block + state + receipts atomically -> node restarts -> storage reads the latest block correctly | Chain Builder |
| **Storage backend swap** | Implement `BlockStore`, `StateStore`, `ReceiptStore` for an alternative backend -> wire into node binary -> run test suite -> all tests pass | Open-Source Contributor |
| **PoA block production** | Configure validator set -> start node -> PoA engine produces blocks at configured interval -> round-robin leader rotation is correct -> blocks are valid | Chain Builder |
| **Custom consensus** | Implement `ConsensusEngine` trait in downstream crate -> wire into custom node binary -> compile against unmodified upstream -> run alongside PoA on devnet | Chain Builder |

---

## 3. High-Level Requirements

### Phase 1.3 — `karoowa-storage` (RocksDB)

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-SC-001 | `BlockStore` trait: `put_block`, `get_block_by_hash`, `get_block_by_height`, `head`, `iter_blocks(range)` | US-SC-001, US-SC-002 | Must Have | See below |
| REQ-SC-002 | `StateStore` trait: `get_account`, `put_account`, `get_storage`, `put_storage`, `commit(diff: StateDiff) -> StateRoot` | US-SC-001, US-SC-002 | Must Have | See below |
| REQ-SC-003 | `ReceiptStore` trait: `put_receipt`, `get_receipt_by_tx_hash` | US-SC-001, US-SC-002 | Must Have | See below |
| REQ-SC-004 | RocksDB implementation of all three traits with column families: `blocks`, `block_index_by_height`, `state_accounts`, `state_storage`, `receipts`, `tx_index` | US-SC-001 | Must Have | See below |
| REQ-SC-005 | Atomic writes via RocksDB write batches — block, state, and receipts committed together in a single write batch | US-SC-001 | Must Have | See below |
| REQ-SC-006 | Integration tests using `tempfile`: soak test writing 10k blocks, reading random blocks, verifying round-trip | US-SC-001 | Must Have | See below |

### Phase 1.4 — `karoowa-consensus` (trait + PoA)

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-SC-007 | `ConsensusEngine` trait with `propose_block`, `validate_block`, `current_leader`, `name`, `is_validator`. Async via `async_trait` | US-SC-003, US-SC-004 | Must Have | See below |
| REQ-SC-008 | `ConsensusError` enum covering validation failures, leader selection errors, and signature mismatches | US-SC-003 | Must Have | See below |
| REQ-SC-009 | PoA validator set type: ordered list of validator addresses with round-robin leader selection | US-SC-003 | Must Have | See below |
| REQ-SC-010 | `PoAEngine` struct implementing `ConsensusEngine`: block production (sign, bundle transactions, return `Block`) and block validation (signature check, leader-for-round check, parent hash linkage) | US-SC-003 | Must Have | See below |
| REQ-SC-011 | `BlockProducer` task driver: tokio task running the proposer loop, calling `propose_block` at configured interval, handing blocks to storage and network layers | US-SC-003 | Must Have | See below |
| REQ-SC-012 | Consensus tests: single-validator block production, multi-validator round-robin, invalid block rejection, signature mismatch rejection | US-SC-003 | Must Have | See below |

### BDD Scenarios

#### REQ-SC-001: BlockStore trait

**Scenario: Block is stored and retrieved by hash**
**Given** a RocksDB-backed `BlockStore` initialized in a temporary directory
**And** a valid `Block` with a known hash
**When** the developer calls `put_block(block)`
**And** calls `get_block_by_hash(block_hash)`
**Then** the returned block equals the stored block

**Scenario: Block is retrievable by height**
**Given** a `BlockStore` containing blocks at heights 0 through 9
**When** the developer calls `get_block_by_height(5)`
**Then** the returned block has height 5

**Scenario: Head returns the latest block**
**Given** a `BlockStore` containing blocks at heights 0 through 9
**When** the developer calls `head()`
**Then** the returned block has height 9

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-002: StateStore trait

**Scenario: Account state is written and read back**
**Given** a `StateStore` and an `Account` with address `0xabc...`
**When** the developer calls `put_account(address, account)`
**And** calls `get_account(address)`
**Then** the returned account equals the stored account

**Scenario: StateDiff is committed and produces a state root**
**Given** a `StateStore` with initial account states
**When** the developer constructs a `StateDiff` with balance changes and nonce increments
**And** calls `commit(diff)`
**Then** a `StateRoot` hash is returned
**And** subsequent `get_account` calls reflect the updated values

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-003: ReceiptStore trait

**Scenario: Receipt is stored and retrieved by transaction hash**
**Given** a `ReceiptStore` and a `Receipt` for transaction `tx_hash`
**When** the developer calls `put_receipt(receipt)`
**And** calls `get_receipt_by_tx_hash(tx_hash)`
**Then** the returned receipt equals the stored receipt

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-004: RocksDB implementation

**Scenario: RocksDB column families are created correctly**
**Given** a fresh RocksDB instance opened with the Karoowa storage configuration
**When** the developer inspects the column families
**Then** the families `blocks`, `block_index_by_height`, `state_accounts`, `state_storage`, `receipts`, and `tx_index` all exist

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-005: Atomic writes

**Scenario: Block, state, and receipts are committed atomically**
**Given** a new block with associated state changes and receipts
**When** the developer commits all three in a single write batch
**Then** either all three are persisted or none are
**And** a read immediately after commit returns the new block, updated state, and receipts

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-006: Soak test

**Scenario: Storage handles 10k blocks without error**
**Given** a RocksDB-backed storage in a temporary directory
**When** the test writes 10,000 blocks with associated state and receipts
**And** reads 100 random blocks by hash and 100 by height
**Then** all writes succeed without error
**And** all reads return the correct block data
**And** `head()` returns the block at height 9,999

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-007: ConsensusEngine trait

**Scenario: Downstream team implements a custom consensus engine**
**Given** a downstream Rust crate that depends on `karoowa-consensus`
**When** a developer implements `ConsensusEngine` for a custom struct `MyEngine`
**Then** the downstream crate compiles against an unmodified upstream `karoowa-consensus`
**And** the custom engine's `name()` returns a unique identifier
**And** `propose_block` and `validate_block` are callable

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-008: ConsensusError enum

**Scenario: Validation failure returns a typed error**
**Given** a `PoAEngine` and an invalid block (e.g., wrong proposer for this round)
**When** the engine calls `validate_block(block)`
**Then** the result is `Err(ConsensusError::InvalidLeader { expected, got })`
**And** the error message is descriptive

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-009: PoA validator set and leader rotation

**Scenario: Round-robin leader selection is deterministic**
**Given** a validator set of addresses `[A, B, C, D]`
**When** the engine is asked for the leader at heights 0, 1, 2, 3, 4
**Then** the leaders are `A, B, C, D, A` respectively

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-010: PoAEngine block production and validation

**Scenario: Single-validator produces a valid block**
**Given** a `PoAEngine` with a single validator
**When** the validator calls `propose_block` with a set of pending transactions
**Then** a `Block` is returned with a valid signature from the proposer
**And** `validate_block` on the same engine returns `Ok`

**Scenario: Multi-validator round-robin produces blocks in order**
**Given** a `PoAEngine` with validators `[A, B, C]`
**When** each validator proposes a block at their assigned height
**Then** blocks at heights 0, 1, 2 are proposed by A, B, C respectively
**And** each block's `parent_hash` links to the previous block

**Scenario: Block from wrong proposer is rejected**
**Given** a `PoAEngine` with validators `[A, B, C]` at height 1 (validator B's turn)
**When** validator A proposes a block at height 1
**Then** `validate_block` returns `Err(ConsensusError::InvalidLeader)`

**Scenario: Block with invalid signature is rejected**
**Given** a valid block with its signature tampered
**When** the engine calls `validate_block`
**Then** the result is `Err(ConsensusError::InvalidSignature)`

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-011: BlockProducer task driver

**Scenario: BlockProducer loop produces blocks at configured interval**
**Given** a `BlockProducer` task configured with a 2-second block time and a running `PoAEngine`
**When** the task is started
**Then** it calls `propose_block` approximately every 2 seconds
**And** each produced block is handed to the storage layer for persistence

**Sad Paths** *(to be added during refinement)*

#### REQ-SC-012: Consensus test suite

**Scenario: All consensus test cases pass**
**Given** the `karoowa-consensus` test suite
**When** `cargo test -p karoowa-consensus` runs
**Then** single-validator production, multi-validator round-robin, invalid block rejection, and signature mismatch rejection tests all pass

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-SC-001 | Performance | RocksDB block write latency (single block + state + receipts) | < 10ms p99 |
| NFR-SC-002 | Performance | RocksDB block read latency (by hash, warm cache) | < 5ms p99 |
| NFR-SC-003 | Reliability | No data loss on clean shutdown and restart | Verified by soak test |
| NFR-SC-004 | Maintainability | Storage traits are backend-agnostic — no RocksDB-specific types leak into trait signatures | Enforced by code review |
| NFR-SC-005 | Portability | RocksDB builds on Linux x86_64 and macOS (dev) | CI verifies Linux; macOS best-effort |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-SC-001 | RocksDB is the right hot-path storage engine for M1-M3. (Inherits parent ASM-017) | Storage rewrite at M4+ | Soak test at projected scale during M2/M3 |
| ASM-SC-002 | Column families provide sufficient isolation for blocks, state, receipts, and tx index | If isolation is insufficient (e.g., compaction interference), column family layout needs revision | Monitor compaction metrics during soak tests |
| ASM-SC-003 | PoA is sufficient as the only consensus engine through M1. PoS and BFT ship in M2. | If PoS is needed earlier (e.g., for staking demos), M2 work pulls forward | Confirm with sponsor that M1 demos use PoA only |
| ASM-SC-004 | `async_trait` is acceptable for the `ConsensusEngine` trait despite its heap allocation. Performance is not a bottleneck at the consensus trait boundary. | If the async overhead matters, the trait needs to use GATs or concrete futures | Benchmark consensus trait call overhead; unlikely to be significant vs. crypto and I/O costs |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-SC-001 | Feature PRD 1 (Foundation & Core) — crypto primitives and core types | Karoowa team | Pending | Storage stores `Block`, `Account`, `Receipt` types; consensus uses `Keypair`, `Signature`, `Hash` |
| DEP-SC-002 | `rocksdb` crate (Rust bindings) | Upstream | Resolved | Storage backend |
| DEP-SC-003 | `tempfile` crate (dev dependency) | Upstream | Resolved | Integration test isolation |
| DEP-SC-004 | `async-trait` crate | Upstream | Resolved | ConsensusEngine trait |
| DEP-SC-005 | `tokio` async runtime | Upstream | Resolved | BlockProducer task driver |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| Mempool | Mempool is M2 scope (Phase 2.0). M1 PoA uses a placeholder in-memory pending pool. | M2 |
| PoS consensus engine | M2 Phase 2.3 | M2 |
| BFT consensus engine | M2 Phase 2.4 | M2 |
| State pruning / archival modes | Not needed for M1 devnet scale | Post-M1 |
| PostgreSQL indexing (L2 storage) | Deferred until a real consumer needs it (parent ASM-019) | When needed |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Parent PRD | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan | `specs/development/dev_plan.md` (Phases 1.3, 1.4) | Authoritative |
| Predecessor PRD | `specs/requirements/product/m1_foundation_core/prd_foundation_core.md` | Draft |
| Database strategy | Parent PRD REQ-017 | Approved |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-SC-001 | Should `StateStore::commit` return only a state root hash, or also the intermediate trie nodes for state-sync support later? Designing for state sync now may save a rewrite at M4. | Tech lead | Before Phase 1.3 | — | Open |
| OQ-SC-002 | Should the `BlockProducer` task driver be part of `karoowa-consensus` or a separate coordinator crate? It couples consensus with storage and network. | Tech lead | Before T1.4.6 | — | Open |
| OQ-SC-003 | What is the default block time for PoA? dev_plan says 2s. Confirm this is the right default for devnet use. | Tech lead | Before T1.4.4 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| State sync protocol | M4 scope | M4 (v0.4) |
| Storage compaction tuning | Not needed at devnet scale | M2/M3 soak testing |
| Consensus finality guarantees | PoA provides instant finality; probabilistic finality is a BFT concern | M2 (BFT) |
| Validator slashing / penalties | PoS/BFT scope | M2 |
| Write-ahead log / crash recovery beyond RocksDB's built-in WAL | RocksDB's WAL is sufficient for M1 | Post-M1 if needed |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phases 1.3-1.4. | Generated from `dev_plan.md` Phases 1.3-1.4 and parent PRD |
