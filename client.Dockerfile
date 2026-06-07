# Stage 1: Build the Rust application
FROM rust:1.75 AS builder

WORKDIR /usr/src/app
COPY . .

# Build the client binary in release mode
RUN cargo build --release --bin client_cli

# Stage 2: Create a minimal runner image
FROM debian:bookworm-slim

# Install ca-certificates for secure HTTP requests
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/client_cli /app/client_cli

# Set the default entrypoint
ENTRYPOINT ["/app/client_cli"]
