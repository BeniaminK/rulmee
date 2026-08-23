# LiDM: C vs. Rust Implementation Analysis & Feature Parity TODO

[ ] Logging - distribute logging to stdout and to files or just to file but in a better - more convenient way (for stdout because of the coloring...)


This document provides a detailed technical comparison between the C implementation and the Rust rewrite of **LiDM** (Lightweight Display Manager), highlighting implementation details, missing features, architectural differences, and a concrete TODO roadmap for full parity.

> **Current Parity Progress (Updated 2026-08-08):**
> - [x] **[P0] Session Environment & Login Shell Architecture** (Completed — `exec.rs` delegates to `$SHELL -l -c`)
> - [x] **[P0] PAM Session Teardown** (Completed — `AuthSession` RAII `close()`/`Drop` executes `pam_close_session`)
> - [x] **[P1] Freedesktop `Exec` Parsing** (Completed — `parse_exec_string` handles quotes & `%` specifiers)
> - [x] **[P2] Process Group & Signal Teardown** (Completed — `setpgid` process group isolation & `SIGTERM` cleanup handler)
> - [x] **[P1] FIDO2 / Passwordless Authentication Hotkey (`fido`)** (Completed — `fido` key handling in `UIAdapter`, `UIState`, and `config.rs`)
> - [x] **[P1] Interactive PAM Message Callbacks (`PAM_TEXT_INFO` / `PAM_ERROR_MSG`)** (Completed — custom PAM conversation handler in `auth.rs` captures messages for `UIState` and TUI rendering)
> - [x] **[P1] Visual Error Feedback on Auth Failure (`e_badpasswd`)** (Completed — apply `e_badpasswd` style on auth failure in TUI)
> - [x] **[P2] Environment Profile Sourcing (`source` and `user_source`)** (Completed — process `config.behavior.source` & `user_source` environment script files)
> - [x] **[P2] Custom UI Box Border Characters (`[chars]` Config Table)** (Completed — implement `[chars]` config table and border rendering)
> - [x] **[P2] Configured Session Type Strings (`s_wayland`, `s_xorg`, `s_shell` in `strings`)** (Completed — reference `strings` labels during UI rendering)
> - [x] **[P2] Configurable Refresh Rate (`refresh_rate`)** (Completed — use `config.behavior.refresh_rate` in UI event loop polling)
> - [x] **[P2] Hostname Truncation (`ellipsis`)** (Completed — hostnames exceeding width are truncated with `config.strings.ellipsis`)
> - [x] **[P2] Build Metadata in Version Flag (`-v` / `--version`)** (Completed — include Git revision, build timestamp, and compiler info)
> - [x] **[P2] Log File Path Environment Variable (`LIDM_LOG`)** (Completed — support `LIDM_LOG` env var for custom log file path)

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
    *   `pam_conversation`: Captures `PAM_TEXT_INFO` and `PAM_ERROR_MSG` strings and forwards them to `print_pam_msg()` for UI display.

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

*   **`auth.rs` (PAM Authentication & RAII Teardown)**
    *   Uses `pam_client2` crate.
    *   `authenticate`: Initializes `Context` with service name (`LIDM_PAM_SERVICE` or `"login"`), calls `context.authenticate(Flag::NONE)`, `context.acct_mgmt(Flag::NONE)`, `session.open_session(Flag::NONE)`, `session.reinitialize_credentials(Flag::NONE)`.
    *   `session.leak()` converts the session into a `SessionToken` stored inside `AuthSession` alongside the `Context`.
    *   Extracts PAM environment variables into `HashMap<String, String>`.
    *   `AuthSession::close()` / `Drop`: Reclaims session via `context.unleak_session(token)` and calls `session.close(Flag::NONE)`, ensuring `pam_close_session()` and `pam_setcred(PAM_DELETE_CRED)` are executed upon session exit.

