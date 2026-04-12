# Karoowa — External Audit Scope

**Status:** Draft for audit firm engagement
**Target commit tag:** `v1.0.0-rc1` (to be cut at the end of Phase 6.2)
**Budget envelope:** USD 300K–800K across two engagements

This document defines exactly what is in- and out-of-scope for the Karoowa
v1.0 external security audit. Auditors should quote against this scope.
Any change to the scope after the commit is tagged will be billed as a
re-engagement.

---

## Two-Engagement Model

Karoowa will run **two parallel audit engagements** to get specialist
coverage on the two hardest surfaces.

### Engagement 1 — Systems Audit

**Target firm:** Trail of Bits or Halborn
**Budget:** USD 150K–500K
**Duration:** 6–10 weeks

**In scope:**
- `karoowa-crypto` — signature schemes, hash usage, address derivation
- `karoowa-core` — header/tx/block types, serialization
- `karoowa-trie` — Sparse Merkle Trie, proof generation + verification
- `karoowa-storage` — RocksDB persistence, column family isolation
- `karoowa-consensus` — PoA, PoS, Tendermint-style BFT, pluggable engine
  trait, block producer, mempool
- `karoowa-network` — libp2p behaviour composition, gossipsub validation,
  request-response codecs, peer-id handling
- `karoowa-light` — light-client header verification + validator-set
  handoff
- `karoowa-bridge` — relayer, packet verification, escrow primitives,
  replay protection
- `karoowa-governance` — proposal lifecycle, parameter registry, voting
  thresholds, timelock, veto
- Genesis + config parsing

**Out of scope for Engagement 1:**
- WASM VM + contract execution (see Engagement 2)
- Agent runtime (off-chain, non-consensus)
- Enterprise layer (separate audit track)

### Engagement 2 — WASM VM Audit

**Target firm:** Zellic
**Budget:** USD 150K–300K
**Duration:** 4–8 weeks

**In scope:**
- `karoowa-vm` — wasmtime configuration, fuel metering, memory limits,
  host function surface, ABI encoder/decoder, contract deployment path,
  security sandbox
- Interaction between VM and state trie (how contract writes reach the
  SMT)
- Gas/fuel accounting correctness
- Whitelist of allowed WASM features + imports

**Out of scope for Engagement 2:**
- Consensus, networking, storage (Engagement 1)

---

## Shared Deliverables

Both engagements produce:
1. A findings report classified Critical / High / Medium / Low /
   Informational, each with a reproduction PoC or at minimum a concrete
   attack narrative.
2. A recommendations document for architectural changes.
3. A fuzz target wishlist.
4. A post-fix re-check against all Critical and High findings.
5. A public audit report (published on docs.karoowa.io and linked from
   the repo README).

## Documents Auditors Will Receive

- This scope document (`specs/audit/audit-scope.md`)
- Threat model (`specs/audit/threat-model.md`)
- Architecture overview (`specs/audit/architecture.md`)
- Dev plan M1–M6 (`specs/development/dev_plan.md`,
  `specs/development/dev_plan_m4_m6.md`)
- The tagged commit + full test suite (`v1.0.0-rc1`)
- Access to the testnet and public RPC

## Process

1. **NDAs in place** — before any code sharing.
2. **Kickoff call** — walk auditors through architecture + threat model.
3. **Weekly sync** — progress, blockers, preliminary findings.
4. **Findings dropped in private repo** — we triage within 48h.
5. **Fix cycle** — all Critical + High fixed before audit sign-off.
6. **Public report** — published within 2 weeks of sign-off.

## Fix SLA Commitment

- **Critical** — fix before mainnet, cannot launch with open Criticals.
- **High** — fix before mainnet, or launch with a documented mitigation
  approved by both audit firms.
- **Medium** — fix in a follow-up release within 90 days of mainnet.
- **Low / Informational** — tracked but not blocking.
