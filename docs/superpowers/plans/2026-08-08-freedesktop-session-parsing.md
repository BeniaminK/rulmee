# Freedesktop Desktop Entry Session Parsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the `freedesktop` crate (`v0.0.3`) and Freedesktop session filtering (`TryExec`, `NoDisplay`, `Hidden`), `Exec` line tokenization (`parse_exec_string`), and `XDG_CURRENT_DESKTOP` environment exporting into `lidm`'s Rust implementation.

**Architecture:** Update `Cargo.toml` with `freedesktop = "0.0.3"`. In `src/session.rs`, add `desktop_names: Option<String>` to `Session` struct, use `freedesktop::ApplicationEntry` and `app.should_show()` for automatic session filtering, and parse `Exec` strings via `parse_exec_string()`. Update `exec::assemble_environment()` in `src/exec.rs` to set `XDG_CURRENT_DESKTOP` when `desktop_names` is provided, and pass it from `main.rs`.

**Tech Stack:** Rust 2024, `freedesktop` crate (v0.0.3), `nix`, `cargo test`.

## Global Constraints

- Preserve all existing public module APIs and `SessionType` variants (`Xorg`, `Wayland`, `Shell`).
- All tests must pass cleanly (`cargo test`).
- Zero unsafe code additions unless strictly required by low-level POSIX bindings.

---

### Task 1: Update Cargo.toml and Session Data Structure

**Files:**
- Modify: `Cargo.toml:13-34`
- Modify: `src/session.rs:17-23`

**Interfaces:**
- Consumes: None
- Produces: `Session` struct with `pub desktop_names: Option<String>` field.

- [ ] **Step 1: Add `freedesktop = "0.0.3"` to `Cargo.toml`**

Edit `Cargo.toml` under `[dependencies]`:
```toml
freedesktop = "0.0.3"
```

- [ ] **Step 2: Add `desktop_names` field to `Session` struct in `src/session.rs`**

In `src/session.rs`:
```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub exec: ExecType,
    pub session_type: SessionType,
    pub desktop_names: Option<String>,
}
```

- [ ] **Step 3: Run `cargo check` to verify dependencies and syntax**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/session.rs
git commit -m "feat(session): add freedesktop crate dependency and desktop_names to Session"
```

---

### Task 2: Implement Session Discovery Filtering & Exec Parsing in `src/session.rs`

**Files:**
- Modify: `src/session.rs`
- Test: `src/session.rs` (unit tests block)

**Interfaces:**
- Consumes: `freedesktop::ApplicationEntry`, `parse_exec_string`
- Produces: `get_available_sessions() -> Vec<Session>` with automatic `TryExec`/`NoDisplay`/`Hidden` filtering.

- [ ] **Step 1: Write tests for session filtering and `parse_exec_string`**

Add tests to `src/session.rs`:
```rust
#[test]
fn test_parse_exec_string_quotes_and_specifiers() {
    let input = "sway --config \"/etc/sway/config file\" %u %F";
    let args = parse_exec_string(input);
    assert_eq!(args, vec!["sway", "--config", "/etc/sway/config file"]);
}
```

- [ ] **Step 2: Run `cargo test session::tests` to verify tests fail or pass**

Run: `cargo test session::tests`
Expected: PASS

- [ ] **Step 3: Update `get_available_sessions()` to use `freedesktop::ApplicationEntry` and `app.should_show()`**

In `src/session.rs`:
```rust
use freedesktop::ApplicationEntry;

