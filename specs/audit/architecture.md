# Karoowa — Architecture Overview for Auditors

**Audience:** external auditors for the v1.0 security review.
**Companion docs:** `threat-model.md`, `audit-scope.md`.

This is a fast-reading tour of the system. It is not the full design doc;
its job is to get an auditor productive within a day.

---

## 1. Layered View

```
+---------------------------------------------------------------+
|  karoowa-api (Axum)          karoowa-sdk (client)             |
+---------------------------------------------------------------+
|  karoowa (node binary)                                        |
+-------------------+-----------------+------------------------+
|  karoowa-         |  karoowa-       |  karoowa-agents        |
|  consensus        |  vm             |  (off-chain helpers)   |
|  (PoA/PoS/BFT)    |  (wasmtime)     |                        |
+-------------------+-----------------+------------------------+
|  karoowa-governance | karoowa-bridge | karoowa-light         |
+-------------------+-----------------+------------------------+
|  karoowa-network (libp2p: gossipsub/kad/request-response)    |
+---------------------------------------------------------------+
|  karoowa-storage (RocksDB)    karoowa-trie (SMT)             |
+---------------------------------------------------------------+
|  karoowa-core (types)         karoowa-crypto (primitives)    |
+---------------------------------------------------------------+
```

**Zero `unsafe` across the workspace** (verified 2026-04-12).

---

## 2. Block Production (happy path)

```
   Mempool              BlockProducer              ConsensusEngine
      │                       │                          │
      │  submit_tx            │                          │
      ├──────────────────────▶│                          │
      │                       │  current_leader()        │
      │                       ├─────────────────────────▶│
      │                       │◀─── Address ─────────────┤
      │                       │                          │
      │  drain(block_gas_lim) │                          │
      │◀──────────────────────┤                          │
      │                       │  propose_block(txs)      │
      │                       ├─────────────────────────▶│
      │                       │◀─── Block ───────────────┤
      │                       │                          │
      │                       │  validate_block          │
      │                       ├─────────────────────────▶│
      │                       │◀─── Ok ──────────────────┤
      │                       │                          │
      │                       │  gossipsub broadcast     │
      │                       ├────────▶ network         │
```

**Invariants the auditor should attack:**
- `propose_block` runs only if `current_leader() == self`.
- `validate_block` is called on every received block *before* it updates
  local state.
- A Byzantine leader who sends a block with bogus tx signatures / bad
  state root is rejected at `validate_block`.

## 3. BFT Finality (Tendermint-style)

```
   ┌── Propose ──┐   ┌── Prevote ──┐   ┌── Precommit ──┐   ┌── Commit ──┐
   │ leader      │──▶│ 2/3 vote   │──▶│ 2/3 vote      │──▶│ finalize   │
   └─────────────┘   └─────────────┘   └───────────────┘   └────────────┘
```

- Lock/unlock rules enforced in `bft::engine`.
- Double-sign evidence stored and slashed in a follow-up phase (6.3).

## 4. Contract Execution

```
   Transaction                    VM (wasmtime)               State Trie
       │                              │                            │
       │  ContractCall(addr, data)    │                            │
       ├─────────────────────────────▶│                            │
       │                              │  instantiate (fuel, mem)   │
       │                              │  call export               │
       │                              │                            │
       │                              │  host: storage_read(key)   │
       │                              ├───────────────────────────▶│
       │                              │◀──── value ────────────────┤
       │                              │                            │
       │                              │  host: storage_write(k, v) │
       │                              ├───────────────────────────▶│
       │                              │                            │
       │                              │  return                    │
       │◀─── receipt ─────────────────┤                            │
```

**Isolation invariants:**
- A contract's storage key is prefixed with its address; host functions
  refuse to read/write outside that prefix.
- Fuel metering cannot be disabled per call.
- Memory limit is enforced at instantiation and cannot be raised at
  runtime.

## 5. Cross-Chain Bridge (lock-and-mint)

