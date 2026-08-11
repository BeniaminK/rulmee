# Refactoring `src/main.rs` Readability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `src/main.rs` to break down the 215-line `main()` loop into clean helper functions and guard clauses, and encapsulate `unsafe` reboot calls in `src/sys.rs`.

**Architecture:** Encapsulate low-level `libc::reboot` calls in `src/sys.rs`. Extract user/session selection resolution into `resolve_user` and `resolve_session` helpers in `src/main.rs`. Extract login processing into `handle_login`. Refactor `fn main()` to be a short, readable event loop.

**Tech Stack:** Rust (edition 2021), clap, libc, uzers.

## Global Constraints
- Must maintain existing authentication and session launch functionality without breaking changes.
- Must compile without warnings or errors (`cargo check`).
- Must pass all tests (`cargo test`).

---

### Task 1: Add system reboot and poweroff functions in `src/sys.rs`

**Files:**
- Modify: `src/sys.rs`

**Interfaces:**
- Consumes: `libc::reboot`, `libc::RB_POWER_OFF`, `libc::RB_AUTOBOOT`
- Produces: `pub fn poweroff() -> !`, `pub fn reboot() -> !`

- [ ] **Step 1: Write `src/sys.rs` implementation**

Write `src/sys.rs`:
```rust
use std::process::exit;

/// Triggers system poweroff via libc reboot.
pub fn poweroff() -> ! {
    unsafe {
        libc::reboot(libc::RB_POWER_OFF);
    }
    exit(0);
}

/// Triggers system reboot via libc reboot.
pub fn reboot() -> ! {
    unsafe {
        libc::reboot(libc::RB_AUTOBOOT);
    }
    exit(0);
}
```

- [ ] **Step 2: Verify `src/sys.rs` compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit Task 1**

```bash
git add src/sys.rs
git commit -m "refactor(sys): add poweroff and reboot helper functions"
```

---

### Task 2: Add resolution helpers and `handle_login` in `src/main.rs`

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `users::HumanUser`, `session::Session`, `config::Config`, `launch_state`, `auth`, `exec`, `uzers`
- Produces: `struct UserSelection`, `struct SessionSelection`, `fn resolve_user`, `fn resolve_session`, `fn handle_login`

- [ ] **Step 1: Define `UserSelection` and `SessionSelection` structs and resolution helpers in `src/main.rs`**

Add helper structs and functions to `src/main.rs`:

```rust
#[derive(Debug, Clone)]
pub struct UserSelection {
    pub username: String,
    pub shell: String,
}

#[derive(Debug, Clone)]
pub struct SessionSelection {
    pub name: String,
    pub exec_args: Vec<String>,
    pub is_xorg: bool,
    pub desktop_names: Option<Vec<String>>,
}

pub fn resolve_user(users: &[users::HumanUser], user_idx: usize, custom_user: String) -> UserSelection {
    if user_idx < users.len() && custom_user.is_empty() {
        UserSelection {
            username: users[user_idx].username.clone(),
            shell: users[user_idx].shell.clone(),
        }
    } else {
        UserSelection {
            username: custom_user,
            shell: "/bin/bash".to_string(),
        }
    }
}

pub fn resolve_session(
    sessions: &[session::Session],
    session_idx: usize,
    custom_session: String,
    shell: &str,
) -> SessionSelection {
    if session_idx < sessions.len() && custom_session.is_empty() {
        let s = &sessions[session_idx];
        let args = match &s.exec {
            session::ExecType::Shell(sh) => vec![sh.clone()],
            session::ExecType::Desktop(args) => args.clone(),
        };
        SessionSelection {
            name: s.name.clone(),
            exec_args: args,
            is_xorg: s.session_type == SessionType::Xorg,
            desktop_names: s.desktop_names.clone(),
        }
    } else if session_idx == sessions.len() && custom_session.is_empty() {
        SessionSelection {
            name: shell.to_string(),
            exec_args: vec![shell.to_string()],
            is_xorg: false,
            desktop_names: None,
        }
    } else {
        SessionSelection {
            name: custom_session.clone(),
            exec_args: vec![custom_session],
            is_xorg: false,
            desktop_names: None,
        }
    }
}
```

