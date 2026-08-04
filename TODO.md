# LiDM: C vs. Rust Implementation Analysis & Feature Parity TODO

This document provides a detailed technical comparison between the C implementation and the Rust rewrite of **LiDM** (Lightweight Display Manager), highlighting implementation details, missing features, architectural differences, and a concrete TODO roadmap for full parity.

---

## 1. Detailed Description of the C Implementation

The C codebase of **LiDM** is a lightweight, raw-terminal display manager designed for Linux virtual terminals. It provides user/session selection, PAM authentication, environment setup, desktop/shell session execution, and VT switching using custom ANSI terminal drawing and low-level POSIX/Linux APIs.

### 1.1 Architecture & Module Breakdown

*   **`main.c` (Entry Point & Lifecycle Management)**
    *   Initializes file logging if `LIDM_LOG` environment variable is set.
    *   Parses command-line flags (`-v`/`--version`, `-h`/`--help`) or an optional VT number argument passed to `chvt_str`.
    *   Loads and parses configuration from `LIDM_CONF` or `/etc/lidm.ini`.
    *   Invokes `setup()` to configure raw terminal attributes via `termios`.
    *   Enumerates human users via `get_human_users()` and available desktop sessions via `get_avaliable_sessions()`.
    *   Registers a `SIGTERM` handler (`setup_sigterm()`) for process group cleanup.
    *   Enters the main event loop (`load()`). If `load()` returns `0` (e.g., UI refresh requested via F5), re-executes itself via `execl(argv[0], argv[0], NULL)`.

*   **`config.c` & `include/config.h` (Reflection-based Configuration Engine)**
    *   Uses C preprocessor X-macros (`BUILD`, `TABLE_*`, `INTROS_*`) to define configuration tables (`colors`, `chars`, `functions`, `strings`, `behavior`), default values (`DEFAULT_CONFIG`), and struct offset reflection arrays (`CONFIG_INSTROSPECTION`).
    *   Implements an INI file parser (`parse_config`) backed by `read_desktop`.
    *   Supports custom string unquoting, escape sequence parsing (including `\n`, `\t`, `\xNN` hex byte encoding), boolean parsing (`true`/`false`), long integers, key map lookup (`find_keyname`), and dynamic string arrays (`Vector`).

*   **`desktop.c` & `desktop_exec.c` (INI & Exec String Parser)**
    *   `read_desktop`: Streaming line-by-line INI parser that trims whitespace, ignores comment lines (`#`), parses table headers (`[Section]`), and passes `key = value` pairs to callback functions.
    *   `parse_exec_string`: Spec-compliant parser for Freedesktop Entry `Exec` key values. Handles double-quoted arguments, backslash escaping (`\\`, `\"`), and Freedesktop `%` field codes (drops codes like `%u`, `%f`, `%%`).
    *   `desktop_as_cmdline`: Formats parsed argument arrays back into a single shell command line string.

*   **`sessions.c` (Session Discovery & Execution Preparation)**
    *   Scans standard Freedesktop session directories using `ftw()` directory traversal:
        *   `/usr/share/xsessions`, `/usr/local/share/xsessions` (XORG)
        *   `/usr/share/wayland-sessions`, `/usr/local/share/wayland-sessions` (WAYLAND)
    *   Parses `.desktop` files for `Name`, `Exec`, and `TryExec` attributes.
    *   `session_exec_exec`: Executes binary session directly via `execvpe`.
    *   `session_exec_login_through_shell`: Executes session through `LOGIN_SHELL` (`bash`) with `argv[0] = "-bash"` (login shell behavior) and `-c "<cmd>"`.

*   **`users.c` (User Enumeration)**
    *   Enumerates system user accounts using POSIX `getpwent()`.
    *   Filters human users by checking if `pw_dir` starts with `/home/`.
    *   Extracts shell (`pw_shell`), username (`pw_name`), and GECOS display name (`pw_gecos`).

