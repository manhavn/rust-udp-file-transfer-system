# Stage 1: Build the Rust application
FROM rust:alpine AS builder

# Install build dependencies for C-based crates (e.g. SQLite bundled build if needed)
RUN apk add --no-cache musl-dev build-base

WORKDIR /usr/src/app
COPY . .

# Build the client binary in release mode
RUN cargo build --release --bin client_cli

# Stage 2: Create a minimal runner image
FROM alpine:latest

# Install ca-certificates for secure HTTP requests
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/client_cli /app/client_cli

# Set the default entrypoint
ENTRYPOINT ["/app/client_cli"]
