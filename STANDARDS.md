# Linux Display Manager Standards & Specifications Guide (Rulmee)

This document details the Linux specifications, POSIX standards, and freedesktop.org conventions governing **Rulmee** (RUst Login ManagEEr). It serves as an authoritative reference for session discovery, PAM authentication, environment setup, privilege separation, logging standards, and desktop launching.

---

## 1. Freedesktop.org Specifications

Freedesktop.org provides the core standards for Linux desktop session discovery and environment configuration.

### 1.1 Desktop Entry Specification
* **Specification**: [Freedesktop Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/latest/)
* **Session Discovery Directories**:
  * X11 Sessions: `/usr/share/xsessions`, `/usr/local/share/xsessions`
  * Wayland Sessions: `/usr/share/wayland-sessions`, `/usr/local/share/wayland-sessions`
* **Key Attributes**:
  * `Name`: Human-readable name of the session.
  * `Exec`: Command line to execute the session binary.
  * `TryExec`: Binary name/path to test for existence before displaying option.
  * `Type`: Must be `Application`.
* **Field Code Stripping**:
  * DMs must strip Freedesktop `%` specifiers from `Exec` strings (e.g. `%f`, `%F`, `%u`, `%U`, `%i`, `%c`, `%k`, `%%`).

### 1.2 XDG Base Directory Specification
* **Specification**: [Freedesktop XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/)
* **Standard Variables**:
  * `XDG_DATA_HOME`: Default `$HOME/.local/share`
  * `XDG_CONFIG_HOME`: Default `$HOME/.config`
  * `XDG_DATA_DIRS`: Default `/usr/local/share:/usr/share`
  * `XDG_CONFIG_DIRS`: Default `/etc/xdg`
  * `XDG_RUNTIME_DIR`: User-specific runtime directory (`/run/user/<UID>`), managed via `pam_systemd`.

### 1.3 Desktop Environment Variables
* **Standard Variables**:
  * `XDG_SESSION_TYPE`: Session display server protocol (`"x11"`, `"wayland"`, or `"tty"`).
  * `XDG_SESSION_CLASS`: Session classification (`"user"` for user desktops, `"greeter"` for DM UI).
  * `XDG_CURRENT_DESKTOP`: Colon-separated list of current desktop names (e.g. `GNOME`, `KDE`, `sway`, `labwc`).
  * `XDG_SESSION_DESKTOP`: Desktop identity string matching the `.desktop` file basename.

---

## 2. Linux-PAM (Pluggable Authentication Modules) Specification

Linux-PAM standardizes user authentication, account verification, and session credential management.

### 2.1 PAM Session Lifecycle
A display manager must invoke PAM in the following strict order:

1. `pam_start(service_name, username, &conv, &pamh)`: Initialize PAM context (service `login` or `rulmee`).
2. `pam_authenticate(pamh, flags)`: Verify user credentials (password, FIDO, etc.).
3. `pam_acct_mgmt(pamh, flags)`: Verify account validity (password expiry, access rules).
4. `pam_setcred(pamh, PAM_ESTABLISH_CRED)`: Establish user credentials.
5. `pam_open_session(pamh, flags)`: Open user session (mounts keyrings, starts logind tracking, sets up limits).
6. `pam_setcred(pamh, PAM_REINITIALIZE_CRED)`: Reinitialize credentials for session lifetime.
7. **Session Execution** (`fork` + `exec`).
8. `pam_setcred(pamh, PAM_DELETE_CRED)`: Delete credentials after child session exits.
9. `pam_close_session(pamh, flags)`: Close PAM session and release resources.
10. `pam_end(pamh, status)`: Terminate PAM context.

### 2.2 PAM Environment Variable Export
* PAM modules (`pam_env.so`, `pam_systemd.so`, `pam_mail.so`, `pam_gnome_keyring.so`) populate session environment variables during `pam_open_session`.
* **Retrieval API**: `pam_getenvlist(pamh)` / `pam-client2` returns `KEY=VALUE` pairs.
* The DM must merge these PAM environment variables into the session environment before `exec`.

---

## 3. `systemd-logind` Session Specification

`systemd-logind` manages user logins, virtual terminals, seats, and power permissions.

### 3.1 Logind Integration & Session Tracking
* **Trigger**: Invoked automatically when `pam_systemd.so` is included in the PAM configuration stack during `pam_open_session`.
* **Required Environment Variables**:
  * `XDG_SEAT`: Hardware seat assignment (typically `seat0`).
  * `XDG_VTNR`: Virtual Terminal index (e.g. `1` for `/dev/tty1`, `7` for `/dev/tty7`).
  * `XDG_SESSION_TYPE`: `"wayland"` or `"x11"`.
* **Functionality**: `logind` creates `/run/user/<UID>`, registers session ID in `loginctl`, grants unprivileged user access to GPU (`/dev/dri/*`) and input devices (`/dev/input/*`), and authorizes reboot/shutdown requests via `polkit`.

