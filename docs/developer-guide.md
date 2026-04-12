# Karoowa — Developer Guide

**Audience:** application developers building on Karoowa — contract authors, dApp developers, agent builders, SDK consumers.
**Target version:** v1.0.0-rc1.
**Last updated:** 2026-04-12.

---

## 1. What Karoowa Is

Karoowa is an agent-native, Rust-based Layer-1 blockchain framework with:

- **Pluggable consensus** — PoA, PoS, and Tendermint-style BFT ship in-tree; a `ConsensusEngine` trait lets downstream teams plug in custom engines without forking the core.
- **WASM contracts** — wasmtime-backed execution with fuel metering, memory limits, and a small host function surface.
- **EIP-1559 / EIP-2718 / EIP-2930 transaction envelopes** — legacy and typed transactions coexist in the same mempool.
- **Lock-and-mint bridge primitives** — a Karoowa-native cross-chain bridge with Merkle-proven packet commitments.
- **Two-chamber on-chain governance** — validator supermajority for chain-critical parameters, token-weighted voting for treasury and signaling.
- **Agent runtime** — first-class AI agents (governance, treasury, security, optimizer) that run alongside the node.

You get a chain you can spin up today (`karoowa devnet`) and tune for production later.

---

## 2. Quickstart (Devnet)

```bash
# Install
cargo install --git https://github.com/mmxxdynamics/karoowa karoowa

# Spin up a local single-node devnet
karoowa devnet start --home /tmp/karoowa-dev

# In another terminal: send a transaction
karoowa client send-tx \
  --rpc http://localhost:8545 \
  --to 0xdeadbeef... \
  --value 1000 \
  --key /tmp/karoowa-dev/keys/dev.json
```

The devnet runs with a single PoA validator, 1-second blocks, and chain id `karoowa-dev-1`. It's deterministic and resets on restart unless you pass `--persist`.

---

## 3. RPC / API Surface

Karoowa exposes four HTTP surfaces on the same port (`8545` by default):

| Endpoint | Protocol | Use |
|---|---|---|
| `POST /` | JSON-RPC 2.0 | Queries + tx submission |
| `GET /rest/v1/...` | REST | Block/tx/state fetch by hash or height |
| `GET /health`, `/ready` | Plain HTTP | Liveness / readiness probes |
| `GET /metrics` | Prometheus text | Ops scraping |

WebSocket subscriptions live on port `8546`.

### 3.1 Core JSON-RPC methods

| Method | Params | Returns |
|---|---|---|
| `karoowa_chainId` | – | `u64` |
| `karoowa_blockNumber` | – | `u64` |
| `karoowa_getBlockByHash` | `[hash]` | `Block` |
| `karoowa_getBlockByNumber` | `[u64]` | `Block` |
| `karoowa_getTransactionByHash` | `[hash]` | `Transaction` |
| `karoowa_getTransactionReceipt` | `[hash]` | `Receipt` |
| `karoowa_getBalance` | `[address, u64?]` | `u64` |
| `karoowa_getStorageAt` | `[address, slot, u64?]` | `Vec<u8>` |
| `karoowa_getTransactionCount` | `[address, u64?]` | `u64` (nonce) |
| `karoowa_estimateGas` | `[tx]` | `u64` |
| `karoowa_sendRawTransaction` | `[bytes]` | `hash` |
| `karoowa_call` | `[tx, u64?]` | `Vec<u8>` |
| `karoowa_getLogs` | `[filter]` | `Vec<Log>` |
| `karoowa_gasPrice` | – | `u64` |
| `karoowa_getBaseFee` | `[u64?]` | `u64` |

The `Block`, `Transaction`, and `Receipt` JSON shapes match the types in `core/karoowa-core/src/{block,transaction,receipt}.rs` — the RPC serializer is a thin wrapper over `serde_json`.

### 3.2 WebSocket subscriptions

```
wscat -c ws://localhost:8546
> {"jsonrpc":"2.0","id":1,"method":"karoowa_subscribe","params":["newHeads"]}
```

Topics:

- `newHeads` — block headers as they finalize
- `logs` (filter) — event logs matching a filter
- `newPendingTransactions` — tx hashes as they enter the mempool

### 3.3 REST shortcuts

```
GET /rest/v1/block/latest
GET /rest/v1/block/{height}
GET /rest/v1/block/hash/{hash}
GET /rest/v1/tx/{hash}
GET /rest/v1/account/{address}
```

REST is JSON with snake_case field names. It's a thin wrapper over the same handlers as JSON-RPC and lives in `core/karoowa-api/src/rest.rs`.