*   **`pam.c` (PAM Operations & Environment Assembly)**
    *   `get_pamh`: Initializes PAM context via `pam_start` using `LIDM_PAM_SERVICE` or fallback `"login"`. Performs `pam_authenticate`, `pam_acct_mgmt`, `pam_setcred(PAM_ESTABLISH_CRED)`, `pam_open_session`, and `pam_setcred(PAM_REINITIALIZE_CRED)`.
    *   `pamh_get_complete_env`: Retrieves PAM environment via `pam_getenvlist` and merges standard POSIX/XDG variables (`TERM`, `PATH`, `HOME`, `USER`, `SHELL`, `LOGNAME`, `XDG_SESSION_TYPE`).

*   **`auth.c` (Authentication, Environment Sourcing & Session Spawning)**
    *   `source_paths`: Reads shell environment scripts specified in `config.behavior.source` (system paths) and `config.behavior.user_source` (relative to `$HOME`, e.g., `.xprofile`) line by line, adding `KEY=VALUE` pairs to the session environment list.
    *   `launch`: Performs PAM authentication, resolves environment variables, sources config scripts, creates a synchronizing pipe, and forks a child process.
    *   `forked`: Drops root privileges inside child process via `chdir(pw_dir)`, `setgid(pw_gid)`, `initgroups(user, pw_gid)`, and `setuid(pw_uid)`. Inherits/sets `PATH` via `confstr(_CS_PATH)` if missing.
    *   `launch_with_xorg_server`: Spawns Xorg server process with `-displayfd` pipe and `vtN` arguments, waits for display number over pipe, appends `DISPLAY=:N` to environment, forks desktop session, and monitors both child PIDs (`waitpid(-1)`). If either Xorg or session dies, sends `SIGTERM` to the other and cleans up.
    *   Parent process teardown: Waits for child exit, then cleans up PAM credentials via `pam_setcred(PAM_DELETE_CRED)`, closes PAM session via `pam_close_session`, and ends PAM context via `pam_end`.

*   **`ui.c`, `ui_state.c`, `efield.c`, `ofield.c`, `keys.c` (Terminal UI Engine)**
    *   Raw ANSI terminal interface. Uses `termios` to disable canonical mode and echo (`ICANON | ECHO`).
    *   Captures `SIGWINCH` signals for dynamic terminal resizing.
    *   Custom UTF-8 string manipulation module (`utf8.c`) supporting character counting, backward/forward codepoint seeking, and string truncation.
    *   `efield` & `ofield`: Support editable text fields with cursor movement, backspace deletion, and option selection lists (cycling through sessions/users or custom user input).
    *   `keys.c`: Non-blocking key sequence reader using POSIX `select()` with configurable `refresh_rate` timeout.
    *   FIDO support: Allows logging in with empty password if configured via `config.functions.fido`.

*   **`launch_state.c` (State Persistence)**
    *   Reads and writes state file `/var/lib/lidm/state` containing last selected `username` and `session_opt` across restarts.

*   **`chvt.c` (Virtual Terminal Switcher)**
    *   Opens terminal console devices (`/dev/tty`, `/dev/tty0`, `/dev/vc/0`, `/dev/systty`, `/dev/console`).
    *   Queries keyboard type via `ioctl(fd, KDGKBTYPE)` and triggers VT switch via `ioctl(fd, VT_ACTIVATE, n)` and `ioctl(fd, VT_WAITACTIVE, n)`.

*   **`signal_handler.c` (Signal Management)**
    *   Sets process group (`setpgid(0, 0)`) and installs `SIGTERM` handler to send `SIGTERM` to all processes in group (`kill(-getpgrp(), SIGTERM)`) and wait for children to terminate before exiting.

*   **`log.c` & `util/path.c` & `util/vec.c` (Utilities)**
    *   `log.c`: Formatted file logger using `vfprintf`.
    *   `path.c`: Executable binary lookup in `PATH` (`search_path`) and musl-compatible `execvpe` implementation.
    *   `vec.c`: Dynamic pointer vector implementation with geometric reallocation.

---

## 2. Detailed Description of the Rust Implementation

The Rust implementation of **LiDM** is a modern rewrite using standard Rust idioms, safe concurrency patterns, rich TUI libraries (`ratatui`, `crossterm`), and ecosystem crates (`serde`, `toml`, `uzers`, `freedesktop_entry_parser`, `pam-client2`).

### 2.1 Architecture & Module Breakdown

