# Session Environment & Login Shell Architecture Design

**Date**: 2026-08-04  
**Status**: Approved  
**Target Modules**: `src/auth.rs`, `src/exec.rs`, `src/main.rs`

---

## 1. Overview & Objective

In display managers (DMs), environment initialization and session startup follow a two-tier model:

1. **DM/PAM Environment Assembly**: Ingesting PAM environment (`pam_getenvlist()`), POSIX user credentials (`USER`, `HOME`, `SHELL`, `LOGNAME`), display variables (`DISPLAY`, `XAUTHORITY`), and Freedesktop session variables (`XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`, `XDG_SESSION_CLASS`).
2. **Login Shell & Wrapper Delegation**: Spawning the session process inside the target user's login shell via `$SHELL -l -c "<session_exec>"` (when `bypass_shell_login` is `false`). This delegates script sourcing (`/etc/profile`, `~/.profile`, `~/.bash_profile`, `~/.xprofile`, etc.) to the user's shell after root privileges have been dropped to the target user's UID/GID.

This design replaces custom root-level line-parsing of shell files with robust environment assembly and privilege-dropped login shell delegation.

---

## 2. Architecture & Component Responsibilities

### `src/auth.rs`
- Extracts environment variables from the PAM session context after successful authentication.
- Returns an `AuthSession` containing `username`, `uid`, `gid`, `home`, `shell`, and `env` (`HashMap<String, String>`).

### `src/exec.rs`
- Assembles the complete environment `HashMap`:
  - PAM Environment variables (`pam_getenvlist()`)
  - Core POSIX environment (`USER`, `LOGNAME`, `HOME`, `SHELL`, `PATH`)
  - XDG & Display environment (`XDG_SESSION_TYPE`, `XDG_SESSION_CLASS`, `XDG_CURRENT_DESKTOP`, `DISPLAY`)
- Formats command execution vectors according to `config.behavior.bypass_shell_login`:
  - **`bypass_shell_login == false` (Default)**: Constructs `[user_shell, "-l", "-c", "exec <exec_cmd>"]` (e.g. `["/bin/bash", "-l", "-c", "exec sway"]`).
  - **`bypass_shell_login == true`**: Executes the session binary directly (`exec_args`).
- Performs privilege dropping inside child processes (`initgroups`, `setgid`, `setuid`) before `cmd.exec()`.

---

## 3. Environment Merging & Command Construction Rules

### 3.1 Environment Precedence & Merging Order
1. **POSIX & Core User Environment**: `USER`, `LOGNAME`, `HOME`, `SHELL` (derived from system passwd record via `uzers`).
2. **Fallback `PATH`**: Default system `PATH` if not present in PAM (`/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`).
3. **XDG & Session Type Defaults**: `XDG_SESSION_TYPE` (`"wayland"`, `"x11"`, or `"tty"`), `XDG_SESSION_CLASS` (`"user"`), `XDG_CURRENT_DESKTOP` (derived from session name if available).
4. **PAM Environment**: Overrides/merges environment variables provided by PAM modules (`pam_getenvlist()`).
5. **Display Variables**: `DISPLAY` set dynamically for Xorg sessions.

### 3.2 Login Shell Command Formatting
When `bypass_shell_login` is `false`:
- Determine target shell `user_shell` (defaults to `/bin/bash` or `/bin/sh` if user's shell is unreadable).
- Escape or format execution arguments into a single shell command string `<exec_cmd>` (e.g. `sway --unsupported-gpu`).
- Construct execution command vector:
  ```rust
  vec![user_shell, "-l".to_string(), "-c".to_string(), format!("exec {}", exec_cmd)]
  ```
- Using `exec <exec_cmd>` ensures the desktop process replaces the intermediate subshell without leaving extra shell processes running in the background.

---

## 4. Testing Strategy

Unit tests will be implemented in `src/exec.rs` / `src/auth.rs`:

1. **Environment Assembly Test**: Verify PAM environment, POSIX user metadata, and XDG session variables merge with correct precedence into a `HashMap<String, String>`.
2. **Shell Command Vector Generation**:
   - Verify `build_exec_command` with `bypass_shell_login = false` returns `["/bin/bash", "-l", "-c", "exec sway"]`.
   - Verify `build_exec_command` with `bypass_shell_login = true` returns `["sway"]`.
   - Verify handling of multi-word exec strings and complex desktop command lines.

---

## 5. Security & Safety Considerations

- **Privilege Separation**: User shell initialization scripts (`/etc/profile`, `~/.profile`, `~/.bashrc`, `~/.xprofile`) execute exclusively inside the child process **after** `initgroups`, `setgid`, and `setuid` drop root privileges to the unprivileged target user.
- **Root Context Isolation**: The display manager daemon running as `root` does not attempt to parse or interpret untrusted user-controlled files.
