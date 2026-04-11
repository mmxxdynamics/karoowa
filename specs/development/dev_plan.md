# Karoowa Development Plan — M1 → M3

| Field | Value |
|-------|-------|
| Created | 2026-04-10 |
| Owner | Solo (TBD) |
| Spec source | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` |
| Scope | Phases 1.0 → 3.7 (M1, M2, M3) |
| Format | Dependency-ordered task list. Phase 1.0 is fully detailed and ready to execute today; later phases are outlined and refined just-in-time as the previous phase nears completion. |

---

## How to use this plan

1. **Each task is sized for one Claude Code session** (1–4 hours of focused work).
2. **Tasks are dependency-ordered.** Don't skip ahead — earlier tasks define the types and traits later tasks depend on.
3. **Each task has explicit acceptance criteria.** A task is "done" when its acceptance bullets all pass on a fresh clone.
4. **Phase 1.0 is fully detailed** because it's the immediate next thing. Later phases list tasks at the title level — refine each one into full detail as you start the phase.
5. **Cross-reference back to the parent PRD** for spec context. Each phase notes which parent REQ-IDs it implements.
6. **Status tracking:** mark each task `[ ]` → `[~]` (in progress) → `[x]` (done) as you go. Or use a real tracker if you prefer.

---

## Plan summary

| Milestone | Phase | Scope | Approx tasks | Approx size |
|-----------|-------|-------|--------------|------------|
| **M1 (v0.1)** | 1.0 | Workspace skeleton + CI + license stub | 8 | Days |
| | 1.1 | `karoowa-crypto` | 6 | 1–2 weeks |
| | 1.2 | `karoowa-core` primitives | 8 | 2–3 weeks |
| | 1.3 | `karoowa-storage` (RocksDB) | 6 | 2–3 weeks |
| | 1.4 | `karoowa-consensus` (trait + PoA) | 7 | 3–4 weeks |
| | 1.5 | `karoowa-network` (libp2p) | 8 | 4–6 weeks |
| | 1.6 | `karoowa-api` (Axum gateway) | 7 | 3–4 weeks |
| | 1.7 | `karoowa-sdk` | 5 | 1–2 weeks |
| | 1.8 | `karoowa` CLI | 6 | 2–3 weeks |
| | 1.9 | Docker devnet + observability | 5 | 1–2 weeks |
| | 1.10 | Hobbyist install + public devnet | 7 | 2–3 weeks |
| | 1.11 | M1 Dev agent bundle | 10 | 4–6 weeks |
| **M2 (v0.2)** | 2.0 | Mempool | 6 | 2–3 weeks |
| | 2.1 | WebSocket subscription manager | 5 | 1–2 weeks |
| | 2.2 | `kw_subscribe` methods | 4 | 1 week |
| | 2.3 | PoS consensus engine | 7 | 3–4 weeks |
| | 2.4 | BFT consensus engine | 8 | 4–6 weeks |
| | 2.5 | M2 Ops agent: CI/CD & Deployment | 6 | 2–3 weeks |
| | 2.6 | M2 Ops agent: Observability | 6 | 2–3 weeks |
| | 2.7 | Sidecar runtime mode | 5 | 2–3 weeks |
| **M3 (v0.3)** | 3.0 | WASM runtime selection + integration | 6 | 2–3 weeks |
| | 3.1 | Contract execution environment | 7 | 3–4 weeks |
| | 3.2 | ABI encoder/decoder | 5 | 1–2 weeks |
| | 3.3 | Contract deployment + invocation | 6 | 2–3 weeks |
| | 3.4 | Contract SDK | 5 | 1–2 weeks |
| | 3.5 | M3 Security agent: Vulnerability Scanner | 6 | 2–3 weeks |
| | 3.6 | M3 Optimization agent: Auto-Scaling/Gas Optimizer | 6 | 2–3 weeks |
| | 3.7 | Sidecar runtime mandatory enforcement | 4 | 1 week |

**Total:** ~28 phases, ~180 tasks, **realistically 12–18 months solo full-time**. Pad estimates by 50% for unknowns.

---

# M1 — v0.1

## Phase 1.0 — Workspace Skeleton, CI, License Stub

> **Goal:** Establish the empty monorepo, the CI baseline, and the open-core scaffolding *before* writing any blockchain code. Every subsequent phase depends on this layout being right.
>
> **Spec refs:** parent REQ-009 (workspace health), REQ-012 (open-core boundary), REQ-013 (install path — this phase scaffolds the install entrypoint), OQ-022/023/029 (monorepo, CI guardrails, license file).
>
> **Estimated total:** 1–3 days of focused work.

---

### T1.0.1 — Initialize root Cargo workspace + monorepo layout
**Status:** `[ ]` | **Session:** 1–2 hours | **Depends on:** none

**Goal:** Create the directory structure and root `Cargo.toml` workspace declaration.

**Tasks:**
- Create `core/` directory with one subdirectory per crate: `karoowa-crypto`, `karoowa-core`, `karoowa-consensus`, `karoowa-storage`, `karoowa-network`, `karoowa-api`, `karoowa-sdk`, `karoowa` (binary crate).
- Create `enterprise/` directory with a `README.md` placeholder describing what proprietary code lives here.
- Create `docker/`, `docs/`, `scripts/`, `specs/` (already exists) at root.
- Each `core/karoowa-*/` directory gets a stub `Cargo.toml` declaring the crate (name, version, edition, license, empty `[dependencies]`) and an empty `src/lib.rs` (or `src/main.rs` for the binary crate).
- Root `Cargo.toml` declares `[workspace]` with `members = ["core/karoowa-*"]` glob (or explicit list), and a `[workspace.package]` section with shared metadata.
- Root `Cargo.toml` `[workspace.dependencies]` section pre-declares versions for the dependencies we know we'll need: `tokio`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `bincode`, `hex`.
- Add `rust-toolchain.toml` pinning Rust 1.78 (or current stable, whichever is newer).
- Add a top-level `.gitignore` with standard Rust entries (`target/`, `Cargo.lock` left untracked for libraries / tracked for the binary, `.env`, `*.swp`).

**Acceptance:**
- `cargo build --workspace` succeeds (compiling 8 empty stubs).
- `cargo metadata --format-version 1` lists all 8 `karoowa-*` member crates.
- `enterprise/` directory exists, contains only `README.md`, no Rust source.
- `git status` shows the new structure.

---

### T1.0.2 — Add LICENSE files and root README
**Status:** `[ ]` | **Session:** 30–60 min | **Depends on:** T1.0.1

**Goal:** Establish the legal + branding surface.

**Tasks:**
- Add `LICENSE` at root containing the Apache 2.0 license text (the OSS layer license).
- Add `LICENSE-ENTERPRISE.md` at root with a placeholder: *"Karoowa Enterprise License — to be drafted before any enterprise feature ships. Until then, all code in `enterprise/` is a placeholder for the enterprise tier and is not licensed for distribution."*
- Add `README.md` at root with:
  - Project name + tagline ("Karoowa — Light enough to launch anything").
  - One-paragraph description from the parent PRD §1.
  - Quickstart placeholder: *"v0.1 in development. Follow `specs/development/dev_plan.md` for current status."*
  - Link to `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` for the spec.
  - Link to `LICENSE` and `LICENSE-ENTERPRISE.md`.
  - Workspace crate table (8 crates) with one-line descriptions matching the parent PRD.

**Acceptance:**
- `LICENSE`, `LICENSE-ENTERPRISE.md`, `README.md` exist at root.
- README renders correctly on GitHub preview (no broken links).
- README contains zero references to "ChainFlux" or `cf_*`.

---

### T1.0.3 — `LicenseGate` trait stub in `karoowa-core`
**Status:** `[ ]` | **Session:** 1 hour | **Depends on:** T1.0.1

**Goal:** Scaffold the license-gate trait so future enterprise features have somewhere to gate behind. **No enforcement logic** — just the trait surface.

**Tasks:**
- In `core/karoowa-core/src/`, create `license.rs`.
- Define `pub trait LicenseGate { fn license_info(&self) -> LicenseInfo; fn is_feature_enabled(&self, feature: &str) -> bool; }`.
- Define `pub struct LicenseInfo { pub edition: Edition, pub features: Vec<String>, pub expires_at: Option<DateTime<Utc>> }`.
- Define `pub enum Edition { Oss, Enterprise }`.
- Provide a default `OssLicenseGate` impl that always returns `Edition::Oss`, no features enabled.
- Re-export from `karoowa-core/src/lib.rs`.
- Add a unit test verifying `OssLicenseGate::is_feature_enabled("anything")` returns `false`.

**Acceptance:**
- `karoowa-core` compiles.
- `cargo test -p karoowa-core` passes.
- `LicenseGate`, `LicenseInfo`, `Edition`, `OssLicenseGate` are public from `karoowa-core`.
- No license file parsing logic exists (deferred to M4).

---

### T1.0.4 — CI cross-import guardrail script
**Status:** `[ ]` | **Session:** 1–2 hours | **Depends on:** T1.0.1

**Goal:** Build the script that fails CI if any `core/` source file imports from `enterprise/`.

**Tasks:**
- Create `scripts/check-cross-imports.sh` (or `.rs` if you prefer a Rust binary — bash + ripgrep is simpler).
- Script logic: walk every file under `core/`, fail (exit non-zero) if any line matches an import path referencing `enterprise/`. Patterns to catch: `use crate::enterprise`, `use enterprise::`, `mod enterprise;`, `path = "../enterprise"`, etc.
- Script must exit 0 if `core/` is clean.
- Script must exit non-zero with a clear error message naming the offending file + line if a cross-import is found.
- Add a test fixture: temporarily create a file under `core/karoowa-core/src/test_cross_import.rs` containing `use enterprise::foo;`, run the script, confirm it fails, then delete the fixture.
- Document the script in `scripts/README.md`.

**Acceptance:**
- Script exits 0 on a clean tree.
- Script exits non-zero on a tree with a deliberate cross-import.
- Script runs in under 5 seconds on the current tree.
- Script is portable (runs on Linux + macOS).

---

### T1.0.5 — GitHub Actions CI baseline
**Status:** `[ ]` | **Session:** 1–2 hours | **Depends on:** T1.0.1, T1.0.4

**Goal:** Set up CI so every PR runs build, test, lint, format, and the cross-import guardrail.

**Tasks:**
- Create `.github/workflows/ci.yml`.
- Jobs:
  - `fmt`: `cargo fmt --all -- --check`
  - `clippy`: `cargo clippy --workspace --all-targets -- -D warnings`
  - `test`: `cargo test --workspace`
  - `build`: `cargo build --workspace --release`
  - `cross-import-guardrail`: runs `scripts/check-cross-imports.sh`
- All jobs run on `ubuntu-latest`. Add macOS later if needed.
- Cache `~/.cargo/registry` and `target/` between runs.
- Trigger on `push` to `main` and on `pull_request`.

**Acceptance:**
- `.github/workflows/ci.yml` exists.
- Pushing the file (or running `act` locally) starts a successful CI run on the current empty workspace.
- All five jobs pass on the empty workspace.

---

### T1.0.6 — `cargo deny` for license + dependency hygiene
**Status:** `[ ]` | **Session:** 30–60 min | **Depends on:** T1.0.5

**Goal:** Catch bad licenses and known-vulnerable dependencies in CI from day one.

**Tasks:**
- `cargo install cargo-deny` (or note in `scripts/README.md` how to install it).
- Add `deny.toml` at root with:
  - Allowed licenses: `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`, `MPL-2.0` (review later).
  - Deny: `GPL-*`, `AGPL-*` (incompatible with our planned enterprise layer).
- Add a `cargo-deny` job to `.github/workflows/ci.yml`.

**Acceptance:**
- `cargo deny check` passes on the current workspace.
- CI runs the deny check on every PR.

---

### T1.0.7 — Repo metadata + community files
**Status:** `[ ]` | **Session:** 30–60 min | **Depends on:** T1.0.2

**Goal:** Make the repo look like a real OSS project from day one.

**Tasks:**
- Add `.github/CODEOWNERS` (placeholder — assign all of `core/` to `@<your-handle>`).
- Add `CONTRIBUTING.md` referencing `specs/development/dev_plan.md` and the parent PRD.
- Add `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1 standard text).
- Add `.github/PULL_REQUEST_TEMPLATE.md` with checkboxes: "Linked to a task in `dev_plan.md`?", "Tests added?", "Clippy clean?".
- Add `.github/ISSUE_TEMPLATE/` with a bug template and a feature template.

