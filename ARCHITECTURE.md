# Architecture Overview

Deep dive into LobbyGuard's design and implementation patterns.

## High-Level Architecture

```
┌─────────────────────────────────────────────────┐
│         User Application Layer                   │
├──────────────────┬──────────────────────────────┤
│  CLI Interface   │    GUI Interface (winio)     │
│  (compio)        │    (ELM-style components)    │
├──────────────────┴──────────────────────────────┤
│           Shared Core Library                    │
│    (Packet capture, filtering, error types)     │
├──────────────────────────────────────────────────┤
│  Operating System (Windows Only)                 │
│  ├─ WinDivert (packet filtering driver)         │
│  └─ IOCP (async I/O completion ports)           │
└──────────────────────────────────────────────────┘
```

## Module Organization

### lobbyguard-core

**Purpose**: Shared logic for both CLI and GUI applications.

```
src/
├── lib.rs              # Public API exports
├── capture.rs          # PacketCapture struct
├── constants.rs        # Configuration constants
└── error.rs            # Error types with snafu
```

#### Key Components

**PacketCapture Struct**
```rust
pub struct PacketCapture {
    divert: WinDivert<NetworkLayer>,
}

impl PacketCapture {
    pub fn new() -> Result<Self>           // Initialize WinDivert
    pub async fn run(&mut self) -> Result<()>  // Main packet loop
    pub fn shutdown(self) -> Result<()>    // Graceful shutdown
}
```

**Packet Processing Flow**
```
1. WinDivert receives packet
   ↓
2. parse IP header (etherparse)
   ↓
3. parse UDP payload
   ↓
4. check packet size against HEARTBEAT_SIZES
   ↓
5. if heartbeat: forward packet, else: drop
```

**Constants Configuration**
```rust
// Heartbeat detection
const HEARTBEAT_SIZES: &[usize] = &[12, 18, 63];

// WinDivert filter
fn divert_filter() -> String {
    "udp.DstPort == 6672 and tcp.Flags == 0".to_string()
}
```

### lobbyguard-cli

**Purpose**: Minimal command-line interface for packet capture.

```
src/
└── main.rs    # Entry point with signal handling
```

**Execution Flow**
```
1. Parse command-line arguments
   ↓
2. Create PacketCapture instance
3. Spawn signal handler (Ctrl-C via compio)
   ↓
4. Spawn packet capture task
   ↓
5. Wait for signal
   ↓
6. Shutdown gracefully
```

**Async Task Model**
```rust
#[compio::main]                    // Single-threaded runtime
async fn main() -> Result<()> {
    let ctrl_c_task = compio_runtime::spawn(async { /* ... */ });
    let capture_task = compio_runtime::spawn(async move {
        capture.run().await
    });
    
    ctrl_c_task.await.unwrap_or_else(/* handle panic */);
    capture_task.await.unwrap_or_else(/* handle panic */);
}
```

### lobbyguard-gui

**Purpose**: Windows UI for packet capture monitoring.

```
src/
├── main.rs           # App initialization
├── component.rs      # Main ELM component
├── error.rs          # GUI error types
├── capture/          # Packet capture integration
├── process/          # Packet processing UI
├── storage/          # Data persistence
└── ui/               # UI components
```

**Component Architecture (ELM Pattern)**

Each component implements:
- **Model**: State representation
- **Message**: Events/commands
- **View**: UI rendering
- **Update**: State transitions

```rust
pub trait Component: Clone {
    fn event(&mut self, evt: Event) -> Command;
    fn view(&self) -> Element;
}

#[derive(Clone)]
pub struct MainModel {
    state: AppState,
    capture: PacketCapture,
}

impl Component for MainModel {
    fn event(&mut self, evt: Event) -> Command {
        match evt {
            Event::CaptureStarted => {
                self.state = AppState::Running;
                Command::None
            }
            Event::CaptureError(e) => {
                self.state = AppState::Error(e);
                Command::None
            }
            _ => Command::None,
        }
    }

    fn view(&self) -> Element {
        match &self.state {
            AppState::Idle => button("Start Capture").into(),
            AppState::Running => text("Capturing...").into(),
            AppState::Error(e) => text(format!("Error: {}", e)).into(),
        }
    }
}
```

## Error Handling Architecture

**Design Principle**: All fallible operations return `Result<T>` where `T` is a snafu-based error enum.

**Pattern Implementation**

