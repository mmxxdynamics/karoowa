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

## Fuzz / Property Coverage (Phase 6.1 / 6.1.b)

Karoowa uses `proptest` rather than `cargo fuzz` for the core fuzz
harnesses — stays on stable Rust, runs in CI, catches the same class of
crash/panic bugs for our adversarial input surfaces. Current coverage:

| Target | Crate | Cases/prop | Properties |
|---|---|---|---|
| SMT proof system | `karoowa-trie` | 32 | 5 |
| Bridge packet codec | `karoowa-bridge` | 64 | 4 |
| Transaction envelope codec | `karoowa-core` | 64 | 6 |
| WASM deploy path | `karoowa-vm` | 32 | 3 |

Common invariants covered: bincode round-trip, hash determinism, junk
bytes never panic, tamper/forgery detection. See the individual
`tests/proptest_*.rs` files for the full list.

## Supply Chain

- ✅ `cargo deny` in CI — license policy + banned crates
- ✅ `cargo audit` in CI (rustsec/audit-check) — vulnerability DB scan

## Coverage

- ✅ `cargo llvm-cov` job in CI generates `lcov.info` as a workflow
  artifact and prints a summary table to the job log.
- ⏳ ≥80% threshold enforcement on `karoowa-consensus`, `karoowa-vm`,
  `karoowa-bridge` — gate flipped once baselines stabilize.

## Still Open for rc1

- [ ] Nightly `cargo fuzz` targets for VM module validation with
      structured WASM generation (the proptest harness covers random
      bytes + WASM-magic-prefix junk, but not semantically-valid-yet-
      adversarial modules). Deferred to post-rc1 only if the wasmtime
      upstream fuzz suite is deemed sufficient coverage.
- [ ] Coverage ≥80% enforcement flag flipped once the baseline run
      completes and the team signs off on per-crate targets.
- [ ] Miri run on pure-Rust crates (trie, crypto, governance, core).
