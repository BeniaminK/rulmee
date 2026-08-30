# LiDM TODO & Refactoring Roadmap

## Logging & Output
- [ ] **Logging & Output Enhancement**: Distribute logging output efficiently to log files and stdout, adding ANSI color support for interactive stdout debugging.

## Build System & Code Quality
- [ ] **Makefile Update**: Update Makefile targets to streamline building, testing, linting, and release workflows.
- [ ] **Rust Linting & Code Formatting**: Run and enforce `cargo clippy` rules and `cargo fmt` across the codebase to ensure idiomatic Rust code quality.

## Architecture & Code Simplification
- [ ] **Code Shortening**: Reduce overall codebase size by removing redundant boilerplate and refactoring oversized source files.
- [ ] **Structural Design Patterns**: Introduce cleaner software design patterns to improve component separation and state management.

## UI & Ratatui Componentization
- [ ] **Ratatui Widget Componentization**: Refactor UI rendering logic from monolithic loops into modular Ratatui `Widget` and `StatefulWidget` implementations.
- [ ] **Decompose UI Components**: Extract individual UI sections into self-contained widgets such as `HeaderWidget` and `HotkeyBarWidget`.

## Function Modularization & Helper Extraction
- [ ] **Small Composable Functions**: Break down large functions into small, single-responsibility composable functions for better readability and unit testing.
- [ ] **Extract Session Helpers**: Decompose process and session spawning logic into concise helpers such as `spawn_xorg_server`, `wait_for_display_pipe`, `spawn_session_child`, and `split_exec_tokens`.

- [ ] Tworzenie pakietu debianowego
- [ ] Github testowanie i continues deployment
- [ ] Czytanie plików ini
- [ ] Rulmee - RUst Login ManagEEr - update README, lidm reference, why change, how to contribute, STANDARDS.md the standards this login manager is keeping, why? Because it was spamming lots of logs and was hard to edit the configuration, should be self-explained
- [ ] Github repository