**Acceptance:**
- All files exist.
- README links to `CONTRIBUTING.md`.

---

### T1.0.8 — Sanity check + Phase 1.0 sign-off
**Status:** `[ ]` | **Session:** 30 min | **Depends on:** T1.0.1 → T1.0.7

**Goal:** Verify the foundation is solid before starting Phase 1.1.

**Tasks:**
- On a fresh clone (or `git clean -xfd`), run:
  - `cargo build --workspace --release`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `scripts/check-cross-imports.sh`
  - `cargo deny check`
- All six commands must pass.
- Push the branch to GitHub and confirm the CI run is green.
- Tag the commit `v0.0.1-skeleton` (no release, just a marker).
- Update this file: mark Phase 1.0 complete.

**Acceptance:**
- All six checks green locally.
- CI green on GitHub.
- Tag pushed.
- Ready to start Phase 1.1.

---

## Phase 1.1 — `karoowa-crypto`

> **Goal:** Build the crypto primitives all other crates depend on.
>
> **Spec refs:** parent §1 architecture diagram (Crypto Primitives layer), README workspace table.
>
> **Estimated total:** 1–2 weeks.
>
> **Why first after skeleton:** Every other crate depends on `Hash`, `Address`, `Keypair`, `Signature` types. No circular deps possible.

