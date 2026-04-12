# Karoowa — Pre-Audit Code Hygiene Report

**Generated:** 2026-04-12
**Target:** v1.0.0-rc1 (pending tag)

Baseline measurements to hand to auditors so they know what was already
clean going in.

---

## Clippy

- **Command:** `cargo clippy --workspace --all-targets -- -D warnings`
- **Result:** ✅ Clean. Zero warnings across all 14 crates and all
  targets (lib, bin, tests, examples).

## Unsafe Blocks

- **Command:** `rg -n '\bunsafe\b' core/`
- **Result:** ✅ Zero matches. Karoowa contains **no `unsafe` code**
  across all 14 crates.
- **Note:** transitive dependencies (wasmtime, rocksdb, libp2p) contain
  `unsafe`, which is expected for FFI/systems code. Auditors should
  treat those as trusted-third-party per standard audit scoping.

## Build & Test

- **Command:** `cargo build --workspace && cargo test --workspace`
- **Baseline:** workspace builds green on Rust 1.89 stable.
- **Tests:** all crate unit + integration tests pass locally and in CI.

## Dependency Posture

- **Rust edition:** 2021 across all crates.
- **MSRV:** 1.89 (pinned in workspace `rust-version`).
- **Key versions:**
  - `libp2p = 0.56`
  - `wasmtime = 29` (not 43; 43 requires Rust 1.91+)
  - `axum = 0.8`
  - `rocksdb`, `ed25519-dalek 2`, `sha3`, `serde`, `bincode`, `zstd`
- **License:** workspace-wide license set in root `Cargo.toml`.

## Lint Policy Enforced

- `-D warnings` in CI.
- `clippy::pedantic` not enforced workspace-wide but individual crates
  have opted in where noisy lints were low-signal.
- `#[allow(clippy::large_enum_variant)]` used only where the variant
  represents protocol wire types that cannot be `Box`-wrapped without
  breaking serialization.

## What Is *Not* Yet Done (tracked for Phase 6.1)

- [ ] `cargo fuzz` targets for: SMT proof verification, tx decoding,
      WASM instantiation path, bridge packet parsing, governance
      proposal submission.
- [ ] Coverage report ≥ 80% on `karoowa-consensus`, `karoowa-vm`,
      `karoowa-bridge` (currently measured only informally).
- [ ] Miri run on pure-Rust crates that don't pull in FFI.
- [ ] Supply-chain scan via `cargo audit` + `cargo deny` in CI.

These are open tasks for Phase 6.1 and should be completed before the
`v1.0.0-rc1` tag is cut.
