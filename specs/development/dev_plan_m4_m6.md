# Karoowa Development Plan — M4 → M6

| Field | Value |
|-------|-------|
| Created | 2026-04-12 |
| Owner | Solo (TBD) |
| Spec source | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` |
| Scope | Phases 4.0 → 6.7 (M4, M5, M6) |
| Pre-reqs | M1 (v0.1), M2 (v0.2), M3 (v0.3) all tagged |
| Format | Dependency-ordered task list. Phases 4.0-4.1 are fully detailed; later phases outlined for just-in-time refinement. |

---

## How to use this plan

1. Each task is sized for one Claude Code session (1–4 hours).
2. Tasks are dependency-ordered within each phase.
3. Each task has explicit acceptance criteria.
4. Cross-reference parent PRD REQ-004 (M4), REQ-005 (M5), REQ-006 (M6).

---

## Plan summary

| Milestone | Phase | Scope | Approx tasks | Approx size |
|-----------|-------|-------|-------------|-------------|
| **M4 (v0.4)** | 4.0 | Merkle state trie (prerequisite for everything) | 6 | 3–4 weeks |
| | 4.1 | State sync protocol | 7 | 3–4 weeks |
| | 4.2 | Light client support | 6 | 2–3 weeks |
| | 4.3 | EIP-1559 compatible transaction format | 7 | 2–3 weeks |
| | 4.4 | M4 Enterprise agent bundle | 5 | 2–3 weeks |
| **M5 (v0.5)** | 5.0 | IBC core integration (`karoowa-ibc` crate) | 7 | 4–6 weeks |
| | 5.1 | Karoowa light client for IBC | 5 | 2–3 weeks |
| | 5.2 | ICS-20 token transfer | 6 | 3–4 weeks |
| | 5.3 | Bridge security hardening | 5 | 2–3 weeks |
| | 5.4 | Relayer integration | 4 | 1–2 weeks |
| **M6 (v1.0)** | 6.0 | On-chain governance module | 8 | 4–6 weeks |
| | 6.1 | Audit preparation | 6 | 3–4 weeks |
| | 6.2 | Testnet progression (public → incentivized → RC) | 5 | 6–8 weeks |
| | 6.3 | Enterprise layer (RBAC, audit logging, HSM) | 7 | 4–6 weeks |
| | 6.4 | Documentation + community | 5 | 2–3 weeks |
| | 6.5 | External security audit | 4 | 8–16 weeks |
| | 6.6 | Bug bounty program | 3 | 2–4 weeks |
| | 6.7 | Genesis ceremony + mainnet launch | 5 | 2–4 weeks |

**Total:** ~18 phases, ~106 tasks, **estimated 12–18 months**. Critical path: Merkle trie → state sync → light client → IBC → audit → mainnet.

---

# M4 — v0.4

> **Goal:** Enable new nodes to join without full replay (state sync), let light clients verify state without running a full node, and add EIP-1559 fee market for compatibility with Ethereum tooling.
>
> **Spec refs:** parent REQ-004, REQ-010 (EIP compat), REQ-011 (M4 Enterprise agent bundle).
>
> **Critical prerequisite:** Replace the placeholder state root in `StateStore::commit()` with a real Merkle trie. This unlocks state sync verification, light client proofs, AND proper header validation simultaneously.

## Phase 4.0 — Merkle State Trie

> **Goal:** Replace the placeholder state root (`sha3_256(serialized_diff)`) with a real Merkle Patricia Trie or Sparse Merkle Trie backed by RocksDB. This is the single most important infrastructure change for M4.
>
> **Decision (T4.0.1):** Use **Sparse Merkle Trie** (256-bit key space, O(log n) proofs). Rationale: simpler than Ethereum's MPT, better performance for our use case, and Karoowa doesn't need EIP state proof interop (we have our own API). If we later need Ethereum state proof compatibility, we can add an MPT adapter.
>
> **Recommended crate:** `sparse-merkle-tree` by Nervos, or custom implementation using the existing `karoowa-crypto::MerkleTree` as a starting point.
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T4.0.1** — Decide: Sparse Merkle Trie vs Merkle Patricia Trie. Document the decision with rationale. Evaluate `sparse-merkle-tree` crate vs building on `karoowa-crypto::MerkleTree`.
- **T4.0.2** — `karoowa-trie` crate (new). Implement `StateTrie` trait: `insert(key, value)`, `get(key)`, `delete(key)`, `root() -> Hash`, `proof(key) -> MerkleProof`, `verify_proof(root, key, value, proof) -> bool`. Backed by RocksDB column family `state_trie`.
- **T4.0.3** — Integrate with `StateStore::commit()`. Replace `sha3_256(diff_bytes)` with trie operations: for each `AccountChange` in the diff, update the trie; return the trie root as the new state root.
- **T4.0.4** — Update `BlockHeader` validation: `state_root` in proposed blocks must match the trie root after applying the block's transactions.
- **T4.0.5** — `MerkleProof` type and verification. Proof-of-inclusion and proof-of-absence for any account or storage key. This is the foundation for light client queries.
- **T4.0.6** — Tests: insert 10K accounts, verify all proofs, delete half, re-verify. Soak test with trie root stability across restarts.

**Acceptance:**
- `StateStore::commit()` returns a cryptographically meaningful state root.
- Merkle proofs verify correctly for any account in the trie.
- Existing storage tests still pass.
- Trie root is deterministic (same inputs → same root regardless of insertion order).

---

## Phase 4.1 — State Sync Protocol

> **Goal:** A new node can download a recent state snapshot from peers and start validating blocks without replaying from genesis.
>
> **Design:** Snapshot-based. At every epoch boundary (configurable, default every 1000 blocks), the node serializes the state trie into sorted chunks (~4 MB each), builds a chunk Merkle tree, and advertises the snapshot to peers. A syncing node discovers available snapshots, downloads chunks in parallel from multiple peers, verifies each chunk against the chunk Merkle root, and reconstructs the state trie.
>
> **Network protocol:** Uses libp2p request-response (`/karoowa/state-sync/1`), not Gossipsub.
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T4.1.1** — `SnapshotStore` trait: `create_snapshot(height) -> SnapshotManifest`, `list_snapshots() -> Vec<SnapshotManifest>`, `get_chunk(snapshot_height, chunk_index) -> Vec<u8>`.
- **T4.1.2** — Snapshot creation: iterate `CF_ACCOUNTS` + `CF_STORAGE`, serialize into sorted ~4 MB chunks, build a chunk Merkle tree, store manifest (height, state root, chunk count, chunk hashes).
- **T4.1.3** — Snapshot compression using `zstd` (consistent with RocksDB's internal compression).
- **T4.1.4** — libp2p request-response protocol `/karoowa/state-sync/1`: `SnapshotRequest { height }` → `SnapshotManifest`, `ChunkRequest { height, index }` → `ChunkData`.
- **T4.1.5** — Sync manager: discover snapshots from peers, select the most recent one with sufficient attestations, download chunks in parallel (max 4 concurrent), verify chunk Merkle proofs, reconstruct state trie.
- **T4.1.6** — CLI flag `--sync-mode state-sync` on `karoowa node`. Falls back to full sync if no snapshot peers are found.
- **T4.1.7** — Tests: create a 10K-account snapshot, serve it to a syncing node, verify state root matches after restoration.

**Acceptance:**
- A new node with `--sync-mode state-sync` reaches the chain head in < 1/10th the time of full replay.
- State root after snapshot restoration matches the state root at the snapshot height.
- Corrupted chunks are detected and re-requested from other peers.

---

## Phase 4.2 — Light Client Support

> **Goal:** A light client can verify account balances and storage values without downloading the full blockchain. Only block headers and Merkle proofs are needed.
>
> **Design:** The light client stores block headers and the current validator set. For state queries, it requests a Merkle proof from a full node and verifies it locally against the `state_root` in the relevant header.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T4.2.1** — Add `validator_set_hash: Hash` to `BlockHeader`. This allows light clients to track validator set transitions without storing the full set in every header.
- **T4.2.2** — `LightClient` struct: stores headers by height, current validator set, trusted checkpoint. Methods: `verify_header(header)`, `sync_headers(range)`, `query_account(addr) -> (Account, MerkleProof)`.
- **T4.2.3** — libp2p request-response protocol `/karoowa/light/1`: `HeaderRequest { from, to }` → `Vec<BlockHeader>`, `ProofRequest { address, height }` → `(Account, MerkleProof)`.
- **T4.2.4** — Trust model: initial trusted checkpoint (genesis or recent header from a known source). Verify subsequent headers by checking BFT quorum signatures against the known validator set. Validator set changes tracked via `validator_set_hash`.
- **T4.2.5** — `karoowa client` CLI extension: `--light` mode that uses the light client instead of full RPC.
- **T4.2.6** — Tests: light client syncs 100 headers, verifies an account balance via Merkle proof, detects a forged header.

**Acceptance:**
- Light client can verify any account state without a full node.
- Forged headers are detected and rejected.
- Header sync from genesis to height 1000 takes < 5 seconds.

---

## Phase 4.3 — EIP-1559 Compatible Transaction Format

> **Goal:** Modernize the transaction format to support EIP-1559 fee markets and typed transactions, enabling future Ethereum tooling compatibility.
>
> **Design:** Refactor `Transaction` into a `TransactionEnvelope` enum with `Legacy` and `Eip1559` variants. Add `base_fee` to `BlockHeader`. Keep bincode as internal wire format; add RLP encoding behind a feature flag for Ethereum-compatible signing.
>
> **Recommended crate:** `alloy-rlp` for RLP encoding.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T4.3.1** — Refactor `Transaction` into `TransactionEnvelope { Legacy(LegacyTx), Eip1559(Eip1559Tx) }`. `Eip1559Tx` has `max_fee_per_gas` and `max_priority_fee_per_gas` instead of `gas_price`.
- **T4.3.2** — Add `base_fee: u64` to `BlockHeader`. Implement EIP-1559 base fee adjustment algorithm: `new_base_fee = parent_base_fee * (1 + (gas_used - target) / target / 8)`.
- **T4.3.3** — Update consensus engines (PoA, PoS, BFT) to calculate and validate `base_fee` in proposed blocks.
- **T4.3.4** — Effective gas price calculation: `min(max_fee, base_fee + priority_fee)`. Update mempool to sort by effective priority fee.
- **T4.3.5** — Add `alloy-rlp` dependency. Implement RLP encoding for transaction signing behind `#[cfg(feature = "rlp")]`.
- **T4.3.6** — EIP-2930 access list support (struct only, no EVM optimization): `access_list: Vec<(Address, Vec<Hash>)>`.
- **T4.3.7** — Tests: base fee adjustment, effective gas price, backward compat with legacy txs, RLP round-trip.