**Tasks (refine each into a full task spec when starting Phase 1.1):**

- **T1.1.1** — `Hash` type (32-byte) with `From<[u8; 32]>`, `Display`, `FromStr`, `serde` round-trip. SHA3-256 + BLAKE3 hashing functions.
- **T1.1.2** — `Address` type (20 bytes, derived from last 20 bytes of `SHA3-256(public_key)`). Hex encoding/decoding with `0x` prefix.
- **T1.1.3** — `Keypair` (ed25519-dalek wrapper). `Keypair::generate(&mut OsRng)`, `Keypair::from_seed(seed: &[u8; 32])`, `keypair.address()`, `keypair.public_key_bytes()`.
- **T1.1.4** — `Signature` type. `keypair.sign(message)`, `signature.verify(public_key, message)`. Serializable.
- **T1.1.5** — Merkle tree (binary, SHA3-256 internal nodes). `MerkleTree::from_leaves(leaves: Vec<Hash>) -> MerkleTree`. `tree.root() -> Hash`. `tree.proof(index) -> Vec<Hash>`. `verify_proof(root, leaf, index, proof) -> bool`.
- **T1.1.6** — Comprehensive unit tests + property tests (use `proptest`) for all primitives. Round-trip serialization tests. Test vectors against known SHA3 / ed25519 fixtures.

---

## Phase 1.2 — `karoowa-core` primitives

> **Goal:** Build the core domain types: blocks, transactions, state, receipts, config.
>
> **Spec refs:** parent §1 architecture diagram (Core Primitives layer).
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T1.2.1** — `Transaction` type: `from`, `to`, `value`, `nonce`, `gas_price`, `gas_limit`, `data`, `signature`. Hashing, signing, serialization.
- **T1.2.2** — `BlockHeader` type: `parent_hash`, `state_root`, `tx_root`, `receipt_root`, `height`, `timestamp`, `proposer`, `consensus_data`. Hashing.
- **T1.2.3** — `Block` type: `header` + `transactions`. `block.hash() -> Hash`. Validation: `tx_root` matches Merkle root of `transactions`.
- **T1.2.4** — `Receipt` type: `tx_hash`, `status`, `gas_used`, `logs`, `output`. Plus `Log` type with `address`, `topics`, `data`.
- **T1.2.5** — `Account` state type: `nonce`, `balance`, `code_hash`, `storage_root`. Plus `StateDiff` for tracking per-block changes.
- **T1.2.6** — `ChainConfig` and `GenesisConfig` types. Genesis loading from JSON/TOML.
- **T1.2.7** — `Result<T>` + error types via `thiserror`. Define crate-wide error enum.
- **T1.2.8** — Re-exports + module structure cleanup. Comprehensive unit tests with fixed serialization vectors so changes to encoding break tests intentionally.

