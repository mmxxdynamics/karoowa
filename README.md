# Karoowa

**Agent-native, Linux-native, Rust-based blockchain framework**
*Light enough to launch anything.*

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.78%2B-orange)](https://www.rust-lang.org)

---

## Overview

Karoowa is a production-grade blockchain infrastructure framework written in Rust. It provides everything needed to launch, operate, and develop against a custom blockchain network — from genesis to devnet to mainnet — with AI agents built in to help at every stage.

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

---

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `karoowa-crypto` | Hash, keypair, signature, Merkle tree, address |
| `karoowa-core` | Block, transaction, state, receipt, config |
| `karoowa-consensus` | Pluggable PoA / PoS / BFT engines (trait-based) |
| `karoowa-storage` | RocksDB persistence (blocks, state, receipts) |
| `karoowa-network` | libp2p P2P layer (Gossipsub + Kademlia) |
| `karoowa-api` | JSON-RPC 2.0 + REST + WebSocket gateway (Axum) |
| `karoowa-sdk` | Developer SDK — wallet, client, contract ABI |
| `karoowa` (CLI) | Node management, wallet, devnet, genesis |

---

## Status

**v0.0.1 — workspace skeleton.** The crate structure is in place; implementation is in progress.

See [`specs/development/dev_plan.md`](specs/development/dev_plan.md) for the current phase and full task breakdown.

---

## Build from Source

```bash
# Rust 1.78+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/karoowa/karoowa
cd karoowa
cargo build --release
```

The binary lands at `target/release/karoowa`.

---

## Project Roadmap

- [ ] **v0.1 (M1)** — Core primitives, PoA consensus, RocksDB storage, API gateway, Docker devnet, CLI, hobbyist install, M1 Dev agent bundle
- [ ] **v0.2 (M2)** — BFT consensus, PoS engine, mempool, WebSocket subscriptions, M2 Ops agent bundle, sidecar runtime
- [ ] **v0.3 (M3)** — WASM smart-contract VM, ABI encoder/decoder, contract SDK, M3 Security/Optimization agent bundle
- [ ] v0.4 — State sync protocol, light-client support, EIP-compatible tx format
- [ ] v0.5 — Cross-chain bridge primitives, IBC adapter
- [ ] v1.0 — Mainnet-ready, external audit, governance module

---

## Documentation

| Document | Purpose |
|----------|---------|
| [`specs/strategy/01_product_vision_and_strategy.md`](specs/strategy/01_product_vision_and_strategy.md) | Vision, audience, personas, success criteria |
| [`specs/strategy/02_technical_strategy.md`](specs/strategy/02_technical_strategy.md) | Architecture, tech stack, open-core strategy |
| [`specs/strategy/03_decision_log.md`](specs/strategy/03_decision_log.md) | All locked decisions with rationale |
| [`specs/development/dev_plan.md`](specs/development/dev_plan.md) | M1–M3 task breakdown (28 phases, ~180 tasks) |
| [`specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`](specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md) | Full overarching PRD |

---

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Run checks: `cargo test --workspace && cargo clippy --workspace -- -D warnings`
4. Format: `cargo fmt --all`
5. Open a pull request

---

## License

The `core/` directory is licensed under **Apache License 2.0** — see [LICENSE](LICENSE).

The `enterprise/` directory is proprietary — see [LICENSE-ENTERPRISE.md](LICENSE-ENTERPRISE.md).