**Acceptance:**
- Both legacy and EIP-1559 transactions are accepted.
- Base fee adjusts correctly over a sequence of blocks.
- Existing tests pass with legacy transactions unchanged.
- Priority fee ordering in mempool is correct.

---

## Phase 4.4 — M4 Enterprise Agent Bundle

> **Goal:** Ship the Enterprise Governance and Finance/Treasury agents as the M4 agent bundle. These are gated to the enterprise layer per REQ-012.
>
> **Spec refs:** parent REQ-011 M4 Enterprise bundle.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T4.4.1** — `GovernanceAgent` (enterprise-only): tools for creating proposals, monitoring votes, executing passed proposals, reporting governance activity.
- **T4.4.2** — `TreasuryAgent` (enterprise-only): tools for monitoring treasury balance, reviewing grant proposals, disbursement tracking.
- **T4.4.3** — Enterprise license gate: these agents refuse to start without a valid enterprise license file.
- **T4.4.4** — Wire into CLI: `karoowa agent governance --license-file <path>`, `karoowa agent treasury --license-file <path>`.
- **T4.4.5** — Tests: agents reject startup without license, tool definitions are correct.

---

# M5 — v0.5

> **Goal:** Cross-chain interoperability. Karoowa chains can communicate with each other and with Cosmos chains via IBC.
>
> **Spec refs:** parent REQ-005.
>
> **Architecture decision:** **Native IBC** via the `ibc-rs` crate. Rationale: (1) Karoowa's Tendermint-style BFT is directly compatible with IBC's `07-tendermint` light client; (2) `ibc-rs` is modular and designed for embedding in non-Cosmos chains; (3) IBC gives immediate interoperability with 50+ Cosmos chains; (4) building a custom bridge would re-invent what IBC provides without the ecosystem.
>
> **Pre-reqs:** M4 complete (Merkle trie for state proofs, light client support).

