# LobbyGuard GUI

A graphical user interface for the LobbyGuard packet capture system built with winio and compio.

## Building

```bash
cargo build --release
```

## Running

Must be run with administrator privileges:

```bash
cargo run --release
```

## Architecture

The GUI is organized using winio's component-based architecture:

- **main.rs** - Entry point and application initialization
- **component.rs** - Main window component with UI logic
- **error.rs** - Error handling with snafu

## Component Structure

The main component follows the ELM pattern:

1. **init** - Creates window and button widgets
2. **start** - Sets up event listeners
3. **update** - Processes messages and updates state
4. **render** - Layouts widgets using Grid

## Features

- Toggle packet capture on/off
- Visual feedback through button state
- Graceful shutdown on window close
- Administrator rights detection