---

## 4. Contract Development

Karoowa runs arbitrary WASM. You can write contracts in any language that compiles to WASM; Rust is the best-supported today because the SDK lives in Rust.

### 4.1 Rust contract skeleton

```rust
// lib.rs — compile with `cargo build --target wasm32-unknown-unknown --release`

#[link(wasm_import_module = "env")]
extern "C" {
    fn storage_read(key_ptr: i32, key_len: i32, val_ptr: i32) -> i32;
    fn storage_write(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32);
    fn emit_event(topics_ptr: i32, topics_count: i32, data_ptr: i32, data_len: i32);
    fn set_output(ptr: i32, len: i32);
    fn revert(reason_ptr: i32, reason_len: i32);
}

#[no_mangle]
pub extern "C" fn call(input_ptr: i32, input_len: i32) -> i32 {
    // Read input, dispatch, return 0 on success
    0
}

#[no_mangle]
pub extern "C" fn deploy(_args_ptr: i32, _args_len: i32) -> i32 {
    // Constructor logic
    0
}
```

`call` is the runtime entry point for contract invocations. `deploy` is optional — if present, it runs once at deployment time.

### 4.2 Host function catalog

| Import | Signature | Purpose |
|---|---|---|
| `storage_read` | `(key_ptr, key_len, val_ptr) -> i32` | Read a storage slot, returns written length |
| `storage_write` | `(key_ptr, key_len, val_ptr, val_len)` | Write a storage slot |
| `get_caller` | `(buf_ptr)` | Write 20-byte caller address |
| `get_value` | `() -> i64` | Value (tokens) sent with the call |
| `emit_event` | `(topics_ptr, topics_count, data_ptr, data_len)` | Emit an event log |
| `set_output` | `(ptr, len)` | Set return payload |
| `revert` | `(reason_ptr, reason_len)` | Abort with reason string |

See `core/karoowa-vm/src/host.rs` for the authoritative reference.

### 4.3 Gas / fuel accounting

Every WASM instruction costs 1 fuel unit; every host call costs a fixed budget:

| Host call | Fuel cost |
|---|---|
| `storage_read` | 200 + len |
| `storage_write` | 500 + len |
| `emit_event` | 375 + 8 × topics + len |
| `set_output` | 100 + len |
| `revert` | 100 + len |
| `get_caller` / `get_value` | 10 |

Total budget = transaction `gas_limit`. If fuel runs out mid-call, the VM traps and the transaction reverts with `OutOfGas`.

### 4.4 Deploying

```bash
karoowa client deploy \
  --rpc http://localhost:8545 \
  --bytecode target/wasm32-unknown-unknown/release/my_contract.wasm \
  --key /tmp/karoowa-dev/keys/dev.json \
  --gas-limit 10000000
```

The command returns the deployed contract address and the receipt hash.

---

## 5. SDK (Rust)

`core/karoowa-sdk` is a client library for apps that need to build, sign, and submit transactions. The common pattern:

```rust
use karoowa_sdk::{Client, Wallet, TxBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wallet = Wallet::from_file("dev.json")?;
    let client = Client::new("http://localhost:8545")?;

    let tx = TxBuilder::new()
        .from(wallet.address())
        .to("0x...".parse()?)
        .value(1000)
        .gas_limit(21_000)
        .gas_price(client.gas_price().await?)
        .nonce(client.nonce_of(&wallet.address()).await?)
        .chain_id(client.chain_id().await?)
        .build_and_sign(&wallet)?;

    let hash = client.send_raw_tx(&tx).await?;
    println!("submitted: {hash}");
    Ok(())
}
```

`TxBuilder` picks the envelope type automatically:

- Plain builders → `TransactionEnvelope::Legacy`
- `.max_fee_per_gas(...)` + `.max_priority_fee_per_gas(...)` → `Eip1559`
- `.access_list(...)` → `Eip1559` with access list

See `core/karoowa-sdk/examples/` for more.

---

## 6. Agent Integration

Karoowa's agent runtime (`core/karoowa-agents`) runs long-lived AI agents that can observe chain state and submit transactions. Built-in agents:

- **Governance** — watches proposals, summarizes voting, alerts on quorum risk
- **Treasury** — tracks treasury outflows, flags anomalies
- **Security** — monitors for suspicious contract calls, rate-limits RPC
- **Optimizer** — suggests gas-limit and fee-target adjustments based on recent blocks

### 6.1 Running an agent

```bash
karoowa agent run \
  --agent governance \
  --rpc http://localhost:8545 \
  --provider anthropic \
  --api-key $ANTHROPIC_API_KEY
```