## Phase 5.0 — IBC Core Integration

> **Goal:** `karoowa-ibc` crate implementing the IBC transport layer (TAO): clients, connections, channels.
>
> **Estimated total:** 4–6 weeks. **Highest-complexity phase in M5.**

**Tasks:**

- **T5.0.1** — Add `ibc-core`, `ibc-clients`, `ibc-apps` as workspace dependencies. Prototype: instantiate an IBC `07-tendermint` client with Karoowa's block header format.
- **T5.0.2** — `karoowa-ibc` crate (new). Implement `ValidationContext` trait from `ibc-rs`, mapping to Karoowa's RocksDB storage.
- **T5.0.3** — Implement `ExecutionContext` trait: store IBC client states, consensus states, connections, channels in dedicated RocksDB column families.
- **T5.0.4** — IBC message handlers: `MsgCreateClient`, `MsgUpdateClient`, `MsgConnectionOpenInit/Try/Ack/Confirm`, `MsgChannelOpenInit/Try/Ack/Confirm`.
- **T5.0.5** — Wire IBC messages into the transaction processing pipeline: IBC messages are system transactions processed alongside user transactions.
- **T5.0.6** — IBC commitment store: packet commitments and acknowledgements stored as Merkle-provable state (using the trie from Phase 4.0).
- **T5.0.7** — Tests: create client, open connection, open channel between two in-process Karoowa nodes.

