# UDP performance optimizations

This version keeps the existing UDP packet/ACK wire format, but changes the client sender from stop-and-wait to a windowed sender.

## Changes

- CLI: up to 32 UDP chunks are in flight by default (`--window-size`, `WINDOW_SIZE`).
- `client_lib`: uses a 32-chunk in-flight window without changing the C ABI.
- Only timed-out/unacknowledged chunks are retransmitted.
- Resume rewinds one full window before restarting, because the existing server `bytes_received` field is a high-water mark and packets can now arrive out of order.
- Server no longer calls `flush()` after every UDP chunk. Each successful `write_all()` is ACKed after data reaches the OS page cache.
- SQLite progress is checkpointed roughly once per MiB instead of once per UDP chunk; completion is persisted immediately.
- Release builds enable optimization, thin LTO, one codegen unit, and abort-on-panic.

## Suggested tuning

Start with the existing 16 KiB block size and `--window-size 32`. On a clean LAN, benchmark window sizes 16, 32, 64, and 128. If the path has packet fragmentation/loss, benchmark a smaller block size around 1200-1400 bytes together with a larger window.

## Build verification

Run on a machine with Rust installed:

    cargo fmt --all -- --check
    cargo check --workspace
    cargo test --workspace
    cargo build --release

The editing environment used to produce this archive did not contain `cargo`/`rustc`, so these commands must be run before deployment.
