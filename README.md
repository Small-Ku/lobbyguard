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

```bash
# Run CLI (requires admin)
cargo run --release -p lobbyguard-cli

# Run GUI (requires admin)
cargo run --release -p lobbyguard-gui
```

**Important**: Both applications require administrator privileges to capture packets. Run from an admin terminal or use `runas`.

## Architecture

LobbyGuard uses a modular three-crate architecture:

### `lobbyguard-core` - Shared Library
Contains all core packet capture logic:
- **PacketCapture** struct - Manages WinDivert operations
- **Error types** - Snafu-based error handling
- **Constants** - Filter strings, heartbeat sizes, timeouts

```
use lobbyguard_core::capture::PacketCapture;

let mut capture = PacketCapture::new()?;
capture.run().await?;
```

### `lobbyguard-cli` - Command Line Interface
Minimal CLI entry point using core library:
- Graceful Ctrl-C shutdown
- Async signal handling with compio
- Spawns packet capture task

### `lobbyguard-gui` - Graphical Interface  
Windows UI using winio framework:
- ELM-style component architecture
- Async packet capture integration
- Status display and controls

## Key Features

### Error Handling
All errors use **snafu** for structured error handling - no unwrap/panic:

```rust
// Good: Wrapped errors with context
let config = fs::read_to_string(&path)
    .context(ReadConfigurationSnafu { path: &config_path })?;

// Bad: Never do this
let config = fs::read_to_string(&path).unwrap();
```

### Async Runtime
Migrated from tokio to **compio** - thread-per-core architecture:

| Feature | Compio | Tokio |
|---------|--------|-------|
| Main macro | `#[compio::main]` | `#[tokio::main]` |
| Spawn task | `compio_runtime::spawn` | `tokio::spawn` |
| Blocking I/O | `compio_runtime::spawn_blocking` | `tokio::task::spawn_blocking` |
| Signal handling | `compio::signal::ctrl_c` | `tokio::signal::ctrl_c` |

### Packet Filtering
Configurable WinDivert filter for UDP port 6672 heartbeats:

```rust
// From constants.rs
const HEARTBEAT_SIZES: &[usize] = &[12, 18, 63];
const FILTER: &str = "udp.DstPort == 6672";
```

## Development

### Project Structure

```
lobbyguard/
├── Cargo.toml              # Workspace config
├── README.md               # This file
├── DEVELOPMENT.md          # Development guide
├── ARCHITECTURE.md         # Detailed architecture
│
├── lobbyguard-core/        # Shared library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module exports
│       ├── capture.rs      # PacketCapture struct
│       ├── constants.rs    # WinDivert filter, heartbeat sizes
│       └── error.rs        # Error types
│
├── lobbyguard-cli/         # CLI application
│   ├── Cargo.toml
│   ├── build.rs            # Windows resource compilation
│   └── src/main.rs         # Entry point
│
└── lobbyguard-gui/         # GUI application
    ├── Cargo.toml
    └── src/
        ├── main.rs         # App initialization
        ├── component.rs    # Main winio component
        ├── error.rs        # GUI error types
        ├── capture/        # Capture integration
        ├── process/        # Packet processing
        ├── storage/        # Data storage
        └── ui/             # UI components
```

### Common Tasks

#### Adding a New Error Type

```rust
// In lobbyguard-core/src/error.rs
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid configuration: {message}"))]
    InvalidConfig { message: String },
}
```

#### Adding Packet Filtering Logic

```rust
// In lobbyguard-core/src/capture.rs
fn process_packet(&self, data: &[u8]) -> crate::Result<bool> {
    let ip = Ipv4Slice::from_slice(data)
        .ok()
        .context(IpParseFailedSnafu)?;
    
    ensure!(!data.is_empty(), IpParseFailedSnafu);
    
    Ok(is_heartbeat(ip.payload()))
}
```

#### Creating a GUI Component

```rust
// In lobbyguard-gui/src/ui/
#[derive(Default, Clone)]
struct MyComponent {
    state: MyState,
}

impl Component for MyComponent {
    fn event(&mut self, evt: Event) -> Command {
        // Handle events
        Command::None
    }

    fn view(&self) -> Element {
        // Render UI
        text("Hello").into()
    }
}
```

### Error Handling Pattern

Always use snafu's `context()` and `ensure!()`:

```rust
use snafu::prelude::*;
use std::{fs, path::PathBuf};

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Unable to read from {}", path.display()))]
    ReadFile { source: std::io::Error, path: PathBuf },
    
    #[snafu(display("Invalid size: {size}"))]
    InvalidSize { size: usize },
}

type Result<T, E = Error> = std::result::Result<T, E>;

fn read_config(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path)
        .context(ReadFileSnafu { path: path.clone() })?
        .ok()
}

fn validate_size(size: usize) -> Result<()> {
    ensure!(size > 0, InvalidSizeSnafu { size });
    Ok(())
}
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

Core dependencies (unchanged from original):
- **etherparse** - Packet header parsing
- **windivert** - Packet capture/filtering (Windows-only)
- **snafu** - Error handling
- **compio** - Async runtime (replaced tokio)
- **winio** - GUI framework

All dependencies are locked in `Cargo.lock` and vendored versions are available in `target/`.

## Troubleshooting

### "Permission Denied" on Startup
→ Run with administrator privileges. Open command prompt as admin or use `runas`.

### WinDivert Installation Issues
→ WinDivert requires system drivers. Rebuild the project or reinstall WinDivert manually.

### Compiler Errors
→ Ensure you're using Rust 1.70+: `rustc --version`

### No Packets Captured
→ Check Windows Firewall isn't blocking port 6672
→ Verify another application isn't already capturing packets
→ Try running `ipconfig /all` to verify network adapters

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

## License

See LICENSE file for details.

## Additional Resources

- **DEVELOPMENT.md** - Detailed development workflows and examples
- **ARCHITECTURE.md** - Deep dive into component design and async handling
- See individual `README.md` files in `lobbyguard-cli/` and `lobbyguard-gui/` for component-specific details