## Phase 5.1 — Karoowa Light Client for IBC

> **Goal:** Define a custom IBC client type that verifies Karoowa block headers and BFT quorum certificates.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T5.1.1** — `KaroowaClientState` and `KaroowaConsensusState` types conforming to IBC's `ClientState` and `ConsensusState` traits.
- **T5.1.2** — Header verification: validate BFT quorum certificates (from `consensus_data`) against the stored validator set.
- **T5.1.3** — Misbehaviour detection: detect and submit evidence of double-signing (two valid blocks at the same height).
- **T5.1.4** — Client updates: verify header chain, handle validator set rotations.
- **T5.1.5** — Tests: valid header update, invalid header rejection, misbehaviour freeze.

## Phase 5.2 — ICS-20 Token Transfer

> **Goal:** Fungible token transfer between Karoowa chains (and any IBC-enabled chain).
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T5.2.1** — Integrate `ibc-app-transfer` from `ibc-apps`. Implement the `TokenTransferContext` trait mapping to Karoowa's account model.
- **T5.2.2** — Escrow/unescrow logic: lock tokens on the source chain, mint wrapped tokens on the destination.
- **T5.2.3** — Denomination tracking: `ibc/HASH` denomination format for wrapped tokens.
- **T5.2.4** — `kw_ibcTransfer` RPC method and `karoowa client ibc-transfer` CLI command.
- **T5.2.5** — Circuit breakers: rate limit (max transfers per hour), maximum transfer cap, governance-controlled pause.
- **T5.2.6** — End-to-end test: two Karoowa devnets, lock tokens on chain A, mint on chain B, transfer back.

## Phase 5.3 — Bridge Security Hardening

> **Goal:** Harden the IBC implementation against known bridge attack vectors.
>
> **Learned from:** Ronin ($625M, multi-sig compromise), Wormhole ($320M, verification bypass), Nomad ($190M, initialization bug).
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T5.3.1** — Formal specification of bridge invariants: total locked ≥ total minted, no packet replay, timeout correctness.
- **T5.3.2** — Fuzz testing of IBC packet handling with `cargo-fuzz`.
- **T5.3.3** — Initialization safety: all zero-valued states must be explicitly invalid. Upgrade procedures require governance approval with timelock.
- **T5.3.4** — Monitoring integration: the Observability Agent (M2) alerts on unusual bridge activity (large transfers, rapid minting, connection state changes).
- **T5.3.5** — Model checking with Kani (Rust model checker) on the core verification path.

