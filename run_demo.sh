#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Cleanup background server process on exit
trap "echo -e '\n==> Stopping Server...'; kill \$SERVER_PID 2>/dev/null || true; rm -f demo_data.bin" EXIT

echo "==> 1. Building UDP Server and Client in release mode..."
cargo build --release

echo "==> 2. Creating a 1MB demo file (demo_data.bin)..."
dd if=/dev/urandom of=demo_data.bin bs=1024 count=1000 2>/dev/null

echo "==> 3. Starting UDP and HTTP Server in the background..."
./target/release/server &
SERVER_PID=$!

# Give the server a moment to bind to the ports
sleep 1.5

echo "==> 4. Launching Client CLI to upload demo_data.bin..."
# Run the client CLI. We specify a block size of 16384 (16KB)
./target/release/client_cli demo_data.bin --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384

echo "==> 5. Upload finished successfully!"
echo "    -> Dashboard URL: http://localhost:8080"
echo "    -> Completed file: ./uploads/demo_data.bin"
echo "    -> Press Ctrl+C to stop the server and cleanup."

# Keep the script running so the user can interact with the server
wait $SERVER_PID