---

## Phase 1.3 — `karoowa-storage` (RocksDB)

> **Goal:** Persistent storage for blocks, state, receipts, tx index. Backed by RocksDB with column families. Abstracted behind traits so the backend is swappable.
>
> **Spec refs:** parent REQ-017 (database strategy L1), ASM-017.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T1.3.1** — `BlockStore` trait. `put_block`, `get_block_by_hash`, `get_block_by_height`, `head`, `iter_blocks(range)`.
- **T1.3.2** — `StateStore` trait. `get_account`, `put_account`, `get_storage`, `put_storage`, `commit(diff: StateDiff) -> StateRoot`.
- **T1.3.3** — `ReceiptStore` trait. `put_receipt`, `get_receipt_by_tx_hash`.
- **T1.3.4** — RocksDB implementation of all three traits. Column families: `blocks`, `block_index_by_height`, `state_accounts`, `state_storage`, `receipts`, `tx_index`.
- **T1.3.5** — Atomic writes via RocksDB write batches (block + state + receipts committed together).
- **T1.3.6** — Integration tests using `tempfile`. Soak test: write 10k blocks, read random blocks, verify round-trip.

---

## Phase 1.4 — `karoowa-consensus` (trait + PoA)

> **Goal:** Define the `ConsensusEngine` trait and ship a working PoA reference implementation.
>
> **Spec refs:** parent REQ-007 (pluggable consensus), README §Consensus Engines.
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T1.4.1** — `ConsensusEngine` trait with `propose_block`, `validate_block`, `current_leader`, `name`, `is_validator`. Async via `async_trait`.
- **T1.4.2** — `ConsensusError` enum.
- **T1.4.3** — PoA validator set type: ordered list of validator addresses, round-robin leader selection.
- **T1.4.4** — `PoAEngine` struct implementing `ConsensusEngine`. Block production: validator signs, bundles transactions, returns `Block`.
- **T1.4.5** — Block validation: signature check, leader-for-this-round check, parent hash linkage.
- **T1.4.6** — `BlockProducer` task driver: tokio task that runs the proposer loop, calls `propose_block` at the configured interval, hands the block off to the storage and network layers.
- **T1.4.7** — Tests: single-validator block production, multi-validator round-robin, invalid block rejection, signature mismatch rejection.

---

## Phase 1.5 — `karoowa-network` (libp2p)

> **Goal:** P2P networking via libp2p — Gossipsub for block/tx broadcast, Kademlia for peer discovery.
>
> **Spec refs:** parent §1 architecture diagram (P2P Network layer), ASM-004.
>
> **Estimated total:** 4–6 weeks. **This is the longest phase in M1**, mostly because libp2p has a steep learning curve.

**Tasks:**

- **T1.5.1** — Choose libp2p version + transport stack (TCP + Noise + Yamux). Skeleton `Network` struct wrapping a libp2p `Swarm`.
- **T1.5.2** — Identity: derive PeerId from `Keypair` (libp2p ed25519, distinct from validator keys but can share entropy).
- **T1.5.3** — Kademlia: bootnode list config, peer discovery, peer routing.
- **T1.5.4** — Gossipsub: topics for `blocks`, `transactions`. Message validation hooks.
- **T1.5.5** — Outbound API: `broadcast_block(block)`, `broadcast_transaction(tx)`. Inbound API: `subscribe_to_blocks() -> Stream<Block>`, `subscribe_to_transactions() -> Stream<Transaction>`.
- **T1.5.6** — Connection lifecycle: connect, disconnect, peer score, ban list.
- **T1.5.7** — `cf_peerCount` / `kw_peerCount` data: expose current connected peer count.
- **T1.5.8** — Integration tests: spin up two in-process nodes, broadcast a block from one, verify the other receives it within 1 second.

---

## Phase 1.6 — `karoowa-api` (Axum gateway)

> **Goal:** Single-port gateway exposing JSON-RPC, REST, and WebSocket. Implement all 14 inherited methods.
>
> **Spec refs:** parent REQ-008 (single-port multi-protocol), parent REQ-001 (the 14 methods), README §JSON-RPC Methods.
>
> **Estimated total:** 3–4 weeks.

**Tasks:**

- **T1.6.1** — Axum router skeleton. Routes: `/rpc` (POST, JSON-RPC), `/api/v1/*` (REST), `/ws` (WebSocket upgrade), `/health`, `/metrics`.
- **T1.6.2** — JSON-RPC 2.0 dispatcher: parse `JsonRpcRequest`, route to handler by method name, return `JsonRpcResponse`. Error handling.
- **T1.6.3** — Implement read methods: `kw_chainId`, `kw_blockNumber`, `kw_getBlockByNumber`, `kw_getBlockByHash`, `kw_getTransactionByHash`, `kw_getTransactionReceipt`, `kw_getBalance`, `kw_getTransactionCount`, `kw_getCode`, `kw_syncing`, `kw_peerCount`, `kw_nodeInfo`.
- **T1.6.4** — Implement write methods: `kw_sendRawTransaction` (broadcast via network layer + add to pending), `kw_pendingTransactions` (read mempool — note: real mempool comes in M2; M1 uses a placeholder in-memory pending pool).
- **T1.6.5** — REST equivalents: `/api/v1/status`, `/api/v1/blocks/<height>`, `/api/v1/blocks/<hash>`, `/api/v1/tx/<hash>`, etc.
- **T1.6.6** — `/health` returning HTTP 200 with basic node status. `/metrics` exposing Prometheus metrics (block height, peer count, RPC request count, RPC latency histograms).
- **T1.6.7** — WebSocket endpoint: handshake, basic ping/pong, **placeholder** subscribe handler. Real subscription work lands in Phase 2.1.

