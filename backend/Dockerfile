# Build stage
# rust:1.100-bookworm: Cargo.lock is lockfile v4 (needs Cargo >= 1.85; 1.75
# cannot parse it), and the bookworm base keeps glibc compatible with the
# debian:bookworm-slim runtime below.
FROM rust:1.100-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# ca-certificates: TLS trust store for Outline/Google API calls.
# libssl3: the binary dynamically links OpenSSL via reqwest/oauth2's native-tls.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/pdashboard-server /app/server

# Run as a dedicated unprivileged user.
RUN useradd --system --uid 1001 app
USER app

# Config reads PORT/HOST from the environment; HOST defaults to 0.0.0.0,
# which is required to be reachable from inside the container.
EXPOSE 3000
CMD ["/app/server"]