## Phase 5.4 — Relayer Integration

> **Goal:** An operational IBC relayer that shuttles packets between Karoowa chains.
>
> **Estimated total:** 1–2 weeks.

**Tasks:**

- **T5.4.1** — Hermes chain driver for Karoowa: implement the chain-specific interface for the Hermes relayer to talk to Karoowa's RPC.
- **T5.4.2** — Alternatively: minimal standalone relayer using `ibc-relayer-types` that polls two Karoowa nodes and submits relay transactions.
- **T5.4.3** — Relayer Docker image and systemd service for the public devnet.
- **T5.4.4** — Documentation: relayer setup guide, troubleshooting, operational runbook.

---

# M6 — v1.0

> **Goal:** Mainnet-ready. External audit, on-chain governance, reference deployment, enterprise layer, documentation, community, and launch.
>
> **Spec refs:** parent REQ-006, REQ-012 (enterprise), ASM-006 (audit budget).
>
> **Timeline:** This is the longest milestone. Plan for 6–12 months including audit cycles and testnet bake time.

## Phase 6.0 — On-Chain Governance Module

> **Goal:** Token holders and validators can submit proposals, vote, and execute on-chain parameter changes.
>
> **Design:** Two-chamber governance:
> - **Validator chamber:** Parameter changes (block time, gas limits, base fee target) require 2/3+ validator supermajority.
> - **Token chamber:** Treasury disbursements and grants use token-weighted voting with delegation support.
>
> All proposals have a 48-72 hour timelock. Validators can veto with 75% supermajority during the timelock window.
>
> **Reference implementations:** Cosmos SDK `x/gov`, Substrate `pallet-democracy`.
>
> **Estimated total:** 4–6 weeks.

**Tasks:**

- **T6.0.1** — `karoowa-governance` crate (new). Core types: `Proposal`, `Vote`, `ProposalStatus` (Deposit → Voting → Passed/Rejected → Timelock → Executed/Vetoed).
- **T6.0.2** — `GovernableParams` registry: each chain parameter has a valid range, governance tier (validator-only vs. general), and current value. Prevents governance from setting block time to zero.
- **T6.0.3** — Proposal submission: deposit requirement (spam prevention), proposal body (text + executable parameter change).
- **T6.0.4** — Voting: validator-weighted for parameter changes, token-weighted for treasury. Delegation support (delegate your voting power to another address).
- **T6.0.5** — Quorum and threshold logic: configurable per proposal type. Default: 33% quorum, 50% threshold for general; 67% quorum + threshold for parameter changes.
- **T6.0.6** — Timelock and veto: passed proposals enter 48h timelock. 75% validator veto during window.
- **T6.0.7** — Execution: automatic on-chain parameter change at the configured execution height.
- **T6.0.8** — Tests: full proposal lifecycle, quorum failure, veto, parameter change applied, invalid parameter rejected.

## Phase 6.1 — Audit Preparation

> **Goal:** Prepare the codebase for external security audit. Get test coverage, threat model, and documentation to auditor-ready state.
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T6.1.1** — Write threat model document: enumerate trust boundaries, adversary capabilities (network attacker, Byzantine validator, malicious contract), and invariants the system must maintain.
- **T6.1.2** — Achieve ≥80% test coverage on consensus, VM, and bridge code. Add fuzzing targets for critical paths (`cargo-fuzz`).
- **T6.1.3** — Run Clippy, Miri (where applicable), and address all warnings. Audit all `unsafe` blocks (there should be none in Karoowa proper).
- **T6.1.4** — Architecture documentation: sequence diagrams for block production, BFT finality, contract execution, IBC packet flow.
- **T6.1.5** — Freeze audit scope: tag a specific commit (`v1.0.0-rc1`). Auditors charge extra for moving targets.
- **T6.1.6** — Select audit firms. Recommendation: **two engagements** — (1) systems firm (Trail of Bits or Halborn, $150K-$500K) for consensus + networking + crypto; (2) WASM specialist (Zellic) for the VM + contract execution. Budget: $300K-$800K total.

