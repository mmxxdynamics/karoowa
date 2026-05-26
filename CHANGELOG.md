# Changelog

All notable changes to Karoowa are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) once
v1.0 ships. Pre-1.0 releases may make breaking changes between minor versions.

## [Unreleased]

### Security

- **MSRV bumped Rust 1.85 → 1.92** to enable `wasmtime 44`, which closes
  15 RUSTSEC advisories pulled in transitively via `karoowa-vm`:
  - 14 wasmtime vulnerabilities including two sandbox escapes
    (RUSTSEC-2026-0095 Winch, RUSTSEC-2026-0096 aarch64 Cranelift),
    pooling-allocator data leakage (RUSTSEC-2026-0088), component-model
    transcoding OOB read/write (RUSTSEC-2026-0091/-0092/-0093),
    f64 segfaults (RUSTSEC-2026-0006/-0087), and others
    (RUSTSEC-2025-0046, -0118, RUSTSEC-2026-0020/-0021/-0085/-0086/
    -0089/-0094).
  - 1 unmaintained transitive (RUSTSEC-2025-0057 fxhash). The
    RUSTSEC-2024-0436 paste advisory remains tracked because `libp2p-tcp`
    still pulls paste in on Linux via `if-watch → netlink-packet-core`.

### Added

- `[workspace.lints]` block in the root `Cargo.toml` propagates a single set
  of rust/clippy/rustdoc lints to every member crate.
- `SECURITY.md` with a private vulnerability disclosure process and severity
  schedule.
- `RELEASE.md` describing the release cadence, signing scheme, and SBOM
  policy.
- `.github/dependabot.yml` weekly updates for the `cargo` and
  `github-actions` ecosystems.
- `.github/ISSUE_TEMPLATE/` forms for bug reports, feature requests, and
  security questions, plus a `config.yml` redirecting questions away from
  Issues.
- `.github/CODEOWNERS` for review routing.
- `.editorconfig`, `clippy.toml`, and `rustfmt.toml` to lock in editor +
  toolchain defaults.
- Sigstore keyless signing, SLSA build-provenance attestation, and CycloneDX
  SBOM generation on every tagged release.
- `KAROOWA_BOOTNODE` environment override for `scripts/join-devnet.sh`.

### Changed

- All `enterprise/*` crates marked `publish = false` and switched from
  `license-file` to a `LicenseRef-Karoowa-Enterprise` SPDX expression.
- `LICENSE-ENTERPRISE.md` rewritten: explicit "review-only until BSL 1.1
  text is published" notice replaces the placeholder.
- `deny.toml` tightened: stricter advisory policy, OpenSSL/native-tls banned
  in favour of rustls, unknown git registries denied.
- README MSRV row corrected to **Rust 1.85+** (was 1.78), status line updated
  to reflect M1-M6 progress.
- `scripts/install.sh` now aborts on a checksum mismatch (previously it
  swallowed the failure with `|| true`).

### Removed

- AI-assistant attribution lines from spec / dev-plan files. Anthropic
  references that document the *Anthropic LLM provider* feature in
  `karoowa-agents` are kept: they describe a real product integration.

## [0.5.0]: 2026-04-12

- M5: Karoowa bridge primitives (MVP) and libp2p bridge request-response
  protocol.

## [0.4.0]: 2026-04-12

- M4: EIP-1559 / EIP-2718 / EIP-2930 transaction types, libp2p light-client
  request-response, M4 Enterprise agent bundle (Governance + Treasury).

## [0.3.0]

- M3: WASM smart-contract VM, ABI encoder/decoder, contract SDK,
  M3 Security/Optimization agent bundle.

## [0.2.0]

- M2: BFT consensus, PoS engine, mempool, WebSocket subscriptions,
  M2 Ops agent bundle, sidecar runtime.

## [0.1.0]

- M1: core primitives, PoA consensus, RocksDB storage, API gateway,
  Docker devnet, CLI, hobbyist install path, M1 Dev agent bundle.

[Unreleased]: https://github.com/mmxxdynamics/karoowa/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/mmxxdynamics/karoowa/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/mmxxdynamics/karoowa/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/mmxxdynamics/karoowa/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/mmxxdynamics/karoowa/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mmxxdynamics/karoowa/releases/tag/v0.1.0
