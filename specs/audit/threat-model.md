# Karoowa — Threat Model

**Status:** Draft for external audit scoping
**Owner:** Karoowa core
**Last updated:** 2026-04-12
**Audit scope tag:** `v1.0.0-rc1` (pending)

This document is the authoritative pre-audit threat model for Karoowa v1.0.
It enumerates trust boundaries, adversary capabilities, and the invariants
the system must maintain under attack. External auditors should use it to
shape their testing plan and to identify invariants that are **not** yet
documented so we can add them.

---

## 1. System Overview

Karoowa is an agent-native, Rust-based Layer-1 blockchain framework. It
ships as a workspace of 14 crates:

| Crate | Role |
|---|---|
| `karoowa-crypto` | Ed25519 signing, SHA3-256, address derivation |
| `karoowa-core` | Block, Transaction, header, envelope types |
| `karoowa-trie` | Sparse Merkle Trie with inclusion/exclusion proofs |
| `karoowa-storage` | RocksDB persistence with column families |
| `karoowa-consensus` | Pluggable engine: PoA, PoS, Tendermint-style BFT |
| `karoowa-vm` | wasmtime-based contract execution environment |
| `karoowa-light` | Light-client verification (header chain + validator sets) |
| `karoowa-bridge` | Lock-and-mint cross-chain primitives |
| `karoowa-governance` | Two-chamber on-chain governance with timelock + veto |
| `karoowa-network` | libp2p: gossipsub, kademlia, state-sync, light, bridge |
| `karoowa-api` | Axum HTTP RPC gateway |
| `karoowa-sdk` | Client-side SDK |
| `karoowa-agents` | Agent runtime (security, optimizer, governance, treasury) |
| `karoowa` | Node binary |

**Zero `unsafe` blocks** across the entire workspace (verified 2026-04-12).

---

## 2. Trust Boundaries

Each boundary is where untrusted input crosses into trusted code paths.
Auditors should focus exploit research on these.

### 2.1 P2P Network Boundary (`karoowa-network`)
- **Untrusted input:** gossipsub messages (blocks, transactions), kademlia
  DHT records, request-response payloads (state-sync chunks, light-client
  headers, bridge packets).
- **Trust gate:** libp2p signature verification (for gossipsub `Strict`
  mode), message-id dedup, per-protocol codec deserialization.
- **Post-gate validators:** `ConsensusEngine::validate_block`,
  `Mempool::accept_tx`, light-client header verification,
  `BridgeRelayer::receive_packet`.

### 2.2 RPC Boundary (`karoowa-api`)
- **Untrusted input:** JSON-RPC / REST requests from any client.
- **Trust gate:** Axum extractors + explicit request schema.
- **Protected resources:** mempool admission, storage read queries,
  contract deployment, governance proposal submission.

### 2.3 Contract Execution Boundary (`karoowa-vm`)
- **Untrusted input:** WASM bytecode uploaded by any account.
- **Trust gate:** wasmtime instantiation (memory limits, fuel metering,
  disallowed imports).
- **Invariant:** a contract can only touch its own storage column +
  whitelisted host functions (balance, call, storage).

### 2.4 Bridge Boundary (`karoowa-bridge`)
- **Untrusted input:** `BridgePacket` + source-chain Merkle proof from a
  relayer.
- **Trust gate:** `PacketProof::verify_against(source_state_root)` — SMT
  inclusion proof plus recency check against a light-client-attested
  source root.
- **Invariant:** no double-mint, no mint without a matching lock, no
  release without a matching burn.

### 2.5 Governance Boundary (`karoowa-governance`)
- **Untrusted input:** `Proposal`, `Vote`, `add_deposit` calls.
- **Trust gate:** `GovernableParams::validate_change` at submit time,
  chamber eligibility, duplicate-vote check, voting-window check,
  timelock expiry, validator veto check.
- **Invariant:** no parameter can be set outside its declared range
  (block_time cannot be 0, gas_limit cannot be 0, etc.); no proposal
  can be executed before its timelock expires; no non-validator can veto.

### 2.6 Genesis / Config Boundary
- **Untrusted input:** genesis.json, node config.
- **Trust gate:** operator responsibility — auditors should verify the
  parser fails closed on malformed input and that genesis hash is
  deterministically derived from file contents.

---

## 3. Adversary Model

Auditors should assume any combination of the below.

### A1. Network Attacker
- Full control of the public internet. Can drop, delay, reorder, duplicate,
  or inject packets on any link that isn't over an authenticated libp2p
  transport.
- Cannot forge Ed25519 signatures or break SHA3-256.
- Cannot observe node-internal state.

