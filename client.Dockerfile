# Stage 1: Build the Rust application
FROM rtk.app/dep-cache:latest AS builder

WORKDIR /usr/src/app

# Copy the actual source files
COPY common common
COPY client_lib client_lib
COPY client_cli client_cli

# Touch files to force rebuild of the application code
RUN touch common/src/lib.rs client_lib/src/lib.rs client_cli/src/main.rs

# Build the client binary in release mode
RUN cargo build --release --bin client_cli

# Stage 2: Create a minimal runner image
FROM rtk.runtime/base:latest

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/client_cli /app/client_cli

# Set the default entrypoint
ENTRYPOINT ["/app/client_cli"]
