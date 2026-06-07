# Stage 1: Build the Rust application
FROM rtk.app/dep-cache:latest AS builder

WORKDIR /usr/src/app

# Copy the actual source files
COPY common common
COPY server server

# Touch files to force rebuild of the application code
RUN touch common/src/lib.rs server/src/main.rs

# Build the server binary in release mode
RUN cargo build --release --bin server

# Stage 2: Create a minimal runner image
FROM rtk.runtime/base:latest

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/server /app/server

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
