# Karoowa — Technical Strategy

> **Purpose:** A condensed reference for the architectural and tech-stack decisions Karoowa is built around. Distilled from `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` §3–§7 — that PRD remains the canonical, full-detail spec.

---

## High-level architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Developer Interface                   │
│    CLI (karoowa)  ·  SDK  ·  REST/RPC/WS  ·  Agents     │
├──────────────────────┬──────────────────────────────────┤
│   Consensus Engine   │         API Gateway              │
│  PoA · PoS · BFT     │  JSON-RPC 2.0 · REST · WebSocket │
├──────────────────────┴──────────────────────────────────┤
│                    Core Primitives                       │
│        Block · Transaction · State · Receipt             │
├──────────────────┬──────────────────────────────────────┤
│   P2P Network    │           Storage Engine             │
│   libp2p         │  RocksDB  (blocks · state · tx idx)  │
├──────────────────┴──────────────────────────────────────┤
│                  Crypto Primitives                       │
│   Ed25519 · SHA3-256 · BLAKE3 · Merkle · Address        │
└─────────────────────────────────────────────────────────┘
```

The same trait-based seams that let downstream teams plug in custom consensus also let Karoowa swap storage backends, agent runtimes, and LLM providers without leaking into the rest of the codebase.

---

## Workspace crates

| Crate | Purpose |
|-------|---------|
| `karoowa-crypto` | Hash, keypair, signature, Merkle tree, address |
| `karoowa-core` | Block, transaction, state, receipt, config primitives + `LicenseGate` trait |
| `karoowa-consensus` | `ConsensusEngine` trait + reference PoA (M1), PoS (M2), BFT (M2) |
| `karoowa-storage` | RocksDB persistence with column families, behind `BlockStore` / `StateStore` / `ReceiptStore` traits |
| `karoowa-network` | libp2p Gossipsub + Kademlia for block/tx broadcast and peer discovery |
| `karoowa-api` | Axum gateway: JSON-RPC 2.0 + REST + WebSocket on a single port |
| `karoowa-sdk` | Rust client SDK: `NodeClient`, `Wallet`, transaction builders |
| `karoowa` (binary) | CLI: `node`, `wallet`, `devnet`, `client`, `genesis`, `network`, `agent` |
| `karoowa-agents` (M1.11+) | Agent framework + `LlmProvider` trait + LanceDB integration |
| `karoowa-vm` (M3) | WASM execution environment |
| `karoowa-contract-sdk` (M3) | Contract authoring helpers |

---

## Open-core strategy

Karoowa is split into two layers, in **a single monorepo** with directory-level separation enforced by CI.

### Layout

```
karoowa/
├── core/                    ← OSS, Apache 2.0
│   ├── karoowa-crypto/
│   ├── karoowa-core/
│   ├── karoowa-consensus/
│   ├── karoowa-storage/
│   ├── karoowa-network/
│   ├── karoowa-api/
│   ├── karoowa-sdk/
│   └── karoowa/             ← binary
├── enterprise/              ← proprietary, license-gated
│   └── (empty in M1; populated from M4 onward)
├── docker/
├── docs/
├── scripts/
│   └── check-cross-imports.sh
├── specs/
├── Cargo.toml               ← workspace
├── LICENSE                  ← Apache 2.0 (OSS layer)
├── LICENSE-ENTERPRISE.md    ← proprietary license placeholder
└── README.md
```

### What lives where

| OSS layer (`core/`) | Enterprise layer (`enterprise/`) |
|---|---|
| Core blockchain nodes & consensus | Agent governance / policy engine |
| Agent runtime framework | Multi-tenancy + RBAC |
| `LlmProvider` trait + open providers | High-availability nodes |
| Basic on-chain oracles | Advanced analytics / GUI dashboards |
| Dev CLI | SSO / SAML integration |
| SDKs | MPC key management |
| M1–M3 agent bundles | Audit / compliance tooling |
| | M4 Governance + Finance/Treasury agents |
| | Custom SLAs and premium support |

### Enforcement

- **CI guardrail script** (`scripts/check-cross-imports.sh`) fails any build where a `core/` source file imports from `enterprise/`. Bash + ripgrep, runs on every PR.
- **License file** required at startup for any enterprise feature. Modelled on Elasticsearch X-Pack: signed file, optional trial mode, no online phone-home (hostile to air-gapped enterprise deployments).
- **Commercial EULA** covers legal use of `enterprise/` artifacts.
- **Community builds explicitly exclude `enterprise/`** at build time.

---

## Tech stack (locked)

### Languages, runtime, build

| Layer | Choice | Why |
|-------|--------|-----|
| Implementation language | **Rust 1.78+** | Memory safety, no GC, near-C performance for crypto/storage, mature async ecosystem, Linux-first |
| Async runtime | **tokio** | Standard, well-supported, integrates with everything we need |
| Workspace tooling | **Cargo workspace** with `[workspace.dependencies]` for shared versions | Single-source dependency pinning |
| Toolchain pin | **`rust-toolchain.toml`** | Reproducible contributor environments |

### Cryptography

| Primitive | Algorithm | Crate |
|-----------|-----------|-------|
| Hashing (primary) | SHA3-256 | `sha3` |
| Hashing (fast path) | BLAKE3 | `blake3` |
| Signing | Ed25519 | `ed25519-dalek` |
| Merkle tree | Binary, SHA3-256 internal nodes | hand-rolled |
| Addresses | Last 20 bytes of `SHA3-256(public_key)` | hand-rolled |
| Key derivation | OS entropy via `OsRng` | `rand_core` |

**Rule:** No hand-rolled crypto. Audited primitives only.

### Storage (REQ-017 — four-layer architecture)

| Layer | Backend | Status |
|-------|---------|--------|
| **L1** Hot path (blocks, state, receipts, tx index) | **RocksDB** with column families | Locked. Battle-tested in Bitcoin Core, Geth, Solana, Cosmos. |
| **L2** Indexing (dApp queries, future block explorer) | **PostgreSQL** | **Deferred** until an explicit consumer requires it. |
| **L3** Agent memory / RAG | **LanceDB** (Apache 2.0, embedded, in-process) | Locked for M1. Rust-native vector store. |
| **L4** Telemetry | **Prometheus** for metrics | Locked. ClickHouse only if v1.0 surfaces a real need (deferred). |

All four layers sit behind crate-level traits so backends can be swapped without leaking into agent or consensus code.

### Networking

| Concern | Choice |
|---------|--------|
| P2P framework | **libp2p** (Rust) |
| Block/tx broadcast | Gossipsub |
| Peer discovery | Kademlia DHT |
| Transport | TCP + Noise + Yamux |

### API gateway

| Concern | Choice |
|---------|--------|
| HTTP framework | **Axum** (tower-compatible, async-first) |
| Protocols on a single port | JSON-RPC 2.0 + REST + WebSocket |
| Method namespace | `kw_*` (e.g. `kw_blockNumber`, `kw_subscribe`, `kw_sendRawTransaction`) |
| Metrics | Prometheus on `/metrics` (port 9090) |
| Health | `/health` (port 8545) |

### Agent framework

| Concern | Choice |
|---------|--------|
| LLM abstraction | **`LlmProvider` trait** (pluggable per agent, config-selectable, no recompile) |
| Launch providers | Anthropic (hosted) · OpenAI (hosted) · Google Gemma 4 via Ollama (local) · generic GGUF |
| Hobbyist default | **Hosted provider with API key** (Anthropic recommended) |
| No-key fallback | **Gemma 4 E2B** (5B params, Apache 2.0, GGUF via Ollama). **Subject to viability spike T1.11.1.** |
| Memory / RAG | LanceDB embedded (L3 storage layer) |
| Embedding model | Small open-source embedding (TBD during T1.11.6) |

### Agent runtime modes (REQ-015 — three-mode hybrid)

| Mode | When | Where it runs |
|------|------|--------------|
| **In-process** | M1/M2 hobbyist default. Low-end hardware (≤4 GB). | Inside the `karoowa` binary. |
| **Sidecar** | Recommended for ≥8 GB hosts. **Mandatory at M3.** | Separate process, loopback proxy with auth + quota ("padded room" pattern). |
| **Cloud-hosted** | Enterprise capability. | Karoowa-managed agent runtime. Node-side credentials stay on the customer host. |

---

## Hobbyist install (REQ-013) — install channels

All channels supported:

- **One-liner:** `curl -fsSL install.karoowa.io | sh`
- **Prebuilt static binaries:** GitHub Releases for Linux x86_64/aarch64, macOS, Windows
- **Package managers:** Homebrew (macOS), APT (Debian/Ubuntu), RPM (Fedora/RHEL), Chocolatey (Windows), Scoop (Windows)
- **Docker:** Optional containerised install

The install path requires **no Cargo, no Docker, no build toolchain**.

---

## Public Karoowa Devnet (REQ-016)

| Concern | Decision |
|---------|----------|
| Operator | Karoowa Foundation / core team |
| Accountable role | **Karoowa Infrastructure Lead** (named person TBD) |
| Progression | **Devnet → Public Testnet → Mainnet** |
| SLO targets | 99.5% (devnet) → 99.9% (testnet) → 99.95% (mainnet) |
| Budget (M1 default) | **Low scenario ~$850/yr** (1 small VM + static IP + minimal monitoring). Med ~$1,850. High ~$4,850. |
| Funding source | Karoowa treasury / sponsor (final figure TBD) |

---

## Cross-cutting principles

- **Trait-based seams.** Anything that might conceivably need swapping (storage, consensus, LLM provider, agent runtime, license gate) is behind a trait at the crate level.
- **No hand-rolled crypto.** Audited primitives only.
- **No premature operational cost.** Postgres, ClickHouse, hosted services — added when a real consumer requires them, not speculatively.
- **Test-first for the hot path.** `cargo test --workspace` must pass on every PR. Property tests for crypto and serialization. Soak tests for storage and consensus.
- **Linux x86_64 + aarch64 first.** macOS dev support is best-effort.
- **All public APIs have rustdoc.** `#![deny(missing_docs)]` on public crates.

---

## Open technical questions blocking specific phases

These are open in the parent PRD and resolve before the relevant phase starts.

| Phase | Open question | What's needed |
|-------|---------------|---------------|
| Phase 1.0 | OQ-A002 — CI guardrail script language | Bash + ripgrep recommended; confirm |
| Phase 1.10 | OQ-027 — named Infrastructure Lead | Sponsor decision |
| Phase 1.10 | OQ-028 — devnet budget final figure | Sponsor decision |
| Phase 1.11 | OQ-021/024 — local-model viability on 4 GB VPS | Spike T1.11.1 |
| Phase 2.4 | OQ-004 — BFT algorithm | Tech lead decision (Tendermint / HotStuff / custom) |
| Phase 3.0 | OQ-005 — WASM runtime | Tech lead decision (`wasmtime` / `wasmer`) |
| Phases 3.x | OQ-006 — EVM compatibility milestone | Sponsor + tech lead decision |

---

## Where to look for more

- **Full PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`
- **Product vision:** `specs/strategy/01_product_vision_and_strategy.md`
- **Locked decisions:** `specs/strategy/03_decision_log.md`
- **Development plan:** `specs/development/dev_plan.md`