```rust
use snafu::prelude::*;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    // External error with context
    #[snafu(display("Operation failed on {path}: {source}"))]
    FileOperation {
        source: std::io::Error,
        path: String,
    },
    
    // Validation error
    #[snafu(display("Invalid packet size: {size}"))]
    InvalidPacketSize { size: usize },
    
    // Boxed error for complex types
    #[snafu(display("WinDivert error: {source}"))]
    DivertError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

type Result<T, E = Error> = std::result::Result<T, E>;
```

**Usage Pattern**

```rust
// Wrapping external errors with context
fn read_config(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .context(FileOperationSnafu { path })?
}

// Validating with ensure!
fn validate_packet(size: usize) -> Result<()> {
    ensure!(size > 0 && size < 65536, InvalidPacketSizeSnafu { size });
    Ok(())
}
```

**Error Conversion**

Implement `Error` enum for automatic error propagation:

```rust
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Generic winio UI error
    #[snafu(context(false), display("UI error: {source}"))]
    Ui { source: winio::Error },
}

// Allows ? operator to work across error types
fn gui_operation() -> Result<()> {
    let app = winio::App::new("id")?;  // winio::Error converted to our Error, context(false) so it don't need `.context(error::UiSnafu)`
    Ok(())
}
```

## Packet Capture Pipeline

**Detailed Flow**

```
1. WinDivert Driver (Windows Kernel)
   ├─ Intercepts UDP packets on port 6672
   └─ Queues packets for userspace consumption

2. PacketCapture::run() Loop
   ├─ divert.recv()
   │  └─ Blocks on waiting packet arrival (so whole loop block a thread)
   │
   ├─ process_packet(data)
   │  ├─ Parse IPv4 header (etherparse)
   │  ├─ Parse UDP payload
   │  └─ Check size against HEARTBEAT_SIZES
   │
   └─ If heartbeat: divert.send(packet)
      └─ Forward to WinDivert for transmission

3. Shutdown
   ├─ Graceful: shutdown_handle.shutdown()
   └─ WinDivert flushes queue and closes
```

**Memory Management**

- Packet buffer: 65535 bytes (stack-allocated)
- WinDivert queue: Kernel-managed
- Error types: Small enums with boxed error chain
- Task state: Owned by runtime, not cloned

## Performance Characteristics

**Packet Processing**

- **Latency**: < 1ms per packet (blocking I/O + parsing)
- **Throughput**: Limited by WinDivert queue size (~10,000 packets)
- **Memory**: ~65KB per active capture session
- **CPU**: Single core utilization (thread-per-core model)

**Optimization Points**

```rust
// 1. Buffer pooling (future optimization)
// Currently: Stack allocation per loop iteration
// Better: Reuse buffer across loop iterations

// 2. Packet batching
// Currently: Process one packet at a time
// Better: Use WinDivert batch receive

// 3. Filter optimization
// Currently: Simple UDP port filter
// Better: Kernel-level filter for better performance
```

## Extension Points

**Adding New Packet Filters**

1. Update `constants.rs`:
```rust
pub fn divert_filter() -> String {
    "tcp.DstPort == 443 or udp.DstPort == 6672".to_string()
}
```

2. Update `capture.rs` processing logic
3. Add tests

**Adding GUI Components**

1. Create new struct implementing `Component`
2. Add to parent component's model
3. Render in view, handle events in update

**Adding New Error Types**

1. Define variant in `error.rs`
2. Implement message via `#[snafu(display(...))]`
3. Use via `context()` or `ensure!()`

## Design Decisions

**Why Snafu over anyhow?**
- Structured error types enable better diagnostics
- Context information is explicit
- Works well with `#[snafu::report]` for error reporting
- Smaller binary overhead

**Why Compio over Tokio?**
- Thread-per-core model avoids thread pool overhead
- Better for I/O-bound workloads
- Lower latency for packet capture
- No Send bound on tasks

**Why ELM for GUI?**
- Predictable component behavior
- Easy to add new UI elements
- Clear event flow
- Great for async integration

**Why Stack Buffer for Packets?**
- Avoids heap allocation per packet
- Predictable performance
- Most packets << 65KB
- Can be optimized with buffer pool later

## Testing Architecture

**Unit Tests**
- Located in module files
- Test specific functions in isolation
- No external dependencies

**Integration Tests**
- Located in `tests/` directory
- Test interactions between modules
- May require admin privileges

**Test Pattern**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        // Arrange
        let input = setup();
        
        // Act
        let result = function_under_test(input)?;
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Future Improvements

1. **Performance**: Implement packet batching for higher throughput
2. **Features**: Add real-time statistics dashboard
3. **Reliability**: Implement automatic reconnection on WinDivert failure
4. **Testing**: Add property-based tests with proptest
5. **Documentation**: Generate API docs with cargo-doc