- [ ] **Step 2: Add `handle_login` function with guard clauses in `src/main.rs`**

```rust
pub fn handle_login(
    user_idx: usize,
    session_idx: usize,
    password: String,
    custom_session: String,
    custom_user: String,
    config: &config::Config,
    sessions: &[session::Session],
    users: &[users::HumanUser],
    vt: Option<c_int>,
    bypass_shell_login: bool,
) -> Result<(), auth::AuthError> {
    let user_sel = resolve_user(users, user_idx, custom_user);
    let session_sel = resolve_session(sessions, session_idx, custom_session, &user_sel.shell);

    let _ = launch_state::write_launch_state(&launch_state::LaunchState {
        username: user_sel.username.clone(),
        session_opt: session_sel.name.clone(),
    });

    let mut auth_session = auth::authenticate(&user_sel.username, &password, &config.auth.pam_service)?;

    let Some(u) = uzers::get_user_by_name(&user_sel.username) else {
        eprintln!("User not found in system: {}", user_sel.username);
        auth_session.close();
        return Ok(());
    };

    let home_dir = u.home_dir().to_string_lossy().into_owned();
    let uid = u.uid();
    let gid = u.primary_group_id();
    let session_type_str = if session_sel.is_xorg { "x11" } else { "wayland" };

    let env = exec::assemble_environment(
        &auth_session.env,
        &user_sel.username,
        &home_dir,
        &user_sel.shell,
        session_type_str,
        None,
        session_sel.desktop_names.as_deref(),
        &config.behavior.source,
        &config.behavior.user_source,
    );

    if let Err(e) = exec::launch_session(
        &user_sel.username,
        uid,
        gid,
        &env,
        &session_sel.exec_args,
        session_sel.is_xorg,
        vt,
        &user_sel.shell,
        bypass_shell_login,
    ) {
        eprintln!("Failed to launch session: {}", e);
    }

    auth_session.close();
    Ok(())
}
```

- [ ] **Step 3: Verify code compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit Task 2**

```bash
git add src/main.rs
git commit -m "refactor(main): add user/session resolution and handle_login helper"
```

---

### Task 3: Refactor `main()` loop and verify end-to-end build

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `sys::poweroff`, `sys::reboot`, `handle_login`
- Produces: Clean, readable `fn main()`

- [ ] **Step 1: Refactor `fn main()` loop in `src/main.rs`**

Replace `main()` loop in `src/main.rs`:

```rust
        match ui.run() {
            Ok(UIResult::Login(session_idx, user_idx, password, custom_session, custom_user)) => {
                match handle_login(
                    session_idx,
                    user_idx,
                    password,
                    custom_session,
                    custom_user,
                    &config,
                    &sessions,
                    &users,
                    args.vt,
                    bypass_shell_login,
                ) {
                    Ok(()) => {
                        pam_messages.clear();
                        auth_failed = false;
                    }
                    Err(auth_err) => {
                        eprintln!("Auth failed: {}", auth_err);
                        auth_failed = true;
                        pam_messages = auth_err.pam_messages;
                        if pam_messages.is_empty() {
                            pam_messages.push(auth::PamMessage {
                                msg_type: auth::PamMessageType::Error,
                                message: auth_err.message.clone(),
                            });
                        }
                    }
                }
            }
            Ok(UIResult::Poweroff) => sys::poweroff(),
            Ok(UIResult::Reboot) => sys::reboot(),
            Ok(UIResult::Refresh) => continue,
            Ok(UIResult::Exit) => break,
            Err(e) => {
                eprintln!("UI Error: {}", e);
                break;
            }
        }
```

- [ ] **Step 2: Run `cargo check` and `cargo test`**

Run: `cargo check && cargo test`
Expected: PASS with 0 errors/warnings

- [ ] **Step 3: Commit Task 3**

```bash
git add src/main.rs
git commit -m "refactor(main): simplify main loop with clean match and sys wrappers"
```
