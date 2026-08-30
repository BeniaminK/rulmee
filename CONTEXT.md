# Context: Rulmee (RUst Login ManagEEr)

This document defines the canonical domain model, terminology, and invariants for **Rulmee** (formerly LiDM). Implementation details belong in Rust source files; domain definitions belong here.

---

## Core Glossary

### Rulmee
A lightweight, secure, and customizable Terminal User Interface (TUI) display manager written in Rust. Manages virtual terminals, PAM authentication, session discovery, and privilege-separated session execution.

### Desktop Session
A session configuration discovered from `.desktop` files conforming to the **Freedesktop Desktop Entry Specification**. Discovered in `/usr/share/xsessions` (X11) and `/usr/share/wayland-sessions` (Wayland).

### Session Type
The classification of execution protocol for a desktop session: `Wayland`, `X11`, or `Shell` (TTY).

### Legacy INI Theme
A legacy ANSI SGR escape code theme format (`.ini`) supported temporarily for backward compatibility. Parsed into canonical `ThemeStyle` structures with a `log::warn!` deprecation warning recommending migration to TOML.

### TOML Theme
The canonical configuration and styling format. Expressed using TOML files (`/etc/rulmee/theme.toml` or custom theme files) deserialized into structured Rust theme models.

### Configuration Directory
The primary system configuration directory is `/etc/rulmee/`. For backward compatibility, if `/etc/rulmee/config.toml` is not present, Rulmee temporarily falls back to `/etc/lidm/config.ini` with a deprecation warning.

### PAM Session Lifecycle
The strict 10-step sequence for authentication and session credential creation via Linux-PAM (`pam_start` $\rightarrow$ `pam_authenticate` $\rightarrow$ `pam_acct_mgmt` $\rightarrow$ `pam_setcred` $\rightarrow$ `pam_open_session` $\rightarrow$ `exec` $\rightarrow$ `pam_close_session`).

### Privilege Dropping
The security sequence performed in a child process after `fork()` and before `exec()` to drop root privileges (`setgid` $\rightarrow$ `initgroups` $\rightarrow$ `setuid` $\rightarrow$ `chdir`).

### CLI Override
Dynamic command-line override arguments passed via `--set key=value` or subcommands to update configuration options at runtime.
