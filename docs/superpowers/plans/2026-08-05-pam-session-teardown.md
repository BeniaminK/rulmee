# PAM Session Teardown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement proper PAM Session Teardown & Lifecycle Management in Rust to close PAM sessions (`pam_close_session`), release credentials (`pam_setcred(DELETE)`), and clean up `systemd-logind` / UTMP records upon user session exit.

**Architecture:** Encapsulate `pam_client2::Context` and `SessionToken` within the `AuthSession` struct using an RAII pattern in `src/auth.rs`. `main.rs` holds ownership of `AuthSession` across `exec::launch_session`, calling `auth_session.close()` (or relying on `Drop`) when the user session process terminates after `waitpid`.

**Tech Stack:** Rust (2024 edition), `pam-client2` crate, `nix`, `libc`.

## Global Constraints

- Preserve all existing public signatures unless explicitly modified in spec.
- No raw C-FFI bypasses when `pam-client2` abstractions exist (`unleak_session` & `close`).
- All unit tests must pass (`cargo test`).

---

### Task 1: Refactor `AuthSession` with RAII `close()` & `Drop` in `src/auth.rs`

**Files:**
- Modify: `src/auth.rs:5-55`
- Test: `src/auth.rs`

**Interfaces:**
- Consumes: `pam_client2::{Context, Flag, SessionToken}`, `pam_client2::conv_mock::Conversation`
- Produces: `AuthSession { username: String, env: HashMap<String, String>, context: Option<Context<Conversation>>, token: Option<SessionToken> }`, `AuthSession::close(&mut self)`

- [ ] **Step 1: Write the failing unit test for `AuthSession` lifetime and teardown in `src/auth.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_session_teardown_safely_handles_none() {
        let mut session = AuthSession {
            username: "testuser".to_string(),
            env: HashMap::new(),
            context: None,
            token: None,
        };
        session.close();
        assert!(session.context.is_none());
        assert!(session.token.is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib auth::tests::test_auth_session_teardown_safely_handles_none`
Expected: FAIL due to missing fields `context` and `token` on `AuthSession`.

- [ ] **Step 3: Update `AuthSession` definition, `close()`, `Drop`, and `authenticate()` in `src/auth.rs`**

```rust
use std::collections::HashMap;
use pam_client2::{Context, Flag, SessionToken};
use pam_client2::conv_mock::Conversation;

pub struct AuthSession {
    pub username: String,
    pub env: HashMap<String, String>,
    context: Option<Context<Conversation>>,
    token: Option<SessionToken>,
}

impl AuthSession {
    pub fn close(&mut self) {
        if let (Some(mut context), Some(token)) = (self.context.take(), self.token.take()) {
            let session = context.unleak_session(token);
            let _ = session.close(Flag::NONE);
        }
    }
}

impl Drop for AuthSession {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn authenticate(user: &str, password: &str, service: &str) -> Result<AuthSession, String> {
    let mut context = Context::new(
        service,
        Some(user),
        Conversation::with_credentials(user, password),
    ).map_err(|e| format!("pam_start failed: {}", e))?;

    context.authenticate(Flag::NONE)
        .map_err(|e| format!("pam_authenticate failed: {}", e))?;

    context.acct_mgmt(Flag::NONE)
        .map_err(|e| format!("pam_acct_mgmt failed: {}", e))?;

    let mut session = context.open_session(Flag::NONE)
        .map_err(|e| format!("pam_open_session failed: {}", e))?;

    session.reinitialize_credentials(Flag::NONE)
        .map_err(|e| format!("pam_setcred(REINITIALIZE) failed: {}", e))?;

    let token = session.leak();

    let mut env = HashMap::new();
    for (key, value) in context.envlist().iter_tuples() {
        env.insert(
            key.to_string_lossy().into_owned(),
            value.to_string_lossy().into_owned(),
        );
    }

    Ok(AuthSession {
        username: user.to_string(),
        env,
        context: Some(context),
        token: Some(token),
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: PASS (all tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/auth.rs
git commit -m "feat: implement RAII PAM session teardown in AuthSession"
```

---

### Task 2: Wire `AuthSession` Teardown into `src/main.rs`

**Files:**
- Modify: `src/main.rs:150-188`

**Interfaces:**
- Consumes: `auth::authenticate()`, `AuthSession::close()`
- Produces: Clean post-session PAM teardown in main event loop

- [ ] **Step 1: Update login handling in `src/main.rs`**

Update `match auth::authenticate(...)` in `src/main.rs`:

```rust
                match auth::authenticate(&username, &password, &pam_service) {
                    Ok(mut auth_session) => {
                        if let Some(u) = uzers::get_user_by_name(&username) {
                            let home_dir = u.home_dir().to_string_lossy().into_owned();
                            let uid = u.uid();
                            let gid = u.primary_group_id();
                            let session_type_str = if is_xorg { "x11" } else { "wayland" };

                            let env = exec::assemble_environment(
                                &auth_session.env,
                                &username,
                                &home_dir,
                                &shell,
                                session_type_str,
                                None,
                            );

                            if let Err(e) = exec::launch_session(
                                &username,
                                uid,
                                gid,
                                &env,
                                &exec_args,
                                is_xorg,
                                args.vt,
                                &shell,
                                bypass_shell_login,
                            ) {
                                eprintln!("Failed to launch session: {}", e);
                            }

                            // Teardown PAM session cleanly after session process completes
                            auth_session.close();
                        } else {
                            eprintln!("User not found in system: {}", username);
                        }
                    }
                    Err(e) => {
                        eprintln!("Auth failed: {}", e);
                        // Continue loop back to UI
                    }
                }
```

- [ ] **Step 2: Run compiler check and unit tests**

Run: `cargo check && cargo test`
Expected: PASS with zero warnings or errors

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: trigger PAM session teardown after session termination in main loop"
```

---

### Task 3: Update `TODO.md` Roadmap & Documentation

**Files:**
- Modify: `TODO.md:161,185-188,206-208`

**Interfaces:**
- Updates: Reflects completion of PAM Session Teardown task `[x]` in `TODO.md`

- [ ] **Step 1: Update `TODO.md`**

In Section 3 (Comparison Summary Matrix):
Change PAM Session Teardown row to:
`| **PAM Session Teardown (`pam_close_session`)** | Supported (cleans PAM creds and closes session after `waitpid`) | `AuthSession` closes session & deletes creds via `close()`/`Drop` | ✅ **Implemented in Rust** |`

In Section 4.1 (Detailed Missing Features):
Update item 3:
`3. **PAM Session Closure & Teardown:**`
`    * Implemented in Rust via `AuthSession` RAII lifecycle management. When `exec::launch_session` finishes waiting for the child process (`waitpid`), `auth_session.close()` invokes `context.unleak_session(token)` and `session.close()`, calling `pam_close_session()` and `pam_setcred(PAM_DELETE_CRED)` to deregister the logind session.`

In Section 5 (Actionable Roadmap):
Change item 2 to:
`- [x] **[P0] PAM Session Teardown**`

- [ ] **Step 2: Run `cargo test` to verify build integrity**

Run: `cargo test`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add TODO.md
git commit -m "docs: mark PAM Session Teardown complete in TODO.md"
```