## Phase 6.2 — Testnet Progression

> **Goal:** Run a credible public testnet for 3-6 months before mainnet.
>
> **Estimated total:** 6–8 weeks of setup + 3–6 months of bake time.

**Tasks:**

- **T6.2.1** — **Public testnet launch:** open validator onboarding, public RPC, faucet, block explorer (basic). Upgrade from current devnet (single validator) to multi-validator PoS/BFT.
- **T6.2.2** — **Incentivized testnet:** reward participants with future mainnet tokens for running validators, deploying contracts, reporting bugs. Run for 4-8 weeks.
- **T6.2.3** — **Stress testing:** sustained load generation (transactions, contract deployments, IBC transfers). Target: 100 TPS sustained for 7 days.
- **T6.2.4** — **Mainnet candidate (RC):** feature-frozen testnet mirroring exact mainnet genesis config. Run 2-4 weeks with zero resets.
- **T6.2.5** — **Operator documentation:** hardware requirements, key management (HSM recommendations), upgrade procedures, backup/restore, runbooks for chain halt / consensus split / key compromise.

## Phase 6.3 — Enterprise Layer

> **Goal:** Ship the proprietary enterprise features gated behind the license file.
>
> **License model:** Apache 2.0 for the core chain. BSL (Business Source License) for `enterprise/` modules, with 4-year conversion to open source.
>
> **Revenue model:** Managed deployment ("Karoowa Cloud") + enterprise support tiers + certified AI agent marketplace.
>
> **Estimated total:** 4–6 weeks.

**Tasks:**

- **T6.3.1** — `enterprise/karoowa-rbac`: Role-Based Access Control for node operations and contract deployment. Roles: Admin, Operator, Deployer, Reader.
- **T6.3.2** — `enterprise/karoowa-audit-log`: Immutable, exportable audit log of all admin actions (key rotations, config changes, deployments). SOC 2 compatible format.
- **T6.3.3** — `enterprise/karoowa-hsm`: Hardware Security Module integration for validator key management. Support: AWS CloudHSM, YubiHSM 2.
- **T6.3.4** — `enterprise/karoowa-ha`: High-availability node clustering (active/standby failover).
- **T6.3.5** — `LicenseGate` enforcement: connect the M1 trait stub to actual license file parsing and feature gating.
- **T6.3.6** — Enterprise CLI flags: `--rbac-config`, `--audit-log-path`, `--hsm-provider`.
- **T6.3.7** — Enterprise agent marketplace: curated registry of certified AI agents with enterprise-grade attestation.

## Phase 6.4 — Documentation + Community

> **Goal:** Complete documentation suite and community infrastructure for mainnet launch.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T6.4.1** — **Operator guide:** Full deployment manual (bare metal, Docker, Kubernetes), monitoring setup, upgrade procedures.
- **T6.4.2** — **Developer docs:** RPC/API reference (generated from code), contract SDK tutorial, chain architecture overview, agent integration guide.
- **T6.4.3** — **Tokenomics paper:** Supply schedule, fee model (EIP-1559), staking rewards, treasury funding (% of block rewards), validator economics.
- **T6.4.4** — **Ambassador program:** Launch 6+ months before mainnet. Developer grants for early contract deployments.
- **T6.4.5** — **Website:** karoowa.io with docs, download links, testnet status, community links.

## Phase 6.5 — External Security Audit

> **Goal:** Complete the audit with zero unresolved high-severity findings.
>
> **Estimated total:** 8–16 weeks (audit firm timeline).

**Tasks:**

- **T6.5.1** — **Audit engagement 1:** Systems audit (consensus, networking, crypto). Firm: Trail of Bits or Halborn.
- **T6.5.2** — **Audit engagement 2:** WASM VM + contract execution. Firm: Zellic.
- **T6.5.3** — **Fix cycle:** Address all high and medium findings. Re-submit for re-check.
- **T6.5.4** — **Publish audit report:** Full public disclosure on docs site and GitHub. Link from README.

