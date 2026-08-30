# Codebase Structure & Architecture

This document describes the module organization and code layout for the project.

---

## Codebase Organization

Modules are located in `src/`:

```
src/
├── main.rs         # Application entry point, CLI parsing, event loop orchestration
├── args.rs         # Command-line argument parsing and dynamic overrides (--set)
├── config.rs       # TOML configuration loader, fallbacks, and validation
├── theme.rs        # Theme discovery, TOML theme parsing, and legacy INI converter
├── legacy_ini.rs   # Deprecated INI theme support and migration warnings
├── logging.rs      # Tracing subscriber setup (stderr, /tmp/rulmee.log, TUI buffer)
├── pam.rs          # Linux-PAM authentication lifecycle and privilege dropping
├── session.rs      # Freedesktop .desktop session discovery (Wayland/X11/Shell)
└── ui/             # Ratatui TUI rendering, keybindings, and log overlay viewer
```

---

## Key Modules

### `main.rs` & `args.rs`
Handles program initialization, command-line flags, configuration merging, terminal initialization via Crossterm, and the main event loop.

### `config.rs` & `theme.rs`
Loads TOML configuration files (`/etc/rulmee/default.toml` and themes).

### `logging.rs`
Configures a multi-destination `tracing` subscriber:
- **`stderr`**: Routed to `systemd-journald` without stdout corruption.
- **Log File**: Async file appender writing to `/tmp/rulmee.log`.
- **TUI Ring Buffer**: In-memory ring buffer powering the <kbd>F4</kbd> live log overlay.

### `pam.rs`
Manages the 10-step PAM authentication flow and child privilege dropping (`setgid` $\rightarrow$ `initgroups` $\rightarrow$ `setuid` $\rightarrow$ `chdir`).

### `session.rs`
Scans `/usr/share/xsessions` and `/usr/share/wayland-sessions` to build available desktop session descriptors.