---

## Phase 1.7 — `karoowa-sdk`

> **Goal:** Rust client SDK so dApp developers don't have to hand-roll JSON-RPC clients.
>
> **Spec refs:** README §SDK Usage.
>
> **Estimated total:** 1–2 weeks.

**Tasks:**

- **T1.7.1** — `NodeClient` struct wrapping `reqwest`. Methods mirroring the JSON-RPC surface: `chain_id()`, `block_number()`, `get_balance(addr)`, etc.
- **T1.7.2** — `Wallet` struct wrapping a `karoowa-crypto::Keypair`. `Wallet::generate(chain_id)`, `wallet.address()`, `wallet.sign_transfer(to, value, nonce, gas_price, gas_limit)`.
- **T1.7.3** — Transaction builder helpers. `TransferBuilder`, `ContractCallBuilder` (latter is a placeholder for M3).
- **T1.7.4** — Async examples in `examples/` directory matching the README §SDK Usage snippet.
- **T1.7.5** — Integration tests against a live in-process node.

---

## Phase 1.8 — `karoowa` CLI

> **Goal:** Single binary with all 6 inherited subcommands.
>
> **Spec refs:** README §CLI Reference.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T1.8.1** — `clap` skeleton. Top-level binary with subcommands `node`, `wallet`, `devnet`, `client`, `genesis`, `network`.
- **T1.8.2** — `karoowa wallet` — `new`, `address <key>`, `sign <key> <message>`.
- **T1.8.3** — `karoowa node` — start a node with `--validator-key`, `--consensus`, `--data-dir`, `--bootnodes`, `--rpc-port`, `--metrics-port`, `--license-file` (no-op for now per T1.0.3).
- **T1.8.4** — `karoowa genesis` — `generate`, `validate`. Genesis config schema + validator.
- **T1.8.5** — `karoowa client` — quick wrapper over the SDK for one-shot RPC calls.
- **T1.8.6** — `karoowa devnet` and `karoowa network` — utilities for local devnet bring-up + peer info dumps.

---

## Phase 1.9 — Docker devnet + observability

> **Goal:** Single-node + 4-validator Docker setup, Grafana dashboard.
>
> **Spec refs:** parent REQ-001 BDD scenarios, README §Docker.
>
> **Estimated total:** 1–2 weeks.

**Tasks:**

- **T1.9.1** — `docker/Dockerfile` — multi-stage build, statically linked release binary, minimal base image (`gcr.io/distroless/cc-debian12` or `alpine` with musl target). Image tagged `karoowa/karoowa:dev`.
- **T1.9.2** — `docker/docker-compose.yml` — single-node, persistent volume, port mapping for `8545` (RPC) and `9090` (metrics).
- **T1.9.3** — `docker/devnet.yml` — 4-validator setup, shared bridge network, env-var injected validator keys.
- **T1.9.4** — Grafana + Prometheus stack in Compose. Pre-loaded dashboard JSON showing block height, peer count, RPC throughput, RPC latency.
- **T1.9.5** — End-to-end test: bring up devnet, wait for 10 blocks, verify all 4 validators agree on the head block hash, tear down.

---

## Phase 1.10 — Hobbyist install + public devnet

> **Goal:** A solo dev with no Rust toolchain can install Karoowa with one command and join the public devnet.
>
> **Spec refs:** parent REQ-013 (install paths), REQ-016 (public devnet), ASM-011, OQ-027/028.
>
> **Estimated total:** 2–3 weeks.

**Tasks:**

- **T1.10.1** — GitHub Releases pipeline: cross-compile `karoowa` binary for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Upload to releases on tag.
- **T1.10.2** — `install.sh` script (`curl -fsSL install.karoowa.io | sh`): detect OS + arch, download matching binary from GitHub Releases, install to `~/.karoowa/bin/karoowa`, add to PATH instructions.
- **T1.10.3** — Homebrew formula in a `karoowa/homebrew-tap` repo. Test `brew install karoowa/tap/karoowa`.
- **T1.10.4** — `.deb` and `.rpm` packages via `cargo-deb` / `cargo-rpm`. Optional: APT/RPM repo hosting.
- **T1.10.5** — Provision the public devnet: 1 small VM (low-cost scenario from OQ-028), bootnode running, faucet running, status page.
- **T1.10.6** — Faucet: simple Axum service exposing `POST /faucet` that signs a transfer from a treasury key. Rate-limited per IP.
- **T1.10.7** — `karoowa node --join public-devnet` flag: pre-configured bootnode list points to the public devnet IPs.

---

## Phase 1.11 — M1 Dev agent bundle

