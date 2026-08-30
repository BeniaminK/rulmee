[![GitHub license](https://badgen.net/github/license/BeniaminK/rulmee)](https://github.com/BeniaminK/rulmee/blob/main/LICENSE)
[![GitHub branches](https://badgen.net/github/branches/BeniaminK/rulmee)](https://github.com/BeniaminK/rulmee)
[![Latest Release](https://badgen.net/github/release/BeniaminK/rulmee)](https://github.com/BeniaminK/rulmee/releases)

# Rulmee (RUst Login ManagEEr)

**Rulmee** (RUst Login ManagEEr) is a secure and highly customizable Terminal User Interface (TUI) display manager written in Rust. It is a completely rewritten and enhanced version of [LiDM](https://github.com/javalsai/lidm).

Like traditional display managers (such as SDDM or GDM), Rulmee handles user authentication, session discovery, and desktop launching—all within a text-based TUI directly on Linux virtual terminals (TTYs).

![demo gif](assets/media/lidm.gif)

---

## Motivation & Architecture Shift

Rulmee was originally developed in C as [LiDM](https://github.com/javalsai/lidm). As security, stability, and desktop session complexity grew, the codebase was rewritten from the ground up in Rust.

### Why Rust?
- **Memory Safety & Privilege Isolation**: Display managers execute as root before dropping privileges to spawn user sessions. Rust eliminates memory safety issues (buffer overflows, use-after-free, unsafe string parsing) at compile time.
- **Modern TUI Rendering**: Built on [`ratatui`](https://ratatui.rs) and [`crossterm`](https://crates.io/crates/crossterm), providing clean rendering, double-buffering, and zero terminal screen flicker.
- **Structured Diagnostics**: Utilizes [`tracing`](https://crates.io/crates/tracing) to separate TUI rendering output (`stdout`) from systemd/journald log records (`stderr`) and live in-app log viewing (<kbd>F4</kbd>).
- **Type-Safe Configuration**: Migrated from legacy INI files to structured TOML files (`config.toml` & `theme.toml`).

---

## Features

- **Modern & Memory Safe**: Powered by Rust (2024 edition), `ratatui`, and `nix`.
- **Freedesktop Compliant**: Discovers Wayland and X11 sessions from `/usr/share/xsessions` and `/usr/share/wayland-sessions`.
- **Strict Security Boundaries**: Complies with Linux-PAM specifications and POSIX privilege separation (`setgid` $\rightarrow$ `initgroups` $\rightarrow$ `setuid` $\rightarrow$ `chdir`). Root context never evaluates user shell profiles.
- **Structured TOML Configuration**: Easily configure keybindings, strings, layout, and colors in `/etc/rulmee/config.toml` and `/etc/rulmee/theme.toml`.
- **Multi-Destination Logging**: Non-corrupting `stderr` (FD 2) logging for `systemd-journald`, file logging to `/tmp/rulmee.log`, and an in-app log inspector (<kbd>F4</kbd>).
- **YubiKey / FIDO Support**: Hardware key authentication via `pam_u2f` (see notes in [yubikey.md](./docs/yubikey.md)).
- **Init System Agnostic**: Ready for `systemd`, `dinit`, `runit`, `openrc`, and `s6`.

---

# Table of Contents

- [Motivation & Architecture Shift](#motivation--architecture-shift)
- [Features](#features)
- [Usage](#usage)
  - [CLI Arguments & Overrides](#cli-arguments--overrides)
  - [Commands](#commands)
  - [TUI Interface Controls](#tui-interface-controls)
- [Requirements](#requirements)
- [Building & Installation](#building--installation)
- [Configuration](#configuration)
- [PAM Authentication](#pam-authentication)
- [Logging & Systemd Architecture](#logging--systemd-architecture)
- [Inspiration & History](#inspiration--history)
- [Contributing](#contributing)
- [License](#license)

---

# Usage

### CLI Arguments & Overrides

Rulmee supports command-line overrides to update configuration values dynamically at runtime:

```bash
# Target specific virtual terminal TTY
rulmee 7

# Dynamic config override
rulmee --set behavior.include_defshell=true
```

### Commands

- `rulmee copy-config [DEST]`: Copy default configuration to user config (`~/.config/rulmee/config.toml`) or a specified destination path.

```bash
# Copy to default user config location (~/.config/rulmee/config.toml)
rulmee copy-config

# Copy to custom system path
rulmee copy-config /etc/rulmee/config.toml
```

### TUI Interface Controls

- **Navigation**: Use Up/Down arrow keys or <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> to move between fields (username, password, session selector).
- **Option Switcher**: Use Left/Right arrow keys on selector fields to cycle available desktop sessions and users.
- **Log Viewer**: Press <kbd>F4</kbd> anytime to open the live in-app log overlay.
- **Login**: Press <kbd>Enter</kbd> to authenticate and launch the selected desktop session.

---

# Requirements

- **Rust Toolchain**: `cargo` and `rustc` (edition 2024 / Rust 1.85+).
- **PAM Library**: Linux-PAM header files (`libpam0g-dev` on Debian/Ubuntu, `pam-devel` on Fedora/Arch/RHEL).

---

# Building & Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/BeniaminK/rulmee.git
cd rulmee

# Build release binary
cargo build --release
```

The compiled binary will be located at `target/release/rulmee`.

### Installation

Install the binary to your system path:

```bash
cargo install --path .
```

To install default configuration files, man pages, and service descriptors, consult the [Installation Guide](./docs/INSTALL.md) and [Packagers Guide](./docs/PACKAGERS.md).

---

# Configuration

Rulmee reads configuration from `/etc/rulmee/config.toml` (or user config at `~/.config/rulmee/config.toml`).

Themes are loaded from `/etc/rulmee/theme.toml` or packaged themes in `/usr/share/rulmee/themes/`.

> [!NOTE]
> **Backward Compatibility**: If `/etc/rulmee/config.toml` is absent, Rulmee temporarily falls back to legacy `/etc/lidm/config.ini` and emits a deprecation warning urging migration to TOML. This fallback support will be removed when Rulmee gains more GitHub stars than LiDM (as an indicator of adoption and migration completeness).

---

# PAM Authentication

Rulmee initializes Linux-PAM authentication using the `login` PAM service (`/etc/pam.d/login`) by default. You can override the target service name by setting `RULMEE_AUTH_PAM_SERVICE`:

```bash
export RULMEE_AUTH_PAM_SERVICE="rulmee"
```

---

# Logging & Systemd Architecture

Rulmee features a multi-destination logging architecture specifically engineered to prevent TUI screen corruption:

- **Journald (`stderr` / FD 2)**: External service logs write to `stderr` (FD 2). Because Ratatui renders the TUI interface on `stdout` (FD 1), process logs are collected by `systemd-journald` cleanly without visual corruption.
- **Log File (`/tmp/rulmee.log`)**: Non-blocking log persistence using `tracing-appender`.
- **In-App Console Viewer (<kbd>F4</kbd>)**: Log messages are captured into an internal ring buffer for live inspection inside the TUI by pressing <kbd>F4</kbd>.

For complete technical specifications on stdout/stderr isolation and log event standards, see [STANDARDS.md](./STANDARDS.md).

---

# Inspiration & History

I originally really liked [LiDM](https://github.com/javalsai/lidm). However, system logs were displayed directly over the user entry point in the console, which disrupted the terminal UI and made it look bad.

When I started fixing this logging issue, I realized it would be a great opportunity to rewrite the entire display manager in Rust. During the naming phase, I considered `rlidm` (Rust LiDM) and `rulme`, before settling on the slight modification **Rulmee**.

---

# Contributing

Please read our [Contributing Guidelines](docs/CONTRIBUTING.md).

---

# License

This project is licensed under the GNU General Public License v3.0 **only**. See [LICENSE](./LICENSE) for details.

---

🌟 If you find Rulmee useful, consider starring this repo! 🔪
