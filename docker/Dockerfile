# Karoowa multi-stage build.
#
# Stage 1: Build a statically linked release binary (musl).
# Stage 2: Copy into a minimal distroless image.
#
# Usage:
#   docker build -f docker/Dockerfile -t karoowa/karoowa:dev .

# -- Build stage --------------------------------------------------------------
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev clang llvm lld pkgconfig

WORKDIR /build
COPY . .

# Build with musl for static linking. RocksDB needs clang/lld.
ENV RUSTFLAGS="-C target-feature=+crt-static"
ENV CC=clang
ENV CXX=clang++

RUN cargo build --release --package karoowa \
    && strip target/release/karoowa

# -- Runtime stage -------------------------------------------------------------
FROM alpine:3.20 AS runtime

RUN adduser -D -u 1000 karoowa

COPY --from=builder /build/target/release/karoowa /usr/local/bin/karoowa

USER karoowa
WORKDIR /home/karoowa

EXPOSE 8545 30303 9090

ENTRYPOINT ["karoowa"]
CMD ["--help"]
