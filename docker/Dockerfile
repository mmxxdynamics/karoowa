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

RUN apk add --no-cache musl-dev clang llvm lld pkgconfig
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -- Stage 2: builder ---------------------------------------------------------
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS builder

RUN apk add --no-cache musl-dev clang llvm lld pkgconfig
RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build

# No RUSTFLAGS="-C target-feature=+crt-static" here. It is redundant —
# `rustc --print cfg --target x86_64-unknown-linux-musl` already reports
# target_feature="crt-static" — and setting it globally also applies it to
# proc-macro crates, which must be built as dylibs:
#
#   error: cannot produce proc-macro for `async-trait` as the target
#          `x86_64-unknown-linux-musl` does not support these crate types
#
# That aborted the build at `cargo chef cook`. It was never noticed because the
# docker job has never run: it was added after the last tag, and before that the
# musl tarball legs failed first. Verified by building this image locally.
ENV CC=clang
ENV CXX=clang++

# Cook dependencies (cached layer — invalidated only when Cargo.lock changes).
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json --package karoowa

# Build the binary.
COPY . .
RUN cargo build --release --locked --package karoowa \
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