*   **`exec.rs` (Session Launch Execution & Environment Assembly)**
    *   `assemble_environment`: Merges PAM environment variables (`pam_getenvlist()`) with standard POSIX/XDG environment variables (`USER`, `LOGNAME`, `HOME`, `SHELL`, `PATH`, `XDG_SESSION_TYPE`, `XDG_SESSION_CLASS`, `DISPLAY`).
    *   `build_exec_command`: Builds command line vector. If `bypass_shell_login` is `false`, wraps command in `$SHELL -l -c "exec <quoted_args>"`, causing shell startup files (`/etc/profile`, `~/.profile`, `~/.xprofile`) to be evaluated naturally. If `true`, executes target binary directly.
    *   `launch_session`: Routes execution to `launch_xorg` or `launch_direct` with user shell and `bypass_shell_login` configuration.
    *   `launch_direct`: Calls `nix::unistd::fork()`. Child drops privileges (`initgroups`, `setgid`, `setuid`), invokes `build_exec_command`, and executes with assembled environment (`cmd.exec()`). Parent waits via `libc::waitpid`.
    *   `launch_xorg`: Creates pipe via `nix::unistd::pipe()`. Forks child to run `Xorg -displayfd <fd> vt<N>`. Parent reads display number from pipe, sets `DISPLAY=:<N>`, forks second child process for desktop session (dropping user privileges and running `build_exec_command`), and runs `waitpid(-1)` loop to kill the counterpart process if either Xorg or session terminates.

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
| **Shell Script Sourcing (`source`, `user_source`)** | Supported (`source_paths` reads `/etc/profile`, `~/.xprofile`, etc.) | Declared in `Config` struct but not read or processed | ❌ **Pending in Rust** |
| **Login Shell Wrapping (`bypass_shell_login`)** | Supported (runs sessions via `bash -c` with `-bash` arg0 when `false`) | Implemented in `exec.rs` (`build_exec_command` runs `$SHELL -l -c` when `false`) | ✅ **Implemented in Rust** |
| **FIDO Hotkey / Passwordless Login** | Supported (`config.functions.fido` triggers empty pass auth) | `Functions` struct lacks `fido`, `UIAdapter` lacks handler | ❌ **Pending in Rust** |
| **PAM Session Teardown (`pam_close_session`)** | Supported (cleans PAM creds and closes session after `waitpid`) | `AuthSession` closes session & deletes creds via `close()`/`Drop` | ✅ **Implemented in Rust** |
| **Interactive PAM Message Callbacks** | Supported (`pam_conversation` handles `PAM_TEXT_INFO` / `PAM_ERROR_MSG`) | Uses mock conversation, discards text info/error messages | ❌ **Pending in Rust** |
| **Visual Error Feedback on Auth Failure** | Supported (`print_passwd(..., true)` applies `e_badpasswd` style) | `e_badpasswd` defined in `colors.rs` but unused in TUI | ❌ **Pending in Rust** |
| **Freedesktop `Exec` String Parsing** | Full spec parser (`parse_exec_string` handles quotes, escapes, `%` codes) | Handled via Freedesktop parser / spec compliance | ✅ **Implemented in Rust** |
| **Custom UI Box Border Characters (`[chars]`)** | Supported (`[chars]` section: `hb`, `vb`, `ctl`, `ctr`, `cbl`, `cbr`) | Fixed Ratatui border presets (`border`, `block`, `rounded`, `none`) | ❌ **Pending in Rust** |
| **Configured Session Type Strings** | Supported (`strings.s_wayland`, `s_xorg`, `s_shell`) | Declared in `Strings` struct but unused in UI rendering | ❌ **Pending in Rust** |
| **Configurable Refresh Rate (`refresh_rate`)** | Supported (`config.behavior.refresh_rate` sets poll timeout) | `Config` contains `refresh_rate`, but `ui.rs` hardcodes 250ms | ❌ **Pending in Rust** |
| **Hostname Truncation (`ellipsis`)** | Supported (`trunc_gethostname` appends `config.strings.ellipsis`) | Hostname not truncated using `ellipsis` | ❌ **Pending in Rust** |
| **Build Metadata in Version Flag (`-v`)** | Supported (outputs Git rev, build timestamp, compiler info) | Standard `clap` version output without build metadata | ❌ **Pending in Rust** |
| **Log File Path Env Var (`LIDM_LOG`)** | Supported (`LIDM_LOG` sets custom log file path) | `LIDM_LOG` checked before CLI arg or `/tmp/lidm.log` | ✅ **Implemented in Rust** |
| **Process Re-execution on Refresh (F5)** | Re-executes self via `execl()` | Re-loops internally in `main.rs` | 🔄 **Different Paradigm** |
| **Process Group Signal Cleanup** | Creates process group + `SIGTERM` forwarder (`signal_handler.c`) | Process group isolation & `SIGTERM` cleanup implemented | ✅ **Implemented in Rust** |
| **Kernel Console Interceptor (`show_console`)** | Not implemented | Implemented via PTY `TIOCCONS` + TUI Widget | 🚀 **New in Rust** |
| **Terminal UI Framework** | Hand-rolled ANSI escape codes + `select()` + custom UTF-8 | `ratatui` + `crossterm` + `tui_input` | 🚀 **Superior in Rust** |
| **CLI Argument Parsing** | Custom manual check (`-v`, `-h`, positional VT) | `clap` derive parser | 🚀 **Superior in Rust** |
| **Structured Logging** | Basic file logger (`LIDM_LOG`) | `env_logger` + file + live TUI buffer | 🚀 **Superior in Rust** |

