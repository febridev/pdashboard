# Build stage
FROM rust:1.100-bookworm AS builder
RUN rustup target add wasm32-unknown-unknown
# dx must match the dioxus crate version in Cargo.lock (0.6.3).
RUN cargo install dioxus-cli --version 0.6.3 --locked
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY index.html Dioxus.toml ./
RUN dx build --release

# Runtime stage - serve static files with nginx and proxy API/auth to the backend.
FROM nginx:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
# dx writes the deployable site into release/web/public (contains index.html,
# wasm/ and assets/).
COPY --from=builder /app/target/dx/pdashboard-frontend/release/web/public /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
