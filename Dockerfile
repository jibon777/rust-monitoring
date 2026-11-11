# ==============================================================================
# Multi-stage Dockerfile for Dioxus Fullstack System Monitor
# ==============================================================================

# Stage 1: Build the client WASM and server executable
FROM rust:bookworm AS builder

# Install WebAssembly target
RUN rustup target add wasm32-unknown-unknown

# Install Dioxus CLI pre-built binary (v0.6.3)
RUN curl -L -o /tmp/dx.tar.gz https://github.com/DioxusLabs/dioxus/releases/download/v0.6.3/dx-x86_64-unknown-linux-gnu-v0.6.3.tar.gz \
    && tar -xzf /tmp/dx.tar.gz -C /usr/local/bin/ \
    && chmod +x /usr/local/bin/dx \
    && rm /tmp/dx.tar.gz

WORKDIR /app

# Copy project manifests and source files
COPY Cargo.toml Cargo.lock* Dioxus.toml ./
COPY assets ./assets
COPY public ./public
COPY src ./src

# Build fullstack bundle (both server binary and client WASM in /target/dx/.../release/web)
RUN dx build --platform web --fullstack --release

# ==============================================================================
# Stage 2: Minimal runtime image
# ==============================================================================
FROM debian:bookworm-slim AS runner

# Install essential runtime utilities and SSL certs
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Server binding configuration
ENV PORT=8080
ENV IP=0.0.0.0

# Copy server executable and public assets folder explicitly
COPY --from=builder /app/target/dx/system-monitor/release/web/server /app/server
COPY --from=builder /app/target/dx/system-monitor/release/web/public /app/public

# Expose Web Dashboard Port
EXPOSE 8080

# Run the Dioxus fullstack server
CMD ["/app/server"]