*   **`main.rs` (Control Loop & Orchestration)**
    *   Uses `clap` CLI parser (`Args`) supporting version output, help menus, `vt` positional argument, `--log-file`, and `--conf-path` (defaulting to `/etc/lidm.ini` or `LIDM_CONF` env var).
    *   Instantiates thread-safe console ring buffer (`ConsoleBuffer = Arc<Mutex<VecDeque<String>>>`).
    *   Initializes logging framework (`logging::initialize_logging`).
    *   Invokes VT switch if `args.vt` is specified (`vt::chvt`).
    *   Enters main continuous loop:
        1. Reads and parses TOML configuration (`Config::parse`).
        2. Enumerates desktop sessions (`session::get_available_sessions`) and human users (`users::get_human_users`).
        3. Reads last launch state (`launch_state::read_launch_state`).
        4. Spawns console output interceptor (`ConsoleInterceptor::intercept`).
        5. Instantiates and runs `UI` event loop (`ui.run()`).
        6. Handles `UIResult`:
           * `UIResult::Poweroff`: Calls `libc::reboot(libc::RB_POWER_OFF)`.
           * `UIResult::Reboot`: Calls `libc::reboot(libc::RB_AUTOBOOT)`.
           * `UIResult::Refresh`: Continues loop (reloads configuration and UI).
           * `UIResult::Exit`: Breaks main loop.
           * `UIResult::Login`: Writes launch state, authenticates via `auth::authenticate`, extracts user metadata (`uzers::get_user_by_name`), sets `USER` and `HOME` environment variables, and executes session via `exec::launch_session`.

*   **`config.rs` & `colors.rs` (TOML Configuration System)**
    *   `Config`: Deserializes TOML configuration using `serde`, `toml`, and `serde_ignored` (warning on unknown fields).
    *   Sections: `colors`, `functions`, `strings`, `behavior`.
    *   `Colors` & `ThemeStyle`: Converts hex colors, named ANSI colors, and text modifiers (`bold`, `italic`, `underlined`, `reversed`, etc.) directly into `ratatui::style::Style`.

*   **`session.rs` (Session Discovery)**
    *   Scans `/usr/share/xsessions`, `/usr/local/share/xsessions`, `/usr/share/wayland-sessions`, `/usr/local/share/wayland-sessions`.
    *   Uses `freedesktop_entry_parser` to parse `.desktop` files.
    *   Splits `Exec` strings by whitespace (`exec.split_whitespace()`) into argument vectors.
    *   Classifies sessions into `SessionType::Xorg`, `SessionType::Wayland`, or `SessionType::Shell`.

*   **`users.rs` (User Enumeration)**
    *   Uses `uzers` crate (`all_users()`) to iterate Unix accounts.
    *   Filters users whose home directories start with `/home/`.
    *   Extracts username, shell path, and first field of GECOS string for display name.

*   **`auth.rs` (PAM Authentication)**
    *   Uses `pam_client2` crate.
    *   `authenticate`: Initializes `Context` with service name (`LIDM_PAM_SERVICE` or `"login"`), calls `context.authenticate(Flag::NONE)`, `context.acct_mgmt(Flag::NONE)`, `session.open_session(Flag::NONE)`, `session.reinitialize_credentials(Flag::NONE)`.
    *   Leaks PAM session (`session.leak()`) to maintain open PAM credentials across child process execution.
    *   Extracts PAM environment variables into `HashMap<String, String>`.

*   **`exec.rs` (Session Launch Execution)**
    *   `launch_session`: Routes to `launch_xorg` or `launch_direct`.
    *   `launch_direct`: Calls `nix::unistd::fork()`. Child drops privileges via `initgroups`, `setgid`, `setuid`, and calls `std::process::Command::new(&exec_args[0]).envs(env).exec()`. Parent calls `libc::waitpid`.
    *   `launch_xorg`: Creates pipe via `nix::unistd::pipe()`. Forks child to run `Xorg -displayfd <fd> vt<N>`. Parent reads display number from pipe, sets `DISPLAY=:<N>`, forks second child process for desktop session (dropping user privileges and executing binary), and runs `waitpid(-1)` loop to kill the counterpart process if either Xorg or session terminates.