```
  Chain A (source)                              Chain B (destination)
       │                                              │
       │  1. user lock(token, amount, recipient_b)    │
       │  2. emit BridgePacket (seq, ...)             │
       │  3. commit packet.hash() in SMT at           │
       │     key = "packet/commitment/{hash}"         │
       │                                              │
       │                  Relayer                     │
       │        ┌───────────────────┐                 │
       │        │ watches A, builds │                 │
       │        │ Merkle proof      │                 │
       │        └─────────┬─────────┘                 │
       │                  │                           │
       │                  │  submit_packet(           │
       │                  │    packet,                │
       │                  │    PacketProof,           │
       │                  │    source_state_root)     │
       │                  └──────────────────────────▶│
       │                                              │
       │                                              │  4. verify proof
       │                                              │     against root
       │                                              │  5. check replay
       │                                              │  6. mint wrapped
       │                                              │     token_B
       │                                              │  7. return Ack
```

**Bridge safety invariants:**
- Replay: `packet.hash()` is recorded on first successful processing;
  subsequent attempts return an error ack.
- Conservation: wrapped supply on B == locked supply on A at any
  finalized height.
- Proof soundness: the destination never mints without a valid SMT
  inclusion proof against a source state root it has been told is
  recent by its light client.

## 6. Governance

```
           submit                          add_deposit (if needed)
             │                                     │
             ▼                                     ▼
   ┌──────────────────┐                  ┌──────────────────┐
   │     Deposit      │─────────────────▶│      Voting      │
   └──────────────────┘                  └────────┬─────────┘
                                                  │ voting_end
                                    ┌─────────────┼──────────────┐
                                    ▼             ▼              ▼
                           ┌─────────────┐ ┌──────────┐ ┌────────────┐
                           │  Timelock   │ │ Rejected │ │   Vetoed   │
                           └──────┬──────┘ └──────────┘ └────────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    ▼             ▼             ▼
                (validator     timelock      executed
                 veto)         not reached
                 Vetoed        TimelockActive
```

- **Validator chamber** (2/3+): parameter changes tier = `ValidatorOnly`.
- **Token chamber** (40% quorum, 50%+1): treasury, non-critical params,
  signaling text.
- **Auto-progression** via `GovernanceModule::tick(height)` called from
  the block producer each block. Deterministic id order.
- **Parameter safety:** `GovernableParams::validate_change` rejects
  values outside the declared range at submit time. Cannot set
  `block_time_ms = 0`.

## 7. Light Client

- Tracks validator set externally from `BlockHeader` to avoid breaking
  header-compat with full nodes.
- Verifies header chain via signatures from the current validator set.
- Validator set handoff requires a signed attestation from the outgoing
  set.
- Used by the bridge destination chain to gate packet processing.

## 8. State Commitment (SMT)

- 256-bit keys, bottom-up build via `BTreeMap` with empty-subtree
  short-circuiting (O(N log N)).
- Proof verification walks leaf → root with `key_bits[255 - depth]`.
  Bit ordering is load-bearing and tested in `karoowa-trie`.
- `BlockHeader.state_root` commits to the full post-state. Two honest
  nodes applying the same block sequence must compute bit-identical
  roots.

## 9. Crates Auditors Should Read First

**Engagement 1 priority order:**
1. `karoowa-crypto/src/` (≈300 LOC) — primitives
2. `karoowa-core/src/` — types
3. `karoowa-trie/src/` — SMT
4. `karoowa-bridge/src/relayer.rs` — bridge verification
5. `karoowa-consensus/src/bft/engine.rs` — BFT state machine
6. `karoowa-governance/src/module.rs` — governance state machine
7. `karoowa-network/src/behaviour.rs` — libp2p composition
8. `karoowa-network/src/bridge.rs` + `light_client.rs` + `state_sync.rs`

**Engagement 2 priority order:**
1. `karoowa-vm/src/lib.rs` — wasmtime config
2. `karoowa-vm/src/host.rs` — host function surface
3. `karoowa-vm/src/abi/` — ABI encoding/decoding
4. `karoowa-vm/src/deploy.rs` — contract deployment path
