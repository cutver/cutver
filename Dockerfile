# syntax=docker/dockerfile:1

# ==============================================================================
# Stage 1: Build static binary with Rust musl
# ==============================================================================
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependency compilation layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Compile application source code
COPY src ./src
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