---

## 4. POSIX.1 (IEEE Std 1003.1) Shell Standards

POSIX defines standards for process execution, privilege management, and login shell behavior.

### 4.1 Login Shell Behavior (`-l` / `--login`)
* POSIX specifies that when a shell is invoked as a **login shell** (indicated by `-l`, `--login`, or `argv[0]` starting with `-` e.g. `-bash`):
  1. It evaluates system-wide profile scripts: `/etc/profile`
  2. It evaluates user-specific profile scripts: `~/.profile` (or `~/.bash_profile`, `~/.zprofile`).
* **Session Wrapper Execution**: Running session commands via `$SHELL -l -c "exec <session_cmd>"` ensures POSIX-compliant script sourcing without manual C/Rust script parsing.

### 4.2 Process Image Replacement (`exec`)
* Using `exec <session_cmd>` inside the login subshell replaces the shell process image with the desktop compositor (`sway`, `i3`, `gnome-session`).
* Prevents intermediate wrapper subshells from idling in the process tree during desktop runtime.

---

## 5. Security & Privilege Separation Standard

Display managers run as `root` but must execute user code strictly in unprivileged user context.

### 5.1 Privilege Dropping Sequence
Inside the child process after `fork()`, privileges must be dropped in the following exact order **before** calling `exec()`:

```
1. setgid(user_gid)              # Drop group privileges
2. initgroups(username, user_gid)# Initialize supplementary groups (/etc/group)
3. setuid(user_uid)              # Drop user privileges to target UID
4. chdir(user_home_dir)          # Change working directory to $HOME
```

### 5.2 Root Context Isolation
* User-controlled profile files (`~/.profile`, `~/.xprofile`, `~/.bashrc`) **must never** be parsed or evaluated inside the root DM process.
* Delegating profile evaluation to the login shell after dropping privileges prevents local privilege escalation (LPE) vulnerabilities.

---

## 6. Summary Matrix: Rulmee Architecture Alignment

| Architectural Component | Standard / Specification | Rulmee Implementation Strategy |
| :--- | :--- | :--- |
| **Session Discovery** | Freedesktop Desktop Entry Spec | Scans `/usr/share/{xsessions,wayland-sessions}`, strips `%` specifiers. |
| **Authentication & PAM** | Linux-PAM API Specification | Executes full `pam_open_session` lifecycle; retrieves `pam_getenvlist()`. |
| **Session Tracking & VTs** | `systemd-logind` Specification | Inherits `XDG_RUNTIME_DIR`, passes `XDG_SEAT` and `XDG_VTNR` via `pam_systemd`. |
| **Environment Sourcing** | POSIX.1 Login Shell Standard | Spawns session via `$SHELL -l -c "exec <cmd>"` when `bypass_shell_login` is `false`. |
| **Privilege Separation** | POSIX Security Standards | `setgid` $\rightarrow$ `initgroups` $\rightarrow$ `setuid` in child process before shell execution. |
| **Logging & Diagnostics** | Tracing / Systemd Journal Spec | Multi-destination subscriber (`stderr` for journald, `/tmp/rulmee.log` file, `<kbd>F4</kbd>` in-app TUI ring buffer). |

---

## 7. Logging Standards & Architecture

Rulmee implements a multi-destination logging architecture utilizing `tracing`, `tracing-subscriber`, and `tracing-appender` to support thorough diagnostics without corrupting the terminal user interface:

### 7.1 Screen Corruption Prevention (FD 1 vs FD 2)
* **Stdout (FD 1)**: Standard output is reserved exclusively for Ratatui and Crossterm TUI drawing and ANSI escape sequences. No log subscriber writes to `stdout`.
* **Stderr (FD 2)**: Application logs (`info!`, `warn!`, `error!`, `debug!`, `trace!`) write to `stderr`. When executed as a service under `systemd` or another supervisor, `systemd-journald` captures service logs from `stderr` cleanly without visual corruption on `stdout`.

### 7.2 Non-Blocking File Logging
* Application events are persisted to disk at `/tmp/rulmee.log` (or custom configured paths).
* File writes use `tracing-appender` for non-blocking I/O to guarantee that logging disk latency never stutters the TUI event loop.

### 7.3 In-App TUI Log Viewer (<kbd>F4</kbd>)
* Log messages are recorded in a thread-safe, bounded in-memory ring buffer.
* Pressing <kbd>F4</kbd> within the TUI opens an overlay log inspector, permitting real-time inspection of PAM and session events directly inside the application interface.

### 7.4 Deprecation Warning Standard
* Fallback paths (such as loading legacy ANSI `.ini` theme files) emit `tracing::warn!` notices.
* Deprecation logs explicitly advise administrators to update their deployment to `/etc/rulmee/default.toml` or TOML theme files.
