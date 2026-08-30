# Domain Models & Struct Parameters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor sprawling parameter lists in `handle_login`, `assemble_environment`, `launch_session`, and `UI::new`/`UIAdapter::new` into cohesive domain models and struct parameters (`LoginRequest`, `LoginContext`, `EnvironmentOptions`, `LaunchContext`, `UIContext`).

**Architecture:** Introduce domain model structs that group related primitives and context objects, update function signatures and call sites across `src/exec.rs`, `src/ui_state.rs`, `src/ui_adapter.rs`, `src/ui.rs`, and `src/main.rs`, and update tests to leverage ergonomic struct construction.

**Tech Stack:** Rust (2024 edition), `ratatui`, `tui-input`, `nix`, `pam-client2`.

## Global Constraints
- All existing functionality and behavior must be preserved verbatim.
- Zero compile warnings (`cargo check`), zero lint issues (`cargo clippy`), and all unit tests must pass (`cargo test`).

---

### Task 1: Refactor `src/exec.rs` with `EnvironmentOptions` and `LaunchContext`

**Files:**
- Modify: `src/exec.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct EnvironmentOptions<'a> {
      pub pam_env: &'a HashMap<String, String>,
      pub username: &'a str,
      pub home_dir: &'a str,
      pub shell: &'a str,
      pub session_type: &'a str,
      pub display: Option<&'a str>,
      pub desktop_names: Option<&'a str>,
      pub system_sources: &'a [String],
      pub user_sources: &'a [String],
  }
  pub fn assemble_environment(opts: &EnvironmentOptions) -> HashMap<String, String>;

  pub struct LaunchContext<'a> {
      pub user: &'a str,
      pub uid: u32,
      pub gid: u32,
      pub env: &'a HashMap<String, String>,
      pub exec_args: &'a [String],
      pub is_xorg: bool,
      pub vt: Option<c_int>,
      pub user_shell: &'a str,
      pub bypass_shell_login: bool,
  }
  pub fn launch_session(ctx: &LaunchContext) -> Result<(), String>;
  ```

- [ ] **Step 1: Define `EnvironmentOptions` and `LaunchContext` in `src/exec.rs` and update function signatures**
- [ ] **Step 2: Update `launch_direct` and `launch_xorg` to accept `&LaunchContext`**
- [ ] **Step 3: Update unit tests in `src/exec.rs` to construct `EnvironmentOptions`**
- [ ] **Step 4: Run `cargo test --lib exec` to verify tests pass**

---

### Task 2: Refactor `src/ui_state.rs`, `src/ui_adapter.rs`, and `src/ui.rs` with `LoginRequest` and `UIContext`

**Files:**
- Modify: `src/ui_state.rs`
- Modify: `src/ui_adapter.rs`
- Modify: `src/ui.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub struct LoginRequest {
      pub session_idx: usize,
      pub user_idx: usize,
      pub password: String,
      pub custom_session: String,
      pub custom_user: String,
  }

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

  pub enum UIResult {
      Login(LoginRequest),
      Poweroff,
      Reboot,
      Exit,
  }
  ```

- [ ] **Step 1: Add `LoginRequest` to `src/ui_state.rs` and update `fido_login_request`**
- [ ] **Step 2: Define `UIContext` and update `UIAdapter::new(ctx: UIContext)` in `src/ui_adapter.rs`, adding `login_request(&self) -> LoginRequest` and `fido_login_request(&self) -> LoginRequest`**
- [ ] **Step 3: Update `UIResult::Login(LoginRequest)` and `UI::new(ctx: UIContext)` in `src/ui.rs`**
- [ ] **Step 4: Update unit tests in `src/ui_adapter.rs` and `src/ui.rs` using `UIContext { config, ..Default::default() }`**
- [ ] **Step 5: Run `cargo test --lib ui` and `cargo test --lib ui_adapter` to verify**

---

### Task 3: Refactor `src/main.rs` with `LoginContext`, `LoginRequest`, and Call Sites

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct LoginContext<'a> {
      pub config: &'a config::Config,
      pub sessions: &'a [session::Session],
      pub users: &'a [users::LocalUser],
      pub vt: Option<c_int>,
      pub bypass_shell_login: bool,
  }
  pub fn handle_login(request: &LoginRequest, ctx: &LoginContext) -> Result<(), auth::AuthError>;
  ```

- [ ] **Step 1: Define `LoginContext` in `src/main.rs`**
- [ ] **Step 2: Update `handle_login(request: &LoginRequest, ctx: &LoginContext)` implementation to use `EnvironmentOptions` and `LaunchContext`**
- [ ] **Step 3: Update `main()` loop to initialize `UIContext` and call `handle_login(&request, &login_ctx)` on `UIResult::Login(request)`**
- [ ] **Step 4: Update any tests in `src/main.rs` if needed**
- [ ] **Step 5: Run `cargo test` across the entire workspace**

---

### Task 4: Verification and Quality Checks

- [ ] **Step 1: Run `cargo test` and ensure all 60+ unit and integration tests pass**
- [ ] **Step 2: Run `cargo clippy --all-targets --all-features` to ensure no lint warnings**
- [ ] **Step 3: Run `cargo check` in release mode to verify clean build**