*   **`console.rs` (Kernel Console Interceptor)**
    *   `ConsoleInterceptor`: Allocates a pseudoterminal via `nix::pty::openpty()`.
    *   Redirects kernel/systemd `/dev/console` output to the slave PTY using `tioccons(slave_fd, 0)` ioctl.
    *   Spawns a background thread reading from master PTY and appending console output lines to thread-safe `ConsoleBuffer` ring buffer (max 50 lines).
    *   On drop, closes master/slave FDs to restore standard console routing.

*   **`ui.rs`, `ui_adapter.rs`, `ui_state.rs` (Ratatui TUI Framework)**
    *   Built on `ratatui`, `crossterm`, and `tui_input`.
    *   `UIState`: Pure state container storing field indices, input models (`tui_input::Input`), focus state (`Field::Session`, `Field::User`, `Field::Password`), and custom input flags.
    *   `UIAdapter`: Encapsulates input events, state transitions, hotkey detection, cursor offset calculation, and string display queries.
    *   `UI`: Renders background blocks, customizable box borders (`none`, `block`, `rounded`, `plain`), hostname, date/time (via `chrono`), interactive input fields, optional console panel widget (`show_console`), and hotkey footer.

*   **`logging.rs` (Multi-Writer Logger)**
    *   Configures `env_logger` using `LIDM_LOGLEVEL` filter.
    *   Sends log records to both file (`/tmp/lidm.log` or `--log-file`) and the shared `ConsoleBuffer` for live display in the UI.

*   **`launch_state.rs` & `vt.rs` (State & VT Switching)**
    *   `launch_state.rs`: Simple string IO for `/var/lib/lidm/state`.
    *   `vt.rs`: Uses `nix::fcntl::open` and `nix::ioctl_write_int_bad!` macros to check keyboard type (`KDGKBTYPE`) and switch virtual terminals (`VT_ACTIVATE`, `VT_WAITACTIVE`).

---

## 3. Comparison Summary Matrix

| Feature / Aspect | C Implementation | Rust Implementation | Status in Rust |
| :--- | :--- | :--- | :--- |
| **Shell Script Sourcing (`source`, `user_source`)** | Supported (`source_paths` reads `/etc/profile`, `~/.xprofile`, etc.) | Config fields parsed but **unused in code** | ❌ **Missing in Rust** |
| **Login Shell Wrapping (`bypass_shell_login`)** | Supported (runs sessions via `bash -c` with `-bash` arg0 when `false`) | Config field parsed but **unused in code** | ❌ **Missing in Rust** |
| **FIDO Hotkey / Passwordless Login** | Supported (`config.functions.fido` triggers empty pass auth) | Field and handling absent | ❌ **Missing in Rust** |
| **PAM Session Teardown (`pam_close_session`)** | Supported (cleans PAM creds and closes session after `waitpid`) | `session.leak()` used, session **never closed** | ❌ **Missing in Rust** |
| **Freedesktop `Exec` String Parsing** | Full spec parser (`parse_exec_string` handles quotes, escapes, `%` codes) | Basic `split_whitespace()` | ⚠️ **Incomplete in Rust** |
| **Config Format & Box Characters** | Custom INI format + custom drawing chars (`table_chars`) | TOML format + Ratatui border styles | 🔄 **Different Paradigm** |
| **Process Re-execution on Refresh (F5)** | Re-executes self via `execl()` | Re-loops internally in `main.rs` | 🔄 **Different Paradigm** |
| **Process Group Signal Cleanup** | Creates process group + `SIGTERM` forwarder (`signal_handler.c`) | Standard `waitpid` loop | ⚠️ **Incomplete in Rust** |
| **Kernel Console Interceptor (`show_console`)** | Not implemented | Implemented via PTY `TIOCCONS` + TUI Widget | 🚀 **New in Rust** |
| **Terminal UI Framework** | Hand-rolled ANSI escape codes + `select()` + custom UTF-8 | `ratatui` + `crossterm` + `tui_input` | 🚀 **Superior in Rust** |
| **CLI Argument Parsing** | Custom manual check (`-v`, `-h`, positional VT) | `clap` derive parser | 🚀 **Superior in Rust** |
| **Structured Logging** | Basic file logger (`LIDM_LOG`) | `env_logger` + file + live TUI buffer | 🚀 **Superior in Rust** |