---

## 4. Detailed Missing Features & Technical Differences

### 4.1 Completed Implementations

1. **Login Shell Execution (`bypass_shell_login`):**
   * Implemented in Rust in `exec.rs` via `build_exec_command()`. When `false`, it executes via `$SHELL -l -c "exec <quoted_args>"`; when `true`, it executes the target binary arguments directly.

2. **PAM Session Closure & Teardown:**
   * Implemented in Rust via `AuthSession` RAII lifecycle management (`pam_close_session` and `pam_setcred(PAM_DELETE_CRED)`).

3. **Freedesktop `Exec` String Parsing:**
   * Implemented spec-compliant parser handling double-quoted arguments and `%` field codes.

4. **Process Group & Signal Teardown:**
   * Implemented process group creation (`setpgid`) and `SIGTERM` cleanup handling for spawned session processes.

### 4.2 Pending Implementation Gaps (Actionable TODO Items)

1. **Environment Profile Sourcing (`source` and `user_source`):**
   * **C**: `src/auth.c` defines `source_paths()` which parses and sources environment files listed in `config.behavior.source` (system-wide) and `config.behavior.user_source` (relative to user home, e.g. `~/.profile`, `~/.xprofile`). `KEY=VALUE` pairs are injected into the session environment.
   * **Rust**: `src/config.rs` declares `pub source: Vec<String>` and `pub user_source: Vec<String>` in `Behavior`, but these fields are never read or processed in the Rust codebase.

2. **FIDO2 / Passwordless Authentication Hotkey (`fido`):**
   * **C**: `include/config.h` and `src/ui.c` define `functions.fido` and `strings.f_fido`. Pressing the FIDO hotkey triggers authentication with an empty password `""` (for FIDO2/passwordless PAM modules) and displays the shortcut in the UI footer.
   * **Rust**: `src/config.rs` has no `fido` field in `Functions` or `f_fido` in `Strings`. No FIDO / passwordless shortcut handler in the Rust UI or key event adapter.

3. **Custom UI Box Border Characters (`[chars]` Config Table):**
   * **C**: `include/config.h` defines a `[chars]` section (`hb`, `vb`, `ctl`, `ctr`, `cbl`, `cbr`) allowing custom character strings for box borders.
   * **Rust**: `src/config.rs` lacks a `[chars]` table. Border rendering in `src/ui.rs` is restricted to preset styles (`box_type = "border"`, `"block"`, `"rounded"`, `"none"`).

4. **Interactive PAM Message Callbacks (`PAM_TEXT_INFO` / `PAM_ERROR_MSG`):**
   * **C**: `src/pam.c` implements `pam_conversation()`, capturing `PAM_TEXT_INFO` and `PAM_ERROR_MSG` strings and forwarding them to `print_pam_msg()` to display PAM module feedback on screen.
   * **Rust**: `src/auth.rs` uses `pam_client2::conv_mock::Conversation::with_credentials()`, which discards PAM text info and error messages.

5. **Visual Error Feedback on Auth Failure (`e_badpasswd`):**
   * **C**: `src/ui.c` calls `print_passwd(..., true)` on authentication failure, applying the `e_badpasswd` style (red, italic, underlined) to highlight the password field.
   * **Rust**: `src/colors.rs` defines `e_badpasswd`, but it is never used in rendering or UI state. On auth failure, `src/main.rs` prints to stderr and resets without visual error feedback on the TUI.

6. **Configured Session Type Strings (`s_wayland`, `s_xorg`, `s_shell` in `strings`):**
   * **C**: `include/config.h` and `src/ui.c` use `strings.s_wayland`, `strings.s_xorg`, and `strings.s_shell` to render custom labels for session types.
   * **Rust**: `src/config.rs` defines these fields in `Strings`, but they are never referenced during UI rendering in `src/ui.rs` or `src/ui_adapter.rs`.

7. **Configurable Refresh Rate (`refresh_rate`):**
   * **C**: `include/config.h` and `src/ui.c` use `config.behavior.refresh_rate` to set the event loop sleep/poll interval.
   * **Rust**: `src/config.rs` defines `refresh_rate`, but `src/ui.rs` hardcodes a `250ms` polling duration (`Duration::from_millis(250)`), ignoring the configured `refresh_rate`.