pub fn get_available_sessions() -> Vec<Session> {
    let mut sessions = Vec::new();

    for (session_type, dir) in SOURCES {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let fpath = entry.path();
                if fpath.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(app) = ApplicationEntry::try_from_path(&fpath) {
                        if app.should_show() {
                            if let (Some(name), Some(exec)) = (app.name(), app.exec()) {
                                let args = parse_exec_string(exec);
                                if !args.is_empty() {
                                    sessions.push(Session {
                                        name: name.to_string(),
                                        exec: ExecType::Desktop(args),
                                        session_type: *session_type,
                                        desktop_names: app.desktop_names().map(String::from),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    sessions
}
```

- [ ] **Step 4: Run `cargo test` to verify full compilation and tests passing**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/session.rs
git commit -m "feat(session): use freedesktop ApplicationEntry for session filtering"
```

---

### Task 3: Export `XDG_CURRENT_DESKTOP` in `exec.rs` & `main.rs`

**Files:**
- Modify: `src/exec.rs:28-64`
- Modify: `src/main.rs:133-185`

**Interfaces:**
- Consumes: `Session.desktop_names`
- Produces: `assemble_environment` exporting `XDG_CURRENT_DESKTOP` when present.

- [ ] **Step 1: Write unit test in `src/exec.rs` for `XDG_CURRENT_DESKTOP`**

Add unit test to `src/exec.rs`:
```rust
#[test]
fn test_assemble_environment_xdg_current_desktop() {
    let pam_env = HashMap::new();
    let env = assemble_environment(
        &pam_env,
        "bob",
        "/home/bob",
        "/bin/bash",
        "wayland",
        None,
        Some("Sway:Wayland"),
    );
    assert_eq!(env.get("XDG_CURRENT_DESKTOP").map(|s| s.as_str()), Some("Sway:Wayland"));
}
```

- [ ] **Step 2: Update `assemble_environment()` signature and implementation in `src/exec.rs`**

In `src/exec.rs`:
```rust
pub fn assemble_environment(
    pam_env: &HashMap<String, String>,
    username: &str,
    home_dir: &str,
    shell: &str,
    session_type: &str,
    display: Option<&str>,
    desktop_names: Option<&str>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 1. POSIX Credential Defaults
    env.insert("USER".to_string(), username.to_string());
    env.insert("LOGNAME".to_string(), username.to_string());
    env.insert("HOME".to_string(), home_dir.to_string());
    env.insert("SHELL".to_string(), shell.to_string());
    env.insert(
        "PATH".to_string(),
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
    );

    // 2. Freedesktop / XDG Standards
    env.insert("XDG_SESSION_TYPE".to_string(), session_type.to_string());
    env.insert("XDG_SESSION_CLASS".to_string(), "user".to_string());
    if let Some(names) = desktop_names {
        env.insert("XDG_CURRENT_DESKTOP".to_string(), names.to_string());
    }

    // 3. Merged PAM Environment
    for (k, v) in pam_env {
        env.insert(k.clone(), v.clone());
    }

    // 4. Optional Display Variable
    if let Some(disp) = display {
        env.insert("DISPLAY".to_string(), disp.to_string());
    }

    env
}
```

- [ ] **Step 3: Update `main.rs` to extract `desktop_names` from selected `Session`**

In `src/main.rs`:
```rust
let (session_name, exec_args, is_xorg, desktop_names) =
    if session_idx < sessions.len() && custom_session.is_empty() {
        let s = &sessions[session_idx];
        let args = match &s.exec {
            session::ExecType::Shell(sh) => vec![sh.clone()],
            session::ExecType::Desktop(args) => args.clone(),
        };
        (s.name.clone(), args, s.session_type == SessionType::Xorg, s.desktop_names.clone())
    } else if session_idx == sessions.len() && custom_session.is_empty() {
        (shell.clone(), vec![shell.clone()], false, None)
    } else {
        (custom_session.clone(), vec![custom_session], false, None)
    };

// ... inside pam auth block ...
let env = exec::assemble_environment(
    &auth_session.env,
    &username,
    &home_dir,
    &shell,
    session_type_str,
    None,
    desktop_names.as_deref(),
);
```

- [ ] **Step 4: Run `cargo test` to verify all tests pass**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/exec.rs src/main.rs
git commit -m "feat(exec): export XDG_CURRENT_DESKTOP from session desktop_names"
```