> **Goal:** Ship the first agents — the CLI/Dev Agent, basic Monitoring Agent, and Onboarding Agent persona — running in **in-process** mode.
>
> **Spec refs:** parent REQ-011 (M1 Dev bundle), REQ-014 (LLM provider trait), REQ-015 (in-process runtime mode), REQ-017 (LanceDB for L3 agent memory), ASM-014a, ASM-018, OQ-021/024 (the viability spike happens here).
>
> **Estimated total:** 4–6 weeks. **Highest-risk phase in M1 — viability spike T1.11.1 may force scope changes.**

**Tasks:**

- **T1.11.1** — **VIABILITY SPIKE** for OQ-021/024: provision a 4 GB VPS, install `ollama`, pull Gemma 4 E2B (5B), run a sample prompt, measure peak RAM and latency. Decide: viable, viable-with-tweaks, or not viable. Document the result. **If not viable**, hosted LLM becomes the strict default and the no-key fallback is dropped or reduced to a smaller model.
- **T1.11.2** — `karoowa-agents` crate (new). Define `LlmProvider` trait: `complete(prompt: Prompt) -> Result<Completion>`. Provider config struct.
- **T1.11.3** — `AnthropicProvider` impl: HTTPS client, API key from env, completion request. Tested against a real key.
- **T1.11.4** — `GemmaLocalProvider` impl: shells out to `ollama` or talks to its HTTP API on localhost. Configurable model name.
- **T1.11.5** — `Agent` trait: `name`, `system_prompt`, `tools`, `step(input) -> Output`. Tool-use via the chosen LLM provider's tool-calling shape.
- **T1.11.6** — `LanceDB` integration in `karoowa-agents` for agent memory. Embedding model selection (small open-source embedding via `ollama` or hosted). `MemoryStore::insert`, `MemoryStore::query`.
- **T1.11.7** — `OnboardingAgent` (persona form for the Solo Operator): tools = `run_install`, `generate_wallet`, `join_devnet`, `wait_for_block`, `explain_error`. System prompt focused on first-time-user guidance.
- **T1.11.8** — `MonitoringAgent` (basic — full Operator Agent ships in M2): tools = `read_metrics`, `read_logs`, `report_status`. Polls `/metrics` and `/health`, summarizes.
- **T1.11.9** — `CliDevAgent` (M1 Dev bundle component): wraps the CLI itself, takes natural-language requests, suggests commands.
- **T1.11.10** — `karoowa agent <name>` CLI subcommand entry point. In-process runtime mode only. Sidecar mode is Phase 2.7.

---

# M2 — v0.2

> **Goal of M2:** Production-grade transaction handling (mempool), real-time WebSocket subscriptions, BFT and PoS consensus engines, and the Operator-tier agent bundle.
>
> **Spec refs:** parent REQ-002, REQ-011 M2 Ops bundle, REQ-015 sidecar.
>
> **Pre-reqs:** All of M1 complete and tagged.

## Phase 2.0 — Mempool

> **Spec refs:** parent REQ-002 BDD scenario 2.
> **Estimated total:** 2–3 weeks.

**Tasks (refine when starting Phase 2.0):**
- **T2.0.1** — Mempool data structure (tx by hash, sorted by gas price, indexed by sender + nonce).
- **T2.0.2** — Eviction policy: max size, expiry, replace-by-fee.
- **T2.0.3** — Pre-validation: signature, balance, nonce, gas. Reject early.
- **T2.0.4** — Network integration: incoming tx from gossip → mempool. Outgoing tx from RPC → mempool + gossip.
- **T2.0.5** — Block proposer reads from mempool, removes included txs.
- **T2.0.6** — Tests: insertion, eviction, replace-by-fee, mempool consistency under churn.

## Phase 2.1 — WebSocket subscription manager

> **Spec refs:** parent REQ-002 BDD scenario 1.
> **Estimated total:** 1–2 weeks.

**Tasks:**
- **T2.1.1** — `SubscriptionManager` actor: track subscriptions by ID, fan out events.
- **T2.1.2** — Event sources: new block (from consensus), pending tx (from mempool), log (from receipt processing).
- **T2.1.3** — Backpressure handling: drop slow subscribers with a clear close code.
- **T2.1.4** — Reconnection / resubscription guidance documented.
- **T2.1.5** — Integration tests with a real WS client.

## Phase 2.2 — `kw_subscribe` methods

> **Spec refs:** parent REQ-002 BDD scenario 1.
> **Estimated total:** 1 week.

**Tasks:**
- **T2.2.1** — Wire `kw_subscribe(["newBlocks"])` → SubscriptionManager.
- **T2.2.2** — `kw_subscribe(["pendingTransactions"])`.
- **T2.2.3** — `kw_subscribe(["logs", { address, topics }])`.
- **T2.2.4** — `kw_unsubscribe`.

## Phase 2.3 — PoS consensus engine

> **Spec refs:** parent REQ-002, README §Consensus Engines.
> **Estimated total:** 3–4 weeks.

**Tasks:**
- **T2.3.1** — Validator set state: stake amounts, commission, jailing status.
- **T2.3.2** — Stake/unstake transactions (system tx type).
- **T2.3.3** — Weighted leader selection (proportional to stake).
- **T2.3.4** — Block reward distribution.
- **T2.3.5** — Slashing primitives (double-sign detection).
- **T2.3.6** — `PoSEngine` impl of `ConsensusEngine`.
- **T2.3.7** — Tests: stake → become validator → produce block → reward distributed.

## Phase 2.4 — BFT consensus engine

> **Spec refs:** parent REQ-002, OQ-004 (algorithm choice).
> **Estimated total:** 4–6 weeks. **Highest-risk phase in M2.**

