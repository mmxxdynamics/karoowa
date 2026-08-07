# Karoowa

**Agent-native, Linux-native, Rust-based blockchain framework.**
*Light enough to launch anything.*

[![CI](https://github.com/mmxxdynamics/karoowa/actions/workflows/ci.yml/badge.svg)](https://github.com/mmxxdynamics/karoowa/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![License: BSL-1.1 (enterprise)](https://img.shields.io/badge/Enterprise-BSL_1.1-orange.svg)](LICENSE-ENTERPRISE.md)
[![Rust](https://img.shields.io/badge/Rust-1.94%2B-orange?logo=rust)](https://www.rust-lang.org)
[![docs](https://img.shields.io/badge/docs-mdbook-green)](https://docs.karoowa.io)
[![Security policy](https://img.shields.io/badge/security-policy-blue)](SECURITY.md)

---

## Overview

Karoowa is a production-grade blockchain infrastructure framework written in
Rust. It provides everything needed to launch, operate, and develop against a
custom blockchain network, from genesis to devnet to mainnet, with AI agents
built in to help at every stage.

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

## Status

**`0.6.0-dev`: pre-release.** Milestones M1 through M6 (audit prep) have
landed; the M6 audit and v1.0 mainnet cut are next. APIs are stabilising but
may still change between minor releases until v1.0. See
[`CHANGELOG.md`](CHANGELOG.md) for what shipped in each release and
[`specs/development/dev_plan_m4_m6.md`](specs/development/dev_plan_m4_m6.md)
for what's in flight.

## Install

```sh
# Pre-built binaries (Linux, macOS, Windows). Verifies SHA-256 checksums.
curl -fsSL https://install.karoowa.io | bash

# Or, build from a tagged source release
cargo install --git https://github.com/mmxxdynamics/karoowa --tag v0.5.0 karoowa
```

Releases produced by the v0.6+ release workflow ship Sigstore keyless
signatures, SLSA build-provenance attestations, and a CycloneDX SBOM
(see [`RELEASE.md`](RELEASE.md) for the full verification commands):

```sh
gh attestation verify karoowa-v0.6.0-x86_64-unknown-linux-gnu.tar.gz \
    --repo mmxxdynamics/karoowa
```

## Quickstart

```sh
# 1. Generate a wallet
karoowa wallet new --output ./validator.key

# 2. Start a single-node devnet (PoA)
karoowa node \
    --validator-key ./validator.key \
    --consensus poa \
    --data-dir ./.karoowa/data \
    --rpc-port 8545 \
    --p2p-port 30303

# 3. Hit the JSON-RPC endpoint
curl -s http://localhost:8545/health
```

Multi-validator devnet via Docker:

```sh
karoowa genesis generate --validators 4 --output docker/genesis.toml
docker compose -f docker/devnet.yml up -d
```

## Build from source

```sh
# Toolchain pinned by rust-toolchain.toml (stable, components: rustfmt, clippy)
git clone https://github.com/mmxxdynamics/karoowa
cd karoowa
cargo build --release
./target/release/karoowa --version
```

**MSRV: Rust 1.94.** Bumped from 1.92 to support `wasmtime 47`, which
ships fixes for 16 RUSTSEC advisories (see CHANGELOG). The project tracks
the **N-2** stable Rust policy: the two most recent stables are always
supported.

## Workspace layout

| Crate                   | Purpose                                                    |
| ----------------------- | ---------------------------------------------------------- |
| `karoowa-crypto`        | Hash, keypair, signature, Merkle tree, address             |
| `karoowa-core`          | Block, transaction, state, receipt, config                 |
| `karoowa-consensus`     | Pluggable PoA / PoS / BFT engines (trait-based)            |
| `karoowa-storage`       | RocksDB persistence (blocks, state, receipts)              |
| `karoowa-network`       | libp2p P2P layer (Gossipsub + Kademlia)                    |
| `karoowa-api`           | JSON-RPC 2.0 + REST + WebSocket gateway (Axum)             |
| `karoowa-sdk`           | Developer SDK: wallet, client, transaction builders        |
| `karoowa-vm`            | WASM smart-contract VM and ABI                             |
| `karoowa-trie`          | Sparse Merkle Trie state storage                           |
| `karoowa-light`         | Light-client and state-sync protocol                       |
| `karoowa-bridge`        | Cross-chain bridge primitives                              |
| `karoowa-governance`    | On-chain governance module                                 |
| `karoowa-agents`        | Pluggable LLM-provider agents (OpenAI, Anthropic, local)   |
| `karoowa` (CLI)         | Node, wallet, devnet, genesis, network subcommands         |
| `enterprise/*`          | Proprietary add-ons (BSL 1.1): see LICENSE-ENTERPRISE.md   |

## Documentation

**Guides** (forward-looking sections in the dev/operator guides flagged
inline as _v1.0_):

| Document                                                                                                | Purpose                                                  |
| ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [`docs/developer-guide.md`](docs/developer-guide.md)                                                    | SDK usage, RPC surface, contract dev, agent integration  |
| [`docs/operator-guide.md`](docs/operator-guide.md)                                                      | Running validators, hardening, monitoring, incident runbooks |
| [`docs/tokenomics.md`](docs/tokenomics.md)                                                              | Network economics, staking, rewards                      |

**Project / contributor docs:**

| Document                                                                                                | Purpose                                                  |
| ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [`CHANGELOG.md`](CHANGELOG.md)                                                                          | What shipped in each release                             |
| [`RELEASE.md`](RELEASE.md)                                                                              | Release cadence, signing, SBOM, rollback                 |
| [`SECURITY.md`](SECURITY.md)                                                                            | Vulnerability disclosure + supported versions            |
| [`CONTRIBUTING.md`](CONTRIBUTING.md)                                                                    | Dev checks, commit convention, DCO, open-core boundary   |
| [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)                                                              | Contributor Covenant 2.1                                 |

**Strategy specs:**

| Document                                                                                                | Purpose                                                  |
| ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [`specs/strategy/01_product_vision_and_strategy.md`](specs/strategy/01_product_vision_and_strategy.md)  | Vision, audience, personas, success criteria             |
| [`specs/strategy/02_technical_strategy.md`](specs/strategy/02_technical_strategy.md)                    | Architecture, tech stack, open-core strategy             |
| [`specs/strategy/03_decision_log.md`](specs/strategy/03_decision_log.md)                                | All locked decisions with rationale                      |

## Roadmap

- [x] **v0.1 (M1)**: Core primitives, PoA consensus, RocksDB storage, API gateway, Docker devnet, CLI, hobbyist install, M1 Dev agent bundle
- [x] **v0.2 (M2)**: BFT consensus, PoS engine, mempool, WebSocket subscriptions, M2 Ops agent bundle, sidecar runtime
- [x] **v0.3 (M3)**: WASM smart-contract VM, ABI encoder/decoder, contract SDK, M3 Security/Optimization agent bundle
- [x] **v0.4 (M4)**: State sync, light-client, EIP-1559 / EIP-2718 / EIP-2930 transaction types
- [x] **v0.5 (M5)**: Cross-chain bridge primitives, libp2p bridge request-response
- [ ] **v0.6 (M6)**: Audit-prep, on-chain governance, Enterprise crates (license, audit-log, RBAC, HSM, HA, marketplace)
- [ ] **v1.0**: Mainnet-ready, external audit, bounty programme

## Contributing

We welcome PRs. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development
checks, commit-message convention (Conventional Commits), and the open-core
boundary rules. Report security issues privately via
[`SECURITY.md`](SECURITY.md).

## License

- The `core/` directory is licensed under **Apache License 2.0**,
  see [`LICENSE`](LICENSE).
- The `enterprise/` directory is **proprietary** (Business Source Licence
  1.1, four-year convert to Apache 2.0); see
  [`LICENSE-ENTERPRISE.md`](LICENSE-ENTERPRISE.md).
- Unless explicitly noted, contributions are accepted under the same dual
  licence: anything you submit to `core/` is dual-licensed Apache-2.0 + MIT
  for downstream compatibility.
