# Design Spec: Domain Models & Struct Parameters Refactoring

## Overview
This refactoring replaces sprawling positional parameter lists and loose primitive arguments across the codebase with cohesive context structs and domain models. This adheres to Rust best practices, reduces cognitive overhead, prevents argument order bugs, and makes unit test construction ergonomic.

## Components & Proposed Models

### 1. `src/exec.rs`: Environment Assembly & Session Launch

#### `EnvironmentOptions`
```rust
#[derive(Debug, Clone)]
pub struct EnvironmentOptions<'a> {
    pub pam_env: &'a std::collections::HashMap<String, String>,
    pub username: &'a str,
    pub home_dir: &'a str,
    pub shell: &'a str,
    pub session_type: &'a str,
    pub display: Option<&'a str>,
    pub desktop_names: Option<&'a str>,
    pub system_sources: &'a [String],
    pub user_sources: &'a [String],
}
```
- Replaces the 9-argument signature of `assemble_environment`.
- Updated signature:
  `pub fn assemble_environment(opts: &EnvironmentOptions) -> std::collections::HashMap<String, String>`

#### `LaunchContext`
```rust
#[derive(Debug)]
pub struct LaunchContext<'a> {
    pub user: &'a str,
    pub uid: u32,
    pub gid: u32,
    pub env: &'a std::collections::HashMap<String, String>,
    pub exec_args: &'a [String],
    pub is_xorg: bool,
    pub vt: Option<std::ffi::c_int>,
    pub user_shell: &'a str,
    pub bypass_shell_login: bool,
}
```
- Replaces the 9-argument `launch_session`, 7-argument `launch_direct`, and 8-argument `launch_xorg`.
- Updated signatures:
  - `pub fn launch_session(ctx: &LaunchContext) -> Result<(), String>`
  - `fn launch_direct(ctx: &LaunchContext) -> Result<(), String>`
  - `fn launch_xorg(ctx: &LaunchContext) -> Result<(), String>`

---

### 2. `src/ui.rs`, `src/ui_adapter.rs`, `src/ui_state.rs`: UI Context & Login Payloads

#### `UIContext`
```rust
#[derive(Default)]
pub struct UIContext<'a> {
    pub config: Config,
    pub sessions: Vec<Session>,
    pub users: Vec<LocalUser>,
    pub initial_user: Option<&'a str>,
    pub initial_session: Option<&'a str>,
    pub console_buffer: Option<ConsoleBuffer>,
    pub pam_messages: Vec<crate::auth::PamMessage>,
    pub auth_error: bool,
}
```
- Replaces 8-argument constructor signatures in `UI::new` and `UIAdapter::new`.
- Updated signatures:
  - `UI::new(ctx: UIContext) -> Self`
  - `UIAdapter::new(ctx: UIContext) -> Self`
- Unit tests can use `UIContext { config, sessions, ..Default::default() }`.

#### `LoginRequest`
```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginRequest {
    pub session_idx: usize,
    pub user_idx: usize,
    pub password: String,
    pub custom_session: String,
    pub custom_user: String,
}
```
- Replaces 5-tuple in `UIResult::Login(LoginRequest)`.
- Updates `UIAdapter::login_request(&self) -> LoginRequest` (renamed from `login_data`).
- Updates `UIAdapter::fido_login_request(&self) -> LoginRequest` (renamed from `fido_login_data`).
- Updates `UIState::fido_login_request(&self) -> LoginRequest` (replacing `fido_login_tuple`).

---

### 3. `src/main.rs`: Login Handling Context

#### `LoginContext`
```rust
pub struct LoginContext<'a> {
    pub config: &'a config::Config,
    pub sessions: &'a [session::Session],
    pub users: &'a [users::LocalUser],
    pub vt: Option<std::ffi::c_int>,
    pub bypass_shell_login: bool,
}
```
- Replaces the 10 loose positional arguments of `handle_login`.
- Updated signature:
  `pub fn handle_login(request: &LoginRequest, ctx: &LoginContext) -> Result<(), auth::AuthError>`

---

## Verification Plan
1. **Compilation**: Run `cargo check` to verify zero compile or type errors across the crate.
2. **Automated Unit & Integration Tests**: Run `cargo test` to ensure all existing and updated tests pass seamlessly.
3. **Clippy Checks**: Run `cargo clippy` to ensure idiom adherence with no warnings.