**Tasks (after OQ-004 is resolved):**
- **T2.4.1** — Resolve OQ-004: pick algorithm (Tendermint vs HotStuff vs custom). Document the decision.
- **T2.4.2** — Round/step state machine.
- **T2.4.3** — Vote types (prevote, precommit), vote aggregation.
- **T2.4.4** — Quorum certificate / commit certificate types.
- **T2.4.5** — Network message types + gossip integration.
- **T2.4.6** — Liveness + safety properties; document them.
- **T2.4.7** — `BFTEngine` impl of `ConsensusEngine`.
- **T2.4.8** — Tests: 4-validator BFT devnet, recover from one-validator outage, never fork under 1 byzantine validator.

## Phase 2.5 — M2 Ops agent: CI/CD & Deployment

> **Spec refs:** parent REQ-011 M2 Ops bundle.
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T2.5.1** — Tools: `read_release_artifacts`, `deploy_to_target`, `rollback`, `verify_deployment`.
- **T2.5.2** — Integrates with GitHub Releases (T1.10.1) and `ssh`/`docker` deployment.
- **T2.5.3** — Approval workflow: agent proposes a deployment plan, human approves, agent executes.
- **T2.5.4** — Rollback playbook.
- **T2.5.5** — Tests against a staging devnet.
- **T2.5.6** — Documentation in `docs/agents/cicd-agent.md`.

## Phase 2.6 — M2 Ops agent: Observability

> **Spec refs:** parent REQ-011 M2 Ops bundle. Replaces the basic monitoring agent from Phase 1.11.
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T2.6.1** — Tools: `query_prometheus`, `read_logs`, `summarize_alerts`, `acknowledge_alert`.
- **T2.6.2** — Alert rule library: peer drop, block production stall, high RPC latency, disk pressure.
- **T2.6.3** — Remediation playbooks: restart peer connection, clear log directory, rotate RPC port, etc.
- **T2.6.4** — Audit log of every remediation action.
- **T2.6.5** — Escalation rules: when to page a human.
- **T2.6.6** — Tests with synthetic alerts.

## Phase 2.7 — Sidecar runtime mode

> **Spec refs:** parent REQ-015 (three-mode runtime — sidecar mode required ≥M3, available in M2).
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T2.7.1** — `karoowa-agent-sidecar` binary (separate process).
- **T2.7.2** — Loopback proxy: agent → proxy → node API. Auth via local-only token. Per-tool quota enforcement.
- **T2.7.3** — Agent process lifecycle: start, supervise, restart on crash.
- **T2.7.4** — Padded-room verification: agent process has no direct file/network access except via the proxy.
- **T2.7.5** — `karoowa agent <name> --mode sidecar` CLI flag.

---

# M3 — v0.3

> **Goal of M3:** Smart contracts. WASM execution environment, ABI tooling, contract SDK, plus the Security/Optimization agent bundle. Sidecar mode becomes mandatory.
>
> **Spec refs:** parent REQ-003, REQ-011 M3 Security/Optimization bundle, REQ-015 sidecar mandatory.
>
> **Pre-reqs:** All of M2 complete and tagged.

## Phase 3.0 — WASM runtime selection + integration

> **Spec refs:** parent REQ-003, OQ-005 (runtime choice).
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T3.0.1** — Resolve OQ-005: pick `wasmtime` vs `wasmer`. Decision criteria: license (Apache 2.0 fit), determinism guarantees, gas-metering support, embedded API ergonomics.
- **T3.0.2** — Add the chosen runtime as a workspace dependency. Spike: load and execute a hello-world WASM contract.
- **T3.0.3** — `karoowa-vm` crate (new). Wrapper exposing `WasmVm::new`, `WasmVm::execute(contract_bytes, input, gas_limit)`.
- **T3.0.4** — Determinism review: floats, NaN, instruction ordering.
- **T3.0.5** — Sandboxing review: memory limits, no host imports beyond what we allow.
- **T3.0.6** — Tests with fixture contracts.

## Phase 3.1 — Contract execution environment

> **Spec refs:** parent REQ-003.
> **Estimated total:** 3–4 weeks.

**Tasks:**
- **T3.1.1** — Gas metering: instruction-level via runtime hooks.
- **T3.1.2** — Host functions: `storage_read`, `storage_write`, `caller`, `value`, `block_height`, `emit_event`, `revert`.
- **T3.1.3** — Storage isolation per contract address.
- **T3.1.4** — Reentrancy protection or documented behavior.
- **T3.1.5** — Receipt enrichment: gas used, events emitted, return value.
- **T3.1.6** — Failure modes: out-of-gas, trap, revert with reason.
- **T3.1.7** — Tests: counter contract, ERC20-like token contract.

## Phase 3.2 — ABI encoder/decoder

> **Spec refs:** parent REQ-003.
> **Estimated total:** 1–2 weeks.