**Acceptance:**
- 0 unresolved high-severity findings.
- All medium findings either resolved or accepted with documented mitigation.
- Audit report published alongside the v1.0 release.

## Phase 6.6 — Bug Bounty Program

> **Goal:** Continuous security coverage via external researchers.
>
> **Estimated total:** 2–4 weeks to set up; ongoing after launch.

**Tasks:**

- **T6.6.1** — Launch on Immunefi (or HackerOne). Scope: consensus, VM, bridge, agent runtime.
- **T6.6.2** — Tier payouts: Critical ($50K–$250K), High ($10K–$50K), Medium ($2K–$10K), Low ($500–$2K).
- **T6.6.3** — Triage process: dedicated security channel, 48h response SLA, responsible disclosure policy.

## Phase 6.7 — Genesis Ceremony + Mainnet Launch

> **Goal:** Coordinated launch with external validators.
>
> **Estimated total:** 2–4 weeks.

**Tasks:**

- **T6.7.1** — **Genesis parameters:** Publish chain ID, initial supply, validator requirements, staking economics. Public review period (2 weeks).
- **T6.7.2** — **Validator onboarding:** Validators submit signed gentx (genesis transactions) to a public repo. Minimum: 10 external validators.
- **T6.7.3** — **Deterministic binary:** `cargo build --locked` reproducible build. Verify all validators run identical binaries.
- **T6.7.4** — **Launch coordination:** Countdown, genesis block timestamp, coordinated node start.
- **T6.7.5** — **Post-launch monitoring:** 24/7 on-call rotation for first 2 weeks. Incident response runbooks. SLO: 99.95% uptime.

**Acceptance:**
- Mainnet produces blocks with ≥10 validators.
- Governance module active from block 1.
- IBC connections established with ≥1 external chain within 30 days.
- Bug bounty program live.
- Public audit report published.

---

## Decision Log (M4–M6)

| Date | Decision | Rationale | Spec ref |
|------|----------|-----------|----------|
| 2026-04-12 | Sparse Merkle Trie for state root (not MPT) | Simpler, better performance, no Ethereum state proof interop needed | Phase 4.0 |
| 2026-04-12 | Native IBC via `ibc-rs` (not custom bridge) | Direct compatibility with Tendermint BFT, 50+ chain ecosystem, modular Rust impl | Phase 5.0 |
| 2026-04-12 | Tendermint-style BFT confirmed as IBC-compatible | Instant finality, validator set tracking, quorum certificates | Phase 5.1 |
| 2026-04-12 | Two-chamber governance (validator + token) | Balances expert decisions with community input | Phase 6.0 |
| 2026-04-12 | Two audit firms (systems + WASM specialist) | Different expertise needed for different attack surfaces | Phase 6.5 |
| 2026-04-12 | Apache 2.0 core + BSL enterprise | Protects revenue without alienating OSS contributors | Phase 6.3 |
| 2026-04-12 | Immunefi for bug bounty | Industry standard for blockchain; largest researcher pool | Phase 6.6 |

---

## Open Risks

| Phase | Risk | Mitigation |
|-------|------|------------|
| 4.0 | Sparse Merkle Trie performance on large state sets | Benchmark at 1M accounts before committing; MPT fallback if needed |
| 4.3 | EIP-1559 base fee adjustment complexity | Start with fixed base fee, evolve to dynamic |
| 5.0 | `ibc-rs` integration complexity for non-Cosmos chain | Prototype early; engage Informal Systems community for guidance |
| 5.2 | Bridge security — all bridges are attack targets | Defense in depth: circuit breakers, monitoring, formal verification, audit |
| 6.1 | Audit budget ($300K–$800K) | Confirm funding source by M5; scope control if budget is limited |
| 6.2 | Testnet bake time (3–6 months) | Start public testnet during M5 development, not after M5 is done |
| 6.3 | Enterprise market demand unknown | Validate with 3–5 enterprise prospects before building |
| 6.7 | External validator recruitment | Start ambassador program during M4; target blockchain operator communities |
