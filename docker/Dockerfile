# Karoowa container image.
#
# Stage 1 (planner)  — analyse Cargo workspace with `cargo-chef` for cache reuse.
# Stage 2 (builder)  — cook deps, then build the static `karoowa` binary (musl).
# Stage 3 (runtime)  — copy the binary into a minimal `distroless/cc` image.
#
# Build:
#   docker build -f docker/Dockerfile -t ghcr.io/mmxxdynamics/karoowa:dev .

# Must be >= the workspace `rust-version` in Cargo.toml, or cargo refuses to
# build. release.yml overrides this with the MSRV read straight from Cargo.toml
# so the two cannot drift; keep this default in step for local `docker build`.
ARG RUST_VERSION=1.94
ARG ALPINE_VERSION=3.20
ARG DISTROLESS_TAG=nonroot

# OCI labels for the published image.
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

# -- Stage 1: planner ---------------------------------------------------------
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS planner

# A pre-FROM ARG reaches FROM lines but is NOT in scope inside a stage without
# re-declaring it. RUSTUP_TOOLCHAIN is what actually pins the compiler: the
# rust:*-alpine images install via rustup, and `COPY . .` brings
# rust-toolchain.toml (channel = "stable") into the build context, which would
# otherwise override the base image's toolchain. Same trap as
# .github/workflows/ci.yml's MSRV job.
ARG RUST_VERSION
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}

RUN apk add --no-cache musl-dev clang llvm lld pkgconfig
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -- Stage 2: builder ---------------------------------------------------------
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS builder

# See the planner stage. Without this, `cargo chef cook` (before COPY . .) runs
# under the base toolchain while `cargo build` (after) runs under stable —
# different rustc versions, different fingerprints, so the entire cooked
# dependency layer is discarded and RocksDB rebuilds from scratch.
ARG RUST_VERSION
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}

RUN apk add --no-cache musl-dev clang llvm lld pkgconfig
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build

ENV RUSTFLAGS="-C target-feature=+crt-static"
ENV CC=clang
ENV CXX=clang++

# Cook dependencies (cached layer — invalidated only when Cargo.lock changes).
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package karoowa

# Build the binary.
COPY . .
RUN cargo build --release --package karoowa \
    && strip target/release/karoowa

# -- Stage 3: runtime ---------------------------------------------------------
# Distroless `cc` because RocksDB still pulls in libstdc++ on certain configs.
# If you build a fully static binary, switch to `distroless/static`.
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

# Distroless has no shell, so HEALTHCHECK uses the binary itself. The CLI
# `health` subcommand is a thin wrapper over `GET /health` on the local RPC port.
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD ["karoowa", "health", "--rpc-port", "8545"]

ENTRYPOINT ["karoowa"]
CMD ["--help"]
