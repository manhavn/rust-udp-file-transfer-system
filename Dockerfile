# Stage 1: Build the Rust application
FROM rust:1.75 AS builder

WORKDIR /usr/src/app
COPY . .

# Build the server and client binaries in release mode
RUN cargo build --release --bin server --bin client_cli

# Stage 2: Create a minimal runner image
FROM debian:bookworm-slim

# Install ca-certificates for secure HTTP requests
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binaries from the builder stage
COPY --from=builder /usr/src/app/target/release/server /app/server
COPY --from=builder /usr/src/app/target/release/client_cli /app/client_cli

# Expose ports (UDP for transfer, TCP for Dashboard)
EXPOSE 5000/udp
EXPOSE 8080/tcp

# Default production configurations via environment variables
ENV UDP_PORT=5000
ENV HTTP_PORT=8080
ENV UPLOAD_DIR=/app/uploads
ENV DB_PATH=/app/db/data.sqlite
ENV DISABLE_REQUEST_LOG=true

# Create data directories
RUN mkdir -p /app/uploads /app/db

# Launch the server
CMD ["/app/server"]
