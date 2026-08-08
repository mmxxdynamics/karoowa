# Changelog

All notable changes to Karoowa are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) once
v1.0 ships. Pre-1.0 releases may make breaking changes between minor versions.

## [Unreleased]

### Added

- **`karoowa node --rpc-bind <ADDR>`** to choose the RPC listen address.
  Defaults to `0.0.0.0`, which is the existing behaviour, so nothing changes
  unless you pass it. The RPC is unauthenticated, so the node now logs a
  warning at startup whenever it is bound to a non-loopback address. Pass
  `--rpc-bind 127.0.0.1` on hosts that do not need remote RPC — but not inside
  a container, where a loopback bind is unreachable through a published port.
- **`aarch64-unknown-linux-gnu` release target**, built natively on
  `ubuntu-24.04-arm`. arm64 Linux operators were previously offered only the
  musl build, which never worked.
- arm64 Linux to the CI test matrix, so that target is exercised on every PR
  rather than for the first time at tag time.

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

- **Private key files are now written `0600`** by `karoowa wallet new`,
  `karoowa genesis generate`, the onboarding agent's `generate_wallet` tool,
  and the SoftHSM key store (`enterprise/karoowa-hsm`, which holds secret keys
  for every key in the store). The SOC 2 audit log
  (`enterprise/karoowa-audit-log`) is likewise created `0600`, since its records
  carry HSM key ids, backends and signing reasons. They previously used the process umask, commonly `0644` — any local
  user or any process sharing the container could read a validator key.

  **Migration.** The key must be readable by whoever runs the node:

  - **systemd** — if you generate as `root` but run as `User=karoowa`,
    `chown karoowa:karoowa` the key file. `scripts/server/harden.sh` now prints
    this step.
  - **Backups** — archive with mode-preserving flags (`tar -p`, `rsync -a`),
    and `chown` after restoring as root.
  - **Containers** — the image runs as `nonroot` (uid 65532); a bind-mounted
    key generated on the host is not readable by it. For the throwaway local
    keys, `chmod 0644 docker/genesis.validator*.key` (4-validator devnet, which
    `docker/test-devnet.sh` now does for you) or `chmod 0644
    docker/validator.key` (single-node `docker-compose.yml`).

  Existing files are also tightened when rewritten.

- **BREAKING (artifacts): the musl release targets are no longer published.**
  `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` are removed from
  the release matrix. Both had failed on every tag from v0.1.0 to v0.5.0, and
  because `release` depends on the build job they took the entire release with
  them — which is why no Karoowa release has ever been published. Linux
  operators should use the new `-gnu` tarballs or the container image. See #41
  for the plan to reintroduce a static build as a non-blocking artifact.
- **Linux tarballs now require glibc >= 2.39** (built on Ubuntu 24.04). They
  will not run on Debian 12, RHEL 9 or Ubuntu 22.04; use the container image or
  build from source on those.
- `scripts/install.sh` and the Homebrew formula now fetch `-gnu` artifacts, and
  both had been pointing at the wrong GitHub org (`karoowa/karoowa`).

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

### Fixed

- **The SBOM job could never have succeeded**, and `release` depends on it.
  `cargo cyclonedx --output-pattern bom -f -` used a flag that has never existed
  in cargo-cyclonedx, and `-f` is the short form of `--format`.
- **`scripts/install.sh` could never verify a checksum on any platform.** It
  saved the download under a name that does not appear in
  `checksums-sha256.txt`, so `sha256sum -c` always failed and the installer
  refused to install. It also assumed GNU `sha256sum` (absent on macOS) and
  chmod'd the wrong filename on Windows.
- **The container image could never have been built.** It compiled musl-static
  on Alpine, where `librocksdb-sys`'s bindgen step `dlopen`s libclang and a
  statically linked build script cannot `dlopen` at all. It also set
  `RUSTFLAGS="-C target-feature=+crt-static"` globally, which additionally
  broke proc-macro crates, and never installed `libclang-dev`. The build now
  runs on Debian 12 (`rust:<msrv>-bookworm`), matching the
  `distroless/cc-debian12` runtime's glibc, and is verified by an actual local
  build — the first Karoowa image that has ever built and run.
- **The published container image would have reported itself unhealthy
  forever.** Its `HEALTHCHECK` invoked a `karoowa health` subcommand that does
  not exist; clap exits 2 on an unknown subcommand. Removed — distroless has no
  shell to probe with; orchestrators should use `GET /health`.
- `docker/Dockerfile` pinned Rust 1.85 against a 1.94 workspace. `release.yml`
  now passes the MSRV read from `Cargo.toml` as a build-arg, and
  `rust-toolchain.toml` is kept out of the build context so the base image is
  the single toolchain pin.
- `workflow_dispatch` releases built the wrong tree (the dispatch branch rather
  than the requested tag) and pushed no version-tagged image.

### Security

- **MSRV bumped Rust 1.85 → 1.94** to enable `wasmtime 47.0.3`, which closes
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
