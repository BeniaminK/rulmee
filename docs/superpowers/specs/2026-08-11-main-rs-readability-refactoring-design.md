# Refactoring `src/main.rs` for Readability and Maintainability Design

## Objective
Refactor `src/main.rs` in `lidm` to replace the monolithic 200+ line `main()` loop with modular helper functions, guard clauses, and system abstraction helpers. This eliminates deep nesting (the Arrow anti-pattern), isolates user/session resolution, encapsulates raw `unsafe` libc reboot calls in `src/sys.rs`, and improves code clarity.

## Architecture & Components

### 1. System Operations Abstraction (`src/sys.rs`)
Encapsulate low-level system call logic for shutdown and reboot:
- `pub fn poweroff() -> !`: Invokes `unsafe { libc::reboot(libc::RB_POWER_OFF); }` and exits with status 0.
- `pub fn reboot() -> !`: Invokes `unsafe { libc::reboot(libc::RB_AUTOBOOT); }` and exits with status 0.

### 2. Domain Data Structures in `src/main.rs`
Introduce lightweight helper structs to hold resolved user and session details:
- `struct UserSelection`: Contains `username: String` and `shell: String`.
- `struct SessionSelection`: Contains `name: String`, `exec_args: Vec<String>`, `is_xorg: bool`, and `desktop_names: Option<Vec<String>>`.
- `struct LoginRequest`: Holds login parameters extracted from `UIResult::Login`.

### 3. Resolution Helper Functions
- `fn resolve_user(users: &[users::HumanUser], user_idx: usize, custom_user: String) -> UserSelection`
  - If `user_idx < users.len()` and `custom_user` is empty, returns chosen user's username and shell.
  - Otherwise, returns `custom_user` and default shell (`"/bin/bash"`).

- `fn resolve_session(sessions: &[session::Session], session_idx: usize, custom_session: String, shell: &str) -> SessionSelection`
  - If `session_idx < sessions.len()` and `custom_session` is empty, extracts session name, exec args, type (Xorg vs Wayland), and desktop names.
  - If `session_idx == sessions.len()` (custom shell option) and `custom_session` is empty, returns shell as session name and exec args.
  - Otherwise, uses `custom_session`.

### 4. Login Execution Handler
- `fn handle_login(...) -> Result<(), auth::AuthError>`
  - Resolves user and session using `resolve_user` and `resolve_session`.
  - Persists launch state using `launch_state::write_launch_state`.
  - Performs authentication via `auth::authenticate`.
  - Uses guard clause `let Some(u) = uzers::get_user_by_name(&username) else { ... }` to fetch POSIX user metadata cleanly without nesting.
  - Assembles environment and launches session via `exec::launch_session`.
  - Closes PAM session on completion (`auth_session.close()`).

### 5. Streamlined `fn main()`
The main loop in `src/main.rs` will be reduced to ~40 lines:
- Initializes config, logging, and VT.
- Runs UI via `ui.run()`.
- Uses clean pattern matching on `UIResult` (`Login`, `Poweroff`, `Reboot`, `Refresh`, `Exit`).

## Verification Plan
- Build project using `cargo check` and `cargo build`.
- Run tests (`cargo test`) to ensure no regressions.
- Verify CLI `copy-config` subcommand still works.