8. **Hostname Truncation (`ellipsis`):**
   * **C**: `src/ui.c` truncates long hostnames using `trunc_gethostname()` and appends `config.strings.ellipsis`.
   * **Rust**: `src/config.rs` defines `ellipsis`, but `src/ui.rs` never truncates hostnames or uses `ellipsis`.

9. **Build Metadata in Version Flag (`-v` / `--version`):**
   * **C**: `src/main.c` outputs Git commit revision (`LIDM_GIT_REV`), build date (`LIDM_BUILD_TS`), and compiler version (`COMPILER_VERSION`).
   * **Rust**: `src/main.rs` uses standard `clap` version output which prints `lidm 1.0.0` without build or compiler metadata.

10. **Log File Path Environment Variable (`LIDM_LOG`):**
    * **C**: `src/main.c` reads `LIDM_LOG` environment variable to set a custom log file path.
    * **Rust**: `src/logging.rs` accepts log file path as CLI argument or defaults to `/tmp/lidm.log`; `LIDM_LOG` env var is ignored.

---

## 5. Actionable Roadmap / TODO List for Rust Parity

### Completed Items
- [x] **[P0] Session Environment & Login Shell Architecture (`bypass_shell_login` & Wrapper)**
- [x] **[P0] PAM Session Teardown**
- [x] **[P1] Freedesktop `Exec` Parsing**
- [x] **[P2] Process Group & Signal Teardown**
- [x] **[P2] Log File Path Environment Variable (`LIDM_LOG`)**
- [x] **Item 1: [P1] FIDO2 / Passwordless Authentication Hotkey (`fido`)**

### Pending Items (To Be Implemented)

- [x] **Item 2: [P1] Interactive PAM Message Callbacks (`PAM_TEXT_INFO` / `PAM_ERROR_MSG`)**
  - Implement a custom PAM conversation callback in `auth.rs` (`CapturingConversation`).
  - Capture `PAM_TEXT_INFO` and `PAM_ERROR_MSG` prompt messages emitted during authentication.
  - Expose PAM messages to `UIState` and `UIAdapter` to render feedback banners/messages in `ui.rs`.

- [x] **Item 3: [P1] Visual Error Feedback on Auth Failure (`e_badpasswd`)**
  - Update `UIState` to track authentication failure status.
  - Apply `colors.e_badpasswd` style to password input field in `ui.rs` when authentication fails.
  - Reset error state on next keypress or field change.

- [x] **Item 4: [P2] Environment Profile Sourcing (`source` and `user_source`)**
  - Read `config.behavior.source` (system paths) and `config.behavior.user_source` (relative to home).
  - Parse environment script files for `KEY=VALUE` pairs or source them before launching non-login sessions.
  - Merge variables into session environment in `exec.rs`.

- [x] **Item 5: [P2] Custom UI Box Border Characters (`[chars]` Config Table)**
  - Add `[chars]` section (`hb`, `vb`, `ctl`, `ctr`, `cbl`, `cbr`) to `Config` struct in `config.rs`.
  - Extend `ui.rs` border rendering to construct custom border symbols from `[chars]` configuration when specified.

- [x] **Item 6: [P2] Configured Session Type Strings (`s_wayland`, `s_xorg`, `s_shell` in `strings`)**
  - Read `strings.s_wayland`, `strings.s_xorg`, and `strings.s_shell` from `Config`.
  - Use these configured strings when displaying session types in `UIAdapter` and `ui.rs`.

- [x] **Item 7: [P2] Configurable Refresh Rate (`refresh_rate`)**
  - Read `config.behavior.refresh_rate` in `ui.rs`.
  - Replace hardcoded `Duration::from_millis(250)` event polling interval with configured `refresh_rate` duration.

- [x] **Item 8: [P2] Hostname Truncation (`ellipsis`)**
  - Read `config.strings.ellipsis` in `ui.rs`.
  - Truncate long hostname strings in header rendering using `ellipsis` when hostname exceeds container width.

- [x] **Item 9: [P2] Build Metadata in Version Flag (`-v` / `--version`)**
  - Add `build.rs` script to pass Git revision, build timestamp, and Rust compiler version via `env!` / `option_env!`.
  - Update `clap` version formatting in `main.rs` to print full build metadata.

- [x] **Item 10: [P2] Log File Path Environment Variable (`LIDM_LOG`)**
  - Update `logging.rs` / `main.rs` to check `LIDM_LOG` environment variable.
  - Fall back to `--log-file` argument or `/tmp/lidm.log` if `LIDM_LOG` is not set.
