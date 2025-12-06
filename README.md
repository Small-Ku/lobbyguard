# LobbyGuard - Packet Capture & Filtering System

A high-performance Windows packet capture and filtering system written in Rust. LobbyGuard captures UDP packets on port 6672, identifies heartbeat packets by size, and forwards them through WinDivert.

## Quick Start

### Prerequisites
- Windows 10+ with administrator privileges
- Rust 1.70+ (2024 edition)
- WinDivert installed (automatically handled by the build)

### Building

```bash
# Build all crates
cargo build --release

# Build individual crates
cargo build --release -p lobbyguard-core
cargo build --release -p lobbyguard-cli
cargo build --release -p lobbyguard-gui
```

### Running

**Important**: Both applications require administrator privileges to capture packets. Run from an admin terminal or use `runas`.

#### LobbyGuard CLI

The CLI is a minimal command-line interface for packet capture.

**Usage**

1. Run with administrator privileges: `cargo run --release -p lobbyguard-cli`
2. The application will start capturing UDP heartbeat packets.
3. Press Ctrl-C to exit gracefully.

**Output example:**
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

#### LobbyGuard GUI

A graphical user interface for the LobbyGuard packet capture system.

**Features**

- Toggle packet capture on/off
- Visual feedback through button state
- Graceful shutdown on window close
- Administrator rights detection

**Running**

Run with administrator privileges: `cargo run --release -p lobbyguard-gui`

## Architecture

LobbyGuard uses a modular three-crate architecture:

### `lobbyguard-core` - Shared Library
Contains all core packet capture logic.

### `lobbyguard-cli` - Command Line Interface
Minimal CLI entry point using core library.

### `lobbyguard-gui` - Graphical Interface
Windows UI using winio framework.

For a detailed architecture overview, see `AGENTS.md`.

## Development

For a detailed development guide, see `AGENTS.md`.

### Project Structure

```
lobbyguard/
├── Cargo.toml              # Workspace config
├── README.md               # This file
├── AGENTS.md               # Development and Architecture details
│
├── lobbyguard-core/        # Shared library
│   ├── ...
│
├── lobbyguard-cli/         # CLI application
│   ├── ...
│
└── lobbyguard-gui/         # GUI application
    ├── ...
```

### Testing

```bash
# Run all tests
cargo test --all

# Run specific test
cargo test -p lobbyguard-core packet_filter

# Run with output
cargo test -- --nocapture

# Run doc tests
cargo test --doc
```

### Building for Release

```bash
# Build optimized binaries
cargo build --release

# Output location
# ./target/release/lobbyguard-cli.exe
# ./target/release/lobbyguard-gui.exe

# Strip debug symbols (optional)
strip target/release/lobbyguard-cli.exe
```

## Dependencies

Core dependencies:
- **etherparse** - Packet header parsing
- **windivert** - Packet capture/filtering (Windows-only)
- **snafu** - Error handling
- **compio** - Async runtime
- **winio** - GUI framework

## Troubleshooting

### "Permission Denied" on Startup
→ Run with administrator privileges. Open command prompt as admin or use `runas`.

### WinDivert Installation Issues
→ WinDivert requires system drivers. Rebuild the project or reinstall WinDivert manually.

### Compiler Errors
→ Ensure you're using Rust 1.70+: `rustc --version`

### No packets captured
- Check Windows Firewall isn't blocking port 6672
- Verify another application isn't already capturing packets
- Try running `ipconfig /all` to verify network adapters
- Verify network traffic is actually occurring
- Check filter matches your network configuration
- Confirm target application is running

## Contributing

Before submitting code:

1. **Error handling**: Use snafu with context() and ensure!(), never unwrap/panic
2. **Documentation**: Add doc comments to public APIs
3. **Tests**: Add tests for new functionality
4. **Code style**: Run `cargo fmt` and `cargo clippy`

```bash
# Code quality checks
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all
```