# Karoowa container image.
#
# Stage 1 (planner)  — analyse Cargo workspace with `cargo-chef` for cache reuse.
# Stage 2 (builder)  — cook deps, then build `karoowa` against Debian glibc.
# Stage 3 (runtime)  — copy the binary into a minimal `distroless/cc` image.
#
# Build and runtime are both Debian 12, so the glibc the binary links against is
# by construction the glibc it runs on.
#
# This used to build musl-static on Alpine, which could not work: librocksdb-sys
# runs bindgen, bindgen dlopens libclang, and a statically linked build script
# cannot dlopen at all — "Dynamic loading not supported". Alpine also forced a
# musl-linked image, which is a poor fit for RocksDB (128 KB default thread
# stacks for its C++ compaction threads, and a markedly slower allocator under
# multi-threaded load). Neither was ever noticed because this image has never
# been built by CI. See #41.
#
# Build:
#   docker build -f docker/Dockerfile -t ghcr.io/mmxxdynamics/karoowa:dev .

# Must be >= the workspace `rust-version` in Cargo.toml, or cargo refuses to
# build. release.yml overrides this with the MSRV read straight from Cargo.toml
# so the two cannot drift; keep this default in step for local `docker build`.
ARG RUST_VERSION=1.94
ARG DISTROLESS_TAG=nonroot

# OCI labels for the published image.
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

# -- Stage 1: planner ---------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm AS planner

# libclang-dev is required, not optional: librocksdb-sys runs bindgen, which
# dlopens libclang.so at build time. g++/make come from the buildpack-deps base.
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -- Stage 2: builder ---------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm AS builder

# libclang-dev is required, not optional: librocksdb-sys runs bindgen, which
# dlopens libclang.so at build time. g++/make come from the buildpack-deps base.
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build

# No RUSTFLAGS="-C target-feature=+crt-static". It was set when this built on
# Alpine, where it was both redundant (musl targets already default to
# crt-static) and fatal — applied globally it also hits proc-macro crates, which
# must be dylibs, so the build died on `async-trait`. Not applicable to a glibc
# target either way; RocksDB is compiled by the base image's g++.

# Cook dependencies (cached layer — invalidated only when Cargo.lock changes).
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json --package karoowa

# Build the binary.
COPY . .
RUN cargo build --release --locked --package karoowa \
    && strip target/release/karoowa

# -- Stage 3: runtime ---------------------------------------------------------
# Distroless `cc` (Debian 12) supplies the glibc, libgcc and libstdc++ that the
# binary and RocksDB need. Matches the builder's Debian 12 exactly.
FROM gcr.io/distroless/cc-debian12:${DISTROLESS_TAG} AS runtime

# OCI image labels — picked up by GHCR.
ARG VCS_REF
ARG BUILD_DATE
LABEL org.opencontainers.image.title="karoowa" \
      org.opencontainers.image.description="Karoowa — agent-native, Rust-based blockchain framework" \
      org.opencontainers.image.url="https://karoowa.io" \
      org.opencontainers.image.source="https://github.com/mmxxdynamics/karoowa" \
      org.opencontainers.image.documentation="https://docs.karoowa.io" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.vendor="Karoowa" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.created="${BUILD_DATE}"

COPY --from=builder /build/target/release/karoowa /usr/local/bin/karoowa

USER nonroot:nonroot
WORKDIR /home/nonroot

EXPOSE 8545 30303 9090

# No HEALTHCHECK. There is no way to self-probe from inside this image: it is
# distroless (no shell, no curl) and the binary has no `health` subcommand —
# `karoowa` exposes exactly Node, Wallet, Genesis, Client, Devnet, Network and
# Agent. A HEALTHCHECK invoking `karoowa health` was present here and would have
# made every published container report unhealthy forever, since clap exits 2 on
# an unknown subcommand. It went unnoticed because no image has ever been pushed.
#
# Orchestrators should probe `GET /health` on the RPC port directly. Kubernetes
# ignores Docker HEALTHCHECK anyway and wants an httpGet probe; see
# docs/operator-guide.md §4.2 and §5.3.

ENTRYPOINT ["karoowa"]
CMD ["--help"]
