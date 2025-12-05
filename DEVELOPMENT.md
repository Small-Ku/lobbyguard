# Development Guide

Complete workflows for common development tasks.

## Development Setup

```bash
# Clone and enter workspace
cd lobbyguard

# Verify build works
cargo build --all

# Run tests
cargo test --all

# Check code quality
cargo fmt --all
cargo clippy --all -- -D warnings
```

## Common Workflows

### Adding a Packet Filter

1. Update filter constants in `lobbyguard-core/src/constants.rs`:
```rust
pub fn divert_filter() -> String {
    "udp.DstPort == 6672 and ip.Length > 20".to_string()
}

pub const HEARTBEAT_SIZES: &[usize] = &[12, 18, 63, 100]; // Add new size
```

2. Update packet processing in `lobbyguard-core/src/capture.rs`:
```rust
fn process_packet(&self, data: &[u8]) -> crate::Result<bool> {
    let ip = Ipv4Slice::from_slice(data)
        .context(IpParseFailedSnafu)?;

    let udp = UdpSlice::from_slice(&ip.payload().payload)
        .context(UdpParseFailedSnafu)?;

    let size = udp.payload().len();
    
    // Check if this is a heartbeat packet
    if HEARTBEAT_SIZES.iter().any(|&x| x == size) {
        Ok(true)
    } else {
        Ok(false)
    }
}
```

3. Add tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_heartbeat_size() {
        assert!(HEARTBEAT_SIZES.contains(&100));
    }
}
```

### Adding Error Handling

Follow the snafu pattern with context and ensure!:

```rust
// In error.rs
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to initialize: {reason}"))]
    InitializationFailed { reason: String },
}

// In usage
use snafu::prelude::*;

fn initialize() -> Result<()> {
    // Use context() to wrap external errors
    let config = std::fs::read_to_string("config.toml")
        .context(ReadFileSnafu { path: "config.toml" })?;
    
    // Use ensure!() for validation
    ensure!(!config.is_empty(), InitializationFailedSnafu { 
        reason: "Config is empty" 
    });
    
    Ok(())
}
```

### Adding a GUI Component

1. Create component file in `lobbyguard-gui/src/ui/`:
```rust
// src/ui/my_component.rs
use winio::prelude::*;

#[derive(Default, Clone)]
pub struct MyComponent {
    value: String,
}

impl Component for MyComponent {
    fn event(&mut self, evt: Event) -> Command {
        match evt {
            Event::Button(id) => {
                if id == "submit" {
                    println!("Submitted: {}", self.value);
                }
                Command::None
            }
            _ => Command::None,
        }
    }

    fn view(&self) -> Element {
        column![
            text("Enter value:"),
            input("", &self.value).on_change(|v| /* update */),
            button("Submit").on_press(/* action */)
        ]
        .into()
    }
}
```

2. Export from `src/ui/mod.rs`:
```rust
pub mod my_component;
pub use my_component::MyComponent;
```

3. Use in main component:
```rust
// In src/component.rs
struct MainModel {
    my_component: MyComponent,
}

impl Component for MainModel {
    fn view(&self) -> Element {
        column![
            self.my_component.view(),
        ]
        .into()
    }
}
```

### Working with Async Tasks

Use compio instead of tokio:

```rust
// Don't use tokio::spawn
// let task = tokio::spawn(async { /* ... */ });

// Use compio
let task = compio_runtime::spawn(async {
    // Runs on current thread in thread-per-core model
    expensive_operation().await
});

// Handle the result (doesn't need Send bound)
let result = task.await.unwrap_or_else(|e| std::panic::resume_unwind(e));
```

For blocking I/O:

```rust
// Instead of tokio::task::block_in_place
// Use compio's spawn_blocking
let result = compio_runtime::spawn_blocking(|| {
    // Runs in thread pool
    std::fs::read_to_string("file.txt")
})
.await
.unwrap_or_else(|e| std::panic::resume_unwind(e))?;
```

### Running with Debugging

```bash
# Run CLI with debug output
RUST_LOG=debug cargo run -p lobbyguard-cli

# Run tests with output
cargo test -p lobbyguard-core -- --nocapture --test-threads=1

# Run with backtrace
RUST_BACKTRACE=1 cargo run -p lobbyguard-cli

# Use rust-gdb for step-by-step debugging
# (requires lldb/gdb installed)
rust-gdb target/debug/lobbyguard-cli
```

### Performance Profiling

```bash
# Build release version
cargo build --release -p lobbyguard-cli

# Profile with perf (Linux) or similar tools on Windows
# Or use built-in Rust profiling: --profile release-with-debug-info
```

## Testing Strategy

### Unit Tests

```bash
# Test specific module
cargo test -p lobbyguard-core capture::

# Run single test
cargo test -p lobbyguard-core test_heartbeat_sizes

# Run with output
cargo test -- --nocapture
```

### Example Test

```rust
// In src/capture.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_detection() {
        let capture = PacketCapture::new().unwrap();
        assert!(HEARTBEAT_SIZES.len() > 0);
    }

    #[test]
    fn test_filter_string() {
        let filter = crate::constants::divert_filter();
        assert!(filter.contains("6672"));
    }
}
```

### Integration Testing

Create `tests/integration_test.rs`:

```rust
use lobbyguard_core::capture::PacketCapture;

#[test]
fn test_packet_capture_initialization() {
    // This requires admin privileges
    if cfg!(not(target_os = "windows")) {
        return; // Skip on non-Windows
    }
    
    match PacketCapture::new() {
        Ok(_) => println!("Capture initialized"),
        Err(e) => println!("Expected admin error: {}", e),
    }
}
```

## Troubleshooting Development Issues

### Compilation Errors

**Error**: "error: field `0` of struct variant is private"
→ Remove invalid borrow syntax, use proper pattern matching

**Error**: "cannot find type `LayoutError`"  
→ Use generic error type or implement proper From trait

**Error**: "expected `impl Future` but got `Box<dyn Future>`"
→ Ensure return types match, use `.boxed()` if needed

### Async Runtime Issues

**Issue**: Task panics aren't caught
→ Use `.await.unwrap_or_else(|e| std::panic::resume_unwind(e))` to handle panics

**Issue**: Tasks don't finish
→ Ensure you're awaiting JoinHandles, not dropping them

### Debugging Packet Issues

```rust
// Add logging to packet processing
fn process_packet(&self, data: &[u8]) -> crate::Result<bool> {
    eprintln!("Processing {} bytes", data.len());
    
    let ip = Ipv4Slice::from_slice(data)
        .context(IpParseFailedSnafu)?;
    
    eprintln!("IP parsed: {:?}", ip);
    
    Ok(true)
}
```

## Code Quality Checklist

Before committing:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Lint with clippy
cargo clippy --all -- -D warnings

# Run all tests
cargo test --all

# Build in release mode
cargo build --release --all

# Check documentation
cargo doc --no-deps --open
```

## Performance Notes

- **Compio**: Thread-per-core model is more efficient for I/O-bound workloads like packet capture
- **WinDivert**: IOCP-based, completion-driven I/O on Windows
- **Packet Buffer**: 65535 bytes (MAX_PACKET_SIZE) - increase if needed for larger packets
- **Heartbeat Detection**: O(n) where n = number of heartbeat sizes (~3)

## Next Steps

1. Study the code examples in README.md
2. Read ARCHITECTURE.md for component design details
3. Check component-specific READMEs in `lobbyguard-cli/` and `lobbyguard-gui/`
4. Start with small modifications and gradually add features
