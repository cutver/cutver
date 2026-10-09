# syntax=docker/dockerfile:1

# ==============================================================================
# Stage 1: Build static binary with Rust musl
# ==============================================================================
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependency compilation layer.
#
# Every workspace member's manifest has to be copied here, with a placeholder source,
# or Cargo cannot resolve the workspace at all. Adding a member means adding its
# manifest to this COPY and its source directory to the one below; when that was
# missed for `crates/cutver-pdk` the image build failed with a manifest-not-found
# error that only a release event would have surfaced.
COPY Cargo.toml Cargo.lock ./
COPY crates/cutver-pdk/Cargo.toml ./crates/cutver-pdk/Cargo.toml
RUN mkdir -p src crates/cutver-pdk/src && \
    echo "fn main() {}" > src/main.rs && \
    touch crates/cutver-pdk/src/lib.rs && \
    cargo build --release && \
    rm -rf src crates/cutver-pdk/src

# Compile application source code
COPY src ./src
COPY crates/cutver-pdk/src ./crates/cutver-pdk/src
RUN touch src/main.rs && \
    cargo build --release --locked && \
    strip target/release/cutver

# ==============================================================================
# Stage 2: Minimal runtime image with Git
# ==============================================================================
FROM alpine:3.24 AS runtime

# Install Git, CA certificates, and SSH client for remote forge operations
RUN apk add --no-cache \
    git \
    ca-certificates \
    openssh-client && \
    git config --system --add safe.directory '*'

# Copy static cutver binary
COPY --from=builder /app/target/release/cutver /usr/local/bin/cutver

# Set default workspace directory
WORKDIR /workspace

ENTRYPOINT ["/usr/local/bin/cutver"]
CMD ["--help"]
