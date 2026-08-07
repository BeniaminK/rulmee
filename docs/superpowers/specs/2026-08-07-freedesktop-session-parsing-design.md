# Design Specification: Freedesktop Desktop Entry Session Discovery & Execution in LiDM

**Date:** 2026-08-07  
**Status:** Approved  

---

## 1. Overview & Goals

This specification details the design for integrating specification-compliant Freedesktop `.desktop` session discovery, filtering, `Exec` line tokenization, and `XDG_CURRENT_DESKTOP` environment exporting into the Rust implementation of **LiDM**.

The primary goals are:
- Full parity with standard Linux display managers (GDM, SDDM, ReGreet, LightDM).
- Automatic filtering of uninstalled or hidden desktop sessions (`TryExec`, `NoDisplay`, `Hidden`).
- Accurate Freedesktop `Exec` string tokenization (handling quotes, escaped spaces, and stripping `%` field codes).
- Exporting `XDG_CURRENT_DESKTOP` for desktop environment portal integration.

---

## 2. Architecture & Module Modifications

### 2.1 Dependencies (`Cargo.toml`)
Add `freedesktop` (`0.0.3`) to `Cargo.toml`:
```toml
[dependencies]
freedesktop = "0.0.3"
```

---

### 2.2 Session Data Model & Discovery (`src/session.rs`)

1. **`Session` Struct Modification:**
   ```rust
   #[derive(Debug, Clone)]
   pub struct Session {
       pub name: String,
       pub exec: ExecType,
       pub session_type: SessionType,
       pub desktop_names: Option<String>,
   }
   ```

2. **`get_available_sessions()` Workflow:**
   - Scan standard XDG session directories (`/usr/share/xsessions`, `/usr/local/share/xsessions`, `/usr/share/wayland-sessions`, `/usr/local/share/wayland-sessions`).
   - For each `.desktop` file, invoke `freedesktop::ApplicationEntry::try_from_path(&fpath)`.
   - Call `app.should_show()` to automatically check:
     - `NoDisplay=true` / `Hidden=true` flags.
     - `TryExec` binary availability in `$PATH`.
     - `OnlyShowIn` / `NotShowIn` restrictions.
   - If `should_show()` returns `true`, extract `exec` string via `app.exec()`.
   - Tokenize `app.exec()` via `parse_exec_string(exec)` to handle quotes, backslash escapes, and strip `%` field specifiers (`%u`, `%f`, `%i`, `%c`, `%k`, `%%`).
   - Extract `desktop_names` from entry attributes.

---

### 2.3 Environment Assembly & Launch Integration (`src/exec.rs` & `src/main.rs`)

1. **`exec::assemble_environment()` Signature & Logic:**
   Update `assemble_environment` to accept `desktop_names: Option<&str>`:
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
       // ... existing environment population ...
       if let Some(names) = desktop_names {
           env.insert("XDG_CURRENT_DESKTOP".to_string(), names.to_string());
       }
       env
   }
   ```

2. **`main.rs` Login Workflow:**
   Pass the selected session's `desktop_names` into `exec::assemble_environment()` prior to calling `exec::launch_session()`.

---

## 3. Verification & Testing Strategy

1. **Unit Tests in `src/session.rs`:**
   - Test `parse_exec_string` against quoted arguments, escaped spaces, and `%` field specifier stripping.
2. **Unit Tests in `src/exec.rs`:**
   - Verify `XDG_CURRENT_DESKTOP` insertion when `desktop_names` is provided.
3. **Integration Test / Build Check:**
   - Run `cargo test` and `cargo check` to verify full compilation and test suite passing.
