# LobbyGuard Agent Guide

This document provides essential technical details for developing LobbyGuard.

---

## 1. High-Level Architecture

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

## 2. Core Crates & Modules

The project is a Cargo workspace with three main crates:

### `lobbyguard-core`
**Purpose**: Shared logic for packet capture, filtering, and data structures.
```
src/
├── lib.rs              # Public API exports
├── capture.rs          # PacketCapture struct & capture logic
├── constants.rs        # Filter strings and packet size constants
├── error.rs            # `snafu`-based error types
└── packet_data.rs      # Data structures for captured packets
```
- **Key Struct**: `PacketCapture` manages the WinDivert session and packet processing loop.

### `lobbyguard-cli`
**Purpose**: A minimal, command-line interface for packet capture.
```
src/
└── main.rs    # Entry point with signal handling
```
- **Execution**: Initializes `PacketCapture` from `lobbyguard-core`, spawns a capture task, and waits for a Ctrl-C signal to shut down gracefully.

### `lobbyguard-gui`
**Purpose**: A Windows GUI for packet capture monitoring and viewing.
```
src/
├── main.rs           # App initialization
├── component.rs      # Main window component (MainModel)
├── viewer.rs         # Packet viewer component (PacketViewerModel)
└── ...               # Other supporting modules
```
- **Key Components**:
    - `MainModel`: The primary UI component managing the main window, buttons, and capture state.
    - `PacketViewerModel`: A child component for the secondary window that displays captured packet details.

---

## 3. Key Patterns & Technologies

### Async Runtime: `compio`
- The project uses `compio` instead of `tokio`.
- It employs a thread-per-core model, which is efficient for I/O-bound workloads like packet capture and avoids `Send` bounds on tasks.
- Use `#[compio::main]` for the entry point and `compio_runtime::spawn` for tasks.

### Error Handling: `snafu`
- All fallible operations must return a `Result<T, E>` where `E` is a project-specific error enum created with `snafu`.
- This provides structured, context-rich errors.
- Use `.context(...)` to wrap errors from external libraries.
- Use `ensure!(...)` for validation checks.

### GUI: `winio` and the ELM Pattern
The GUI follows an architecture inspired by Elm (ELM):
- **Model**: A struct holding the component's state (e.g., `MainModel`).
- **Message**: An enum defining actions that can update the state (e.g., `MainMessage`).
- **Update**: An `update` method that processes messages and modifies the model.
- **View**: A `render` method that defines the UI layout based on the state.

#### Component Communication
- **Parent-to-Child**:
    - `post(message)`: Asynchronously queues a message for the child to process. (Standard)
    - `emit(message)`: Asynchronously calls the child's `update` method directly. (For immediate updates)
- **Child-to-Parent**:
    1. Child uses `sender.output(event)` to send an event upwards.
    2. Parent uses the `start!` macro in its `start` method to listen for child events and map them to its own messages.

This creates a clear, unidirectional data flow for events and state updates.

---

## 4. Development & Verification

### Initial Setup
Use these commands to build and test the entire workspace.
```bash
# Build all crates
cargo build --all

# Run all tests
cargo test --all
```

### Code Quality
Before committing, always run the following commands to ensure code quality and consistency.
```bash
# Format all code
cargo fmt --all

# Lint all code (with warnings as errors)
cargo clippy --all -- -D warnings
```