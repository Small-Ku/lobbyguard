# LobbyGuard CLI

Command-line interface for the LobbyGuard packet capture system.

## Building

```bash
cargo build --release -p lobbyguard-cli
```

## Running

Must be run with administrator privileges:

```bash
cargo run --release -p lobbyguard-cli
```

Or directly:

```bash
./target/release/lobbyguard-cli.exe
```

## Features

- ✅ Async packet capture using tokio
- ✅ Automatic heartbeat packet detection
- ✅ Graceful shutdown (Ctrl-C handling)
- ✅ Detailed error reporting
- ✅ Admin rights detection

## Usage

1. Run with administrator privileges
2. The application will start capturing UDP heartbeat packets
3. Press Ctrl-C to exit gracefully

Output example:
```
LobbyGuard CLI - Packet Capture System
Make sure this is run with administrator privileges!
Press Ctrl-C to exit.

Starting packet capture...
HEARTBEAT PACKET PASSED [12]
HEARTBEAT PACKET PASSED [18]
^C
Ctrl-C received! Shutting down gracefully...
LobbyGuard CLI exited successfully.
```

## Troubleshooting

### "Failed to create WinDivert"
- Ensure running as administrator
- Check WinDivert driver installation
- Try running from Administrator command prompt

### No packets captured
- Verify network traffic is actually occurring
- Check filter matches your network configuration
- Confirm target application is running

## Architecture

The CLI uses:
- **compio**: Async runtime for handling Ctrl-C and packet processing
- **lobbyguard-core**: Shared packet capture logic
- **snafu**: Error handling with context

## Code Organization

```
src/
├── main.rs    # Entry point and main logic
```

The CLI is intentionally simple - all complex logic is in `lobbyguard-core`.

## Extending the CLI

To add new features:

1. Add error types to error handling section if needed
2. Implement feature using Result<T> pattern
3. Never use unwrap/panic
4. Document new features

Example:

```rust
async fn new_feature() -> Result<()> {
    // Your async code here
    Ok(())
}
```

## Performance

- Packet capture runs at kernel level via WinDivert
- Async processing prevents blocking
- Minimal memory footprint

## Debugging

Enable rust backtrace for detailed errors:

```bash
RUST_BACKTRACE=1 cargo run --release -p lobbyguard-cli
```

---

See `../README.md` for project-wide documentation.