### A2. Byzantine Validator (< 1/3)
- Controls < 1/3 of validator stake.
- Signs equivocating messages, censors transactions, votes against the
  honest leader, proposes invalid blocks.
- Goal: halt chain, censor txs, fork, double-spend.

### A3. Byzantine Validator Coalition (≥ 1/3, < 2/3)
- Can stall finality but cannot commit invalid blocks.
- Goal: DoS the network.

### A4. Malicious Contract
- Uploads arbitrary WASM. Goal: escape the sandbox, read/write other
  contracts' storage, exhaust node memory, cause panics in host code.

### A5. Malicious Relayer (bridge)
- Submits forged packets, replays packets, submits packets with stale
  proofs, equivocates on source state roots.
- Goal: mint wrapped tokens without a corresponding source lock.

### A6. Malicious Governance Participant
- Submits spam proposals, votes twice, votes after close, vetoes without
  validator status, executes before timelock, proposes out-of-range params.
- Goal: brick a chain parameter, drain treasury, bypass safety ranges.

### A7. Compromised Operator
- Out of scope for the protocol audit but in scope for the enterprise
  audit (Phase 6.3): RBAC bypass, audit-log tamper, HSM key exfiltration.

---

## 4. Invariants the System Must Maintain

### I1. Consensus Safety
- No two valid blocks at the same height can be finalized under BFT.
- PoA/PoS cannot produce a block signed by a non-leader for that slot.
- Validator set changes take effect only at the configured epoch boundary.

### I2. State Integrity
- SMT root in `BlockHeader.state_root` deterministically commits to the
  full post-state. Two nodes that applied the same block sequence must
  compute bit-identical state roots.
- No storage write can bypass the SMT update path.

### I3. Transaction Integrity
- A tx's signature is verified before admission to the mempool and again
  at block validation.
- A tx cannot execute twice (nonce monotonicity).
- Gas metering cannot be bypassed; a tx cannot exceed its declared gas
  limit; a block cannot exceed `block_gas_limit`.

### I4. Contract Isolation
- A contract cannot read or write another contract's storage column.
- A contract cannot allocate memory beyond its configured limit.
- A contract cannot consume fuel beyond its gas budget.
- Host function calls are whitelisted; unknown imports fail instantiation.

### I5. Bridge Conservation
- Σ(minted wrapped tokens) == Σ(locked source tokens) at all times.
- A packet with hash `h` can be processed at most once on the destination
  (replay protection).
- A packet cannot be processed against a forged source state root.

### I6. Governance Safety
- `block_time_ms`, `block_gas_limit`, `min_gas_price`, and similar
  validator-tier params can only be changed via `ValidatorOnly` chamber
  with ≥ 2/3 supermajority.
- A proposal cannot transition `Voting → Executed` without passing
  through `Timelock`.
- A parameter value outside its declared range is rejected at submit.

### I7. Light Client Soundness
- A light client that accepts a header sequence cannot be convinced of a
  state root that conflicts with what full nodes would compute.
- Validator set transitions in light-client mode require a signed handoff
  from the prior set.

### I8. Liveness
- If < 1/3 of validators are Byzantine and the network is synchronous,
  the chain makes progress.

### I9. No Unchecked Panics on Untrusted Input
- No `unwrap`, `expect`, array indexing, or integer overflow on a path
  reachable from untrusted input (P2P, RPC, WASM host calls, bridge
  packets) can panic the node process.

---

## 5. Known Limitations / Out of Scope for v1.0 Audit

- **Full IBC:** v1.0 ships a Karoowa-native bridge (lock-and-mint). Full
  `ibc-rs` integration is deferred to a post-1.0 milestone and will be
  audited separately.
- **Enterprise layer:** RBAC, audit log, HSM integration, HA clustering
  are separate Phase 6.3 deliverables and get their own audit track.
- **Agent runtime:** `karoowa-agents` (security, optimizer, governance,
  treasury agents) are off-chain helpers. They make no state commitments
  and their compromise does not break chain safety.
- **Genesis ceremony:** manual/operational — covered by the Phase 6.7
  runbook, not by the protocol audit.

---

## 6. What Auditors Should Deliver

1. **Invariant coverage gap list** — invariants in §4 for which we have
   no (or weak) test coverage; missing invariants we haven't listed.
2. **Exploit findings** classified Critical / High / Medium / Low /
   Informational, with reproduction PoC where applicable.
3. **Architectural recommendations** — particularly around the
   consensus/VM interface, bridge proof verification, and governance
   execution flow.
4. **Fuzz target wishlist** — paths they found brittle that we should
   add to `cargo fuzz` coverage.