### 6.2 Writing a new agent

Implement the `Agent` trait in `core/karoowa-agents/src/agent.rs`:

```rust
#[async_trait]
impl Agent for MyAgent {
    fn name(&self) -> &'static str { "my-agent" }

    async fn tick(&mut self, ctx: &AgentContext) -> Result<(), AgentError> {
        let head = ctx.client.block_number().await?;
        // ... observe, reason, act
        Ok(())
    }
}
```

Agents run inside the node process by default. They share the HTTP client with the node but are isolated from consensus — an agent crash cannot halt the chain.

### 6.3 Certified agents (Enterprise)

For production deployments, third-party agents must be certified via `enterprise/karoowa-marketplace`. A certified agent ships as a `CertifiedAgent` JSON attestation + bytecode pair; the enterprise loader refuses to run anything without a valid Karoowa vendor signature. See `enterprise/karoowa-marketplace/src/lib.rs`.

---

## 7. Architecture Reference

### 7.1 Workspace layout

```
core/
  karoowa-crypto    — ed25519, sha3, address derivation
  karoowa-core      — block, tx, state, receipt, config, license
  karoowa-trie      — sparse Merkle trie (state commitment)
  karoowa-storage   — RocksDB persistence
  karoowa-consensus — PoA, PoS, BFT, producer, mempool
  karoowa-vm        — wasmtime contract executor + host functions
  karoowa-light     — light client
  karoowa-bridge    — cross-chain primitives
  karoowa-governance — on-chain governance state machine
  karoowa-network   — libp2p (gossipsub, kad, state-sync, light, bridge)
  karoowa-api       — Axum RPC/REST/WS gateway
  karoowa-sdk       — client library
  karoowa-agents    — agent runtime
  karoowa           — node binary
enterprise/
  karoowa-license karoowa-audit-log karoowa-rbac
  karoowa-hsm karoowa-ha karoowa-marketplace
```

### 7.2 Block production happy path

```
Mempool → BlockProducer → ConsensusEngine.propose_block()
       → ConsensusEngine.validate_block()
       → Storage.put_block()
       → Gossipsub broadcast
       → Peers validate & import
```

The producer runs as a single tokio task. Its cadence is `block_time_ms` from the governable parameter registry.

### 7.3 Contract call path

```
RPC → Mempool.accept_tx()
   → BlockProducer includes tx
   → WasmVm.execute(bytecode, "call", input, gas_limit, …)
   → HostState records storage writes + events
   → Receipt built from ExecutionResult
   → State trie updated
   → BlockHeader.state_root commits
```

### 7.4 Governance flow

```
Submit → Deposit → Voting → Timelock → Executed
                          ↘ Rejected ↙
                          ↘ Vetoed ↙
```

`GovernanceModule::tick(height)` runs every block and auto-advances the state machine. See `core/karoowa-governance/src/module.rs`.

### 7.5 Full architecture diagrams

`specs/audit/architecture.md` has sequence diagrams for block production, BFT finality, contract execution, bridge lock-and-mint, governance lifecycle, and light-client verification.

---

## 8. Testing Locally

### 8.1 Single-node devnet

```bash
karoowa devnet start --home /tmp/kar --persist
```

### 8.2 Multi-node devnet

```bash
karoowa devnet start --home /tmp/kar --nodes 4 --bft
```

Spins up four nodes on localhost with BFT consensus. Useful for testing governance voting and validator rotation flows.

### 8.3 Running the test suite

```bash
# Unit + integration tests across the workspace
cargo test --workspace

# Only the fuzz harnesses (proptest)
cargo test --workspace --test 'proptest_*'

# Coverage report (needs cargo-llvm-cov)
cargo llvm-cov --workspace --summary-only
```

CI also enforces `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny`, `cargo audit`, and the coverage gate (≥80% on `karoowa-consensus`, `karoowa-bridge`, `karoowa-vm`).

---

## 9. Where to Read Next

- `docs/operator-guide.md` — running a production node
- `docs/tokenomics.md` — supply schedule, fees, staking, treasury
- `specs/audit/architecture.md` — full sequence diagrams for auditors
- `specs/audit/threat-model.md` — trust boundaries and invariants
- `specs/development/dev_plan_m4_m6.md` — the M4–M6 build plan
- `CONTRIBUTING.md` — how to send a PR

## 10. Community

- **Discord:** <https://discord.gg/karoowa>
- **GitHub:** <https://github.com/mmxxdynamics/karoowa>
- **Grants / partnerships:** `partnerships@karoowa.io`