**Tasks:**
- **T3.2.1** — ABI schema format (likely JSON-described, similar to Ethereum's).
- **T3.2.2** — Encoder: typed values → bytes per the schema.
- **T3.2.3** — Decoder: bytes → typed values per the schema.
- **T3.2.4** — Function selector derivation.
- **T3.2.5** — Tests against fixture schemas.

## Phase 3.3 — Contract deployment + invocation

> **Spec refs:** parent REQ-003 BDD scenario.
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T3.3.1** — Deployment transaction type: `data` field contains WASM bytes + constructor args.
- **T3.3.2** — Deployment flow: deploy → contract address derived from sender + nonce → bytes stored under that address.
- **T3.3.3** — Invocation transaction: `to = contract address`, `data = ABI-encoded call`.
- **T3.3.4** — `kw_call` read-only call for view functions.
- **T3.3.5** — `kw_getCode` returns the deployed WASM bytes.
- **T3.3.6** — Tests: deploy → invoke → read state.

## Phase 3.4 — Contract SDK

> **Spec refs:** parent REQ-003.
> **Estimated total:** 1–2 weeks.

**Tasks:**
- **T3.4.1** — `karoowa-contract-sdk` crate: macros and helpers for writing Karoowa contracts in Rust → WASM.
- **T3.4.2** — Storage helpers: typed keys, derive macros.
- **T3.4.3** — Event emission helpers.
- **T3.4.4** — Example contract: a simple token.
- **T3.4.5** — `cargo karoowa contract build` toolchain wrapper.

## Phase 3.5 — M3 Security agent: Vulnerability Scanner

> **Spec refs:** parent REQ-011 M3 Security bundle.
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T3.5.1** — Tools: `scan_dependencies` (cargo-audit), `scan_contract_bytecode`, `check_known_patterns`, `report_findings`.
- **T3.5.2** — Continuous fuzzing setup against `karoowa-vm` and the consensus state machine.
- **T3.5.3** — Reporting: structured output, severity grading, issue creation.
- **T3.5.4** — Integration with CI: findings block PRs above a threshold.
- **T3.5.5** — Documentation in `docs/agents/vulnerability-scanner.md`.
- **T3.5.6** — Tests against deliberately-vulnerable fixture contracts.

## Phase 3.6 — M3 Optimization agent: Auto-Scaling/Gas Optimizer

> **Spec refs:** parent REQ-011 M3 Optimization bundle.
> **Estimated total:** 2–3 weeks.

**Tasks:**
- **T3.6.1** — Tools: `analyze_gas_usage`, `suggest_contract_optimization`, `recommend_node_resources`.
- **T3.6.2** — Gas profiler: per-function gas histograms over recent blocks.
- **T3.6.3** — Suggestion engine: pattern-match against known gas anti-patterns.
- **T3.6.4** — Resource recommendations: based on metrics from Phase 2.6 Observability Agent.
- **T3.6.5** — Tests with fixture contracts of known sub-optimal patterns.
- **T3.6.6** — Documentation.

## Phase 3.7 — Sidecar runtime mandatory enforcement

> **Spec refs:** parent REQ-015 (sidecar mandatory at M3).
> **Estimated total:** 1 week.

**Tasks:**
- **T3.7.1** — Add a hard check: if `--mode in-process` is passed and the binary is built from M3+ tag, refuse to start with a clear error.
- **T3.7.2** — Migration documentation: hobbyists upgrading from M1/M2 must adopt sidecar mode.
- **T3.7.3** — Update install scripts to provision the sidecar binary alongside the node binary.
- **T3.7.4** — Update CI to test sidecar mode end-to-end.

---

## After M3

M4 → M6 are described in the parent overarching PRD §3 (REQ-004, REQ-005, REQ-006). They should each get their own dev plan when M3 is complete.

---

## Decision log

This section records irreversible-ish decisions made during execution. Append, don't edit. Keep entries terse — link to the spec or PR for the long version.

| Date | Decision | Rationale | Spec ref |
|------|----------|-----------|----------|
| 2026-04-10 | Karoowa is greenfield, not inherited from a built ChainFlux v0.1 | The `files/` directory contained only design sketches | parent §3 M1 row |
| 2026-04-10 | Monorepo + `core/`/`enterprise/` + CI guardrails | Solo dev convenience, Strapi precedent | parent OQ-022/023/029 |
| 2026-04-10 | Hobbyist LLM default = hosted; no-key fallback = Gemma 4 E2B | 7B too heavy for 4 GB VPS; viability of E2B still subject to T1.11.1 spike | parent REQ-014, ASM-014a |
| 2026-04-10 | L3 agent memory = LanceDB | Apache 2.0, embedded, zero-ops | parent REQ-017, ASM-018 |
| 2026-04-10 | Three-mode agent runtime: in-process M1/M2, sidecar from M2 onward, mandatory at M3 | OQ-021/024 sidecar overhead on hobbyist hardware | parent REQ-015 |

---

## Open risks tracked from the parent PRD

These are spec-level open questions that affect this plan. Resolve before the relevant phase starts.

| Phase impacted | Parent OQ | What's open |
|---------------|-----------|-------------|
| Phase 2.4 | OQ-004 | BFT algorithm choice (Tendermint / HotStuff / custom) |
| Phase 3.0 | OQ-005 | WASM runtime choice (`wasmtime` / `wasmer`) |
| Phases 3.x | OQ-006 | EVM compatibility milestone slot — currently unassigned, leans M3 or M3.5 |
| Phase 1.11 | OQ-021/024 | Local-model viability on 4 GB VPS — resolved by T1.11.1 spike |
| Phase 1.10 | OQ-027 | Karoowa Infrastructure Lead (named person) |
| Phase 1.10 | OQ-028 | Devnet budget line item (final figure + source) |
| Phase 1.0 | OQ-A002 | CI guardrail script language (bash + ripgrep recommended) |
| Phase 1.0 | OQ-A007 | Repo URL (`karoowa/karoowa`?) |
