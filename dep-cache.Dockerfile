FROM rtk.builder/base:latest AS builder

WORKDIR /usr/src/app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Create dummy source files for all workspace members
RUN mkdir -p common/src server/src client_cli/src client_lib/src \
    && echo "pub fn dummy() {}" > common/src/lib.rs \
    && echo "pub fn dummy() {}" > client_lib/src/lib.rs \
    && echo "fn main() {}" > server/src/main.rs \
    && echo "fn main() {}" > client_cli/src/main.rs

# Copy individual Cargo.toml manifests
COPY common/Cargo.toml common/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY client_cli/Cargo.toml client_cli/Cargo.toml
COPY client_lib/Cargo.toml client_lib/Cargo.toml

# Pre-compile external dependencies only
RUN cargo build --release