---

## 4. Detailed Missing Features & Technical Differences

### 4.1 Missing Features in Rust

1.  **Environment File Sourcing & Session Environment Architecture:**
    *   In C, `source_paths()` reads system scripts (`/etc/profile`, etc.) and user home scripts (`~/.xprofile`, etc.) line-by-line for `KEY=VALUE` environment variables before launching a session.
    *   In Rust, manual line-by-line shell script parsing is replaced with standard DM architecture: the DM initializes PAM & core system environment variables (`pam_getenvlist()`, `USER`, `HOME`, `SHELL`, `PATH`, `DISPLAY`, `XDG_*`), drops root privileges, and executes the session through a login shell (`$SHELL -l -c "<session_exec>"`) or a session wrapper script (e.g. `/etc/lidm/Xsession`), allowing user shell scripts (`/etc/profile`, `~/.profile`, `~/.xprofile`) to be evaluated naturally by the shell.

2.  **Login Shell Execution (`bypass_shell_login`):**
    *   In C, if `bypass_shell_login` is `false` (default), sessions run inside `bash -c "<exec_cmd>"` with `argv[0] = "-bash"`, causing `bash` to run as a login shell and initialize user environment files.
    *   In Rust, `exec.rs` always executes the target binary directly via `std::process::Command::new()`, ignoring `bypass_shell_login`.

3.  **PAM Session Closure & Teardown:**
    *   In C, after `waitpid` detects child session termination, the parent process calls `pam_setcred(PAM_DELETE_CRED)`, `pam_close_session()`, and `pam_end()`.
    *   In Rust, `auth.rs` calls `session.leak()`. No post-session handler exists to close the PAM session, leaving logind/UTMP/PAM credentials open.

4.  **FIDO / Passwordless Hotkey:**
    *   In C, pressing the configured `fido` function key attempts login with an empty password.
    *   In Rust, `Functions` struct lacks `fido` and `UIAdapter` does not check for a FIDO hotkey.

5.  **Freedesktop `Exec` String Parsing:**
    *   In C, `parse_exec_string()` handles quoted arguments with spaces and strips `%` field codes (e.g. `%f`, `%u`, `%%`).
    *   In Rust, `session.rs` performs naive `exec.split_whitespace()`, breaking on quotes or passing unhandled `%` specifiers.

---

## 5. Actionable Roadmap / TODO List for Rust Parity

- [x] **[P0] Session Environment & Login Shell Architecture (`bypass_shell_login` & Wrapper)**
  - Collect PAM environment variables via `pam_getenvlist()` / `pam-client2` and merge with standard POSIX/XDG variables (`USER`, `HOME`, `SHELL`, `PATH`, `DISPLAY`, `XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`).
  - Pass merged environment to child process after dropping privileges (`setuid`/`setgid`).
  - Implement session execution via login shell / session wrapper (`$SHELL -l -c "<cmd>"` or `/etc/lidm/Xsession`) when `config.behavior.bypass_shell_login` is `false`, allowing user shell scripts (`/etc/profile`, `~/.profile`, `~/.xprofile`) to be evaluated naturally by the user shell rather than manually parsing `KEY=VALUE` lines.

- [ ] **[P0] PAM Session Teardown**
  - Modify `auth.rs` / `exec.rs` to handle PAM session cleanup after `waitpid` finishes, ensuring `pam_close_session` and `pam_setcred(DELETE)` are invoked.

- [ ] **[P1] Freedesktop `Exec` Parsing**
  - Replace `exec.split_whitespace()` in `session.rs` with `shell-words` or custom parser handling quotes and `%` field codes.

- [ ] **[P1] FIDO Hotkey Support**
  - Add `fido: Option<String>` to `Functions` in `config.rs`.
  - Implement FIDO key detection in `UIAdapter` and trigger empty password login flow.

- [ ] **[P2] Process Group & Signal Teardown**
  - Implement process group creation (`setpgid`) and `SIGTERM` cleanup handling for spawned session processes.
