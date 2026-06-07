# Stage 1: Build the Rust application
FROM rtk.builder/base:latest AS builder

WORKDIR /usr/src/app
COPY . .

# Build the client binary in release mode
RUN cargo build --release --bin client_cli

# Stage 2: Create a minimal runner image
FROM rtk.runtime/base:latest

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/client_cli /app/client_cli

# Set the default entrypoint
ENTRYPOINT ["/app/client_cli"]
